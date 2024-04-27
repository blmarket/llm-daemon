use llm_daemon::{LlamaConfig, LlamaDaemon, LlmDaemon};

fn main() -> anyhow::Result<()> {
    let daemon = LlamaDaemon::new(LlamaConfig::default());
    daemon.fork_daemon()?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.spawn(daemon.heartbeat());
    runtime.block_on(async {
        println!("Hello world");
        Ok(())
    })
}
