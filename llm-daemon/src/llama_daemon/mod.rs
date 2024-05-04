mod daemon_ext;
mod llamafile;
pub mod daemon;

pub use daemon_ext::{llama_config_map, Daemon, LlamaConfig, LlamaConfigs};
pub use llamafile::{Config as LlamafileConfig, Llamafile};

#[cfg(test)]
mod tests {
    use tokio::runtime::Builder as RuntimeBuilder;
    use tracing_test::traced_test;

    use super::{Daemon, LlamaConfig};
    use crate::daemon_trait::LlmConfig as _;
    use crate::llama_daemon::{llama_config_map, LlamaConfigs};
    use crate::{Generator, LlmDaemon as _};

    #[traced_test]
    #[test]
    fn it_works() -> anyhow::Result<()> {
        let config = LlamaConfig::default();
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

    #[traced_test]
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
