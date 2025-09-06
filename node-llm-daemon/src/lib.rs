#![deny(clippy::all)]

use llm_daemon::{Daemon3, Daemon3Params, LlmDaemon};
use napi_derive::napi;

#[napi]
pub fn spawn_daemon(hf_repo: String, args: Vec<String>) {
  let daemon = Daemon3::from(Daemon3Params {
    hf_repo,
    args: Some(args),
    port: None,
    server_binary: Some(std::env::current_dir().unwrap().join("artifacts/server")),
  });
  let _ = daemon.fork_daemon();
}
