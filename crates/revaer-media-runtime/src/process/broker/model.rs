use std::fmt;

use thiserror::Error;

/// Closed codec errors; no untrusted protocol data is retained or logged.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// Required bytes were absent.
    #[error("broker frame is truncated")]
    Truncated,
    /// Magic, registry, version, reserved bytes, or payload shape was invalid.
    #[error("broker frame is malformed")]
    Malformed,
    /// An approved count, size, or duration bound was exceeded.
    #[error("broker frame exceeds a protocol bound")]
    LimitExceeded,
    /// The frame kind is not expected by the owning lifecycle.
    #[error("broker frame is not expected")]
    UnexpectedFrame,
    /// Lane, request, nonce, executable, or process identity did not match.
    #[error("broker frame identity does not match")]
    IdentityMismatch,
    /// The caller supplied an invalid expected identifier or stream limit.
    #[error("broker frame expectation is invalid")]
    InvalidExpectation,
    /// A bounded allocation could not be obtained.
    #[error("broker frame allocation failed")]
    AllocationFailed,
}

/// The two approved serial broker lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// Capability discovery and inspection.
    Control,
    /// Transcode, analysis, playback, and output verification.
    Job,
}

impl Lane {
    pub(super) const fn byte(self) -> u8 {
        match self {
            Self::Control => 1,
            Self::Job => 2,
        }
    }

    pub(super) const fn parse(value: u8) -> Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::Control),
            2 => Ok(Self::Job),
            _ => Err(ProtocolError::Malformed),
        }
    }
}

/// An explicitly assigned RVB1 frame kind, independent of Rust enum layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// Parent handshake.
    Hello,
    /// Broker handshake response.
    HelloAck,
    /// Parent native-process request.
    Request,
    /// Broker terminal response.
    Response,
}

impl FrameKind {
    pub(super) const fn byte(self) -> u8 {
        match self {
            Self::Hello => 1,
            Self::HelloAck => 2,
            Self::Request => 0x10,
            Self::Response => 0x11,
        }
    }

    pub(super) const fn parse(value: u8) -> Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::HelloAck),
            0x10 => Ok(Self::Request),
            0x11 => Ok(Self::Response),
            _ => Err(ProtocolError::Malformed),
        }
    }
}

/// Only native terminal outcomes are valid on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    /// Complete bounded native output.
    Success,
    /// Native spawn failed.
    SpawnFailed,
    /// The request was cancelled.
    Cancelled,
    /// The deadline expired.
    DeadlineExceeded,
    /// Standard output exceeded its limit.
    StdoutLimitExceeded,
    /// Standard error exceeded its limit.
    StderrLimitExceeded,
    /// Native execution exited unsuccessfully.
    ExitFailed,
    /// Native supervision failed.
    SupervisionFailed,
}

impl ResponseStatus {
    pub(super) const fn byte(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::SpawnFailed => 1,
            Self::Cancelled => 2,
            Self::DeadlineExceeded => 3,
            Self::StdoutLimitExceeded => 4,
            Self::StderrLimitExceeded => 5,
            Self::ExitFailed => 6,
            Self::SupervisionFailed => 7,
        }
    }

    pub(super) const fn parse(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::SpawnFailed),
            2 => Ok(Self::Cancelled),
            3 => Ok(Self::DeadlineExceeded),
            4 => Ok(Self::StdoutLimitExceeded),
            5 => Ok(Self::StderrLimitExceeded),
            6 => Ok(Self::ExitFailed),
            7 => Ok(Self::SupervisionFailed),
            _ => Err(ProtocolError::Malformed),
        }
    }
}

/// Exact executable bytes claimed by the handshake; not a verified authority.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ExecutableIdentity {
    /// Descriptor byte length.
    pub length: u64,
    /// SHA-256 of the same descriptor's bytes.
    pub sha256: [u8; 32],
}

/// The immutable original request's stream limits, including meaningful zero.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StreamLimits {
    /// Maximum accepted standard-output bytes.
    pub stdout: u32,
    /// Maximum accepted standard-error/detail bytes.
    pub stderr: u32,
}

/// The parent handshake data. Nonce generation belongs to the lifecycle owner.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hello {
    /// Selected lane.
    pub lane: Lane,
    /// Exact sixteen-byte nonce.
    pub nonce: [u8; 16],
}

