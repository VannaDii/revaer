use super::{ExpectedFrame, Frame, HEADER_BYTES, ProtocolError, VERSION, validate};

/// Encode one canonical frame after validating all bounds and caller context.
///
/// This serializes data only. The lifecycle must independently establish the
/// executable authority, remaining deadline, admission state, and containment.
///
/// # Errors
/// Reject invalid data, mismatched expectations, or a failed bounded allocation.
pub fn encode_frame(frame: &Frame<'_>, expected: ExpectedFrame) -> Result<Vec<u8>, ProtocolError> {
    let length = validate::frame(frame, expected)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(validate::sum(HEADER_BYTES, length)?)
        .map_err(|_| ProtocolError::AllocationFailed)?;
    bytes.extend_from_slice(b"RVB1");
    bytes.push(frame.kind().byte());
    bytes.extend_from_slice(&[0; 3]);
    write_length(&mut bytes, length)?;
    bytes.extend_from_slice(&VERSION.to_be_bytes());
    match frame {
        Frame::Hello(value) => {
            bytes.extend_from_slice(&[value.lane.byte(), 0]);
            bytes.extend_from_slice(&value.nonce);
        }
        Frame::HelloAck(value) => {
            bytes.extend_from_slice(&[value.hello.lane.byte(), 0]);
            bytes.extend_from_slice(&value.hello.nonce);
            bytes.extend_from_slice(&value.process_id.to_be_bytes());
            bytes.extend_from_slice(&value.process_group_id.to_be_bytes());
            bytes.extend_from_slice(&value.executable.length.to_be_bytes());
            bytes.extend_from_slice(&value.executable.sha256);
        }
        Frame::Request(value) => {
            bytes.extend_from_slice(&[value.lane.byte(), 0]);
            bytes.extend_from_slice(&value.id.to_be_bytes());
            bytes.extend_from_slice(&value.remaining_ms.to_be_bytes());
            bytes.extend_from_slice(&value.limits.stdout.to_be_bytes());
            bytes.extend_from_slice(&value.limits.stderr.to_be_bytes());
            bytes.extend_from_slice(&value.native_identity);
            write_length(&mut bytes, value.program.len())?;
            bytes.extend_from_slice(value.program);
            write_length(&mut bytes, value.arguments.len())?;
            for argument in &value.arguments {
                write_length(&mut bytes, argument.len())?;
                bytes.extend_from_slice(argument);
            }
        }
        Frame::Response(value) => {
            bytes.extend_from_slice(&[value.lane.byte(), value.status.byte()]);
            bytes.extend_from_slice(&value.id.to_be_bytes());
            bytes.extend_from_slice(&[u8::from(value.evidence_truncated), 0, 0, 0]);
            write_length(&mut bytes, value.stdout.len())?;
            write_length(&mut bytes, value.detail.len())?;
            write_length(&mut bytes, validate::evidence_length(&value.evidence)?)?;
            bytes.extend_from_slice(value.stdout);
            bytes.extend_from_slice(value.detail);
            write_length(&mut bytes, value.evidence.len())?;
            for item in &value.evidence {
                write_length(&mut bytes, item.len())?;
                bytes.extend_from_slice(item.as_bytes());
            }
        }
    }
    if bytes.len() != validate::sum(HEADER_BYTES, length)? {
        return Err(ProtocolError::Malformed);
    }
    Ok(bytes)
}

fn write_length(bytes: &mut Vec<u8>, length: usize) -> Result<(), ProtocolError> {
    let value = u32::try_from(length).map_err(|_| ProtocolError::LimitExceeded)?;
    bytes.extend_from_slice(&value.to_be_bytes());
    Ok(())
}
