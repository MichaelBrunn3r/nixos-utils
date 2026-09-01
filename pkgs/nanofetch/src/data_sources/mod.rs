//! System data sources, one module per source.
//!
//! Sources expose only their own (nearly) raw data; merging across sources
//! happens in `gather`. Every source follows the same shape:
//!
//! - a **model** — the typed facts the source produces (e.g. `Load`), kept in
//!   the source's module;
//! - a **collect** function — `pub fn collect() -> anyhow::Result<Model>` —
//!   that reads the source (files, commands, cache) and returns the model;
//!   failures surface as an `Err` for `gather` to downgrade to `None`, never
//!   panic;
//! - a private **parse** — `fn parse(&str) -> Option<...>` — the pure
//!   text-to-model step, separated from the IO so it can be unit-tested with
//!   sample input.

pub mod cpuinfo;
pub mod df;
pub mod ip_addr;
pub mod loadavg;
pub mod lsblk;
pub mod meminfo;
pub mod net_dev;
pub mod os_release;
pub mod uptime;

use std::fs;

use anyhow::Context;
pub use cpuinfo::CpuInfo;
pub use df::Fs;
pub use loadavg::Load;
pub use lsblk::LsblkDisk;
pub use meminfo::Memory;
pub use uptime::Uptime;

/// Read a file's contents.
pub fn read_file(path: &str) -> anyhow::Result<String> {
    fs::read_to_string(path).with_context(|| format!("read {path}"))
}

/// Read the first non-empty line of a file, trimmed.
pub fn read_line(path: &str) -> anyhow::Result<String> {
    read_file(path)?
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .with_context(|| format!("empty file: {path}"))
}
