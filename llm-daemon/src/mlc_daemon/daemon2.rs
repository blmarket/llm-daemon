use std::fs::{File, Permissions};
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use daemonize::{Daemonize, Stdio};
use futures::Future;
use tempfile::TempDir;
use tokio::io::AsyncWriteExt as _;
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{info, trace, warn};
use url::Url;

use crate::daemon_trait::LlmConfig;
use crate::mlc_daemon::bootstrap::{PYPROJECT, SCRIPT};
use crate::LlmDaemon;

pub struct DaemonConfig {
    pub sock_file: String,
    pub pid_file: String,
    pub stdout: String,
    pub stderr: String,
    // default: 127.0.0.1
    pub host: String,
    // default: 8000
    pub port: u16,
    // default: HF://mlc-ai/gemma-2b-it-q4f16_1-MLC
    pub model: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            sock_file: "/tmp/mlc-daemon2.sock".to_string(),
            pid_file: "/tmp/mlc-daemon2.pid".to_string(),
            stdout: "/tmp/mlc-daemon2.stdout".to_string(),
            stderr: "/tmp/mlc-daemon2.stderr".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            model: "HF://mlc-ai/gemma-2b-it-q4f16_1-MLC".to_string(),
        }
    }
}

impl LlmConfig for DaemonConfig {
    fn endpoint(&self) -> Url {
        url::Url::parse(&format!(
            "http://{}:{}/v1/completions",
            self.host, self.port
        ))
        .expect("failed to parse url")
    }
}

pub struct Daemon {
    config: DaemonConfig,
}

impl Daemon {
    pub fn new(config: DaemonConfig) -> Self {
        Self { config }
    }
}

impl LlmDaemon for Daemon {
    type Config = DaemonConfig;

