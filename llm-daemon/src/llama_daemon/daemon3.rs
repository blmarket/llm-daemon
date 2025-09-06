use std::borrow::Borrow;
use std::hash::{Hash, Hasher as _};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, info};

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

#[derive(Clone, Debug)]
pub struct Daemon3Params {
    pub hf_repo: String,
    pub args: Option<Vec<String>>,
    pub port: Option<u16>,
    pub server_binary: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct Daemon3 {
    server_path: PathBuf,
    hf_repo: String,
    config: LlamaConfig,
    custom_args: Vec<String>,
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

impl From<Daemon3Params> for Daemon3 {
    fn from(params: Daemon3Params) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        let _ = &params.hf_repo.hash(&mut hasher);
        let port = params.port.unwrap_or_else(|| 9000u16 + (hasher.finish() & 0xff) as u16);
        let server_path = params.server_binary.unwrap_or_else(infer_server_path);
        let custom_args = params.args.unwrap_or_default();
        Self {
            server_path,
            hf_repo: params.hf_repo,
            config: LlamaConfig {
                port,
                pid_file: PathBuf::from(format!("/tmp/llm-{}.pid", port)),
                stdout: PathBuf::from(format!("/tmp/llm-{}.stdout", port)),
                stderr: PathBuf::from(format!("/tmp/llm-{}.stderr", port)),
                sock_file: PathBuf::from(format!("/tmp/llm-{}.sock", port)),
            },
            custom_args,
        }
    }
}

impl Daemon3 {
    pub fn new(params: Daemon3Params) -> Self {
        params.into()
    }

    #[deprecated(note = "Use Daemon3::new with Daemon3Params instead")]
    pub fn with_port(
        hf_repo: String,
        port: u16,
        custom_args: Vec<String>,
    ) -> Self {
        Daemon3Params {
            hf_repo,
            args: Some(custom_args),
            port: Some(port),
            server_binary: None,
        }.into()
    }
}

impl LlmDaemonCommand for Daemon3 {
    type State = ();

    fn spawn(&self) -> std::io::Result<(tokio::process::Child, ())> {
        let daemon = self;
        info!("Server path: {:?}", &daemon.server_path);
        let mut cmd = Command::new(&daemon.server_path);

        // Add default arguments
        cmd.arg("--port")
            .arg(daemon.config.port.to_string())
            .arg("-hf")
            .arg(&daemon.hf_repo);

        // Add custom arguments provided by user
        for arg in &daemon.custom_args {
            cmd.arg(arg);
        }

        cmd.kill_on_drop(true).spawn().map(|c| (c, ()))
    }

    fn stdout(&self) -> &PathBuf {
        &self.borrow().config.stdout
    }

    fn stderr(&self) -> &PathBuf {
        &self.borrow().config.stderr
    }

    fn pid_file(&self) -> &PathBuf {
        &self.borrow().config.pid_file
    }

    fn sock_file(&self) -> &PathBuf {
        &self.borrow().config.sock_file
    }
}

impl LlmDaemon for Daemon3 {
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
    use super::Daemon3;
    use crate::LlmDaemon;

    #[test]
    fn launch_daemon() -> anyhow::Result<()> {
        let daemon = Daemon3::new(Daemon3Params {
            hf_repo: "microsoft/Phi-3-mini-4k-instruct-gguf".to_string(),
            args: Some(vec!["-ngl".to_string(), "99".to_string(), "-fa".to_string()]),
            port: None,
            server_binary: None,
        });
        daemon.fork_daemon()?;
        Ok(())
    }
}
