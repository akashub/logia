use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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
            let opened = open(sender, errors, stopping.clone());
            match opened {
                Ok((stream, rate)) => {
                    let _ = ready.send(Ok(rate));
                    let start = Instant::now();
                    while !stopping.load(Ordering::Acquire)
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
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No microphone found. Connect one and try again.")?;
    let supported = device
        .default_input_config()
        .map_err(|_| "Microphone is unavailable")?;
    let config: cpal::StreamConfig = supported.clone().into();
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
    Ok((stream, config.sample_rate.0))
}

fn build<T: SizedSample + Sample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sender: SyncSender<Vec<f32>>,
    failed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let errors = failed.clone();
    device.build_input_stream(
        config,
        move |input: &[T], _| {
            enqueue(input, channels, &sender, &failed, &stop);
        },
        move |_| {
            errors.store(true, Ordering::Release);
        },
        None,
    )
}

fn enqueue<T: Sample>(
    input: &[T],
    channels: usize,
    sender: &SyncSender<Vec<f32>>,
    failed: &AtomicBool,
    stop: &AtomicBool,
) where
    f32: FromSample<T>,
{
    if failed.load(Ordering::Relaxed) {
        return;
    }
    for input_chunk in input.chunks(CHUNK * channels) {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let ready: Vec<f32> = input_chunk
            .chunks_exact(channels)
            .map(|frame| {
                frame
                    .iter()
                    .map(|sample| sample.to_sample::<f32>())
                    .sum::<f32>()
                    / channels as f32
            })
            .collect();
        if ready.iter().any(|sample| !sample.is_finite()) || sender.try_send(ready).is_err() {
            failed.store(true, Ordering::Release);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_cuts_off_audio_even_when_inference_is_not_draining() {
        let (sender, receiver) = sync_channel(2);
        let failed = AtomicBool::new(false);
        let stop = AtomicBool::new(false);
        enqueue(&[0.25f32; CHUNK], 1, &sender, &failed, &stop);
        stop.store(true, Ordering::Release);
        enqueue(&[0.75f32; CHUNK], 1, &sender, &failed, &stop);
        assert_eq!(receiver.try_recv().unwrap(), vec![0.25; CHUNK]);
        assert!(receiver.try_recv().is_err());
        assert!(!failed.load(Ordering::Acquire));
    }
    #[test]
    fn callback_overflow_fails_without_blocking_or_growing_queue() {
        let (sender, receiver) = sync_channel(1);
        let failed = AtomicBool::new(false);
        let stop = AtomicBool::new(false);
        enqueue(&[0.1f32; CHUNK * 3], 1, &sender, &failed, &stop);
        assert!(failed.load(Ordering::Acquire));
        assert_eq!(receiver.try_recv().unwrap().len(), CHUNK);
        assert!(receiver.try_recv().is_err());
    }
}