    fn fork_daemon(&self) -> anyhow::Result<()> {
        let config = &self.config;
        let port_str = config.port.to_string();
        let args: Vec<&str> =
            vec![&config.model, "--host", &config.host, "--port", &port_str];

        let stdout: Stdio = File::create(&config.stdout)
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stdout: {:?}", err);
                Stdio::keep()
            });
        let stderr: Stdio = File::create(&config.stderr)
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stderr: {:?}", err);
                Stdio::keep()
            });

        let daemon = Daemonize::new()
            .pid_file(&config.pid_file)
            .stdout(stdout)
            .stderr(stderr);

        match daemon.execute() {
            daemonize::Outcome::Child(res) => {
                if res.is_err() {
                    let _ = std::io::stdout().write_all(format!("Failed to spawn a daemon - maybe another daemon is already running?: {:?}\n", res.err()).as_bytes());
                    exit(0)
                }
                let _ = std::io::stdout().write_all(
                    format!("Daemon spawned: {:?}\n", std::env::current_dir())
                        .as_bytes(),
                );
                let runtime = RuntimeBuilder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .expect("failed to create runtime");
                runtime.block_on(async {
                    let bootstrap: anyhow::Result<(TempDir, PathBuf)> = (|| {
                        let temp_dir = tempfile::tempdir()?;
                        let _ = std::io::stdout().write_all(format!("temp dir: {:?}\n", temp_dir.path()).as_bytes());
                        let file1_path = temp_dir.path().join("pyproject.toml");
                        let mut file1 = File::create(file1_path)?;
                        file1.write_all(PYPROJECT.as_bytes())?;
                        file1.sync_all()?;
                        let file2_path = temp_dir.path().join("script.sh");
                        let mut file2 = File::create(file2_path.clone())?;
                        file2.write_all(SCRIPT.as_bytes())?;
                        file2.sync_all()?;
                        std::fs::set_permissions(file2_path.clone(), Permissions::from_mode(0o755))?;
                        Ok((temp_dir, file2_path))
                    })();

                    let Ok((temp_dir, file2_path)) = bootstrap else {
                        let _ = std::io::stdout().write_all(format!("failed to bootstrap: {:?}\n", bootstrap.err()).as_bytes());
                        panic!("what should I do");
                    };
                    let mut cmd = Command::new(file2_path)
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .current_dir(temp_dir.path())
                        .args(args)
                        .spawn()
                        .expect("failed to spawn child");

                    let _ = std::io::stdout().write_all(format!("child: {:?}\n", cmd.id()).as_bytes());

                    let listener =
                        UnixListener::bind(&config.sock_file).expect("Failed to open socket");
                    let mut sigterms =
                        signal(SignalKind::terminate()).expect("failed to add SIGTERM handler");
                    let _ = std::io::stdout().write_all("Starting loop\n".as_bytes());
                    let _ = std::io::stdout().flush();
                    loop {
                        select! {
                           _ = sigterms.recv() => {
                               let _ = std::io::stdout().write_all("Got SIGTERM, closing\n".as_bytes());
                               break;
                           },
                           exit_status = cmd.wait() => {
                               let _ = std::io::stdout().write_all(format!("Child got closed: {:?}\n", exit_status).as_bytes());
                               break;
                           },
                           res = listener.accept() => {
                               let Ok((mut stream, _)) = res else {
                                   let _ = std::io::stdout().write_all("Failed to accept a socket, closing\n".as_bytes());
                                   break;
                               };
                               let mut buf = [0u8; 32];
                               loop {
                                   let _ = stream.readable().await;
                                   match stream.try_read(&mut buf) {
                                        Ok(sz) => {
                                            let _ = std::io::stdout().write_all(format!("Got {:?}, continue...\n", sz).as_bytes());
                                            break;
                                        },
                                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                            continue;
                                        },
                                        Err(err) => {
                                            let _ = std::io::stdout().write_all(format!("Error {:?}, exiting...\n", err).as_bytes());
                                            break;
                                        }
                                    }
                               }
                               let _ = stream.shutdown().await;
                           },
                           _ = tokio::time::sleep(Duration::from_secs(2)) => {
                               let _ = std::io::stdout().write_all("No activitiy for 10 seconds, closing...\n".as_bytes());
                               break;
                           },
                        }
                    }
                    // Child might be already killed, so ignore the error
                    let _ = std::io::stdout().write_all(format!("Closing child: {:?}\n", cmd.id()).as_bytes());
                    cmd.kill().await.ok();
                    // To make sure temp_dir is alive until here
                    drop(temp_dir);
                });
                std::fs::remove_file(&config.sock_file).ok();
                let _ = std::io::stdout()
                    .write_all("Server closed\n".as_bytes());
                exit(0)
            },
            daemonize::Outcome::Parent(res) => {
                res.expect("parent should have no problem");
            },
        }
        Ok(())
    }

    fn heartbeat(
        &self,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
        let sock_file = self.config.sock_file.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                trace!(sock_file = &sock_file, "Running scheduled loop");
                if std::fs::metadata(&sock_file).is_err() {
                    trace!(
                        sock_file = &sock_file,
                        "Cannot get file metadata, retry later"
                    );
                    continue;
                }
                let stream = UnixStream::connect(&sock_file).await?;
                stream.writable().await?;
                match stream.try_write(&[0]) {
                    Ok(_) => {
                        info!("sent heartbeat");
                    },
                    Err(err) => {
                        warn!("Cannot send heartbeat: {:?}", err);
                    },
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Builder as RuntimeBuilder;
    use tracing_test::traced_test;

    use super::{Daemon, DaemonConfig};
    use crate::daemon_trait::LlmConfig as _;
    use crate::test_client::Generator;
    use crate::LlmDaemon as _;

    #[traced_test]
    #[test]
    fn launch_daemon() -> anyhow::Result<()> {
        let conf = DaemonConfig::default();
        let endpoint = conf.endpoint();
        let inst = Daemon::new(conf);

        inst.fork_daemon()?;
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .expect("failed to create runtime");
        runtime.spawn(inst.heartbeat());
        runtime.block_on(async {
            let gen = Generator::new(endpoint, None);
            let resp = gen
                .generate("<bos>Sum of 7 and 8 is ".to_string())
                .await
                .expect("failed to generate");
            assert!(resp.contains("15"));
        });
        Ok(())
    }
}
