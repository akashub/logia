use crate::capture::{Capture, CHUNK};
use crate::messages::MAX_SECONDS;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub enum AudioSource {
    Microphone(Capture),
    File {
        chunks: std::vec::IntoIter<Vec<f32>>,
        rate: u32,
    },
}
impl AudioSource {
    pub fn open(file: Option<&Path>, stop: Arc<AtomicBool>) -> Result<Self, String> {
        let Some(path) = file else {
            return Capture::start(stop).map(Self::Microphone);
        };
        let mut reader = hound::WavReader::open(path).map_err(|_| "Could not open test WAV")?;
        let spec = reader.spec();
        if spec.bits_per_sample != 16
            || spec.sample_format != hound::SampleFormat::Int
            || !(1..=2).contains(&spec.channels)
            || !(8000..=192000).contains(&spec.sample_rate)
            || reader.duration() as u64 > spec.sample_rate as u64 * MAX_SECONDS
        {
            return Err("Test WAV must be 16-bit PCM, mono/stereo, and at most 60 seconds".into());
        }
        let samples = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Invalid WAV samples")?;
        let mono: Vec<f32> = samples
            .chunks_exact(spec.channels as usize)
            .map(|frame| {
                frame.iter().map(|&v| v as f32 / 32768.0).sum::<f32>() / spec.channels as f32
            })
            .collect();
        let chunks = mono
            .chunks(CHUNK)
            .map(|chunk| chunk.to_vec())
            .collect::<Vec<_>>()
            .into_iter();
        Ok(Self::File {
            chunks,
            rate: spec.sample_rate,
        })
    }
    pub fn rate(&self) -> u32 {
        match self {
            Self::Microphone(mic) => mic.rate,
            Self::File { rate, .. } => *rate,
        }
    }
    pub fn next(&mut self) -> Result<Option<Vec<f32>>, String> {
        match self {
            Self::Microphone(mic) => receive(&mic.failed, || {
                mic.receiver.recv_timeout(Duration::from_millis(50))
            }),
            Self::File { chunks, .. } => Ok(chunks.next()),
        }
    }
    pub fn stop(&mut self) {
        if let Self::Microphone(mic) = self {
            mic.stop();
        }
    }
}

fn receive(
    failed: &AtomicBool,
    wait: impl FnOnce() -> Result<Vec<f32>, std::sync::mpsc::RecvTimeoutError>,
) -> Result<Option<Vec<f32>>, String> {
    const FAILURE: &str =
        "Microphone disconnected or audio processing fell behind. Your partial text is available.";
    if failed.load(Ordering::Acquire) {
        return Err(FAILURE.into());
    }
    let received = wait();
    // A callback can fail while receive is blocked, then close the channel as
    // capture shuts down. That is an interrupted session, never clean EOF.
    if failed.load(Ordering::Acquire) {
        return Err(FAILURE.into());
    }
    match received {
        Ok(chunk) => Ok(Some(chunk)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(Some(Vec::new())),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::RecvTimeoutError::{Disconnected, Timeout};

    #[test]
    fn failed_capture_cannot_return_clean_eof_or_audio_after_waiting() {
        for result in [Err(Disconnected), Err(Timeout), Ok(vec![0.1])] {
            let failed = AtomicBool::new(false);
            assert!(receive(&failed, || {
                failed.store(true, Ordering::Release); // Callback fails while receive is blocked.
                result
            })
            .is_err());
        }
        assert!(receive(&AtomicBool::new(true), || panic!("must not wait")).is_err());
    }

    #[test]
    fn healthy_capture_distinguishes_audio_timeout_and_clean_stop() {
        let healthy = AtomicBool::new(false);
        assert_eq!(
            receive(&healthy, || Ok(vec![0.1])).unwrap(),
            Some(vec![0.1])
        );
        assert_eq!(receive(&healthy, || Err(Timeout)).unwrap(), Some(vec![]));
        assert_eq!(receive(&healthy, || Err(Disconnected)).unwrap(), None);
    }
}
