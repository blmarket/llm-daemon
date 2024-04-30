use std::fs::{File, Permissions};
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;

use futures::Future;
use tempfile::TempDir;
use tokio::process::Command;
use url::Url;

use crate::daemon_trait::LlmConfig;
use crate::mlc_daemon::bootstrap::{script, PYPROJECT};
use crate::util::LlmDaemonCommand;
use crate::LlmDaemon;

pub struct DaemonConfig {
    pub sock_file: PathBuf,
    pub pid_file: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub host: String,
    pub port: u16,
    pub model: String,
    pub python: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            sock_file: PathBuf::from("/tmp/mlc-daemon2.sock"),
            pid_file: PathBuf::from("/tmp/mlc-daemon2.pid"),
            stdout: PathBuf::from("/tmp/mlc-daemon2.stdout"),
            stderr: PathBuf::from("/tmp/mlc-daemon2.stderr"),
            host: "127.0.0.1".to_string(),
            port: 8000,
            model: "HF://mlc-ai/gemma-2b-it-q4f16_1-MLC".to_string(),
            python: "python3.11".to_string(),
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

struct State {
    // This will keep the tempdir stay alive
    #[allow(unused)]
    temp_dir: TempDir,
}

impl LlmDaemonCommand<State> for Daemon {
    fn spawn(&self) -> std::io::Result<(tokio::process::Child, State)> {
        let bootstrap: anyhow::Result<(TempDir, PathBuf)> = (|| {
            let temp_dir = tempfile::tempdir()?;
            let _ = std::io::stdout().write_all(
                format!("temp dir: {:?}\n", temp_dir.path()).as_bytes(),
            );
            let file1_path = temp_dir.path().join("pyproject.toml");
            let mut file1 = File::create(file1_path)?;
            file1.write_all(PYPROJECT.as_bytes())?;
            file1.sync_all()?;
            let file2_path = temp_dir.path().join("script.sh");
            let mut file2 = File::create(file2_path.clone())?;
            file2.write_all(script(&self.config.python).as_bytes())?;
            file2.sync_all()?;
            std::fs::set_permissions(
                file2_path.clone(),
                Permissions::from_mode(0o755),
            )?;
            Ok((temp_dir, file2_path))
        })();
        let Ok((temp_dir, file2_path)) = bootstrap else {
            let _ = std::io::stdout().write_all(
                format!("failed to bootstrap: {:?}\n", bootstrap.err())
                    .as_bytes(),
            );
            panic!("what should I do");
        };
        let port = self.config.port.to_string();
        let args: Vec<&str> = vec![
            &self.config.model,
            "--host",
            &self.config.host,
            "--port",
            &port,
        ];
        Command::new(file2_path)
            .current_dir(temp_dir.path())
            .args(args)
            .kill_on_drop(true)
            .spawn()
            .map(|v| (v, State { temp_dir }))
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

impl LlmDaemon for Daemon {
    type Config = DaemonConfig;

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn fork_daemon(&self) -> anyhow::Result<()> {
        LlmDaemonCommand::fork_daemon(self)
    }

    fn heartbeat<'a, 'b>(
        &'b self,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'a
    where
        'a: 'b,
    {
        LlmDaemonCommand::heartbeat(self)
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
            inst.ready().await;
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
