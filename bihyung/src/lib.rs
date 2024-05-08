use futures::TryFutureExt as _;
use llm_daemon::{
    self, llama_config_map, LlamaConfig, LlamaConfigs, LlamaDaemon as Daemon,
    LlmConfig as _, LlmDaemon as _, MlcConfig, ProxyConfig,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_asyncio::tokio::get_runtime;
use tokio::task::JoinHandle;

#[pyclass]
pub enum Model {
    Llama3_8b,
    Phi3_3b,
    Gemma2b,
}

#[pyclass]
pub struct DaemonHandle {
    daemon: Daemon,
    handle: Option<JoinHandle<PyResult<()>>>,
    endpoint: String,
}

#[pymethods]
impl DaemonHandle {
    pub fn __enter__(&mut self) -> PyResult<()> {
        self.daemon.fork_daemon().expect("failed to fork daemon");
        
        if self.handle.is_some() {
            panic!("cannot enter twice");
        }
        let runtime = get_runtime();
        dbg!("beating");
        let daemon = self.daemon.clone();
        self.handle = Some(runtime.spawn({
            daemon
                .heartbeat()
                .map_err(|e| PyErr::new::<PyTypeError, _>(e.to_string()))
        }));
        
        Ok(())
    }

    pub fn __exit__<'a>(
        &mut self,
        _a: Option<&'a PyType>,
        _b: Option<PyObject>,
        _c: Option<PyObject>,
    ) -> PyResult<bool> {
        dbg!("joining");
        if self.handle.is_none() {
            panic!("cannot exit twice");
        }
        self.handle.as_mut().unwrap().abort();
        self.handle = None;
        Ok(false)
    }
    
    pub fn endpoint(&self) -> String {
        self.endpoint.clone()
    }
}

#[pyfunction]
pub fn _daemon_from_model<'a>(
    model: &'a Model,
    server_path: String,
) -> PyResult<DaemonHandle> {
    let conf = match model {
        Model::Llama3_8b => llama_config_map()[&LlamaConfigs::Llama3].clone(),
        Model::Phi3_3b => llama_config_map()[&LlamaConfigs::Phi3].clone(),
        Model::Gemma2b => llama_config_map()[&LlamaConfigs::Gemma2b].clone(),
    };
    let endpoint = conf.endpoint();
    let daemon = (conf, server_path).into();
    Ok(DaemonHandle {
        endpoint: endpoint.to_string(),
        daemon,
        handle: None,
    })
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
    inner: llm_daemon::Proxy<Daemon>,
    endpoint: String,
}

#[pymethods]
impl ProxyDaemon {
    #[new]
    pub fn new() -> Self {
        let conf = ProxyConfig::default();
        let endpoint = conf.endpoint();
        let inner =
            llm_daemon::Proxy::new(conf, Daemon::new(LlamaConfig::default()));

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
    // // TODO: Allow user to change log level, for debugging?
    // let subscriber = tracing_subscriber::FmtSubscriber::builder()
    //     .with_max_level(tracing::Level::WARN)
    //     .finish();

    // tracing::subscriber::set_global_default(subscriber)
    //     .expect("failed to config logging");
    // info!("This will be logged to stdout");
    m.add_class::<MlcDaemon>()?;
    m.add_class::<ProxyDaemon>()?;
    m.add_class::<Model>()?;
    m.add_class::<DaemonHandle>()?;
    m.add_function(wrap_pyfunction!(_daemon_from_model, m)?)?;
    Ok(())
}
