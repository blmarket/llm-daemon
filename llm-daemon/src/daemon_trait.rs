use futures::Future;
use tracing::{debug, info};
use url::Url;

pub trait LlmConfig {
    fn endpoint(&self) -> Url;
    fn health_url(&self) -> Url;
}

/// Represents a generic daemon capable of performing background tasks, including spawning itself,
/// maintaining a heartbeat, and generating responses based on prompts.
pub trait LlmDaemon {
    type Config: LlmConfig;

    fn config(&self) -> &Self::Config;

    /// Spawns the daemon, initializing any necessary resources or processes.
    /// This method is expected to be called before creation of tokio runtime, mostly
    /// due to the use of the `fork`. User is free to use async runtime after
    /// calling this.
    fn fork_daemon(&self) -> anyhow::Result<()>;

    /// Creates a task which maintains a periodic heartbeat to the daemon.
    /// Daemon is expected to terminate if there's no heartbeat for a certain period of time.
    /// Keeping this task within async runtime will ensure that the daemon is kept running
    /// during the application.
    /// FIXME: Extract heartbeat / ready to Async util?
    fn heartbeat<'a, 'b>(
        &'b self,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'a
    where
        'a: 'b;

    /// Send a single heartbeat to let daemon know there is a client.
    /// Can be useful if you don't want to use heartbeat.
    fn ping(&self) -> anyhow::Result<()>;

    /// FIXME: Extract heartbeat / ready to Async util?
    fn ready<'a>(&self) -> impl Future<Output = ()> + Send + 'a {
        let client = reqwest::Client::new();
        let endpoint = self.config().health_url().clone();
        async move {
            loop {
                debug!("Checking healthcheck endpoint: {}", endpoint.as_str());
                let res = client.get(endpoint.as_str()).send().await;
                info!("{:?}", &res);
                match res {
                    Ok(x) if x.status().is_success() => {
                        break;
                    },
                    _ => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1))
                            .await;
                    },
                }
            }
        }
    }
}
