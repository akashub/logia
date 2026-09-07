use logia_recognition_spike::preview::*;
use logia_recognition_spike::{audio::*, audio_queue::*, events::*, protocol::*};
use std::io::{Cursor, Read};

fn partial(session: u64, seq: u64, stable: &str, tail: &str) -> Event {
    Event::Partial {
        session,
        seq,
        stable: stable.into(),
        tail: tail.into(),
    }
}
fn pcm(index: u64, samples: usize, last: bool) -> AudioFrame {
    AudioFrame::new(8, index, 16_000, 1, last, vec![0.25; samples]).unwrap()
}
fn wire(json: &[u8]) -> Vec<u8> {
    let mut bytes = (json.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(json);
    bytes
}

#[test]
fn versioned_control_and_event_roundtrip() {
    for message in [
        Message::Control(Control::Start { session: 8 }),
        Message::Control(Control::Finish { session: 8 }),
        Message::Control(Control::Cancel { session: 8 }),
        Message::Event(partial(8, 1, "hello", " there")),
        Message::Event(Event::ModelReady {}),
    ] {
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).unwrap();
        assert_eq!(
            read_message(&mut Cursor::new(bytes)).unwrap(),
            Some(message)
        );
    }
    assert_eq!(read_message(&mut Cursor::new([])).unwrap(), None);
}

#[test]
fn malformed_wire_is_rejected_without_content_in_error() {
    for bytes in [vec![1, 0], wire(b"{"), wire(br#"{"version":2,"message":{"kind":"event","value":{"type":"model_ready"}}}"#), wire(br#"{"version":1,"message":{"kind":"event","value":{"type":"invented","text":"private"}}}"#)] {
        let error = read_message(&mut Cursor::new(bytes)).unwrap_err();
        assert!(!format!("{error:?}").contains("private"));
    }
    assert_eq!(
        read_message(&mut Cursor::new(wire(
            br#"{"version":2,"message":{"kind":"event","value":{"type":"model_ready"}}}"#
        ))),
        Err(ProtocolError::UnsupportedVersion)
    );
    let mut truncated = wire(b"abc");
    truncated.pop();
    assert_eq!(
        read_message(&mut Cursor::new(truncated)),
        Err(ProtocolError::Truncated)
    );
}

#[test]
fn oversized_header_is_rejected_before_reading_payload() {
    struct HeaderOnly(Cursor<[u8; 4]>);
    impl Read for HeaderOnly {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            assert!(self.0.position() < 4, "must not read oversized body");
            self.0.read(buf)
        }
    }
    let mut input = HeaderOnly(Cursor::new(65_537u32.to_le_bytes()));
    assert_eq!(read_message(&mut input), Err(ProtocolError::TooLarge));
    let mut output = Vec::new();
    assert_eq!(
        write_message(
            &mut output,
            &Message::Event(partial(8, 1, "", &"x".repeat(65_536)))
        ),
        Err(ProtocolError::TooLarge)
    );
    assert!(output.is_empty());
    assert_eq!(
        write_message(
            &mut output,
            &Message::Event(partial(8, 1, "", &"\n".repeat(40_000)))
        ),
        Err(ProtocolError::TooLarge)
    );
    assert!(output.is_empty());
}

#[test]
fn audio_binary_roundtrip_and_validation() {
    for frame in [pcm(0, 320, false), pcm(1, 7, true)] {
        let mut bytes = Vec::new();
        write_audio(&mut bytes, &frame).unwrap();
        assert!(bytes.windows(4).any(|v| v == 0.25f32.to_le_bytes()));
        assert_eq!(read_audio(&mut Cursor::new(bytes)).unwrap(), Some(frame));
    }
    for (rate, channels, last, samples) in [
        (48_000, 1, false, vec![0.; 320]),
        (16_000, 2, false, vec![0.; 320]),
        (16_000, 1, false, vec![0.; 319]),
        (16_000, 1, true, vec![]),
        (16_000, 1, true, vec![0.; 321]),
        (16_000, 1, true, vec![f32::NAN]),
        (16_000, 1, true, vec![f32::INFINITY]),
    ] {
        assert!(AudioFrame::new(8, 0, rate, channels, last, samples).is_err());
    }
    assert_eq!(
        read_audio(&mut Cursor::new(65_537u32.to_le_bytes())),
        Err(ProtocolError::TooLarge)
    );
    let mut bytes = Vec::new();
    write_audio(&mut bytes, &pcm(0, 320, false)).unwrap();
    bytes.pop();
    assert_eq!(
        read_audio(&mut Cursor::new(bytes)),
        Err(ProtocolError::Truncated)
    );
}