/// Broker handshake claims, which require independent operating-system proof.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HelloAck {
    /// Echoed handshake.
    pub hello: Hello,
    /// Positive Linux process identifier.
    pub process_id: u32,
    /// Broker process-group identifier, equal to the process identifier.
    pub process_group_id: u32,
    /// Same-binary executable claim.
    pub executable: ExecutableIdentity,
}

/// Request data only; this type confers no native execution authority.
#[derive(Clone, PartialEq, Eq)]
pub struct Request<'a> {
    /// Explicit broker lane.
    pub lane: Lane,
    /// Nonzero identifier assigned by the manager before its first wire byte.
    pub id: u64,
    /// Already floor-rounded remaining deadline, never a fresh timeout.
    pub remaining_ms: u64,
    /// Captured-stream limits.
    pub limits: StreamLimits,
    /// Full ADR 519 closure identity, not merely the program-file digest.
    pub native_identity: [u8; 32],
    /// Absolute Unix program bytes, with no text conversion.
    pub program: &'a [u8],
    /// Exact argument bytes; an empty argument is meaningful.
    pub arguments: Vec<&'a [u8]>,
}

/// One native terminal response, not proof that parent cancellation is absent.
#[derive(Clone, PartialEq, Eq)]
pub struct Response<'a> {
    /// Explicit broker lane.
    pub lane: Lane,
    /// Active nonzero identifier.
    pub id: u64,
    /// Closed native outcome.
    pub status: ResponseStatus,
    /// Complete standard output on success; empty on failure.
    pub stdout: &'a [u8],
    /// Bounded standard error or the existing failure diagnostic.
    pub detail: &'a [u8],
    /// Nonempty UTF-8 items in strictly increasing byte order.
    pub evidence: Vec<&'a str>,
    /// Whether the evidence producer omitted any item or byte.
    pub evidence_truncated: bool,
}

/// An RVB1 frame with borrowed byte fields. Debug output contains only its kind.
#[derive(Clone, PartialEq, Eq)]
pub enum Frame<'a> {
    /// Parent handshake.
    Hello(Hello),
    /// Broker handshake claim.
    HelloAck(HelloAck),
    /// Native request data.
    Request(Request<'a>),
    /// Native terminal response data.
    Response(Response<'a>),
}

impl Frame<'_> {
    /// Return the explicit wire kind without inspecting sensitive payload data.
    #[must_use]
    pub const fn kind(&self) -> FrameKind {
        match self {
            Self::Hello(_) => FrameKind::Hello,
            Self::HelloAck(_) => FrameKind::HelloAck,
            Self::Request(_) => FrameKind::Request,
            Self::Response(_) => FrameKind::Response,
        }
    }
}

impl fmt::Debug for Frame<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("BrokerFrame")
            .field(&self.kind())
            .finish()
    }
}

/// Expected direction/state supplied by the lifecycle, not inferred from bytes.
///
/// The codec does not advance lifecycle state or establish OS/closure identity.
#[derive(Clone, Copy)]
pub enum ExpectedFrame {
    /// First parent frame for this lane.
    Hello {
        /// Selected lane.
        lane: Lane,
    },
    /// One response to the existing hello and known child handle.
    HelloAck {
        /// Original parent hello.
        hello: Hello,
        /// Existing child-handle PID.
        process_id: u32,
        /// Descriptor-verified application identity.
        executable: ExecutableIdentity,
    },
    /// Next request in the successfully handshaken broker lifetime.
    Request {
        /// Selected lane.
        lane: Lane,
        /// Exact next nonzero identifier.
        id: u64,
    },
    /// One response for the active request.
    Response {
        /// Selected lane.
        lane: Lane,
        /// Active request identifier.
        id: u64,
        /// Original request limits.
        limits: StreamLimits,
    },
}

impl ExpectedFrame {
    pub(super) const fn kind(self) -> FrameKind {
        match self {
            Self::Hello { .. } => FrameKind::Hello,
            Self::HelloAck { .. } => FrameKind::HelloAck,
            Self::Request { .. } => FrameKind::Request,
            Self::Response { .. } => FrameKind::Response,
        }
    }
}
