#![allow(non_local_definitions)]

use llm_daemon::{
    self, LlamaConfig, LlmConfig as _, LlmDaemon as _, MlcConfig, ProxyConfig,
};
use pyo3::prelude::*;
use pyo3_asyncio::tokio::get_runtime;
use tracing::info;

#[pyclass]
pub struct Generator {
    inner: llm_daemon::Generator,
}

#[pymethods]
impl Generator {
    #[new]
    pub fn new(endpoint: String, model: Option<String>) -> Self {
        Self {
            inner: llm_daemon::Generator::new(
                url::Url::parse(&endpoint).expect("failed to parse url"),
                model,
            ),
        }
    }

    pub fn generate<'a>(
        &'a self,
        py: Python<'a>,
        prompt: String,
    ) -> PyResult<&PyAny> {
        let fut = self.inner.generate(prompt);
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let tmp = fut.await;
            Ok(tmp.expect("failed to get string"))
        })
    }
}

#[pyclass]
pub struct LlamaDaemon {
    inner: llm_daemon::LlamaDaemon,
    endpoint: String,
}

#[pymethods]
impl LlamaDaemon {
    #[new]
    pub fn new() -> Self {
        let conf = LlamaConfig::default();
        let endpoint = conf.endpoint();
        let inner = llm_daemon::LlamaDaemon::new(conf);

        Self {
            endpoint: endpoint.to_string(),
            inner,
        }
    }

    pub fn fork_daemon(&self) -> PyResult<()> {
        self.inner.fork_daemon().expect("failed to fork daemon");
        Ok(())
    }

    pub fn heartbeat(&self) -> PyResult<()> {
        let runtime = get_runtime();
        // FIXME: join later
        let _handle = runtime.spawn(self.inner.heartbeat());
        Ok(())
    }

    pub fn endpoint(&self) -> String {
        self.endpoint.clone()
    }
}

#[pyclass]
pub struct MlcDaemon {
    inner: llm_daemon::MlcDaemon,
    endpoint: String,
}

#[pymethods]
impl MlcDaemon {
    #[new]
    pub fn new() -> Self {
        let conf = MlcConfig::default();
        let endpoint = conf.endpoint();
        let inner = llm_daemon::MlcDaemon::new(conf);

        Self {
            endpoint: endpoint.to_string(),
            inner,
        }
    }

    pub fn fork_daemon(&self) -> PyResult<()> {
        self.inner.fork_daemon().expect("failed to fork daemon");
        Ok(())
    }

    pub fn heartbeat(&self) -> PyResult<()> {
        let runtime = get_runtime();
        // FIXME: join later
        let _handle = runtime.spawn(self.inner.heartbeat());
        Ok(())
    }

    pub fn endpoint(&self) -> String {
        self.endpoint.clone()
    }
}

#[pyclass]
pub struct ProxyDaemon {
    inner: llm_daemon::Proxy<llm_daemon::LlamaDaemon>,
    endpoint: String,
}

#[pymethods]
impl ProxyDaemon {
    #[new]
    pub fn new() -> Self {
        let conf = ProxyConfig::default();
        let endpoint = conf.endpoint();
        let inner = llm_daemon::Proxy::new(
            conf,
            llm_daemon::LlamaDaemon::new(LlamaConfig::default()),
        );

        Self {
            endpoint: endpoint.to_string(),
            inner,
        }
    }

    pub fn fork_daemon(&self) -> PyResult<()> {
        self.inner.fork_daemon().expect("failed to fork daemon");
        Ok(())
    }

    pub fn heartbeat(&self) -> PyResult<()> {
        let runtime = get_runtime();
        // FIXME: join later
        let _handle = runtime.spawn(self.inner.heartbeat());
        Ok(())
    }

    pub fn endpoint(&self) -> String {
        self.endpoint.clone()
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn bihyung(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // TODO: Allow user to change log level, for debugging?
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::WARN)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to config logging");
    info!("This will be logged to stdout");
    m.add_class::<Generator>()?;
    m.add_class::<LlamaDaemon>()?;
    m.add_class::<MlcDaemon>()?;
    m.add_class::<ProxyDaemon>()?;
    Ok(())
}
