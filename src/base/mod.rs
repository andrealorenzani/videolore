// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Helpers shared by many modules, with no dependency on any other module of videolore.
//!
//! This is the bottom layer: it uses only the standard library and external crates, so it can
//! be in no dependency cycle (checked by `tests/module_cycles.rs`).

pub mod font;
pub mod io;
pub mod language;
pub mod names;
pub mod pos;
pub mod private;
pub mod shadow;
#[cfg(test)]
pub mod testing;
