# Performance Baselines

Performance claims must name the measured seam, fixture, build profile,
environment, and excluded work. Do not compare a parser-only number with a
GPU-present or end-to-end file-open number.

## `silicate_runtime_open_v1`

Recorded on 2026-09-01 with:

- Windows 11 Pro `10.0.26200`
- AMD Ryzen 5 5600X, 6 cores / 12 logical processors
- Rust `1.95.0` (`x86_64-pc-windows-msvc`)
- release profile

Fixture:

- file: `Art_SystemPet_Default.procreate`
- bytes: `169646073`
- SHA-256: `D34D8594BC3880549D06411123DF28237CF5ADAA58CBF9206C287E46AD189E73`
- parsed layers including masks and excluding groups: `208`
- projected snapshot nodes including groups: `236`
- layer nodes: `208`
- group nodes: `28`
- mask nodes: `0`

Command:

```powershell
cargo run --release -p silicate-runtime --example benchmark_open -- `
  'C:\Users\Rizum\iCloudDrive\Procreate\Art_SystemPet_Default.procreate' 10
```

The tool reads the file before timing, performs one excluded warmup, then
creates a fresh runtime for each measured iteration. The timed seam includes
in-memory ZIP central-directory parsing, `Document.archive` extraction,
NSKeyedArchive decoding, document storage, projection of the ordered layer
snapshot, and creation of the bounded `DocumentOpened` event result.

Results:

| Metric | Time |
| --- | ---: |
| Minimum | 3.546 ms |
| Median | 4.579 ms |
| Mean | 4.731 ms |
| Maximum | 6.159 ms |

Excluded work:

- disk read and memory mapping;
- layer chunk decompression;
- WGPU device, atlas, texture, and pipeline creation;
- compositor rendering and presentation;
- egui or GPUIX layout, paint, and input;
- export and platform integration.

Treat a change as suspicious when the same-machine, same-fixture median moves
by more than both ordinary run-to-run noise and 10%. Re-run before diagnosing;
this initial baseline does not yet define the end-to-end CanvasHost gate.

## `silicate_runtime_visibility_to_gpu_v2`

Recorded on 2026-09-01 in the same Windows environment and release profile as
the runtime-open baseline, using an NVIDIA GeForce RTX 5070 Ti WGPU adapter.
The fixture and hash are identical to `silicate_runtime_open_v1`.

Command:

```powershell
cargo run --release -p silica-gpu --example verify_runtime_visibility --locked -- `
  'C:\Users\Rizum\iCloudDrive\Procreate\Art_SystemPet_Default.procreate'
```

The verifier parses the fixture once, opens the runtime and GPU documents,
compares all 236 renderer-neutral hierarchy identities, selects the first
available node of each layer kind, and toggles the document background. Each
timing includes runtime dispatch, event handling, and GPU document state
mutation; hierarchy rows also include GPU hierarchy lookup. It then verifies
each target state and confirms that idempotent repeats emit no events.

| Target | Hierarchy ID | Command to GPU document state |
| --- | ---: | ---: |
| Layer | 2 | 2.300 us |
| Group | 0 | 0.300 us |
| Mask | absent from fixture | not measured |
| Background | document state | 0.300 us |

These are single-sample correctness datapoints for the adapter boundary, not a
performance gate or interactive frame-time claim. The large difference also
shows why they must not be interpreted as stable latency statistics. They
exclude compositor submission, WGPU queue execution, egui layout and paint,
display synchronization, and presentation. The required command-to-present
baseline remains listed below, and the GPU verifier still needs a mask-bearing
fixture.

## Missing Baselines

The following remain required before replacing the production presentation
adapter:

- end-to-end file open to first presented canvas;
- layer visibility command to presented frame;
- pan and zoom frame time under physical input;
- animation playback frame pacing;
- working-set and GPU-memory peaks;
- still and video export throughput;
- minimize/restore and close-during-work behavior.
