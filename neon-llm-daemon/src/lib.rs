use std::path::PathBuf;

use llm_daemon::{Daemon3, Daemon3Params, LlmDaemon as _};
use neon::context::Context;
use neon::object::Object;
use neon::prelude::FunctionContext;
use neon::types::{Finalize, JsArray, JsBox, JsString, JsUndefined};

#[derive(Debug)]
struct DaemonWrapper(Daemon3);

impl Finalize for DaemonWrapper {
    fn finalize<'a, C: Context<'a>>(self, _cx: &mut C) {}
}

#[neon::export]
fn start_daemon<'a, 'b: 'a>(
    cx: &'a mut neon::context::FunctionContext<'b>,
) -> neon::result::JsResult<'b, JsBox<DaemonWrapper>> {
    let server_binary = cx.argument::<JsString>(0)?.value(cx);
    let hf_repo = cx.argument::<JsString>(1)?.value(cx);
    let args_js = cx.argument::<JsArray>(2)?;
    let mut args = Vec::new();
    let len = args_js.len(cx);
    for i in 0..len {
        let arg_js: neon::handle::Handle<'_, neon::types::JsValue> = args_js.get(cx, i).unwrap();
        let arg = arg_js
            .downcast_or_throw::<JsString, FunctionContext>(cx)?
            .value(cx);
        args.push(arg);
    }
    let daemon = Daemon3::from(Daemon3Params {
        hf_repo,
        args: Some(args),
        port: None,
        server_binary: Some(PathBuf::from(server_binary)),
    });
    // TODO: error handling
    let _ = daemon.fork_daemon();
    let wrapper = DaemonWrapper(daemon);
    Ok(cx.boxed(wrapper))
}

#[neon::export]
fn ping<'a, 'b: 'a>(
    cx: &'a mut neon::context::FunctionContext<'b>,
) -> neon::result::JsResult<'b, JsUndefined> {
    let wrapper = cx.argument::<JsBox<DaemonWrapper>>(0)?;
    let daemon = &wrapper.0;
    // TODO: error handling
    let _ = daemon.ping();
    Ok(cx.undefined())
}
