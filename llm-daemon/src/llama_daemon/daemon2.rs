use std::hash::{Hash, Hasher as _};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::debug;

use crate::daemon_trait::LlmConfig;
use crate::util::LlmDaemonCommand;
use crate::LlmDaemon;

#[derive(Debug, Clone)]
pub struct LlamaConfig {
    pub pid_file: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub sock_file: PathBuf,
    pub port: u16,
}

impl LlmConfig for LlamaConfig {
    fn endpoint(&self) -> url::Url {
        url::Url::parse(&format!("http://127.0.0.1:{}/v1", self.port))
            .expect("failed to parse url")
    }

    fn health_url(&self) -> url::Url {
        url::Url::parse(&format!("http://127.0.0.1:{}/health", self.port))
            .expect("failed to parse url")
    }
}

#[derive(Clone)]
pub struct Daemon {
    server_path: PathBuf,
    hf_repo: String,
    config: LlamaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Completion {
    content: String,
}

fn infer_server_path() -> PathBuf {
    let mut server_path = std::env::current_exe().unwrap();
    server_path.pop();
    if server_path.ends_with("deps") {
        server_path.pop();
    }
    // FIXME: for running examples code... But what if the distro location ended up with `examples`?
    if server_path.ends_with("examples") {
        server_path.pop();
    }
    server_path.push("server");
    debug!("Resolved server path: {:?}", &server_path);
    server_path
}

impl From<String> for Daemon {
    fn from(hf_repo: String) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        let _ = &hf_repo.hash(&mut hasher);
        let port = 9000u16 + (hasher.finish() & 0xff) as u16;
        Self {
            server_path: infer_server_path(),
            hf_repo,
            config: LlamaConfig {
                port,
                pid_file: PathBuf::from(format!("/tmp/llm-{}.pid", port)),
                stdout: PathBuf::from(format!("/tmp/llm-{}.stdout", port)),
                stderr: PathBuf::from(format!("/tmp/llm-{}.stderr", port)),
                sock_file: PathBuf::from(format!("/tmp/llm-{}.sock", port)),
            },
        }
    }
}

impl From<(String, u16)> for Daemon {
    fn from(params: (String, u16)) -> Self {
        let (hf_repo, port) = params;
        Self {
            server_path: infer_server_path(),
            hf_repo,
            config: LlamaConfig {
                port,
                pid_file: PathBuf::from(format!("/tmp/llm-{}.pid", port)),
                stdout: PathBuf::from(format!("/tmp/llm-{}.stdout", port)),
                stderr: PathBuf::from(format!("/tmp/llm-{}.stderr", port)),
                sock_file: PathBuf::from(format!("/tmp/llm-{}.sock", port)),
            },
        }
    }
}

impl LlmDaemonCommand for Daemon {
    type State = ();
    fn spawn(&self) -> std::io::Result<(tokio::process::Child, ())> {
        Command::new(&self.server_path)
            .arg("--port")
            .arg(self.config.port.to_string())
            .arg("-ngl")
            .arg("200")
            .arg("-c")
            .arg("8192")
            .arg("--hf-repo")
            .arg(&self.hf_repo)
            .kill_on_drop(true)
            .spawn()
            .map(|c| (c, ()))
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
    type Config = LlamaConfig;

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn fork_daemon(&self) -> anyhow::Result<()> {
        LlmDaemonCommand::fork_daemon(self)
    }

    fn heartbeat<'a, 'b>(
        &'b self,
    ) -> impl futures::prelude::Future<Output = anyhow::Result<()>> + Send + 'a
    where
        'a: 'b,
    {
        LlmDaemonCommand::heartbeat(self)
    }

    fn ping(&self) -> anyhow::Result<()> {
        LlmDaemonCommand::ping(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Daemon;
    use crate::LlmDaemon;

    #[test]
    fn launch_daemon() -> anyhow::Result<()> {
        let daemon: Daemon =
            "microsoft/Phi-3-mini-4k-instruct-gguf".to_string().into();
        daemon.fork_daemon()?;
        Ok(())
    }
}
