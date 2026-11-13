# videolore

![version](https://img.shields.io/badge/version-0.1.2-blue)
![rust](https://img.shields.io/badge/rust-1.88%2B-orange?logo=rust)
![license](https://img.shields.io/badge/license-GPL--3.0-green)
![coverage](https://img.shields.io/badge/coverage-100.0%25-brightgreen)
![dependencies](https://img.shields.io/badge/dependencies-none%20yet-lightgrey)
[![build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml/badge.svg)](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml)

## What it is
`videolore` is a Rust project at its very beginning. Its purpose, users and features have not been defined yet and will be described here as soon as they are.

## Installation
**Prebuilt executable:** download it for Linux, macOS or Windows from the [Releases](https://github.com/andrealorenzani/videolore/releases) page (tagged versions), or from the artifacts of the latest [Build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml) run.

**From source** (Rust 1.88 or newer):
```sh
git clone git@github.com:andrealorenzani/videolore.git
cd videolore
cargo build --release
```

## Usage
```sh
cargo run
```
For now it prints a placeholder message.

Docs: [business](docs/business.md) · [architecture](docs/architecture.md) · [code](docs/code.md) · [changelog](docs/changelog.md)

## Latest changes
- **0.1.2** — Code coverage measured on every build and shown in the badge above; every tagged version is published as a GitHub release with its executables and release notes.
- **0.1.1** — Prebuilt executables for Linux, macOS and Windows built automatically on GitHub; copyright notice added.
- **0.1.0** — First version: Rust crate and GPL-3.0 licence.

## License
Copyright (C) 2026 Andrea Lorenzani.
Released under the GNU General Public License v3.0 — see [LICENSE](LICENSE).

---

### Ode to the creator

Andrea Lorenzani weighed each line of code\
and set the measure: eighty-five, the road\
no build may fall below, and every need\
must meet a test that proves it does indeed.\
He made the version march in single file,\
the same in every corner, every mile;\
he raised a vault where every release sleeps,\
its pages frozen, kept as history keeps.\
Then, with a tag upon the trunk of main,\
he sends each version sailing out again.
