use std::sync::Arc;

use axum::body::BodyDataStream;
use axum::extract::State;
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use hyper::body::Incoming;
use hyper::{Response, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::time::Instant;
use tracing::{debug, warn};

use crate::daemon_trait::LlmConfig;
use crate::LlmDaemon;

pub struct ProxyConfig {
    port: u16,
}

impl LlmConfig for ProxyConfig {
    fn endpoint(&self) -> url::Url {
        url::Url::parse(&format!(
            "http://127.0.0.1:{}/v1/completions",
            self.port
        ))
        .expect("failed to parse url")
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self { port: 8282 }
    }
}

pub struct Proxy<D: LlmDaemon> {
    config: ProxyConfig,
    inner: D,
}

impl<D: LlmDaemon> Proxy<D> {
    pub fn new(config: ProxyConfig, inner: D) -> Self {
        Self { config, inner }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Completion {
    content: String,
}

impl<D: LlmDaemon> LlmDaemon for Proxy<D> {
    type Config = ProxyConfig;

    fn fork_daemon(&self) -> anyhow::Result<()> {
        self.inner.fork_daemon()
    }

    fn heartbeat(
        &self,
    ) -> impl futures::prelude::Future<Output = anyhow::Result<()>> + Send + 'static
    {
        let port = self.config.port;
        let hb = self.inner.heartbeat();
        async move {
            let (r1, r2) = futures::join!(hb, run_proxy(port));
            r1?;
            r2?;
            Ok(())
        }
    }
}

async fn inner(
    client: &Client<HttpConnector, BodyDataStream>,
    req: axum::extract::Request,
) -> Result<Response<Incoming>, hyper_util::client::legacy::Error> {
    // FIXME: 28282 -> configured port
    let mut req_builder = hyper::Request::builder()
        .uri("http://127.0.0.1:28282/v1/completions")
        .method(req.method());
    let headers = req_builder.headers_mut().unwrap();
    req.headers().into_iter().for_each(|(name, value)| {
        headers.append(name, value.clone());
    });

    let request = req_builder
        .body(req.into_body().into_data_stream())
        .unwrap();

    client.request(request).await
}

async fn handle_proxy(
    State((sem, client)): State<(
        Arc<Semaphore>,
        Client<HttpConnector, BodyDataStream>,
    )>,
    req: axum::extract::Request,
) -> Result<impl IntoResponse, StatusCode> {
    let clock = Instant::now();
    let acquired = sem
        .clone()
        .acquire_owned()
        .await
        .expect("failed to acquire semaphore");
    acquired.forget();
    let lock_wait_ms = clock.elapsed().as_millis();
    inner(&client, req)
        .await
        .map(|mut res| {
            let gen_latency = clock.elapsed().as_millis();
            debug!("generated: lock wait: {lock_wait_ms}ms, total gen latency: {gen_latency}ms");
            res.headers_mut().append(
                "x-tracing-info",
                HeaderValue::from_str(
                    &json!({"lock_wait_ms": lock_wait_ms, "gen_latency": gen_latency}).to_string(),
                )
                .expect("fail to create header"),
            );
            res
        })
        .map_err(|e| {
            warn!("error: {} {}", e, e.is_connect());
            if e.is_connect() {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
        .inspect(|_| {
            sem.add_permits(1);
        })
}

pub async fn run_proxy(port: u16) -> anyhow::Result<()> {
    let client =
        hyper_util::client::legacy::Client::builder(TokioExecutor::new())
            .build_http();
    let app = Router::new()
        .route("/completions", post(handle_proxy))
        .route("/v1/completions", post(handle_proxy))
        .with_state((Arc::new(Semaphore::new(1)), client));
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use futures::future;
    use tokio::runtime::Runtime;
    use tokio::sync::Mutex;
    use tracing::error;
    use tracing_test::traced_test;

    use crate::daemon_trait::LlmConfig as _;
    use crate::proxy::Proxy;
    use crate::{Generator, LlamaConfig, LlamaDaemon, LlmDaemon};

    #[traced_test]
    #[test]
    fn proxy_trait_test() -> anyhow::Result<()> {
        type Target = Proxy<LlamaDaemon>;
        let conf = <Target as LlmDaemon>::Config::default();
        let endpoint = conf.endpoint();
        let inst = Target::new(conf, LlamaDaemon::new(LlamaConfig::default()));

        inst.fork_daemon()?;
        let runtime = Runtime::new()?;
        runtime.spawn(inst.heartbeat());
        runtime.block_on(async {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            let gen = Generator::new(endpoint, None);
            let resp = gen
                .generate("<bos>Sum of 7 and 8 is ".to_string())
                .await
                .inspect_err(|err| {
                    error!("error: {:?}", err);
                });
            assert!(resp.is_ok());
            assert!(resp.unwrap().contains("15"));
        });
        Ok(())
    }

    #[tokio::test]
    async fn without_spawn() -> anyhow::Result<()> {
        let mutex = Arc::new(Mutex::new(1));

        let m1 = mutex.clone();
        let h1 = async move {
            let _guard = m1.lock().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        };
        let (h1, flag1) = future::abortable(h1);

        let m2 = mutex.clone();
        let h2 = async move {
            flag1.abort();
            let _guard = m2.lock().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        };

        let r1 = tokio::join!(h1, h2).0;
        r1.expect_err("Should be aborted");

        Ok(())
    }

    #[tokio::test]
    async fn pass_the_mutexguard() -> anyhow::Result<()> {
        let mutex = Arc::new(Mutex::new(1));

        let m1 = mutex.clone();
        let h1 = async move {
            tokio::spawn(async move {
                let _guard = m1.lock().await;
                tokio::time::sleep(Duration::from_secs(1)).await;
            })
            .await
        };
        let (h1, flag1) = future::abortable(h1);

        let m2 = mutex.clone();
        let h2 = async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            flag1.abort();
            let _guard = m2.lock().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        };

        let r1 = tokio::join!(h1, h2).0;
        r1.expect_err("Should be aborted");
        Ok(())
    }

    #[tokio::test]
    async fn scoped_locks() -> anyhow::Result<()> {
        let mutex = Arc::new(Mutex::new(1));

        let m1 = mutex.clone();
        let handle = tokio::spawn(async move {
            let _guard = m1.lock().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        });

        let t1 = handle;

        tokio::time::sleep(Duration::from_millis(5)).await;

        t1.abort();

        let m2 = mutex.clone();
        let handle2 = tokio::spawn(async move {
            let _guard = m2.lock().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        });

        let t2 = handle2.abort_handle();

        tokio::time::sleep(Duration::from_millis(5)).await;

        t2.abort();

        let m3 = mutex.clone();
        let handle3 = tokio::spawn(async move {
            let _guard = m3.lock().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        });

        tokio::time::sleep(Duration::from_millis(5)).await;

        future::abortable(handle3).1.abort();

        let m4 = mutex.clone();
        let handle4 = async move {
            let _guard = m4.lock().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        };
        let h4 = future::abortable(handle4);

        tokio::spawn(h4.0);

        tokio::time::sleep(Duration::from_millis(1005)).await;

        h4.1.abort();

        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(())
    }
}
