mod bootstrap;
mod daemon2;

pub use daemon2::{Daemon as MlcDaemon, DaemonConfig as MlcConfig};

#[cfg(test)]
mod tests {
    use tokio::runtime::Builder as RuntimeBuilder;
    use tracing_test::traced_test;

    use super::{MlcConfig, MlcDaemon};
    use crate::daemon_trait::LlmConfig as _;
    use crate::{Generator, LlmDaemon as _};

    #[traced_test]
    #[test]
    fn it_works() -> anyhow::Result<()> {
        let config = MlcConfig::default();
        let url = config.endpoint().join("/v1/completions")?;
        let daemon = MlcDaemon::new(config);

        daemon.fork_daemon()?;

        let runtime = RuntimeBuilder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .expect("failed to create runtime");

        runtime.spawn(daemon.heartbeat());
        runtime.block_on(async {
            let gen = Generator::new(url, None);
            let response = gen
                .generate("<bos>The sum of 7 and 8 is ".to_string())
                .await;
            assert!(response.is_ok());
            let resp_str = response.unwrap();
            assert!(resp_str.contains("15"));
        });
        Ok(())
    }
}
