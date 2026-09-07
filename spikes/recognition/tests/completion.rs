use logia_recognition_spike::{audio::AudioFrame, completion::*};

fn frame(session: u64, index: u64, last: bool) -> AudioFrame {
    AudioFrame::new(session, index, 16_000, 1, last, vec![0.; 320]).unwrap()
}

#[test]
fn finish_before_audio_waits_for_consumption_of_exact_final_frame() {
    let mut barrier = CompletionBarrier::new(8);
    barrier.finish(8, 2).unwrap();
    assert!(!barrier.ready());
    barrier.consume(&frame(8, 0, false)).unwrap();
    assert!(!barrier.ready());
    barrier.consume(&frame(8, 1, true)).unwrap();
    assert!(barrier.ready());
}

#[test]
fn audio_before_finish_waits_for_matching_declaration() {
    let mut barrier = CompletionBarrier::new(8);
    barrier.consume(&frame(8, 0, false)).unwrap();
    barrier.consume(&frame(8, 1, true)).unwrap();
    assert!(!barrier.ready());
    barrier.finish(8, 2).unwrap();
    assert!(barrier.ready());
}

#[test]
fn finish_between_frames_waits_for_the_remainder() {
    let mut barrier = CompletionBarrier::new(8);
    barrier.consume(&frame(8, 0, false)).unwrap();
    barrier.finish(8, 2).unwrap();
    assert!(!barrier.ready());
    barrier.consume(&frame(8, 1, true)).unwrap();
    assert!(barrier.ready());
}

#[test]
fn empty_audio_requires_explicit_zero_finish_and_rejects_later_audio() {
    let mut barrier = CompletionBarrier::new(8);
    assert!(!barrier.ready());
    barrier.finish(8, 0).unwrap();
    assert!(barrier.ready());
    assert_eq!(
        barrier.consume(&frame(8, 0, true)),
        Err(CompletionError::ExtraFrames)
    );
    assert!(!barrier.ready());
}

#[test]
fn stale_session_does_not_finish_or_consume_active_audio() {
    let mut barrier = CompletionBarrier::new(8);
    assert_eq!(barrier.finish(9, 0), Err(CompletionError::WrongSession));
    assert_eq!(
        barrier.consume(&frame(9, 0, true)),
        Err(CompletionError::WrongSession)
    );
    assert!(!barrier.ready());
    barrier.consume(&frame(8, 0, true)).unwrap();
    barrier.finish(8, 1).unwrap();
    assert!(barrier.ready());
}

#[test]
fn out_of_order_or_repeated_audio_permanently_blocks_finalization() {
    for index in [0, 2, u64::MAX] {
        let mut barrier = CompletionBarrier::new(8);
        barrier.consume(&frame(8, 0, false)).unwrap();
        assert_eq!(
            barrier.consume(&frame(8, index, true)),
            Err(CompletionError::OutOfOrder)
        );
        assert_eq!(
            barrier.consume(&frame(8, 1, true)),
            Err(CompletionError::Failed)
        );
        assert_eq!(barrier.finish(8, 2), Err(CompletionError::Failed));
        assert!(!barrier.ready());
    }
}

#[test]
fn finish_detects_consumed_count_and_end_marker_mismatches() {
    for (last, declared, error) in [
        (true, 0, CompletionError::ExtraFrames),
        (true, 2, CompletionError::EarlyEnd),
        (false, 1, CompletionError::MissingEnd),
    ] {
        let mut barrier = CompletionBarrier::new(8);
        barrier.consume(&frame(8, 0, last)).unwrap();
        assert_eq!(barrier.finish(8, declared), Err(error));
        assert!(!barrier.ready());
        assert_eq!(barrier.finish(8, 1), Err(CompletionError::Failed));
    }
}

#[test]
fn declared_count_requires_end_marker_at_exact_boundary() {
    for (declared, last, error) in [
        (2, true, CompletionError::EarlyEnd),
        (1, false, CompletionError::MissingEnd),
    ] {
        let mut barrier = CompletionBarrier::new(8);
        barrier.finish(8, declared).unwrap();
        assert_eq!(barrier.consume(&frame(8, 0, last)), Err(error));
        assert!(!barrier.ready());
    }
}

#[test]
fn no_audio_is_accepted_after_an_end_marker_with_or_without_finish() {
    for finished in [false, true] {
        let mut barrier = CompletionBarrier::new(8);
        barrier.consume(&frame(8, 0, true)).unwrap();
        if finished {
            barrier.finish(8, 1).unwrap();
        }
        assert_eq!(
            barrier.consume(&frame(8, 1, true)),
            Err(CompletionError::ExtraFrames)
        );
        assert!(!barrier.ready());
    }
}

#[test]
fn duplicate_finish_is_idempotent_but_conflict_invalidates_barrier() {
    for consumed in [false, true] {
        let mut barrier = CompletionBarrier::new(8);
        barrier.finish(8, 1).unwrap();
        if consumed {
            barrier.consume(&frame(8, 0, true)).unwrap();
        }
        barrier.finish(8, 1).unwrap();
        assert_eq!(barrier.ready(), consumed);
        assert_eq!(
            barrier.finish(8, 2),
            Err(CompletionError::ConflictingFinish)
        );
        assert!(!barrier.ready());
        assert_eq!(barrier.finish(8, 1), Err(CompletionError::Failed));
    }
}

#[test]
fn oversized_finish_fails_without_waiting_for_audio() {
    for count in [30_001, u64::MAX] {
        let mut barrier = CompletionBarrier::new(8);
        assert_eq!(
            barrier.finish(8, count),
            Err(CompletionError::TooManyFrames)
        );
        assert!(!barrier.ready());
        assert_eq!(barrier.finish(8, 0), Err(CompletionError::Failed));
    }
}

#[test]
fn ten_minute_bound_applies_even_when_finish_has_not_arrived() {
    for last in [false, true] {
        let mut barrier = CompletionBarrier::new(8);
        for index in 0..29_999 {
            barrier.consume(&frame(8, index, false)).unwrap();
        }
        barrier.consume(&frame(8, 29_999, last)).unwrap();
        if last {
            barrier.finish(8, 30_000).unwrap();
            assert!(barrier.ready());
        } else {
            assert_eq!(
                barrier.consume(&frame(8, 30_000, true)),
                Err(CompletionError::TooManyFrames)
            );
            assert!(!barrier.ready());
        }
    }
}
