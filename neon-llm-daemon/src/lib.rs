use std::path::PathBuf;

use llm_daemon::{Daemon3, Daemon3Params, LlmDaemon as _};
use neon::context::Context;
use neon::object::Object;
use neon::prelude::FunctionContext;
use neon::types::{JsArray, JsString, JsUndefined};

#[neon::export]
fn start_daemon<'a, 'b: 'a>(
    cx: &'a mut neon::context::FunctionContext<'b>,
) -> neon::result::JsResult<'b, JsUndefined> {
    let server_binary = cx.argument::<JsString>(0)?.value(cx);
    let hf_repo = cx.argument::<JsString>(1)?.value(cx);
    let args_js = cx.argument::<JsArray>(2)?;
    let mut args = Vec::new();
    let len = args_js.len(cx);
    for i in 0..len {
        let arg_js: neon::handle::Handle<'_, neon::types::JsValue> =
            args_js.get(cx, i).unwrap();
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
    let _ = daemon.fork_daemon();
    Ok(cx.undefined())
}

// Use #[neon::main] to add additional behavior at module loading time.
// See more at: https://docs.rs/neon/latest/neon/attr.main.html

// #[neon::main]
// fn main(_cx: ModuleContext) -> NeonResult<()> {
//     println!("module is loaded!");
//     Ok(())
// }
