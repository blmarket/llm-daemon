mod daemon;
mod daemon2;
mod daemon3;
pub mod daemon_ext;

pub use daemon::{llama_config_map, Daemon, LlamaConfig, LlamaConfigs};
pub use daemon2::Daemon as Daemon2;
pub use daemon3::Daemon3;

#[cfg(test)]
mod tests {
    use tokio::runtime::Builder as RuntimeBuilder;
    use tracing_test::traced_test;

    use crate::daemon_trait::LlmConfig as _;
    use crate::{
        llama_config_map, Generator, LlamaConfigs, LlamaDaemon as Daemon,
        LlmDaemon as _,
    };

    #[traced_test]
    #[test]
    fn it_works() -> anyhow::Result<()> {
        let config = llama_config_map()[&LlamaConfigs::Gemma2b].clone();
        let url = config.endpoint().join("/completion")?;
        let inst: Daemon = config.into();
        inst.fork_daemon()?;

        let runtime = RuntimeBuilder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .expect("failed to create runtime");

        runtime.spawn(inst.heartbeat());
        runtime.block_on(async {
            inst.ready().await;
            let gen = Generator::new(url, None);
            let response = gen
                .generate("<|begin_of_text|>The sum of 7 and 8 is ".to_string())
                .await;
            assert!(response.is_ok());
            assert!(response.unwrap().contains("15"));
        });
        Ok(())
    }

    #[traced_test]
    #[ignore = "model is not available in my devenv"]
    #[test]
    fn it_works_with_phi3() -> anyhow::Result<()> {
        let config = llama_config_map()[&LlamaConfigs::Phi3].clone();
        let url = config.endpoint().join("/completion")?;
        let inst = Daemon::new(config);
        inst.fork_daemon()?;

        let runtime = RuntimeBuilder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .expect("failed to create runtime");

        runtime.spawn(inst.heartbeat());
        runtime.block_on(async {
            let gen = Generator::new(url, None);
            let response = gen
                .generate("<|begin_of_text|>The sum of 7 and 8 is ".to_string())
                .await;
            assert!(response.is_ok());
            assert!(response.unwrap().contains("15"));
        });
        Ok(())
    }
}
