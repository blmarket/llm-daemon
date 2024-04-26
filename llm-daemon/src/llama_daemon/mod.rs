mod daemon2;

pub use daemon2::{Daemon2, LlamaConfig};

#[cfg(test)]
mod tests {
    use tokio::runtime::Builder as RuntimeBuilder;
    use tracing_test::traced_test;

    use super::{Daemon2 as Daemon, LlamaConfig};
    use crate::daemon_trait::LlmConfig as _;
    use crate::{Generator, LlmDaemon as _};

    #[traced_test]
    #[test]
    fn it_works() -> anyhow::Result<()> {
        let config = LlamaConfig::default();
        let url = config.endpoint();
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
