use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, Criterion};
use llm_daemon::{
    LlamaConfig, LlamaDaemon, LlmConfig as _, LlmDaemon, MlcConfig, MlcDaemon,
    Proxy, ProxyConfig,
};
use tokio::runtime::Builder as RuntimeBuilder;
use url::Url;

fn llama_config() -> (Url, impl LlmDaemon) {
    let config = LlamaConfig::default();
    let endpoint = config.endpoint();
    let daemon = LlamaDaemon::new(config);

    (endpoint, daemon)
}

fn mlc_config() -> (Url, impl LlmDaemon) {
    let config = MlcConfig::default();
    let endpoint = config.endpoint();
    let daemon = MlcDaemon::new(config);

    (endpoint, daemon)
}

fn proxy_config() -> (Url, impl LlmDaemon) {
    let config = ProxyConfig::default();
    let endpoint = config.endpoint();
    let daemon = Proxy::new(config, llama_config().1);

    (endpoint, daemon)
}

fn time_to_port_open<T: LlmDaemon>(
    conf: impl Fn() -> (Url, T),
) -> anyhow::Result<()> {
    let (endpoint, daemon) = conf();

    daemon.fork_daemon()?;
    let runtime = RuntimeBuilder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .expect("failed to create runtime");
    let handle = runtime.spawn(daemon.heartbeat());
    runtime.block_on(async move {
        let client = reqwest::Client::new();
        loop {
            if client.get(endpoint.to_string()).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    handle.abort();

    Ok(())
}

fn cleanup() {
    std::thread::sleep(Duration::from_secs(11));
}

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("port_open");

    group.bench_function("llama_port_open", |b| {
        b.iter_custom(|iters| {
            let mut acc = Duration::ZERO;
            for _i in 0..iters {
                let start = Instant::now();
                let _ = time_to_port_open(llama_config);
                acc += start.elapsed();
                cleanup();
            }
            acc
        })
    });

    group.bench_function("mlc_port_open", |b| {
        b.iter_custom(|iters| {
            let mut acc = Duration::ZERO;
            for _i in 0..iters {
                let start = Instant::now();
                let _ = time_to_port_open(mlc_config);
                acc += start.elapsed();
                cleanup();
            }
            acc
        })
    });

    group.bench_function("proxy_port_open", |b| {
        b.iter_custom(|iters| {
            let mut acc = Duration::ZERO;
            for _i in 0..iters {
                let start = Instant::now();
                let _ = time_to_port_open(proxy_config);
                acc += start.elapsed();
                cleanup();
            }
            acc
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = benchmark
}
criterion_main!(benches);
