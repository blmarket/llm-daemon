use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use daemonize::Daemonize;
use tokio::io::AsyncWriteExt as _;
use tokio::net::{UnixListener, UnixStream};
// use tokio::net::{UnixListener, UnixStream};
use tokio::process::Child;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{debug, error, info, trace};
use tracing_subscriber::util::SubscriberInitExt as _;

pub trait LlmDaemonCommand {
    type State;
    fn spawn(&self) -> std::io::Result<(Child, Self::State)>;
    fn stdout(&self) -> &PathBuf;
    fn stderr(&self) -> &PathBuf;
    fn pid_file(&self) -> &PathBuf;
    fn sock_file(&self) -> &PathBuf;

    fn fork_daemon(&self) -> anyhow::Result<()> {
        let mut open_opts = OpenOptions::new();
        open_opts.write(true).create(true).truncate(false);
        let stdout = open_opts
            .open(self.stdout())
            .expect("failed to open stdout file");
        let stderr = open_opts
            .open(self.stderr())
            .expect("failed to open stderr file");

        let daemon = Daemonize::new()
            .pid_file(self.pid_file())
            .stdout(stdout)
            .stderr(stderr);

        match daemon.execute() {
            daemonize::Outcome::Child(res) => {
                if let Err(err) = res {
                    // Worst code ever! but I have no other way to inspect err
                    if !format!("{}", err)
                        .starts_with("unable to lock pid file")
                    {
                        eprintln!("{}", err);
                    }
                    exit(0)
                }
                let _guard = tracing_subscriber::FmtSubscriber::builder()
                    .pretty()
                    .with_timer(
                        tracing_subscriber::fmt::time::LocalTime::rfc_3339(),
                    )
                    .with_max_level(tracing::Level::TRACE)
                    .with_writer(std::io::stderr)
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
                    let mut idle_secs: i32 = 0;
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
                               idle_secs = 0;
                           },
                           _ = tokio::time::sleep(Duration::from_secs(10)) => {
                               idle_secs += 10;
                               info!(time_to_close = idle_secs >= 30, "no activity for {} seconds", idle_secs);
                               if idle_secs >= 30 {
                                   break;
                               }
                           },
                        }
                    }
                    // Child might be already killed, so ignore the error
                    cmd.kill().await.ok();
                });
                let delete_sock_err =
                    std::fs::remove_file(self.sock_file()).err();
                let delete_pid_err =
                    std::fs::remove_file(self.pid_file()).err();
                info!(
                    delete_sock_err = format!("{:?}", delete_sock_err),
                    delete_pid_err = format!("{:?}", delete_pid_err),
                    "Server closed"
                );
                exit(0)
            },
            daemonize::Outcome::Parent(res) => {
                res.expect("parent should have no problem");
            },
        };
        Ok(())
    }

    fn ping(&self) -> anyhow::Result<()> {
        let sock_file = self.sock_file().clone();
        let mut stream = std::os::unix::net::UnixStream::connect(&sock_file)?;
        stream.write(&[0])?;
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
