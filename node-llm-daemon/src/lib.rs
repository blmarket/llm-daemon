#![deny(clippy::all)]

use llm_daemon::{Daemon3, Daemon3Params, LlmDaemon};
use napi_derive::napi;
use std::path::PathBuf;

#[napi]
pub fn spawn_daemon(hf_repo: String, args: Vec<String>) {
  let daemon = Daemon3::new(Daemon3Params {
    hf_repo,
    args: Some(args),
    port: None,
    server_binary: Some(PathBuf::from("server.linux-x64-gnu")),
  });
  let _ = daemon.fork_daemon();
}
