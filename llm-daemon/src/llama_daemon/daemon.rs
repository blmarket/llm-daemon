use std::collections::HashMap;
use std::env;
use std::hash::{Hash, Hasher as _};
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::daemon_trait::LlmConfig;
use crate::util::LlmDaemonCommand;
use crate::LlmDaemon;

#[derive(Debug, Clone)]
pub struct LlamaConfig {
    pub model_path: PathBuf,
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

#[derive(PartialEq, Eq, Hash)]
pub enum LlamaConfigs {
    Llama3,
    Phi3,
    Gemma2b,
}

pub fn llama_config_map() -> &'static HashMap<LlamaConfigs, LlamaConfig> {
    static CONFIG_MAP: OnceLock<HashMap<LlamaConfigs, LlamaConfig>> =
        OnceLock::new();

    CONFIG_MAP.get_or_init(|| {
        let llama3: LlamaConfig = LlamaConfig {
            model_path: PathBuf::from(env!("HOME"))
                .join("proj/Meta-Llama-3-8B-Instruct-Q5_K_M.gguf"),
            pid_file: PathBuf::from("/tmp/llama3-llamacpp.pid"),
            stdout: PathBuf::from("/tmp/llama3-llamacpp.stdout"),
            stderr: PathBuf::from("/tmp/llama3-llamacpp.stderr"),
            sock_file: PathBuf::from("/tmp/llama3-llamacpp.sock"),
            port: 28282,
        };

        let phi3: LlamaConfig = LlamaConfig {
            model_path: PathBuf::from(env!("HOME"))
                .join("proj/Phi-3-mini-4k-instruct-q4.gguf"),
            pid_file: PathBuf::from("/tmp/phi3-llamacpp.pid"),
            stdout: PathBuf::from("/tmp/phi3-llamacpp.stdout"),
            stderr: PathBuf::from("/tmp/phi3-llamacpp.stderr"),
            sock_file: PathBuf::from("/tmp/phi3-llamacpp.sock"),
            port: 28283,
        };

        let gemma2b: LlamaConfig = LlamaConfig {
            model_path: PathBuf::from(env!("HOME"))
                .join("proj/gemma-2b-it-Q5_K_M.gguf"),
            pid_file: PathBuf::from("/tmp/gemma2b-llamacpp.pid"),
            stdout: PathBuf::from("/tmp/gemma2b-llamacpp.stdout"),
            stderr: PathBuf::from("/tmp/gemma2b-llamacpp.stderr"),
            sock_file: PathBuf::from("/tmp/gemma2b-llamacpp.sock"),
            port: 28284,
        };

        HashMap::from_iter([
            (LlamaConfigs::Llama3, llama3),
            (LlamaConfigs::Phi3, phi3),
            (LlamaConfigs::Gemma2b, gemma2b),
        ])
    })
}

#[derive(Clone, Debug)]
pub struct Daemon {
    server_path: PathBuf,
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
    server_path.push("server");
    server_path
}

impl From<(PathBuf, PathBuf)> for Daemon {
    fn from((model_path, server_path): (PathBuf, PathBuf)) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        let _ = &model_path.hash(&mut hasher);
        let port = 9000u16 + (hasher.finish() & 0xff) as u16;
        Self {
            server_path,
            config: LlamaConfig {
                model_path,
                port,
                pid_file: PathBuf::from(format!("/tmp/llm-{}.pid", port)),
                stdout: PathBuf::from(format!("/tmp/llm-{}.stdout", port)),
                stderr: PathBuf::from(format!("/tmp/llm-{}.stderr", port)),
                sock_file: PathBuf::from(format!("/tmp/llm-{}.sock", port)),
            },
        }
    }
}

impl From<PathBuf> for Daemon {
    fn from(model_path: PathBuf) -> Self {
        let server_path = infer_server_path();
        (model_path, server_path).into()
    }
}

impl Daemon {
    pub fn new(config: LlamaConfig) -> Self {
        let server_path = infer_server_path();
        Self {
            server_path,
            config,
        }
    }
}

impl Into<Daemon> for LlamaConfig {
    fn into(self) -> Daemon {
        let mut server_path = std::env::current_exe().unwrap();
        server_path.pop();
        if server_path.ends_with("deps") {
            server_path.pop();
        }
        server_path.push("server");
        if !server_path.exists() {
            panic!("server path not found: {:?}", server_path);
        }
        (self, server_path).into()
    }
}

impl Into<Daemon> for (LlamaConfig, String) {
    fn into(self) -> Daemon {
        let (config, server_path) = self;
        Daemon {
            server_path: PathBuf::from(server_path),
            config,
        }
    }
}

impl Into<Daemon> for (LlamaConfig, PathBuf) {
    fn into(self) -> Daemon {
        let (config, server_path) = self;
        Daemon {
            server_path,
            config,
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
            .arg("-m")
            .arg(&self.config.model_path)
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
    use super::{llama_config_map, Daemon, LlamaConfigs};
    use crate::LlmDaemon;

    #[test]
    fn launch_daemon() -> anyhow::Result<()> {
        let daemon =
            Daemon::new(llama_config_map()[&LlamaConfigs::Gemma2b].clone());
        daemon.fork_daemon()?;
        Ok(())
    }
}