#[test]
fn queue_bounds_order_and_partial_accounting() {
    let mut queue = AudioQueue::new(8);
    for index in 0..500 {
        queue.try_push(pcm(index, 320, false)).unwrap();
    }
    assert_eq!(queue.sample_bytes(), 640_000);
    assert_eq!(queue.try_push(pcm(500, 320, false)), Err(QueueError::Full));
    assert_eq!(queue.pop().unwrap().index(), 0);
    queue.try_push(pcm(500, 7, true)).unwrap();
    assert_eq!(queue.sample_bytes(), 499 * 1280 + 28);
    assert_eq!(
        queue.try_push(pcm(501, 320, false)),
        Err(QueueError::Finished)
    );
    let mut queue = AudioQueue::new(8);
    assert_eq!(
        queue.try_push(pcm(1, 320, false)),
        Err(QueueError::OutOfOrder)
    );
    assert_eq!(
        queue.try_push(AudioFrame::new(9, 0, 16000, 1, false, vec![0.; 320]).unwrap()),
        Err(QueueError::WrongSession)
    );
    queue.try_push(pcm(0, 320, false)).unwrap();
    assert_eq!(
        queue.try_push(pcm(0, 320, false)),
        Err(QueueError::OutOfOrder)
    );
    queue.clear();
    assert_eq!(queue.sample_bytes(), 0);
    assert!(queue.pop().is_none());
}

#[test]
fn events_reject_stale_reordered_revised_and_post_terminal_text() {
    assert!(!accepts(Some(8), true, 8));
    assert!(!accepts(Some(9), false, 8));
    assert!(!accepts(None, false, 8));
    let mut events = EventMailbox::new(8);
    assert_eq!(
        events.push(partial(9, 1, "", "stale")),
        Err(EventError::WrongSession)
    );
    events.push(partial(8, 1, "hello", " a")).unwrap();
    assert_eq!(
        events.push(partial(8, 1, "hello", " b")),
        Err(EventError::OutOfOrder)
    );
    assert_eq!(
        events.push(partial(8, 0, "hello", " b")),
        Err(EventError::OutOfOrder)
    );
    assert_eq!(
        events.push(partial(8, 2, "changed", "")),
        Err(EventError::StableRevision)
    );
    events.push(partial(8, 2, "hello there", "")).unwrap();
    let final_event = Event::Final {
        session: 8,
        seq: 3,
        text: "hello there!".into(),
    };
    events.push(final_event.clone()).unwrap();
    assert_eq!(
        events.push(partial(8, 4, "hello there", "late")),
        Err(EventError::Closed)
    );
    assert_eq!(events.pop(), Some(final_event));
    assert_eq!(events.pop(), None);
}

#[test]
fn coalescing_cancel_and_size_failures_preserve_known_text() {
    let mut events = EventMailbox::new(8);
    events.push(partial(8, 1, "a", " b")).unwrap();
    events.push(partial(8, 2, "a", " c")).unwrap();
    assert_eq!(events.pop(), Some(partial(8, 2, "a", " c")));
    assert_eq!(events.pop(), None);
    assert_eq!(
        events.push(partial(8, 3, "a", &"x".repeat(65_536))),
        Err(EventError::TooLarge)
    );
    assert_eq!(events.stable(), "a");
    events.cancel();
    assert_eq!(
        events.push(partial(8, 4, "a", " late")),
        Err(EventError::Closed)
    );
    assert!(events.pop().is_none());
}

