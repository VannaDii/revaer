//! Canonical RVB1 data encoding, without process or execution authority.
//!
//! The caller supplies the frame expected by its lifecycle before reading a
//! payload. Decoding validates bytes, not operating-system identity, deadlines,
//! containment, or an executable closure. A decoded request must never bypass
//! the separate verified-executable admission boundary in ADR 558.

mod decode;
mod encode;
mod model;
mod validate;

pub use decode::{FrameHeader, decode_frame};
pub use encode::encode_frame;
pub use model::{
    ExecutableIdentity, ExpectedFrame, Frame, FrameKind, Hello, HelloAck, Lane, ProtocolError,
    Request, Response, ResponseStatus, StreamLimits,
};

const HEADER_BYTES: usize = 12;
const VERSION: u16 = 1;
const REQUEST_BYTES_MAX: usize = 1_048_576;
const RESPONSE_BYTES_MAX: usize = 50_335_744;
const STRING_BYTES_MAX: usize = 4_096;
const ARGUMENTS_MAX: usize = 4_096;
const ARGUMENT_BYTES_MAX: usize = 1_000_000;
const CONTROL_MILLISECONDS_MAX: u64 = 30_000;
const JOB_MILLISECONDS_MAX: u64 = 86_400_000;

#[cfg(test)]
mod tests;
