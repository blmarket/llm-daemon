use std::fs::File;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use daemonize::{Daemonize, Stdio};
use futures::Future;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;
use tokio::net::{UnixListener, UnixStream};
use tokio::process::{Child, Command};
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::util::SubscriberInitExt;

use crate::daemon_trait::LlmConfig;
use crate::LlmDaemon;

#[derive(Debug)]
pub struct Config {
    pub llamafile_path: PathBuf,
    pub pid_file: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub sock_file: PathBuf,
    pub port: u16,
}

impl LlmConfig for Config {
    fn endpoint(&self) -> url::Url {
        url::Url::parse(&format!("http://127.0.0.1:{}/v1", self.port))
            .expect("failed to parse url")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llamafile_path: PathBuf::new(),
            pid_file: PathBuf::from("/tmp/llamafile-daemon.pid"),
            stdout: PathBuf::from("/tmp/llamafile-daemon.stdout"),
            stderr: PathBuf::from("/tmp/llamafile-daemon.stderr"),
            sock_file: PathBuf::from("/tmp/llamafile-daemon.sock"),
            port: 8123,
        }
    }
}

pub struct Llamafile {
    config: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Completion {
    content: String,
}

impl Llamafile {
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            config: Config {
                llamafile_path: path,
                ..Config::default()
            },
        }
    }
}

impl LlmDaemonCommand for Llamafile {
    fn spawn(&self) -> std::io::Result<Child> {
        info!(
            path = self.config.llamafile_path.to_string_lossy().as_ref(),
            "Executing llamafile"
        );
        // Should use 'sh -c'
        // https://github.com/Mozilla-Ocho/llamafile/issues/7
        let ret = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "{} --port {} -ngl 99 -c 4096 --nobrowser",
                self.config.llamafile_path.to_string_lossy(),
                self.config.port
            ))
            .kill_on_drop(true)
            .spawn();
        info!("Child spawned successfully");
        ret
    }

    fn stdout(&self) -> &PathBuf {
        &self.config.stdout
    }

    fn stderr(&self) -> &PathBuf {
        &self.config.stderr
    }

    fn pid_file(&self) -> &PathBuf {
        &self.config.pid_file
    }

    fn sock_file(&self) -> &PathBuf {
        &self.config.sock_file
    }
}

trait LlmDaemonCommand {
    fn spawn(&self) -> std::io::Result<Child>;
    fn stdout(&self) -> &PathBuf;
    fn stderr(&self) -> &PathBuf;
    fn pid_file(&self) -> &PathBuf;
    fn sock_file(&self) -> &PathBuf;

    fn fork_daemon(&self) -> anyhow::Result<()> {
        let stdout: Stdio = File::create(self.stdout())
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stdout: {:?}", err);
                Stdio::keep()
            });
        let stderr: Stdio = File::create(self.stderr())
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stderr: {:?}", err);
                Stdio::keep()
            });

        let daemon = Daemonize::new()
            .pid_file(self.pid_file())
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
                    info!("Starting server");
                    let mut cmd = match self.spawn() {
                        Ok(v) => v,
                        Err(err) => {
                            error!(err = format!("{:?}", err), "failed to execute server");
                            exit(-1)
                        },
                    };

                    let listener =
                        UnixListener::bind(self.sock_file()).expect("Failed to open socket");
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
                                        Ok(len) => {
                                            debug!(len = len, "Got heartbeat");
                                            if len == 0 {
                                                // no more data to get
                                                break;
                                            }
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
                std::fs::remove_file(self.sock_file()).ok();
                info!("Server closed");
                exit(0)
            },
            daemonize::Outcome::Parent(res) => {
                res.expect("parent should have no problem");
            },
        };
        Ok(())
    }

    fn heartbeat<'a, 'b>(
        &'b self,
    ) -> impl futures::prelude::Future<Output = anyhow::Result<()>> + Send + 'a
    where
        'a: 'b,
    {
        let sock_file = self.sock_file().clone();
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
}

impl LlmDaemon for Llamafile {
    fn fork_daemon(&self) -> anyhow::Result<()> {
        LlmDaemonCommand::fork_daemon(self)
    }

    type Config = Config;

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn heartbeat<'a, 'b>(
        &'b self,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'a
    where
        'a: 'b,
    {
        let rr = self;
        let ret = LlmDaemonCommand::heartbeat(rr);
        ret
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tokio::runtime::Builder as RuntimeBuilder;
    use tracing_test::traced_test;

    use super::Llamafile;
    use crate::{Generator, LlmConfig as _, LlmDaemon};

    #[traced_test]
    #[test]
    fn it_works() -> anyhow::Result<()> {
        let inst = Llamafile::from_path(
            PathBuf::from(std::env!("HOME"))
                .join("proj/Meta-Llama-3-8B-Instruct.Q5_K_M.llamafile"),
        );
        inst.fork_daemon()?;
        let url = inst.config().endpoint().join("/completion")?;

        let runtime = RuntimeBuilder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .expect("failed to create runtime");

        runtime.spawn(inst.heartbeat());
        runtime.block_on(async {
            let gen = Generator::new(url, None);
            let response = gen
                .generate("<|begin_of_text|>The sum of 7 and 8 is ".to_string())
                .await;
            assert!(response.is_ok());
            assert!(response.unwrap().contains("15"));
        });
        Ok(())
    }
}
