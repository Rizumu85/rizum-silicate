# Project Guidance

This is the sole project-rule entry point for the Rizum Silicate repository.
Nested `AGENTS.md` files belong to their own embedded repositories only.

## Sources Of Truth

- Read `RIZUM_SILICATE.md` for stable product and architecture direction.
- Read `docs/CAPABILITY_MATRIX.md` and `docs/RUNTIME_HANDOFF.md` for current status.
- Read ADR 0001 before changing the presentation/runtime boundary.
- For Rizum Glass work, read the pinned `design/rizum-glass/skills/rizum-glass/SKILL.md` and only the references it requires.

## Stack And Boundaries

- The product is a native-first Procreate viewer, inspector, and export tool built as a Rust 2024 workspace.
- `silica` owns archive/domain parsing; `silicate-runtime` owns UI-independent commands, snapshots, and events.
- `silica-gpu` owns GPU-ready document upload; `compositor` owns the WGPU rendering pipeline.
- `platform-thumbnail` and platform crates remain independent of presentation frameworks.
- eframe/egui/WGPU is the production adapter. Web is secondary. GPUIX remains a candidate until ADR gates prove its native canvas, lifecycle, input, packaging, and measured performance.
- Keep parser, runtime, export, and platform APIs free of React, egui, GPUIX, and compositor types.
- Do not introduce Electron, Tauri, or WebView presentation. Do not move interactive pixels through N-API, Base64, encoded files, CPU copies, or GPU readback.
- Treat the compositor as the performance spine; change it for correctness, capability, or measured improvement.

## Engineering Practice

- Inspect the relevant architecture and dependencies before adding a feature. Research changing technologies from primary sources and evaluate performance, code quality, and maintenance cost.
- Place features according to the user's workflow and a product designer's information hierarchy, not implementation convenience.
- Diagnose bugs through ownership, invariants, coupling, and data flow. Reproduce and instrument real data before changing behavior; repair the governing cause instead of accumulating special cases.
- When direction changes, complete the replacement and remove obsolete experiments or compatibility paths. Keep a fallback only for a documented, current requirement.
- Frame requirements, diagnostics, and implementation plans positively around the behavior the product should guarantee.

## Comments And Documentation

- Comments record durable reasons, tradeoffs, and user-decided constraints. They do not narrate what readable code already says.
- Add a decision comment only where a future maintainer or AI could otherwise undo an important constraint. Update or remove it when the decision changes; do not repeat it across layers.
- Do not create development diaries or automatically proliferate Markdown. Update documentation when the user asks or when a durable architecture/handoff fact needs one canonical home.
- Prefer links to a source of truth over copied status lists.

## Validation

- Do not use TDD or add ordinary unit/integration test matrices.
- New validation work is limited to benchmarks, smoke tests, and performance tests when performance evidence is needed.
- Existing repository checks may run as regression guards. Format only touched Rust files when unrelated workspace formatting drift exists.

## Git Closeout

- Preserve user changes and avoid destructive history or worktree operations.
- Partition every completed turn's changes into cohesive commits by concern and push each commit immediately after creation.
- Commit and push submodule changes inside the submodule first, then commit and push the parent gitlink update separately.
- If a push is blocked, keep the local commit and report the exact blocker; never call unpushed work backed up.
