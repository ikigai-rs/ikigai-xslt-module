# ikigai-xslt-module

The [`ikigai-xslt`](https://github.com/ikigai-rs/ikigai-xslt) endpoint as a standalone,
dynamically-loadable **WebAssembly module** — a thin [`wasm-bindgen`] wrapper a host
lazy-loads at runtime so the XSLT engine (xrust) never has to be linked into the host's
own binary.

This is the first concrete artifact of the ikigai *module format*
([`ikigai-module`](https://github.com/ikigai-rs/ikigai-module)): a module is an
independently-compiled unit whose endpoints resolve their sub-resources **back through
the host kernel**.

## Exports

- `transform(src, stylesheet, text)` — **by value**: the host has already resolved the
  source and stylesheet and passes their bytes in.
- `transform_refs(srcUri, styleUri, text)` — **by reference**: the host passes IRIs; the
  module resolves them itself by calling back to the host's `hostResolve` global (a
  wasm import), so the module participates in the host's cache/golden-thread tracking.

## Build

```sh
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir <out> \
  target/wasm32-unknown-unknown/release/ikigai_xslt_module.wasm
```

The [`ikigai-web-demo`](https://github.com/ikigai-rs/ikigai-web-demo) builds this in CI
and serves the result alongside its host wasm; a small hand-written `xslt-loader.js` is
the JS byte-channel "transport" between the two wasm instances.

[`wasm-bindgen`]: https://github.com/rustwasm/wasm-bindgen
