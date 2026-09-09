use crate::{
    audio_source::AudioSource,
    capture::CHUNK,
    inference_audio::InferenceAudio,
    messages::{emit, WorkerEvent, MAX_SECONDS},
    model_file,
};
use rubato::{FftFixedIn, Resampler};
use std::{
    io::{self, BufRead},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use transcribe_cpp::{
    CancelToken, CommitPolicy, Model, ModelOptions, MoonshineStreamingOptions, RunOptions,
    StreamExtension, StreamOptions,
};

pub fn run(model_path: &Path, audio: Option<&Path>) -> Result<(), String> {
    emit(&WorkerEvent::Loading)?;
    model_file::verify(model_path)?;
    let finish = Arc::new(AtomicBool::new(false));
    let cancel = CancelToken::new();
    if audio.is_none() {
        let finishing = finish.clone();
        let cancellation = cancel.clone();
        std::thread::spawn(move || {
            // Control is independent of microphone and inference work.
            for line in io::stdin().lock().lines().take(2) {
                match line.as_deref() {
                    Ok("finish") => {
                        finishing.store(true, Ordering::Release);
                    }
                    _ => {
                        cancellation.cancel();
                        return;
                    }
                }
            }
            cancellation.cancel();
            // Even an unresponsive native kernel must not outlive a lost parent.
            std::process::exit(0);
        });
    }
    let model = Model::load_with(model_path, &ModelOptions::default()).map_err(|error| {
        // File mode is an explicit developer check; never log microphone sessions.
        if audio.is_some() {
            eprintln!("Model load diagnostic: {error}");
        }
        "Could not load the English model on this device"
    })?;
    if !model.capabilities().supports_streaming {
        return Err("The selected model does not support live captions".into());
    }
    let mut session = model
        .session()
        .map_err(|_| "Could not start the recognizer")?;
    session.set_cancel_token(&cancel);
    let options = StreamOptions {
        commit_policy: CommitPolicy::OnFinalize,
        family: Some(StreamExtension::MoonshineStreaming(
            MoonshineStreamingOptions {
                min_decode_interval_ms: Some(480),
            },
        )),
        ..Default::default()
    };
    let run = RunOptions {
        language: Some("en".into()),
        ..Default::default()
    };
    let mut stream = session
        .stream(&run, &options)
        .map_err(|_| "Could not start live recognition")?;
    if finish.load(Ordering::Acquire) || cancel.is_cancelled() {
        return Ok(());
    }
    let mut source = AudioSource::open(audio, finish.clone())?;
    let rate = source.rate() as usize;
    let mut resampler = FftFixedIn::<f32>::new(rate, 16000, CHUNK, 2, 1)
        .map_err(|_| "Could not prepare microphone audio")?;
    emit(&WorkerEvent::Listening)?;
    let started = Instant::now();
    let mut stopped = false;
    let mut pending = Vec::with_capacity(CHUNK * 2);
    let mut inference_audio = InferenceAudio::default();
    let mut previous = String::new();
    let mut samples = 0usize;
    loop {
        if cancel.is_cancelled() {
            source.stop();
            return Ok(());
        }
        if !stopped
            && (finish.load(Ordering::Acquire) || started.elapsed().as_secs() >= MAX_SECONDS)
        {
            source.stop();
            stopped = true;
        }
        let Some(chunk) = source.next()? else {
            break;
        };
        samples += chunk.len();
        if samples > rate * MAX_SECONDS as usize + CHUNK {
            source.stop();
            break;
        }
        pending.extend_from_slice(&chunk);
        while pending.len() >= CHUNK {
            let input: Vec<f32> = pending.drain(..CHUNK).collect();
            let normalized = resampler
                .process(&[input], None)
                .map_err(|_| "Audio conversion failed")?;
            inference_audio.push(&normalized[0], |audio| {
                feed(&mut stream, audio, &mut previous)
            })?;
        }
    }
    source.stop();
    if samples == 0 {
        return emit(&WorkerEvent::Final {
            text: String::new(),
        });
    }
    // Flush the resampler's pending input and filter delay before ASR finalization.
    if !pending.is_empty() {
        let normalized = resampler
            .process_partial(Some(&[pending]), None)
            .map_err(|_| "Audio flush failed")?;
        inference_audio.push(&normalized[0], |audio| {
            feed(&mut stream, audio, &mut previous)
        })?;
    }
    let tail = resampler
        .process_partial::<Vec<f32>>(None, None)
        .map_err(|_| "Audio tail flush failed")?;
    inference_audio.push(&tail[0], |audio| feed(&mut stream, audio, &mut previous))?;
    inference_audio.finish(|audio| feed(&mut stream, audio, &mut previous))?;
    stream
        .finalize()
        .map_err(|_| "Could not finalize speech. Your partial text is available.")?;
    let text = stream.text().full.trim().to_owned();
    drop(stream);
    if cancel.is_cancelled() {
        return Ok(());
    }
    if session.was_truncated() {
        return Err("This passage exceeded the recognizer's limit. Your partial text is available; try a shorter passage.".into());
    }
    emit(&WorkerEvent::Final { text })
}

fn feed(
    stream: &mut transcribe_cpp::Stream<'_>,
    audio: &[f32],
    previous: &mut String,
) -> Result<(), String> {
    let update = stream
        .feed(audio)
        .map_err(|_| "Recognition stopped unexpectedly. Your partial text is available.")?;
    if update.result_changed {
        let text = stream.text().full.trim().to_owned();
        if text != *previous {
            emit(&WorkerEvent::Partial { text: text.clone() })?;
            *previous = text;
        }
    }
    Ok(())
}
