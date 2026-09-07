//! Audio uses a separate pipe: u32 LE length, then a 26-byte binary header and PCM.
//! Header: version u16, session u64, index u64, rate u32, channels u8,
//! final-frame flag u8, sample-count u16; every multi-byte value is little endian.
use crate::protocol::{read_payload, write_payload, ProtocolError, SessionId, VERSION};
use std::io::{Read, Write};

pub const FRAME_SAMPLES: usize = 320;
const HEADER_BYTES: usize = 26;

#[derive(Clone, PartialEq)]
pub struct AudioFrame {
    session: SessionId,
    index: u64,
    last: bool,
    samples: Box<[f32]>,
}

impl std::fmt::Debug for AudioFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AudioFrame(<redacted>)")
    }
}

impl AudioFrame {
    pub fn new(
        session: SessionId,
        index: u64,
        rate: u32,
        channels: u8,
        last: bool,
        samples: Vec<f32>,
    ) -> Result<Self, ProtocolError> {
        if rate != 16_000
            || channels != 1
            || samples.is_empty()
            || samples.len() > FRAME_SAMPLES
            || (!last && samples.len() != FRAME_SAMPLES)
            || samples.iter().any(|v| !v.is_finite())
        {
            return Err(ProtocolError::InvalidPayload);
        }
        Ok(Self {
            session,
            index,
            last,
            // Drop spare caller capacity so retained storage follows sample count.
            samples: samples.into_boxed_slice(),
        })
    }
    pub fn session(&self) -> SessionId {
        self.session
    }
    pub fn index(&self) -> u64 {
        self.index
    }
    pub fn is_last(&self) -> bool {
        self.last
    }
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
    pub fn sample_bytes(&self) -> usize {
        self.samples.len() * 4
    }
}

pub fn write_audio(writer: &mut impl Write, frame: &AudioFrame) -> Result<(), ProtocolError> {
    let mut bytes = Vec::with_capacity(HEADER_BYTES + frame.sample_bytes());
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&frame.session.to_le_bytes());
    bytes.extend_from_slice(&frame.index.to_le_bytes());
    bytes.extend_from_slice(&16_000u32.to_le_bytes());
    bytes.push(1);
    bytes.push(u8::from(frame.last));
    bytes.extend_from_slice(&(frame.samples.len() as u16).to_le_bytes());
    for sample in frame.samples.iter() {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    write_payload(writer, &bytes)
}

pub fn read_audio(reader: &mut impl Read) -> Result<Option<AudioFrame>, ProtocolError> {
    let Some(bytes) = read_payload(reader, HEADER_BYTES + FRAME_SAMPLES * 4)? else {
        return Ok(None);
    };
    if bytes.len() < HEADER_BYTES {
        return Err(ProtocolError::InvalidPayload);
    }
    if u16::from_le_bytes(bytes[0..2].try_into().unwrap()) != VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }
    let session = u64::from_le_bytes(bytes[2..10].try_into().unwrap());
    let index = u64::from_le_bytes(bytes[10..18].try_into().unwrap());
    let rate = u32::from_le_bytes(bytes[18..22].try_into().unwrap());
    let count = u16::from_le_bytes(bytes[24..26].try_into().unwrap()) as usize;
    if bytes[23] > 1 || bytes.len() != HEADER_BYTES + count * 4 {
        return Err(ProtocolError::InvalidPayload);
    }
    let samples = bytes[HEADER_BYTES..]
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
        .collect();
    AudioFrame::new(session, index, rate, bytes[22], bytes[23] == 1, samples).map(Some)
}
