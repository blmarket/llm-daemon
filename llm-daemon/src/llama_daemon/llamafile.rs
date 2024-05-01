use std::path::PathBuf;

use futures::Future;
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};
use tracing::info;

use crate::daemon_trait::LlmConfig;
use crate::util::LlmDaemonCommand;
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
    pub fn from(path: PathBuf, name: &str, port: u16) -> Self {
        Self {
            config: Config {
                llamafile_path: path,
                pid_file: PathBuf::from(format!("/tmp/llm-{}.pid", name)),
                stdout: PathBuf::from(format!("/tmp/llm-{}.stdout", name)),
                stderr: PathBuf::from(format!("/tmp/llm-{}.stderr", name)),
                sock_file: PathBuf::from(format!("/tmp/llm-{}.sock", name)),
                port,
            },
        }
    }
}

impl LlmDaemonCommand<()> for Llamafile {
    fn spawn(&self) -> std::io::Result<(Child, ())> {
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
            .spawn()
            .map(|v| (v, ()));
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
        let inst = Llamafile::from(
            PathBuf::from(std::env!("HOME"))
                .join("proj/Meta-Llama-3-8B-Instruct.Q5_K_M.llamafile"),
            "llama3-8b",
            8123,
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
