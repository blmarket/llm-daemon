use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use daemonize::{Daemonize, Stdio};
use tokio::io::AsyncWriteExt as _;
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Child;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::util::SubscriberInitExt;

pub trait LlmDaemonCommand<S> {
    fn spawn(&self) -> std::io::Result<(Child, S)>;
    fn stdout(&self) -> &PathBuf;
    fn stderr(&self) -> &PathBuf;
    fn pid_file(&self) -> &PathBuf;
    fn sock_file(&self) -> &PathBuf;

    fn fork_daemon(&self) -> anyhow::Result<()> {
        let mut open_opts = OpenOptions::new();
        open_opts.write(true).create(true).truncate(false);
        let stdout: Stdio = open_opts.open(self.stdout())
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stdout: {:?}", err);
                Stdio::keep()
            });
        let stderr: Stdio = open_opts.open(self.stderr())
            .map(|v| v.into())
            .unwrap_or_else(|err| {
                warn!("failed to open stderr: {:?}", err);
                Stdio::keep()
            });

        let daemon = Daemonize::new()
            .pid_file(self.pid_file())
            .stdout(stdout)
            .stderr(stderr);

        match daemon.execute() {
            daemonize::Outcome::Child(res) => {
                if let Err(err) = res {
                    // Worst code ever! but I have no other way to inspect err
                    if !format!("{}", err).starts_with("unable to lock pid file")
                    {
                        eprintln!("{}", err);
                    }
                    exit(0)
                }
                let _guard = tracing_subscriber::FmtSubscriber::builder()
                    .compact()
                    .with_max_level(tracing::Level::TRACE)
                    .set_default();
                let runtime = RuntimeBuilder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .expect("failed to create runtime");
                runtime.block_on(async {
                    info!("Starting server");
                    let (mut cmd, _guard_state) = match self.spawn() {
                        Ok(v) => v,
                        Err(err) => {
                            error!(err = format!("{:?}", err), "failed to execute server");
                            exit(-1)
                        },
                    };

                    let listener =
                        UnixListener::bind(self.sock_file()).expect("Failed to open socket");
                    let mut sigterms =
                        signal(SignalKind::terminate()).expect("failed to add SIGTERM handler");
                    loop {
                        select! {
                           _ = sigterms.recv() => {
                               info!("Got SIGTERM, closing");
                               break;
                           },
                           exit_status = cmd.wait() => {
                               error!("Child process got closed: {:?}", exit_status);
                               break;
                           },
                           res = listener.accept() => {
                               let (mut stream, _) = res.expect("failed to create socket");
                               let mut buf = [0u8; 32];
                               loop {
                                   stream.readable().await.expect("failed to read");
                                   match stream.try_read(&mut buf) {
                                        Ok(len) => {
                                            debug!(len = len, "Got heartbeat");
                                            if len == 0 {
                                                // no more data to get
                                                break;
                                            }
                                        }
                                        Err(_) => {
                                            break;
                                        },
                                    }
                               }
                               stream.shutdown().await.expect("failed to close socket");
                           },
                           _ = tokio::time::sleep(Duration::from_secs(60)) => {
                               info!("no activity for 60 seconds, closing...");
                               break;
                           },
                        }
                    }
                    // Child might be already killed, so ignore the error
                    cmd.kill().await.ok();
                });
                std::fs::remove_file(self.sock_file()).ok();
                info!("Server closed");
                exit(0)
            },
            daemonize::Outcome::Parent(res) => {
                res.expect("parent should have no problem");
            },
        };
        Ok(())
    }

    fn heartbeat<'a, 'b>(
        &'b self,
    ) -> impl futures::prelude::Future<Output = anyhow::Result<()>> + Send + 'a
    where
        'a: 'b,
    {
        let sock_file = self.sock_file().clone();
        async move {
            loop {
                trace!("Running scheduled loop");
                let stream = UnixStream::connect(&sock_file).await?;
                stream.writable().await?;
                match stream.try_write(&[0]) {
                    Ok(_) => {},
                    Err(err) => {
                        panic!("something wrong: {}", err);
                    },
                };
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
