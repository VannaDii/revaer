use super::{
    ARGUMENTS_MAX, ExecutableIdentity, ExpectedFrame, Frame, FrameKind, HEADER_BYTES, Hello,
    HelloAck, Lane, ProtocolError, Request, Response, ResponseStatus, STRING_BYTES_MAX,
    StreamLimits, VERSION, validate,
};

/// A validated fixed header, suitable for bounding a subsequent payload read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    kind: FrameKind,
    payload_length: usize,
}

impl FrameHeader {
    /// Validate fixed storage before a caller allocates or reads payload bytes.
    ///
    /// # Errors
    /// Reject invalid expectations, magic, kinds, reserved data, or size bounds.
    pub fn parse(
        bytes: &[u8; HEADER_BYTES],
        expected: ExpectedFrame,
    ) -> Result<Self, ProtocolError> {
        validate::expectation(expected)?;
        if bytes[..4] != *b"RVB1" || bytes[5..8] != [0; 3] {
            return Err(ProtocolError::Malformed);
        }
        let kind = FrameKind::parse(bytes[4])?;
        if kind != expected.kind() {
            return Err(ProtocolError::UnexpectedFrame);
        }
        let length = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let payload_length = usize::try_from(length).map_err(|_| ProtocolError::LimitExceeded)?;
        validate::payload_length(kind, payload_length)?;
        Ok(Self {
            kind,
            payload_length,
        })
    }

    /// Exact validated number of following payload bytes, excluding the header.
    #[must_use]
    pub const fn payload_length(self) -> usize {
        self.payload_length
    }
}

/// Decode exactly one frame without copying its byte/string payload fields.
///
/// Only bounded argument/evidence reference arrays allocate, after proving their
/// counts against both the protocol and the remaining payload. No OS or process
/// action is performed, and no returned value certifies execution authority.
///
/// # Errors
/// Reject malformed, noncanonical, unexpected, mismatched, or over-bound frames.
pub fn decode_frame(bytes: &[u8], expected: ExpectedFrame) -> Result<Frame<'_>, ProtocolError> {
    let header_bytes = bytes.get(..HEADER_BYTES).ok_or(ProtocolError::Truncated)?;
    let header = FrameHeader::parse(
        header_bytes
            .try_into()
            .map_err(|_| ProtocolError::Truncated)?,
        expected,
    )?;
    let total = validate::sum(HEADER_BYTES, header.payload_length)?;
    if bytes.len() < total {
        return Err(ProtocolError::Truncated);
    }
    if bytes.len() != total {
        return Err(ProtocolError::Malformed);
    }
    let mut reader = Reader(&bytes[HEADER_BYTES..]);
    if reader.u16()? != VERSION {
        return Err(ProtocolError::Malformed);
    }
    let lane = Lane::parse(reader.byte()?)?;
    let fourth = reader.byte()?;
    if header.kind != FrameKind::Response && fourth != 0 {
        return Err(ProtocolError::Malformed);
    }
    let frame = match header.kind {
        FrameKind::Hello => Frame::Hello(Hello {
            lane,
            nonce: reader.array()?,
        }),
        FrameKind::HelloAck => Frame::HelloAck(HelloAck {
            hello: Hello {
                lane,
                nonce: reader.array()?,
            },
            process_id: reader.u32()?,
            process_group_id: reader.u32()?,
            executable: ExecutableIdentity {
                length: reader.u64()?,
                sha256: reader.array()?,
            },
        }),
        FrameKind::Request => Frame::Request(request(&mut reader, lane)?),
        FrameKind::Response => {
            Frame::Response(response(&mut reader, lane, ResponseStatus::parse(fourth)?)?)
        }
    };
    reader.finish()?;
    validate::frame(&frame, expected)?;
    Ok(frame)
}

