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

The canonical local entry point validates this identity before running any
measurement or GPU smoke. Pass `-FixturePath`, set
`RIZUM_SILICATE_PRIMARY_FIXTURE`, or let it resolve
`$HOME\iCloudDrive\Procreate\Art_SystemPet_Default.procreate`:

```powershell
.\scripts\primary-fixture.ps1 -Mode identity
.\scripts\primary-fixture.ps1 -Mode animation
.\scripts\primary-fixture.ps1 -Mode video
.\scripts\primary-fixture.ps1 -Mode runtime -Iterations 30
.\scripts\primary-fixture.ps1 -Mode gpu
```

Command:

```powershell
.\scripts\primary-fixture.ps1 -Mode runtime -Iterations 30
```

The tool reads the file before timing, performs one excluded warmup, then
creates a fresh runtime for each measured iteration. The timed seam includes
in-memory ZIP central-directory parsing, `Document.archive` extraction,
NSKeyedArchive decoding, document storage, projection of the ordered layer
snapshot, and creation of the bounded `DocumentOpened` event result.

Results:

| Metric | Time |
| --- | ---: |
| Minimum | 3.830 ms |
| Median | 4.836 ms |
| Mean | 5.102 ms |
| Maximum | 6.873 ms |

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

After adding clipped snapshot state, a same-machine detached build of commit
`ae25362` measured 30 iterations at 4.861/6.253/6.614/9.421 ms
(minimum/median/mean/maximum). The current build measured 4.772/6.812/7.544/
12.186 ms in the immediately following run. The median delta was 8.9%, the
distributions overlapped, and the result stayed below the documented 10% gate;
there is no evidence yet of a material runtime-open regression.

After adding blend-mode snapshot state, the current build measured 30
iterations at 3.830/4.836/5.102/6.873 ms. This is below the previous median;
because these distributions are sensitive to machine state, treat it as
evidence of no regression rather than as a claimed speedup.

After the runtime became the canonical owner of all editable render state, two
current runs measured medians of 6.761 ms and 6.284 ms. A detached build of the
pre-change commit `9c37102`, measured between those runs on the same machine,
had a 6.748 ms median. This controlled comparison found no runtime-open
regression; the wider historical spread confirms that one unusually fast run
must not be used as the sole gate.

## `silicate_runtime_mutations_to_gpu_v7`

Recorded on 2026-09-01 in the same Windows environment and release profile as
the runtime-open baseline, using an NVIDIA GeForce RTX 5070 Ti WGPU adapter.
The fixture and hash are identical to `silicate_runtime_open_v1`.

Command:

```powershell
.\scripts\primary-fixture.ps1 -Mode gpu
```

The verifier parses the fixture once, opens the runtime and GPU documents,
compares all 236 renderer-neutral hierarchy identities, selects the first
available node of each layer kind and the first layer with a clipping base,
then edits background visibility and color, canvas flip, clipping, blend mode,
and opacity. Each timing
includes runtime dispatch, event handling, and GPU document state mutation;
hierarchy rows also include GPU hierarchy lookup. It verifies every target
state, confirms that idempotent repeats emit no events, confirms that the GPU
adapter rejects clipping, blend mode, and opacity on unsupported hierarchy
kinds, confirms that clipping without a base is rejected, and confirms that
runtime rejection does not advance revision. It also
reports the fixture's initial clipping inventory, verifies grouped opacity
undo/redo against GPU state, and verifies dirty-close rejection plus explicit
discard.

| Mutation | Target | Hierarchy ID | Command to GPU document state |
| --- | --- | ---: | ---: |
| Visibility | Layer | 2 | 17.400 us |
| Visibility | Group | 0 | 0.400 us |
| Visibility | Mask | absent from fixture | not measured |
| Visibility | Background | document state | 0.400 us |
| Clipped | Layer | 2 | 0.400 us |
| Blend mode | Layer | 2 | 0.500 us |
| Opacity | Layer | 2 | 3.600 us |
| Background color | document state | n/a | 1.000 us |
| Canvas flip | document state | n/a | 0.200 us |

These are single-sample correctness datapoints for the adapter boundary, not a
performance gate or interactive frame-time claim. The large difference also
shows why they must not be interpreted as stable latency statistics. They
exclude compositor submission, WGPU queue execution, egui layout and paint,
display synchronization, and presentation. The required command-to-present
baseline remains listed below. A separate run with
`demo_files/Mask_Test_File.procreate` verified the mask identity and visibility
path.

## Format Conformance Smokes

`parser_corpus_v1`, recorded on 2026-09-04, recursively streams every
`.procreate` archive in a corpus
through the production parser and inventories raw animation fields. The five
repository fixtures and 152 documents in the local Procreate corpus all parsed
without failure, so no speculative field aliases were added. Every document
stored playback mode `1` and direction `0`; assist state was missing in 46,
disabled in 105, and enabled in 6. Reopening the canonical
`Art_SystemPet_Default.procreate` fixture in Procreate confirmed mode `1` is
Loop. No observed fixture establishes other mode values or stored direction
semantics, so those values remain lossless raw metadata rather than guessed
mappings.

