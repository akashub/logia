use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use std::time::{Duration, Instant};

pub const CHUNK: usize = 1024;
pub struct Capture {
    pub receiver: Receiver<Vec<f32>>,
    pub failed: Arc<AtomicBool>,
    pub rate: u32,
    stop: Arc<AtomicBool>,
}
impl Capture {
    pub fn start(stop: Arc<AtomicBool>) -> Result<Self, String> {
        let (sender, receiver) = sync_channel(128); // At most 512 KiB queued mono PCM.
        let (ready, started) = sync_channel(1);
        let failed = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let errors = failed.clone();
        std::thread::spawn(move || {
            let opened = open(sender, errors.clone(), stopping.clone());
            match opened {
                Ok((stream, rate)) => {
                    let _ = ready.send(Ok(rate));
                    let start = Instant::now();
                    while !stopping.load(Ordering::Acquire)
                        && !errors.load(Ordering::Acquire)
                        && start.elapsed().as_secs() < crate::messages::MAX_SECONDS
                    {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    stopping.store(true, Ordering::Release);
                    drop(stream); // Capture shutdown never waits for inference.
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            }
        });
        let rate = started.recv().map_err(|_| "Microphone setup stopped")??;
        Ok(Self {
            receiver,
            failed,
            rate,
            stop,
        })
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn open(
    sender: SyncSender<Vec<f32>>,
    failed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> Result<(cpal::Stream, u32), String> {
    let id = crate::input_devices::worker_input()?;
    let device = crate::input_devices::capture_device(id.as_deref())?;
    let supported = device
        .default_input_config()
        .map_err(|_| "Microphone is unavailable")?;
    let config: cpal::StreamConfig = supported.into();
    if config.channels == 0 {
        return Err("Microphone has no input channels".into());
    }
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            build::<f32>(&device, &config, sender, failed.clone(), stop.clone())
        }
        cpal::SampleFormat::I16 => {
            build::<i16>(&device, &config, sender, failed.clone(), stop.clone())
        }
        cpal::SampleFormat::U16 => {
            build::<u16>(&device, &config, sender, failed.clone(), stop.clone())
        }
        cpal::SampleFormat::I32 => {
            build::<i32>(&device, &config, sender, failed.clone(), stop.clone())
        }
        cpal::SampleFormat::I24 => {
            build::<cpal::I24>(&device, &config, sender, failed.clone(), stop.clone())
        }
        _ => return Err("This microphone format is not supported in the preview".into()),
    }
    .map_err(|_| {
        "Could not open the microphone. Check Logia's Microphone permission in System Settings."
    })?;
    if !stop.load(Ordering::Acquire) {
        stream
            .play()
            .map_err(|_| "Could not start the microphone")?;
    }
    Ok((stream, config.sample_rate))
}

fn build<T: SizedSample + Sample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sender: SyncSender<Vec<f32>>,
    failed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::Error>
where
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let errors = failed.clone();
    let mut audio = crate::capture_audio::CaptureAudio::new(sender, failed);
    device.build_input_stream(
        *config,
        move |input: &[T], _| {
            audio.push(input, channels, &stop);
        },
        move |_| {
            errors.store(true, Ordering::Release);
        },
        None,
    )
}
