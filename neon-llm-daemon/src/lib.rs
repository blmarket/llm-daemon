use std::path::PathBuf;

use llm_daemon::{Daemon3, Daemon3Params, LlmDaemon as _};
use neon::types::{JsArray, JsValue};

#[neon::export]
fn start_daemon(server_binary: String, hf_repo: String) {
    let daemon = Daemon3::from(Daemon3Params {
        hf_repo,
        args: None, // Some(args),
        port: None,
        server_binary: Some(PathBuf::from(server_binary)),
    });
    let _ = daemon.fork_daemon();
}

// Use #[neon::main] to add additional behavior at module loading time.
// See more at: https://docs.rs/neon/latest/neon/attr.main.html

// #[neon::main]
// fn main(_cx: ModuleContext) -> NeonResult<()> {
//     println!("module is loaded!");
//     Ok(())
// }
