//! `ikigai-xslt-module` — the ikigai-xslt transform as a standalone, dynamically-loadable
//! WASM artifact. A wasm-bindgen wrapper over [`ikigai_xslt`], built to its own `.wasm` and
//! lazy-loaded by a host — so xrust (~2.2 MB) lives in *this* artifact, not in the host's
//! binary, and is fetched only when an `urn:xslt:*` resource is first resolved.
//!
//! Two entry points:
//! - [`transform`] — **by value**: the host resolves the `src`/`stylesheet` references and
//!   passes the bytes in. A self-contained pure function, no callbacks.
//! - [`invoke_session`] — **the real module session**: the host hands the module an encoded
//!   [`ModuleCall::Invoke`](ikigai_module::ModuleCall), the module runs the XSLT endpoint
//!   via [`ikigai_module::run_session`], and each `inv.source` it makes is pumped back to the
//!   host as a `HostCall`/`HostResult` exchange over the JS byte channel ([`host_call_js`]).
//!   This is the same `ModuleCall`/`ModuleReply` protocol the native loopback and UDS
//!   transports speak — the browser is just another transport.

use std::future::Future;
use std::sync::Arc;

use ikigai_core::Space;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Install a panic hook so a Rust panic surfaces in the browser console (once).
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Apply `stylesheet` (XSLT) to `src` (XML). `text` selects string-value serialization
/// (a `method="text"` stylesheet) vs XML/markup. Returns the transformed string, or a
/// JS error with the failure detail. (By value: the host already resolved the refs.)
#[wasm_bindgen]
pub fn transform(src: &str, stylesheet: &str, text: bool) -> Result<String, JsValue> {
    ikigai_xslt::transform_xml(src, stylesheet, text).map_err(|e| JsValue::from_str(&e))
}

// The host's session pump, imported as a global. Hand it the module's encoded `ModuleReply`
// (a `HostCall`) and it returns the host's encoded `ModuleCall` (the `HostResult`) — one
// turn of the byte pump, the host resolving the sub-resource on its own kernel across the
// wasm boundary. `--target web` resolves this `js_name` from the global scope, so the host
// sets `globalThis.hostCall`.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_name = "hostCall")]
    async fn host_call_js(reply: &[u8]) -> Result<JsValue, JsValue>;
}

/// **The real module session.** Given the host's encoded `ModuleCall::Invoke`, run the XSLT
/// endpoint via [`ikigai_module::run_session`], pumping each of its `inv.source` callbacks
/// back to the host as a `HostCall`/`HostResult` exchange over `hostCall`. Returns the
/// encoded final `ModuleReply` (`Resolved`/`Error`). Lazy-loaded and called by the host's
/// `xslt-loader.js`.
#[wasm_bindgen]
pub async fn invoke_session(invoke: Vec<u8>) -> Vec<u8> {
    let space: Arc<dyn Space> = Arc::new(ikigai_xslt::space());
    ikigai_module::run_session(&space, &invoke, host_call).await
}

/// Bridge the `!Send` JS `hostCall` to the `Send` future `run_session` needs: confine the
/// `JsFuture` to a `spawn_local` task and ferry the (`Send`) byte result back through a
/// oneshot — the same `!Send`-confinement pattern the host uses for `fetch`.
fn host_call(reply_bytes: Vec<u8>) -> impl Future<Output = Result<Vec<u8>, String>> + Send {
    let (tx, rx) = futures::channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        let result = match host_call_js(&reply_bytes).await {
            Ok(value) => value
                .dyn_into::<js_sys::Uint8Array>()
                .map(|array| array.to_vec())
                .map_err(|_| "host returned a non-Uint8Array answer to a HostCall".to_string()),
            Err(e) => Err(e.as_string().unwrap_or_else(|| "host call failed".to_string())),
        };
        let _ = tx.send(result);
    });
    async move { rx.await.map_err(|_| "host call task was dropped".to_string())? }
}
