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
            Self::Microphone(mic) => {
                if mic.failed.load(Ordering::Acquire) {
                    return Err("Microphone disconnected or audio processing fell behind. Your partial text is available.".into());
                }
                match mic.receiver.recv_timeout(Duration::from_millis(50)) {
                    Ok(chunk) => Ok(Some(chunk)),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(Some(Vec::new())),
                    Err(_) => Ok(None),
                }
            }
            Self::File { chunks, .. } => Ok(chunks.next()),
        }
    }
    pub fn stop(&mut self) {
        if let Self::Microphone(mic) = self {
            mic.stop();
        }
    }
}
