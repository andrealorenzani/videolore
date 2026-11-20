// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! What the running machine can do, read once at start-up without any network access or
//! benchmark (a few milliseconds): logical and physical cores, installed memory, operating
//! system, processor family and the processor features the speech engine needs.
//!
//! The [`Fingerprint`] is the part stored in the settings file: when it changes, videolore
//! knows it runs on a different machine. Memory is rounded to the nearest GiB so that small
//! differences between boots do not count.

use serde::{Deserialize, Serialize};

/// Bytes in one GiB.
pub const GIB: u64 = 1024 * 1024 * 1024;

/// Processor features the speech engine is built for on x86-64 (ggml's portable baseline).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFeatures {
    pub sse42: bool,
    pub avx: bool,
    pub avx2: bool,
    pub fma: bool,
    pub f16c: bool,
    pub bmi2: bool,
}

impl CpuFeatures {
    /// Every feature present.
    pub const ALL: CpuFeatures = CpuFeatures {
        sse42: true,
        avx: true,
        avx2: true,
        fma: true,
        f16c: true,
        bmi2: true,
    };

    /// Reads the running processor (also checks that the OS enables AVX).
    pub fn detect() -> CpuFeatures {
        #[cfg(target_arch = "x86_64")]
        {
            CpuFeatures {
                sse42: std::is_x86_feature_detected!("sse4.2"),
                avx: std::is_x86_feature_detected!("avx"),
                avx2: std::is_x86_feature_detected!("avx2"),
                fma: std::is_x86_feature_detected!("fma"),
                f16c: std::is_x86_feature_detected!("f16c"),
                bmi2: std::is_x86_feature_detected!("bmi2"),
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            CpuFeatures::ALL
        }
    }

    /// Names of the missing features (for the Advanced section).
    pub fn missing(&self) -> Vec<&'static str> {
        [
            (self.sse42, "SSE4.2"),
            (self.avx, "AVX"),
            (self.avx2, "AVX2"),
            (self.fma, "FMA"),
            (self.f16c, "F16C"),
            (self.bmi2, "BMI2"),
        ]
        .into_iter()
        .filter(|(on, _)| !on)
        .map(|(_, n)| n)
        .collect()
    }
}

/// The detected machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Machine {
    /// Logical cores available to this process (respects affinity and container limits).
    pub logical: u32,
    /// Physical cores (never more than `logical`).
    pub physical: u32,
    /// Installed memory; 0 when unknown.
    pub ram_bytes: u64,
    pub os: &'static str,
    pub arch: &'static str,
    pub cpu: CpuFeatures,
}

/// The machine characteristics stored with the settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub os: String,
    pub arch: String,
    pub logical: u32,
    pub physical: u32,
    /// Memory rounded to the nearest GiB (0: unknown).
    pub ram_gib: u64,
    pub speech_supported: bool,
}

