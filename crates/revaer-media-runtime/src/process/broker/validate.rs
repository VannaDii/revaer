use super::{
    ARGUMENT_BYTES_MAX, ARGUMENTS_MAX, CONTROL_MILLISECONDS_MAX, ExpectedFrame, Frame, FrameKind,
    JOB_MILLISECONDS_MAX, Lane, ProtocolError, REQUEST_BYTES_MAX, RESPONSE_BYTES_MAX, Request,
    Response, ResponseStatus, STRING_BYTES_MAX, StreamLimits,
};
use crate::process::NATIVE_PROCESS_STREAM_LIMIT_BYTES;

pub(super) fn payload_length(kind: FrameKind, length: usize) -> Result<(), ProtocolError> {
    let accepted = match kind {
        FrameKind::Hello => length == 20,
        FrameKind::HelloAck => length == 68,
        FrameKind::Request => (69..=REQUEST_BYTES_MAX).contains(&length),
        FrameKind::Response => (32..=RESPONSE_BYTES_MAX).contains(&length),
    };
    if accepted {
        Ok(())
    } else {
        Err(ProtocolError::LimitExceeded)
    }
}

pub(super) const fn stream_limit(value: usize) -> Result<(), ProtocolError> {
    bounded(value, NATIVE_PROCESS_STREAM_LIMIT_BYTES)
}

pub(super) const fn bounded(value: usize, maximum: usize) -> Result<(), ProtocolError> {
    if value <= maximum {
        Ok(())
    } else {
        Err(ProtocolError::LimitExceeded)
    }
}

pub(super) fn sum(left: usize, right: usize) -> Result<usize, ProtocolError> {
    left.checked_add(right).ok_or(ProtocolError::LimitExceeded)
}

fn valid_pid(value: u32) -> bool {
    (1..=2_147_483_647).contains(&value)
}

fn limits(value: StreamLimits) -> Result<(), ProtocolError> {
    stream_limit(usize::try_from(value.stdout).map_err(|_| ProtocolError::LimitExceeded)?)?;
    stream_limit(usize::try_from(value.stderr).map_err(|_| ProtocolError::LimitExceeded)?)
}

pub(super) fn expectation(expected: ExpectedFrame) -> Result<(), ProtocolError> {
    match expected {
        ExpectedFrame::HelloAck { process_id, .. } if !valid_pid(process_id) => {
            Err(ProtocolError::InvalidExpectation)
        }
        ExpectedFrame::Request { id: 0, .. } | ExpectedFrame::Response { id: 0, .. } => {
            Err(ProtocolError::InvalidExpectation)
        }
        ExpectedFrame::Response { limits: value, .. } => {
            limits(value).map_err(|_| ProtocolError::InvalidExpectation)
        }
        ExpectedFrame::Hello { .. }
        | ExpectedFrame::HelloAck { .. }
        | ExpectedFrame::Request { .. } => Ok(()),
    }
}

pub(super) fn frame(frame: &Frame<'_>, expected: ExpectedFrame) -> Result<usize, ProtocolError> {
    expectation(expected)?;
    if frame.kind() != expected.kind() {
        return Err(ProtocolError::UnexpectedFrame);
    }
    let (identity_matches, length) = match (frame, expected) {
        (Frame::Hello(value), ExpectedFrame::Hello { lane }) => (value.lane == lane, 20),
        (
            Frame::HelloAck(value),
            ExpectedFrame::HelloAck {
                hello,
                process_id,
                executable,
            },
        ) => {
            if !valid_pid(value.process_id) || value.process_group_id != value.process_id {
                return Err(ProtocolError::Malformed);
            }
            (
                value.hello == hello
                    && value.process_id == process_id
                    && value.executable == executable,
                68,
            )
        }
        (Frame::Request(value), ExpectedFrame::Request { lane, id }) => {
            (value.lane == lane && value.id == id, request(value)?)
        }
        (Frame::Response(value), ExpectedFrame::Response { lane, id, limits }) => (
            value.lane == lane && value.id == id,
            response(value, limits)?,
        ),
        _ => return Err(ProtocolError::UnexpectedFrame),
    };
    if !identity_matches {
        return Err(ProtocolError::IdentityMismatch);
    }
    payload_length(frame.kind(), length)?;
    Ok(length)
}

fn string(value: &[u8]) -> Result<(), ProtocolError> {
    bounded(value.len(), STRING_BYTES_MAX)?;
    if value.contains(&0) {
        return Err(ProtocolError::Malformed);
    }
    Ok(())
}

fn request(value: &Request<'_>) -> Result<usize, ProtocolError> {
    if value.id == 0 || value.remaining_ms == 0 || value.program.first() != Some(&b'/') {
        return Err(ProtocolError::Malformed);
    }
    let maximum_ms = match value.lane {
        Lane::Control => CONTROL_MILLISECONDS_MAX,
        Lane::Job => JOB_MILLISECONDS_MAX,
    };
    if value.remaining_ms > maximum_ms {
        return Err(ProtocolError::LimitExceeded);
    }
    limits(value.limits)?;
    string(value.program)?;
    bounded(value.arguments.len(), ARGUMENTS_MAX)?;
    let mut strings = value.program.len();
    let mut length = sum(68, strings)?;
    for argument in &value.arguments {
        string(argument)?;
        strings = sum(strings, argument.len())?;
        length = sum(length, sum(4, argument.len())?)?;
    }
    bounded(strings, ARGUMENT_BYTES_MAX)?;
    Ok(length)
}

pub(super) fn evidence_length(items: &[&str]) -> Result<usize, ProtocolError> {
    let mut length = 4;
    let mut previous: Option<&str> = None;
    for item in items {
        evidence_item(previous, item)?;
        length = sum(length, sum(4, item.len())?)?;
        stream_limit(length)?;
        previous = Some(item);
    }
    Ok(length)
}

pub(super) fn evidence_item(previous: Option<&str>, item: &str) -> Result<(), ProtocolError> {
    if item.is_empty() || previous.is_some_and(|prior| prior.as_bytes() >= item.as_bytes()) {
        return Err(ProtocolError::Malformed);
    }
    Ok(())
}

fn response(value: &Response<'_>, limits: StreamLimits) -> Result<usize, ProtocolError> {
    if value.id == 0 {
        return Err(ProtocolError::Malformed);
    }
    bounded(
        value.stdout.len(),
        usize::try_from(limits.stdout).map_err(|_| ProtocolError::LimitExceeded)?,
    )?;
    bounded(
        value.detail.len(),
        usize::try_from(limits.stderr).map_err(|_| ProtocolError::LimitExceeded)?,
    )?;
    match value.status {
        ResponseStatus::Success if !value.evidence.is_empty() || value.evidence_truncated => {
            return Err(ProtocolError::Malformed);
        }
        ResponseStatus::Success => {}
        _ if !value.stdout.is_empty() => return Err(ProtocolError::Malformed),
        _ => {}
    }
    let length = sum(28, sum(value.stdout.len(), value.detail.len())?)?;
    sum(length, evidence_length(&value.evidence)?)
}
