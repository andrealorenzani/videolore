// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! What the running machine can do: logical cores and installed memory (read once at
//! start-up). Used by the proxy rule and to size the speech-to-text thread count.

/// Bytes in one GiB.
pub const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineInfo {
    pub cores: usize,
    pub ram_bytes: u64,
}

impl MachineInfo {
    /// Reads the real machine (cores from the standard library, memory from sysinfo).
    pub fn detect() -> MachineInfo {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        // Unknown memory counts as 0, which makes the proxy rule conservative.
        let ram_bytes = sysinfo::System::new()
            .map(|mut sys| {
                sys.refresh_memory();
                sys.total_memory()
            })
            .unwrap_or(0);
        MachineInfo { cores, ram_bytes }
    }

    /// Threads for speech recognition: half the cores, between 1 and 4.
    pub fn stt_threads(&self) -> usize {
        (self.cores / 2).clamp(1, 4)
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

    #[test]
    fn detects_something_sane() {
        let m = MachineInfo::detect();
        assert!(m.cores >= 1);
        assert!(m.ram_bytes > 0);
        assert!(own_rss() > 0);
        for (cores, threads) in [(1, 1), (2, 1), (6, 3), (8, 4), (64, 4)] {
            let m = MachineInfo {
                cores,
                ram_bytes: GIB,
            };
            assert_eq!(m.stt_threads(), threads);
        }
    }
}