```powershell
cargo run --locked -p silica --example verify_parser_corpus -- demo_files
cargo run --locked -p silica --example verify_parser_corpus -- "$HOME\iCloudDrive\Procreate"
```

## Rendering Correctness Smokes

`compositor_composition_v5` reads ten 1x1 results back from WGPU. On the
recorded RTX 5070 Ti adapter it verifies background-only rendering, whole-frame
opacity across a two-layer isolation group, base/primary phase ordering,
premultiplied alpha edges, hidden layers, visible and hidden masks, and clipping
alpha including a clipping source's visible and hidden mask states. The observed
RGBA8 pixels were `[64, 127, 191, 255]`,
`[63, 0, 64, 127]`, `[255, 0, 0, 255]`, `[0, 0, 128, 128]`, `[0, 0, 0, 0]`,
`[128, 0, 0, 128]`, `[255, 0, 0, 255]`, `[128, 0, 0, 128]`,
`[64, 0, 0, 64]`, and `[128, 0, 0, 128]`; the verifier permits one LSB of
backend quantization variance.

```powershell
cargo run --release -p silicate-compositor --example verify_composition --locked
```

`still_export_alpha_v1` verifies that the still-export boundary converts the
compositor's premultiplied `[128, 0, 0, 128]` edge to straight-alpha
`[255, 0, 0, 128]`, then preserves that value through a PNG encode/decode round
trip. Interactive rendering remains premultiplied and does not pay this CPU
conversion cost.

```powershell
cargo run --release --example verify_still_export_alpha --locked
```

`fixture_rendering_v2` exercises the production parse, runtime, GPU upload,
projection, compositor, and still-export path. It uses
`Mask_Test_File.procreate` (SHA-256
`17CD313A1B56950738DE392595D33045C423469FF9385B5A63EFB6E3F37AA83C`) and
`Reference_Blend_File 3.procreate` (SHA-256
`8B288E0886B3CC68030DCFDEEDD7CEEB37BB1E41CC3D29AC5061D968821D59DB`). Toggling
the representative mask changed 76,708 pixels; toggling a group changed 80,185;
disabling the persisted clipping layer changed 20,196; and toggling its base
visibility changed 74,360. Restoring group visibility, clipping, and base
visibility reproduced the baseline exactly. Six deterministic topology cases
verify that stacked clipping layers share one base, separate scopes do not leak,
and clipping without a base deactivates. The clipping fixture's exported image
contained 1,626,339 transparent, 433,015 partial-alpha, and 1,829,294 opaque
pixels.

```powershell
cargo run --release --example verify_fixture_rendering --locked -- demo_files/Mask_Test_File.procreate 'demo_files/Reference_Blend_File 3.procreate'
```

A native still-export smoke used `demo_files/Reference_Blend_File.procreate`
(SHA-256 `4DCA07AEA1389ED521EB24F43E1A4E03DCD32D723C657D38F3E44EE69CED02F4`).
Its persisted clockwise-90 orientation exported at 2118x1836, matching the
1024x888 QuickLook aspect and visual orientation; both outputs kept corner
alpha at zero. View rotation remains presentation-only and is excluded from
still output.

`procreate_render_comparison_v1`, recorded on 2026-09-04, renders a fixture
through the production GPU path and compares premultiplied RGBA against both
Procreate's persisted full-resolution composite and its QuickLook PNG. On the
recorded RTX 5070 Ti,
`Mask_Test_File.procreate` differed from the persisted composite by no pixels
over four LSB (MAE `0.0000`, RMSE `0.0069`, maximum `1`) across 4,194,304
pixels. Its resized QuickLook comparison had 2,915 pixels over four LSB (MAE
`0.0325`, RMSE `0.3823`, maximum `15`) across 1,048,576 pixels.

`Reference_Blend_File.procreate` exposed the known blend conformance gap:
46,301 of 3,888,648 full-resolution pixels exceeded four LSB (MAE `0.4418`,
RMSE `7.2327`, maximum `255`). Its QuickLook comparison had 80,927 of 909,312
pixels over four LSB (MAE `1.0844`, RMSE `7.4831`, maximum `255`). Diagnostic
HSV and source/base-swapping variants made the result worse, so the
[W3C non-separable formulas](https://www.w3.org/TR/compositing-1/#blendingnonseparable)
remain the evidence-backed implementation. An optional output directory writes
both references, the Silicate renders, and amplified diff images for future
investigation.

```powershell
cargo run --release --locked --example compare_procreate_fixture -- demo_files/Mask_Test_File.procreate
cargo run --release --locked --example compare_procreate_fixture -- demo_files/Reference_Blend_File.procreate diagnostics/render-comparison
```

## Missing Baselines

The following remain required before replacing the production presentation
adapter:

- end-to-end file open to first presented canvas;
- layer mutation command to presented frame;
- pan and zoom frame time under physical input;
- animation playback frame pacing;
- working-set and GPU-memory peaks;
- still and video export throughput;
- minimize/restore and close-during-work behavior.
