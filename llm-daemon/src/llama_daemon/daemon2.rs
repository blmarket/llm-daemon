use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use daemonize::{Daemonize, Stdio};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::util::SubscriberInitExt;

use crate::daemon_trait::LlmConfig;
use crate::LlmDaemon;

pub struct LlamaConfig {
    pub server_path: PathBuf,
    pub model_path: PathBuf,
    pub pid_file: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub sock_file: PathBuf,
    pub port: u16,
}

impl LlmConfig for LlamaConfig {
    fn endpoint(&self) -> url::Url {
        url::Url::parse(&format!(
            "http://127.0.0.1:{}/v1/completions",
            self.port
        ))
        .expect("failed to parse url")
    }
}

impl Default for LlamaConfig {
    fn default() -> Self {
        Self {
            server_path: PathBuf::from(env!("HOME"))
                .join("proj/llama.cpp/build/bin/server"),
            model_path: PathBuf::from(env!("HOME"))
                .join("proj/Meta-Llama-3-8B-Instruct-Q5_K_M.gguf"),
            pid_file: PathBuf::from("/tmp/llama-daemon.pid"),
            stdout: PathBuf::from("/tmp/llama-daemon.stdout"),
            stderr: PathBuf::from("/tmp/llama-daemon.stderr"),
            sock_file: PathBuf::from("/tmp/llama-daemon.sock"),
            port: 28282,
        }
    }
}

pub struct Daemon2 {
    config: LlamaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Completion {
    content: String,
}

impl Daemon2 {
    pub fn new(config: LlamaConfig) -> Self {
        Self { config }
    }
}

impl LlmDaemon for Daemon2 {
    fn fork_daemon(&self) -> anyhow::Result<()> {
        let stdout: Stdio = File::create(&self.config.stdout)
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stdout: {:?}", err);
                Stdio::keep()
            });
        let stderr: Stdio = File::create(&self.config.stderr)
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stderr: {:?}", err);
                Stdio::keep()
            });

        let daemon = Daemonize::new()
            .pid_file(self.config.pid_file.clone())
            .stdout(stdout)
            .stderr(stderr);

        match daemon.execute() {
            daemonize::Outcome::Child(res) => {
                if res.is_err() {
                    eprintln!(
                        "Maybe another daemon is already running: {:?}",
                        res.err()
                    );
                    exit(0)
                }
                let _guard = tracing_subscriber::FmtSubscriber::builder()
                    .compact()
                    .with_max_level(tracing::Level::TRACE)
                    .set_default();
                let runtime = RuntimeBuilder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .expect("failed to create runtime");
                runtime.block_on(async {
                    let mut cmd = Command::new(self.config.server_path.clone())
                        .arg("--port")
                        .arg(self.config.port.to_string())
                        .arg("-ngl")
                        .arg("40")
                        .arg("-c")
                        .arg("4096")
                        .arg("-m")
                        .arg(&self.config.model_path)
                        .kill_on_drop(true)
                        .spawn()
                        .expect("should run");

                    let listener =
                        UnixListener::bind(&self.config.sock_file).expect("Failed to open socket");
                    let mut sigterms =
                        signal(SignalKind::terminate()).expect("failed to add SIGTERM handler");
                    loop {
                        select! {
                           _ = sigterms.recv() => {
                               info!("Got SIGTERM, closing");
                               break;
                           },
                           exit_status = cmd.wait() => {
                               error!("Child process got closed: {:?}", exit_status);
                               break;
                           },
                           res = listener.accept() => {
                               let (mut stream, _) = res.expect("failed to create socket");
                               let mut buf = [0u8; 32];
                               loop {
                                   stream.readable().await.expect("failed to read");
                                   match stream.try_read(&mut buf) {
                                        Ok(_) => {
                                            debug!("Got something, continuing...");
                                        }
                                        Err(_) => {
                                            break;
                                        },
                                    }
                               }
                               stream.shutdown().await.expect("failed to close socket");
                           },
                           _ = tokio::time::sleep(Duration::from_secs(10)) => {
                               info!("no activity for 10 seconds, closing...");
                               break;
                           },
                        }
                    }
                    // Child might be already killed, so ignore the error
                    cmd.kill().await.ok();
                });
                std::fs::remove_file(&self.config.sock_file).ok();
                info!("Server closed");
                exit(0)
            },
            daemonize::Outcome::Parent(res) => {
                res.expect("parent should have no problem");
            },
        };
        Ok(())
    }

    fn heartbeat(
        &self,
    ) -> impl futures::prelude::Future<Output = anyhow::Result<()>> + Send + 'static
    {
        let sock_file = self.config.sock_file.clone();
        async move {
            loop {
                trace!("Running scheduled loop");
                let stream = UnixStream::connect(&sock_file).await?;
                stream.writable().await?;
                match stream.try_write(&[0]) {
                    Ok(_) => {},
                    Err(err) => {
                        panic!("something wrong: {}", err);
                    },
                };
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    type Config = LlamaConfig;
}

#[cfg(test)]
mod tests {
    use super::{Daemon2, LlamaConfig};

    #[test]
    fn launch_daemon() -> anyhow::Result<()> {
        let _ = Daemon2::new(LlamaConfig::default());
        Ok(())
    }
}