#[test]
fn preview_has_one_active_and_only_newest_pending() {
    fn snapshot(seq: u64) -> PreviewSnapshot {
        PreviewSnapshot::new(8, seq, vec![0.; 320]).unwrap()
    }
    let mut work = PreviewWork::new(8);
    assert_eq!(work.submit(snapshot(1)).unwrap().unwrap().seq(), 1);
    assert!(work.submit(snapshot(2)).unwrap().is_none());
    assert!(work.submit(snapshot(3)).unwrap().is_none());
    assert_eq!(
        work.submit(snapshot(3)).unwrap_err(),
        PreviewError::OutOfOrder
    );
    assert_eq!(
        work.submit(PreviewSnapshot::new(9, 4, vec![0.]).unwrap())
            .unwrap_err(),
        PreviewError::WrongSession
    );
    assert_eq!(work.complete().unwrap().unwrap().seq(), 3);
    assert!(work.complete().unwrap().is_none());
    assert_eq!(work.complete().unwrap_err(), PreviewError::Idle);
    work.submit(snapshot(4)).unwrap();
    work.submit(snapshot(5)).unwrap();
    work.cancel();
    assert!(work.active());
    assert_eq!(work.submit(snapshot(6)).unwrap_err(), PreviewError::Closed);
    assert!(work.complete().unwrap().is_none());
    assert!(!work.active());
    assert!(PreviewSnapshot::new(8, 1, vec![0.; 160_000]).is_ok());
    assert_eq!(
        PreviewSnapshot::new(8, 1, vec![0.; 160_001]).unwrap_err(),
        PreviewError::TooLarge
    );
    assert_eq!(
        PreviewSnapshot::new(8, 1, vec![f32::NAN]).unwrap_err(),
        PreviewError::InvalidAudio
    );
}

#[test]
fn mutated_binary_declarations_and_samples_are_rejected() {
    let mut valid = Vec::new();
    write_audio(&mut valid, &pcm(0, 320, false)).unwrap();
    for (offset, replacement) in [(4, 2), (22, 0), (26, 2), (27, 2), (28, 0)] {
        let mut bytes = valid.clone();
        bytes[offset] = replacement;
        assert!(
            read_audio(&mut Cursor::new(bytes)).is_err(),
            "offset {offset}"
        );
    }
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut bytes = valid.clone();
        bytes[30..34].copy_from_slice(&invalid.to_le_bytes());
        assert_eq!(
            read_audio(&mut Cursor::new(bytes)),
            Err(ProtocolError::InvalidPayload)
        );
    }
    assert_eq!(read_audio(&mut Cursor::new([])).unwrap(), None);
    assert_eq!(
        read_audio(&mut Cursor::new([0, 0])),
        Err(ProtocolError::Truncated)
    );
}

#[test]
fn terminal_failure_is_preserved_and_debug_is_content_free() {
    let mut events = EventMailbox::new(8);
    events.push(partial(8, 1, "known", " private")).unwrap();
    let failure = Event::Failed {
        session: 8,
        code: FailureCode::QueueFull,
    };
    events.push(failure.clone()).unwrap();
    assert_eq!(
        events.push(Event::Canceled { session: 8 }),
        Err(EventError::Closed)
    );
    assert_eq!(events.pop(), Some(failure));
    assert_eq!(events.stable(), "known");
    let event = Event::Final {
        session: 8,
        seq: 2,
        text: "private".into(),
    };
    assert!(!format!("{event:?}").contains("private"));
    assert!(!format!("{:?}", pcm(0, 320, false)).contains("0.25"));
}

#[test]
fn metadata_exact_cap_and_strict_schema() {
    let message = Message::Event(Event::ModelReady {});
    let mut base = Vec::new();
    write_message(&mut base, &message).unwrap();
    let mut padded = base[4..].to_vec();
    padded.resize(65_536, b' ');
    assert_eq!(
        read_message(&mut Cursor::new(wire(&padded))).unwrap(),
        Some(message)
    );
    for json in [br#"{"version":1,"message":{"kind":"event","value":{"type":"model_ready","extra":true}}}"#.as_slice(),br#"{"version":1,"message":{"kind":"event","value":{"type":"failed","session":8,"code":"private"}}}"#] {
        assert_eq!(read_message(&mut Cursor::new(wire(json))),Err(ProtocolError::InvalidPayload));
    }
}
