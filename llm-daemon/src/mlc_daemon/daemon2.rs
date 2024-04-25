use std::fs::File;
use std::fs::Permissions;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::process::exit;
use std::time::Duration;

use daemonize::{Daemonize, Stdio};
use futures::Future;
use tokio::fs;
use tokio::io::AsyncWriteExt as _;
use tokio::net::UnixListener;
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::trace;
use tracing::warn;
use url::Url;

use crate::daemon_trait::LlmConfig;
use crate::mlc_daemon::bootstrap::PYPROJECT;
use crate::mlc_daemon::bootstrap::SCRIPT;
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
        let args: Vec<&str> = vec![&config.model, "--host", &config.host, "--port", &port_str];

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
                    eprintln!("Maybe another daemon is already running: {:?}", res.err());
                    exit(0)
                }
                let runtime = RuntimeBuilder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .expect("failed to create runtime");
                runtime.block_on(async {
                    let temp_dir = tempfile::tempdir().unwrap();
                    let file1_path = temp_dir.path().join("pyproject.toml");
                    let mut file1 = File::create(file1_path).unwrap();
                    file1.write_all(PYPROJECT.as_bytes()).unwrap();
                    drop(file1);
                    let file2_path = temp_dir.path().join("script.sh");
                    let mut file2 = File::create(file2_path.clone()).unwrap();
                    file2.write_all(SCRIPT.as_bytes()).unwrap();
                    drop(file2);
                    fs::set_permissions(file2_path.clone(), Permissions::from_mode(0o755))
                        .await
                        .expect("failed to set permissions");
                    let mut cmd = Command::new(file2_path)
                        .current_dir(temp_dir.path())
                        .args(args)
                        .spawn()
                        .expect("failed to spawn child");

                    eprintln!("child {:?}", cmd.id());

                    let listener =
                        UnixListener::bind(&config.sock_file).expect("Failed to open socket");
                    let mut sigterms =
                        signal(SignalKind::terminate()).expect("failed to add SIGTERM handler");
                    loop {
                        select! {
                           _ = tokio::time::sleep(Duration::from_secs(10)) => {
                               eprintln!("no activity for 10 seconds, closing...");
                               break;
                           },
                           _ = sigterms.recv() => {
                               eprintln!("Got SIGTERM, closing");
                               break;
                           },
                           exit_status = cmd.wait() => {
                               eprintln!("Child process got closed: {:?}", exit_status);
                               break;
                           }
                           res = listener.accept() => {
                               let (mut stream, _) = res.expect("failed to create socket");
                               let mut buf = [0u8; 32];
                               loop {
                                   stream.readable().await.expect("failed to read");
                                   match stream.try_read(&mut buf) {
                                        Ok(_) => {
                                            eprintln!("Got something, continuing...");
                                        }
                                        Err(_) => {
                                            break;
                                        },
                                    }
                               }
                               stream.shutdown().await.expect("failed to close socket");
                           }
                        }
                    }
                    // Child might be already killed, so ignore the error
                    eprintln!("killing child {:?}", cmd.id());
                    cmd.kill().await.ok();
                    // To make sure temp_dir is alive until here
                    drop(temp_dir);
                });
                std::fs::remove_file(&config.sock_file).ok();
                eprintln!("Server closed");
                exit(0)
            }
            daemonize::Outcome::Parent(res) => {
                res.expect("parent should have no problem");
            }
        }
        Ok(())
    }

    fn heartbeat(&self) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
        let sock_file = self.config.sock_file.clone();
        async move {
            loop {
                trace!("Running scheduled loop");
                let stream = UnixStream::connect(&sock_file).await?;
                stream.writable().await?;
                match stream.try_write(&[0]) {
                    Ok(_) => {}
                    Err(err) => {
                        warn!("Cannot send heartbeat: {:?}", err);
                    }
                };
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;
    use tracing_test::traced_test;

    use crate::{daemon_trait::LlmConfig as _, test_client::Generator, LlmDaemon as _};

    use super::{Daemon, DaemonConfig};

    #[traced_test]
    #[test]
    fn launch_daemon() -> anyhow::Result<()> {
        let conf = DaemonConfig::default();
        let endpoint = conf.endpoint();
        let inst = Daemon::new(conf);

        inst.fork_daemon()?;
        let runtime = Runtime::new()?;
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
