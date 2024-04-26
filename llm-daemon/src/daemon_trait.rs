use futures::Future;
use url::Url;

pub trait LlmConfig {
    fn endpoint(&self) -> Url;
}

/// Represents a generic daemon capable of performing background tasks, including spawning itself,
/// maintaining a heartbeat, and generating responses based on prompts.
pub trait LlmDaemon {
    type Config: LlmConfig;

    /// Spawns the daemon, initializing any necessary resources or processes.
    /// This method is expected to be called before creation of tokio runtime, mostly
    /// due to the use of the `fork`. User is free to use async runtime after
    /// calling this.
    fn fork_daemon(&self) -> anyhow::Result<()>;

    /// Creates a task which maintains a periodic heartbeat to the daemon.
    /// Daemon is expected to terminate if there's no heartbeat for a certain period of time.
    /// Keeping this task within async runtime will ensure that the daemon is kept running
    /// during the application.
    fn heartbeat(
        &self,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + 'static;
}
