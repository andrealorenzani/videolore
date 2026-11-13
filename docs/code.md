# Code

Not written yet: the Principal Engineer fills this in with the first feature.

- Build: `cargo build`
- Test: `cargo test`
- CI: `.github/workflows/build.yml` runs the checks and builds release executables (Linux x86_64, macOS arm64, Windows x86_64) on every push to `main`; a `v*` tag attaches them to a GitHub release.
- Licence header in every `.rs` file: `// SPDX-License-Identifier: GPL-3.0-only` + `// Copyright (C) <year> Andrea Lorenzani`.
- Layout: all logic in `src/lib.rs` (tested); `src/main.rs` only wires it.
- Coverage: line coverage ≥ 85% (`scripts/coverage.sh`, needs `cargo-llvm-cov`), every implemented requirement covered by a test marked `// covers: REQ-NNN` (`scripts/feature-coverage.sh`).
- Checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
