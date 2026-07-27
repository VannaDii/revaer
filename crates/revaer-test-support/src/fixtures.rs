//! Test fixtures and environment helpers.

use std::{
    io::ErrorKind,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

const DOCKER_INFO_TIMEOUT: Duration = Duration::from_secs(2);

#[doc = "Returns `true` if a Docker daemon is reachable for integration tests."]
#[must_use]
pub fn docker_available() -> bool {
    docker_available_with_host(
        std::env::var("DOCKER_HOST").ok().as_deref(),
        Path::new("/var/run/docker.sock"),
    )
}

#[doc = "Returns `true` if Docker is reachable via the provided host or fallback socket path."]
#[must_use]
pub fn docker_available_with_host(host: Option<&str>, default_socket: &Path) -> bool {
    if let Some(host) = host {
        return host
            .strip_prefix("unix://")
            .is_none_or(|path| Path::new(path).exists());
    }

    default_socket.exists()
        || command_succeeds_with_timeout("docker", &["info"], DOCKER_INFO_TIMEOUT)
}

fn command_succeeds_with_timeout(program: &str, args: &[&str], timeout: Duration) -> bool {
    let Ok(mut child) = Command::new(program).args(args).spawn() else {
        return false;
    };
    let started_at = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if started_at.elapsed() >= timeout {
                    if let Err(err) = child.kill()
                        && err.kind() != ErrorKind::InvalidInput
                    {
                        return false;
                    }
                    if child.wait().is_err() {
                        return false;
                    }
                    return false;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::command_succeeds_with_timeout;
    use std::time::{Duration, Instant};

    #[test]
    fn command_timeout_returns_false_without_waiting_for_process_exit() {
        let started_at = Instant::now();

        assert!(!command_succeeds_with_timeout(
            "sleep",
            &["5"],
            Duration::from_millis(50)
        ));
        assert!(started_at.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn command_timeout_reports_successful_process() {
        assert!(command_succeeds_with_timeout(
            "sh",
            &["-c", "exit 0"],
            Duration::from_secs(2)
        ));
    }
}
