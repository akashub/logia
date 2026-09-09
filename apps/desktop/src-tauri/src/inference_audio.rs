/// 64 ms at the recognizer's fixed 16 kHz rate, independent of microphone rate.
/// Small resampler outputs otherwise cause many tiny native inference calls.
pub const INFERENCE_SAMPLES: usize = 1024;

pub struct InferenceAudio {
    samples: [f32; INFERENCE_SAMPLES],
    used: usize,
}
impl Default for InferenceAudio {
    fn default() -> Self {
        Self {
            samples: [0.; INFERENCE_SAMPLES],
            used: 0,
        }
    }
}
impl InferenceAudio {
    pub fn push(
        &mut self,
        mut input: &[f32],
        mut feed: impl FnMut(&[f32]) -> Result<(), String>,
    ) -> Result<(), String> {
        while !input.is_empty() {
            let count = input.len().min(INFERENCE_SAMPLES - self.used);
            self.samples[self.used..self.used + count].copy_from_slice(&input[..count]);
            self.used += count;
            input = &input[count..];
            if self.used == INFERENCE_SAMPLES {
                feed(&self.samples)?;
                self.used = 0;
            }
        }
        Ok(())
    }
    pub fn finish(
        &mut self,
        mut feed: impl FnMut(&[f32]) -> Result<(), String>,
    ) -> Result<(), String> {
        if self.used > 0 {
            feed(&self.samples[..self.used])?;
            self.used = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_sizes_do_not_change_inference_cadence_or_drop_samples() {
        let input: Vec<f32> = (0..4103).map(|n| n as f32).collect();
        for size in [1, 127, 341, 1024, 2048] {
            let mut batch = InferenceAudio::default();
            let mut output = Vec::new();
            for chunk in input.chunks(size) {
                batch
                    .push(chunk, |audio| {
                        assert_eq!(audio.len(), INFERENCE_SAMPLES);
                        output.extend_from_slice(audio);
                        Ok(())
                    })
                    .unwrap();
            }
            assert_eq!(output.len(), 4096);
            batch
                .finish(|audio| {
                    assert_eq!(audio.len(), 7);
                    output.extend_from_slice(audio);
                    Ok(())
                })
                .unwrap();
            assert_eq!(output, input);
            batch
                .finish(|_| panic!("tail must not be delivered twice"))
                .unwrap();
        }
    }
    #[test]
    fn empty_or_exact_boundary_finish_adds_no_audio() {
        let mut batch = InferenceAudio::default();
        batch
            .finish(|_| panic!("no empty inference calls"))
            .unwrap();
        batch.push(&[0.; 1024], |_| Ok(())).unwrap();
        batch
            .finish(|_| panic!("no duplicated final frame"))
            .unwrap();
    }
    #[test]
    fn inference_error_propagates_without_accepting_more_audio() {
        let mut batch = InferenceAudio::default();
        let mut calls = 0;
        assert!(batch
            .push(&[0.; 2048], |_| {
                calls += 1;
                Err("failed".into())
            })
            .is_err());
        assert_eq!(calls, 1);
    }
}
