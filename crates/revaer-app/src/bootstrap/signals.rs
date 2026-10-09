//! Unix process signals enter the existing joined runtime shutdown path.

use tokio::signal::unix::{Signal, SignalKind, signal};

use crate::{AppError, AppResult};

pub(super) struct ShutdownSignals {
    interrupt: Signal,
    terminate: Signal,
}

impl ShutdownSignals {
    pub(super) fn install() -> AppResult<Self> {
        Ok(Self {
            interrupt: install(SignalKind::interrupt())?,
            terminate: install(SignalKind::terminate())?,
        })
    }

    pub(super) async fn serve(
        mut self,
        api: revaer_api::ApiServer,
        addr: std::net::SocketAddr,
    ) -> revaer_api::ApiServerResult<()> {
        tokio::select! {
            result = api.serve(addr) => result,
            () = self.wait() => Ok(()),
        }
    }

    async fn wait(&mut self) {
        // A closed signal stream also ends serving: no signal authority remains.
        // recv's optional token denotes stream closure, not an I/O failure.
        tokio::select! {
            token = self.interrupt.recv() => report("interrupt", token),
            token = self.terminate.recv() => report("terminate", token),
        }
    }
}

fn install(kind: SignalKind) -> AppResult<Signal> {
    signal(kind).map_err(|source| {
        tracing::error!(error = %source, "shutdown signal registration failed");
        AppError::Io {
            operation: "bootstrap.shutdown_signal.install",
            path: None,
            source,
        }
    })
}

fn report(kind: &str, token: Option<()>) {
    tracing::info!(
        signal = kind,
        stream_closed = token.is_none(),
        "process shutdown requested"
    );
}