impl Machine {
    /// Reads the real machine.
    pub fn detect() -> Machine {
        let logical = std::thread::available_parallelism()
            .map(|n| u32::try_from(n.get()).unwrap_or(u32::MAX))
            .unwrap_or(1);
        let physical = sysinfo::System::physical_core_count()
            .ok()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0);
        // Unknown memory counts as 0, which makes the proxy rule conservative.
        let ram_bytes = sysinfo::System::new()
            .map(|mut sys| {
                sys.refresh_memory();
                sys.total_memory()
            })
            .unwrap_or(0);
        Machine::new(
            logical,
            physical,
            ram_bytes,
            std::env::consts::OS,
            std::env::consts::ARCH,
            CpuFeatures::detect(),
        )
    }

    /// A machine from raw values: `physical` 0 (unknown) becomes half the logical cores
    /// rounded up; it never exceeds `logical`.
    pub fn new(
        logical: u32,
        physical: u32,
        ram_bytes: u64,
        os: &'static str,
        arch: &'static str,
        cpu: CpuFeatures,
    ) -> Machine {
        let logical = logical.max(1);
        let physical = if physical == 0 {
            logical.div_ceil(2)
        } else {
            physical
        }
        .clamp(1, logical);
        Machine {
            logical,
            physical,
            ram_bytes,
            os,
            arch,
            cpu,
        }
    }

    /// A machine for tests: this OS and processor family, every processor feature.
    pub fn simulated(logical: u32, physical: u32, ram_bytes: u64) -> Machine {
        Machine::new(
            logical,
            physical,
            ram_bytes,
            std::env::consts::OS,
            std::env::consts::ARCH,
            CpuFeatures::ALL,
        )
    }

    /// The speech engine can run here. On x86-64 it needs every [`CpuFeatures`] flag (it is
    /// built for that baseline); other processors (Apple silicon) always can.
    pub fn speech_supported(&self) -> bool {
        self.arch != "x86_64" || self.cpu.missing().is_empty()
    }

    /// Memory in GiB, rounded to the nearest.
    pub fn ram_gib(&self) -> u64 {
        (self.ram_bytes + GIB / 2) / GIB
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint {
            os: self.os.to_string(),
            arch: self.arch.to_string(),
            logical: self.logical,
            physical: self.physical,
            ram_gib: self.ram_gib(),
            speech_supported: self.speech_supported(),
        }
    }
}

/// Resident memory of this process in bytes (0 when unknown). Used by the memory tests.
pub fn own_rss() -> u64 {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return 0;
    };
    let Ok(mut sys) = sysinfo::System::new() else {
        return 0;
    };
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid).map(|p| p.memory()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    // covers: REQ-045
    #[test]
    fn detection_is_quick_offline_and_deterministic() {
        let limit = Duration::from_secs(3);
        let best = crate::base::testing::best_of(5, limit, || {
            let t = Instant::now();
            Machine::detect();
            t.elapsed()
        });
        assert!(best < limit);
        let m = Machine::detect();
        let again = Machine::detect();
        assert_eq!(m.fingerprint(), again.fingerprint());
        assert!(m.logical >= 1);
        assert!(m.physical >= 1 && m.physical <= m.logical);
        assert!(m.ram_bytes > 0);
        assert!(own_rss() > 0);
        assert_eq!(m.os, std::env::consts::OS);
    }

    #[test]
    fn physical_cores_default_and_bounds() {
        assert_eq!(Machine::simulated(8, 0, 0).physical, 4);
        assert_eq!(Machine::simulated(7, 0, 0).physical, 4);
        assert_eq!(Machine::simulated(4, 16, 0).physical, 4);
        assert_eq!(Machine::simulated(0, 0, 0).logical, 1);
        assert_eq!(Machine::simulated(0, 0, 0).physical, 1);
        // an "8 GB" machine reports about 7.6 GiB
        let m = Machine::simulated(8, 4, 7 * GIB + 6 * GIB / 10);
        assert_eq!(m.ram_gib(), 8);
        assert_eq!(Machine::simulated(1, 1, GIB / 3).ram_gib(), 0);
    }

    // covers: REQ-054
    #[test]
    fn old_processors_are_recognised() {
        let mut m = Machine::new(8, 4, 16 * GIB, "linux", "x86_64", CpuFeatures::ALL);
        assert!(m.speech_supported());
        assert!(m.cpu.missing().is_empty());
        m.cpu.avx2 = false;
        m.cpu.fma = false;
        assert!(!m.speech_supported());
        assert_eq!(m.cpu.missing(), vec!["AVX2", "FMA"]);
        assert!(!m.fingerprint().speech_supported);
        // Apple silicon needs none of the x86 flags
        let arm = Machine::new(
            8,
            8,
            16 * GIB,
            "macos",
            "aarch64",
            CpuFeatures {
                avx2: false,
                ..CpuFeatures::ALL
            },
        );
        assert!(arm.speech_supported());
        // the real processor: whatever it is, the answer matches its flags
        let real = Machine::detect();
        assert_eq!(
            real.speech_supported(),
            real.arch != "x86_64" || real.cpu.missing().is_empty()
        );
    }
}
