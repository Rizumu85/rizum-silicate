# Silicate Runtime

`silicate-runtime` is the UI-independent document interface shared by current
and future presentation adapters. It owns parsed Procreate document state and
exposes serializable commands, immutable snapshots, and bounded events.

Current vertical slice:

- open a Procreate archive from bytes;
- return a stable `DocumentId` and metadata snapshot;
- emit the matching `DocumentOpened` event in the operation result;
- dispatch `CloseDocument`, remove the document, and emit `DocumentClosed`;
- benchmark the public open path against a real fixture.

The operation result owns its events; the runtime does not accumulate an
unbounded internal event queue. Presentation adapters may publish those events
through their own channel or FFI transport.

This crate does not yet own the production egui document instances, WGPU
atlas, compositor scheduling, or layer mutation commands. Those remain explicit
migration work. Do not route pixels, GPU handles, egui values, GPUIX values, or
Node objects through this interface.

Run the focused tests with:

```bash
cargo test -p silicate-runtime --locked
```

Run the parser/runtime baseline with:

```bash
cargo run --release -p silicate-runtime --example benchmark_open -- /path/to/document.procreate 10
```
