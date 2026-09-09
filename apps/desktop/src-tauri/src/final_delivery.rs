use crate::messages::WorkerEvent;

// A final alone is not a successful session: require clean worker exit as well.
#[derive(Default)]
pub struct FinalDelivery {
    text: Option<String>,
    failed: bool,
}
impl FinalDelivery {
    pub fn observe(&mut self, event: &WorkerEvent) -> bool {
        match event {
            WorkerEvent::Final { text } if self.text.is_none() => self.text = Some(text.clone()),
            WorkerEvent::Error { .. } => self.failed = true,
            WorkerEvent::Stopped | WorkerEvent::Final { .. } => {
                self.failed = true;
                return false;
            }
            _ if self.text.is_some() => {
                self.failed = true;
                return false;
            }
            _ => (),
        }
        true
    }
    pub fn take(self, clean_exit: bool) -> Option<String> {
        if clean_exit && !self.failed {
            self.text
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn final_event() -> WorkerEvent {
        WorkerEvent::Final {
            text: "Synthetic paragraph".into(),
        }
    }
    #[test]
    fn requires_one_final_and_a_clean_exit() {
        assert!(FinalDelivery::default().take(true).is_none());
        let mut good = FinalDelivery::default();
        assert!(good.observe(&final_event()));
        assert_eq!(good.take(true).as_deref(), Some("Synthetic paragraph"));
        let mut failed_exit = FinalDelivery::default();
        failed_exit.observe(&final_event());
        assert!(failed_exit.take(false).is_none());
        for late in [
            final_event(),
            WorkerEvent::Stopped,
            WorkerEvent::Partial {
                text: "late".into(),
            },
            WorkerEvent::Error {
                message: "failure".into(),
            },
        ] {
            let mut attempt = FinalDelivery::default();
            attempt.observe(&final_event());
            attempt.observe(&late);
            assert!(attempt.take(true).is_none());
        }
    }
}
