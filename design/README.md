# Design Source

`design/rizum-glass` is a Git submodule pinned to the reviewed Rizum Glass
revision used by this project. Read its `DESIGN.md` before UI or UX work and use
its active light and dark galleries for visual comparison.

Initialize a checkout with:

```bash
git submodule update --init --recursive
```

The canonical remote remains in `.gitmodules`. A developer with the local
Rizum Glass repository can use it as the submodule source without changing the
committed configuration:

```powershell
git submodule sync --recursive
git config submodule.design/rizum-glass.url E:/Projects/Design/rizum-glass
git submodule update --init --recursive
```

The local path is an optimization, not the version contract. Silicate records
the accepted Rizum Glass commit as a gitlink. Do not point product builds at a
moving `main` branch or advance the gitlink without the review and verification
defined in `docs/adr/0001-gpuix-shell-with-native-rust-runtime.md`.
