use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::daemon_trait::LlmConfig;
use crate::util::LlmDaemonCommand;
use crate::LlmDaemon;

#[derive(Debug, Clone)]
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
        url::Url::parse(&format!("http://127.0.0.1:{}/v1", self.port))
            .expect("failed to parse url")
    }
}

impl Default for LlamaConfig {
    fn default() -> Self {
        LlamaConfig {
            server_path: PathBuf::from(env!("HOME"))
                .join("proj/llama.cpp/build/bin/server"),
            model_path: PathBuf::from(env!("HOME"))
                .join("proj/Meta-Llama-3-8B-Instruct-Q5_K_M.gguf"),
            pid_file: PathBuf::from("/tmp/llamacpp-llama3.pid"),
            stdout: PathBuf::from("/tmp/llamacpp-llama3.stdout"),
            stderr: PathBuf::from("/tmp/llamacpp-llama3.stderr"),
            sock_file: PathBuf::from("/tmp/llamacpp-llama3.sock"),
            port: 28282,
        }
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
            server_path: PathBuf::from(env!("HOME"))
                .join("proj/llama.cpp/build/bin/server"),
            model_path: PathBuf::from(env!("HOME"))
                .join("proj/Meta-Llama-3-8B-Instruct-Q5_K_M.gguf"),
            pid_file: PathBuf::from("/tmp/llama3-llamacpp.pid"),
            stdout: PathBuf::from("/tmp/llama3-llamacpp.stdout"),
            stderr: PathBuf::from("/tmp/llama3-llamacpp.stderr"),
            sock_file: PathBuf::from("/tmp/llama3-llamacpp.sock"),
            port: 28282,
        };

        let phi3: LlamaConfig = LlamaConfig {
            server_path: PathBuf::from(env!("HOME"))
                .join("proj/llama.cpp/build/bin/server"),
            model_path: PathBuf::from(env!("HOME"))
                .join("proj/Phi-3-mini-4k-instruct-q4.gguf"),
            pid_file: PathBuf::from("/tmp/phi3-llamacpp.pid"),
            stdout: PathBuf::from("/tmp/phi3-llamacpp.stdout"),
            stderr: PathBuf::from("/tmp/phi3-llamacpp.stderr"),
            sock_file: PathBuf::from("/tmp/phi3-llamacpp.sock"),
            port: 28283,
        };

        let gemma2b: LlamaConfig = LlamaConfig {
            server_path: PathBuf::from(env!("HOME"))
                .join("proj/llama.cpp/build/bin/server"),
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

pub struct Daemon {
    config: LlamaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Completion {
    content: String,
}

impl Daemon {
    pub fn new(config: LlamaConfig) -> Self {
        Self { config }
    }
}

impl LlmDaemonCommand<()> for Daemon {
    fn spawn(&self) -> std::io::Result<(tokio::process::Child, ())> {
        Command::new(self.config.server_path.clone())
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
}

#[cfg(test)]
mod tests {
    use super::{llama_config_map, Daemon, LlamaConfigs};

    #[test]
    fn launch_daemon() -> anyhow::Result<()> {
        let _ = Daemon::new(llama_config_map()[&LlamaConfigs::Llama3].clone());
        Ok(())
    }
}