fn request<'a>(reader: &mut Reader<'a>, lane: Lane) -> Result<Request<'a>, ProtocolError> {
    let id = reader.u64()?;
    let remaining_ms = reader.u64()?;
    let limits = StreamLimits {
        stdout: reader.u32()?,
        stderr: reader.u32()?,
    };
    let native_identity = reader.array()?;
    let length = reader.length()?;
    validate::bounded(length, STRING_BYTES_MAX)?;
    let program = reader.bytes(length)?;
    let count = reader.length()?;
    validate::bounded(count, ARGUMENTS_MAX)?;
    validate::bounded(count, reader.0.len() / 4)?;
    let mut arguments = Vec::new();
    arguments
        .try_reserve_exact(count)
        .map_err(|_| ProtocolError::AllocationFailed)?;
    for _ in 0..count {
        let length = reader.length()?;
        validate::bounded(length, STRING_BYTES_MAX)?;
        arguments.push(reader.bytes(length)?);
    }
    Ok(Request {
        lane,
        id,
        remaining_ms,
        limits,
        native_identity,
        program,
        arguments,
    })
}

fn response<'a>(
    reader: &mut Reader<'a>,
    lane: Lane,
    status: ResponseStatus,
) -> Result<Response<'a>, ProtocolError> {
    let id = reader.u64()?;
    let flags = reader.byte()?;
    if flags & !1 != 0 || reader.array::<3>()? != [0; 3] {
        return Err(ProtocolError::Malformed);
    }
    let stdout_length = reader.length()?;
    let detail_length = reader.length()?;
    let evidence_length = reader.length()?;
    for length in [stdout_length, detail_length, evidence_length] {
        validate::stream_limit(length)?;
    }
    let stdout = reader.bytes(stdout_length)?;
    let detail = reader.bytes(detail_length)?;
    let evidence_bytes = reader.bytes(evidence_length)?;
    let evidence = evidence(evidence_bytes)?;
    Ok(Response {
        lane,
        id,
        status,
        stdout,
        detail,
        evidence,
        evidence_truncated: flags == 1,
    })
}

fn evidence(bytes: &[u8]) -> Result<Vec<&str>, ProtocolError> {
    let count = evidence_count(bytes)?;
    let mut reader = Reader(bytes.get(4..).ok_or(ProtocolError::Truncated)?);
    let mut items = Vec::new();
    items
        .try_reserve_exact(count)
        .map_err(|_| ProtocolError::AllocationFailed)?;
    for _ in 0..count {
        items.push(reader.evidence_item()?);
    }
    reader.finish()?;
    Ok(items)
}

// Validate the entire evidence layout before allocating its reference array.
pub(super) fn evidence_count(bytes: &[u8]) -> Result<usize, ProtocolError> {
    validate::stream_limit(bytes.len())?;
    let mut reader = Reader(bytes);
    let count = reader.length()?;
    // Every nonempty item needs its four-byte prefix plus at least one byte.
    validate::bounded(count, reader.0.len() / 5)?;
    let mut previous = None;
    for _ in 0..count {
        let item = reader.evidence_item()?;
        validate::evidence_item(previous, item)?;
        previous = Some(item);
    }
    reader.finish()?;
    Ok(count)
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn evidence_item(&mut self) -> Result<&'a str, ProtocolError> {
        let length = self.length()?;
        validate::bounded(
            length,
            crate::process::NATIVE_PROCESS_STREAM_LIMIT_BYTES - 8,
        )?;
        std::str::from_utf8(self.bytes(length)?).map_err(|_| ProtocolError::Malformed)
    }
    fn bytes(&mut self, length: usize) -> Result<&'a [u8], ProtocolError> {
        let (head, tail) = self
            .0
            .split_at_checked(length)
            .ok_or(ProtocolError::Truncated)?;
        self.0 = tail;
        Ok(head)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ProtocolError> {
        self.bytes(N)?
            .try_into()
            .map_err(|_| ProtocolError::Truncated)
    }
    fn byte(&mut self) -> Result<u8, ProtocolError> {
        Ok(u8::from_be_bytes(self.array()?))
    }
    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    fn length(&mut self) -> Result<usize, ProtocolError> {
        usize::try_from(self.u32()?).map_err(|_| ProtocolError::LimitExceeded)
    }
    const fn finish(&self) -> Result<(), ProtocolError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Malformed)
        }
    }
}
