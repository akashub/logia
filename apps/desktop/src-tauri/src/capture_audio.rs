use crate::capture::CHUNK;
use cpal::{FromSample, Sample};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::SyncSender,
    Arc,
};

/// Coalesce small device callbacks so queue capacity measures full audio chunks.
/// Dropping the callback flushes its pre-Stop tail before disconnecting the pipe.
pub struct CaptureAudio {
    pending: Vec<f32>,
    sender: SyncSender<Vec<f32>>,
    failed: Arc<AtomicBool>,
}
impl CaptureAudio {
    pub fn new(sender: SyncSender<Vec<f32>>, failed: Arc<AtomicBool>) -> Self {
        Self {
            pending: Vec::with_capacity(CHUNK),
            sender,
            failed,
        }
    }
    pub fn push<T: Sample>(&mut self, input: &[T], channels: usize, stop: &AtomicBool)
    where
        f32: FromSample<T>,
    {
        if self.failed.load(Ordering::Acquire) {
            return;
        }
        for frame in input.chunks_exact(channels) {
            if stop.load(Ordering::Acquire) {
                return;
            }
            let mono = frame
                .iter()
                .map(|sample| sample.to_sample::<f32>())
                .sum::<f32>()
                / channels as f32;
            if !mono.is_finite() {
                self.failed.store(true, Ordering::Release);
                return;
            }
            self.pending.push(mono);
            if self.pending.len() == CHUNK && !self.send() {
                return;
            }
        }
    }
    fn send(&mut self) -> bool {
        let audio = std::mem::replace(&mut self.pending, Vec::with_capacity(CHUNK));
        if self.sender.try_send(audio).is_err() {
            self.failed.store(true, Ordering::Release);
            return false;
        }
        true
    }
}
impl Drop for CaptureAudio {
    fn drop(&mut self) {
        if !self.pending.is_empty() && !self.failed.load(Ordering::Acquire) {
            self.send();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::sync_channel;
    #[test]
    fn tiny_callbacks_use_full_queue_slots_and_stop_flushes_only_prior_audio() {
        let (sender, receiver) = sync_channel(2);
        let failed = Arc::new(AtomicBool::new(false));
        let stop = AtomicBool::new(false);
        let mut audio = CaptureAudio::new(sender, failed.clone());
        for _ in 0..9 {
            audio.push(&[0.25f32; 128], 1, &stop);
        }
        stop.store(true, Ordering::Release);
        audio.push(&[0.75f32; 128], 1, &stop);
        drop(audio);
        assert_eq!(receiver.recv().unwrap(), vec![0.25; CHUNK]);
        assert_eq!(receiver.recv().unwrap(), vec![0.25; 128]);
        assert!(receiver.recv().is_err());
        assert!(!failed.load(Ordering::Acquire));
    }
    #[test]
    fn overflow_is_reported_without_blocking_or_growing_the_queue() {
        let (sender, receiver) = sync_channel(1);
        let failed = Arc::new(AtomicBool::new(false));
        let mut audio = CaptureAudio::new(sender, failed.clone());
        audio.push(&[0.1f32; CHUNK * 3], 1, &AtomicBool::new(false));
        assert!(failed.load(Ordering::Acquire));
        assert_eq!(receiver.try_recv().unwrap().len(), CHUNK);
        assert!(receiver.try_recv().is_err());
    }
}
