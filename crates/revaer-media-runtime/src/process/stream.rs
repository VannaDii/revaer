use std::io::{self, Read};
use std::os::fd::{AsFd, BorrowedFd};
use std::process::{ChildStderr, ChildStdout};

use super::{
    NativeProcessError, NativeProcessOutput, NativeProcessRequest, NativeProcessSecondaryEvidence,
};

const READ_BUFFER_BYTES: usize = 8 * 1024;
const MAX_READS_PER_DRAIN: usize = 64;

pub(super) struct ProcessStreams {
    stdout: StreamCapture<ChildStdout>,
    stderr: StreamCapture<ChildStderr>,
}

impl ProcessStreams {
    pub(super) fn new<C>(
        stdout: ChildStdout,
        stderr: ChildStderr,
        request: &NativeProcessRequest,
        mut configure: C,
    ) -> Result<Self, String>
    where
        C: FnMut(BorrowedFd<'_>) -> Result<(), io::Error>,
    {
        configure(stdout.as_fd())
            .map_err(|error| format!("failed to configure stdout capture: {error}"))?;
        configure(stderr.as_fd())
            .map_err(|error| format!("failed to configure stderr capture: {error}"))?;
        Ok(Self {
            stdout: StreamCapture::new(stdout, request.max_stdout_bytes, "stdout"),
            stderr: StreamCapture::new(stderr, request.max_stderr_bytes, "stderr"),
        })
    }

    pub(super) fn drain(&mut self) {
        self.stdout.drain();
        self.stderr.drain();
    }

    pub(super) fn output_limit_error(
        &self,
        request: &NativeProcessRequest,
    ) -> Option<NativeProcessError> {
        if self.stdout.exceeded {
            return Some(NativeProcessError::output_limit_exceeded(
                "stdout",
                request.max_stdout_bytes,
            ));
        }
        self.stderr
            .exceeded
            .then(|| NativeProcessError::output_limit_exceeded("stderr", request.max_stderr_bytes))
    }

    pub(super) fn output_limit_error_from_capture(&self) -> Option<NativeProcessError> {
        if self.stdout.exceeded {
            return Some(NativeProcessError::output_limit_exceeded(
                "stdout",
                self.stdout.maximum_bytes,
            ));
        }
        self.stderr
            .exceeded
            .then(|| NativeProcessError::output_limit_exceeded("stderr", self.stderr.maximum_bytes))
    }

    pub(super) const fn has_read_failure(&self) -> bool {
        self.stdout.failure.is_some() || self.stderr.failure.is_some()
    }

    pub(super) fn evidence(&self) -> NativeProcessSecondaryEvidence {
        let mut evidence = NativeProcessSecondaryEvidence::default();
        for message in [self.stdout.failure.as_ref(), self.stderr.failure.as_ref()]
            .into_iter()
            .flatten()
        {
            evidence.push(message.clone());
        }
        evidence
    }

    pub(super) const fn all_closed(&self) -> bool {
        self.stdout.closed && self.stderr.closed
    }

    pub(super) fn into_output(self) -> NativeProcessOutput {
        NativeProcessOutput {
            stdout: self.stdout.output,
            stderr: self.stderr.output,
        }
    }
}

struct StreamCapture<R> {
    reader: R,
    output: Vec<u8>,
    maximum_bytes: usize,
    stream: &'static str,
    exceeded: bool,
    closed: bool,
    failure: Option<String>,
}

impl<R> StreamCapture<R>
where
    R: Read,
{
    fn new(reader: R, maximum_bytes: usize, stream: &'static str) -> Self {
        Self {
            reader,
            output: Vec::with_capacity(maximum_bytes.min(64 * 1024)),
            maximum_bytes,
            stream,
            exceeded: false,
            closed: false,
            failure: None,
        }
    }

    fn drain(&mut self) {
        if self.closed || self.failure.is_some() {
            return;
        }
        let mut buffer = [0_u8; READ_BUFFER_BYTES];
        for _ in 0..MAX_READS_PER_DRAIN {
            match self.reader.read(&mut buffer) {
                Ok(0) => {
                    self.closed = true;
                    return;
                }
                Ok(count) => self.capture(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                Err(error) => {
                    self.failure = Some(format!("{} read failed: {error}", self.stream));
                    return;
                }
            }
        }
    }

    fn capture(&mut self, bytes: &[u8]) {
        let remaining = self.maximum_bytes.saturating_sub(self.output.len());
        let accepted = remaining.min(bytes.len());
        self.output.extend_from_slice(&bytes[..accepted]);
        if accepted < bytes.len() {
            self.exceeded = true;
        }
    }
}

pub(super) fn configure_nonblocking(descriptor: BorrowedFd<'_>) -> Result<(), io::Error> {
    let flags = rustix::fs::fcntl_getfl(descriptor).map_err(io::Error::from)?;
    rustix::fs::fcntl_setfl(descriptor, flags | rustix::fs::OFlags::NONBLOCK)
        .map_err(io::Error::from)
}

#[cfg(test)]
pub(super) fn deterministic_read_failure() -> NativeProcessSecondaryEvidence {
    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, io::Error> {
            Err(io::Error::other("injected read failure"))
        }
    }

    let mut capture = StreamCapture::new(FailingReader, 16, "stdout");
    capture.drain();
    capture
        .failure
        .map_or_else(NativeProcessSecondaryEvidence::default, |message| {
            NativeProcessSecondaryEvidence::from_message(message)
        })
}
