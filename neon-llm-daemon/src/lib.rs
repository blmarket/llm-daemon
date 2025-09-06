use llm_daemon::{Daemon3, Daemon3Params, LlmDaemon as _};

#[neon::export]
fn start_daemon(hf_repo: String) {
    let daemon = Daemon3::from(Daemon3Params {
        hf_repo,
        args: None, // Some(args),
        port: None,
        server_binary: Some(std::env::current_dir().unwrap().join("server")),
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
