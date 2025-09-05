#![deny(clippy::all)]

use llm_daemon::{Daemon3, LlmDaemon};
use napi_derive::napi;

#[napi]
pub fn spawn_daemon(hf_repo: String, args: Vec<String>) {
  let daemon = Daemon3::new(hf_repo, args);
  let _ = daemon.fork_daemon();
}
