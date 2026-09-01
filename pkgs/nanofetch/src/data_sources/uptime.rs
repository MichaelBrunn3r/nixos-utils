use anyhow::Context;
use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;

use super::read_file;

/// Uptime duration plus the boot timestamp it was derived from.
#[derive(Debug, Default, Serialize)]
pub struct Uptime {
    /// Uptime duration since boot.
    pub duration: TimeDelta,
    /// Boot timestamp.
    pub booted_at: DateTime<Utc>,
}

/// Collect the uptime duration from `/proc/uptime` and derive the boot
/// timestamp.
pub fn collect() -> anyhow::Result<Uptime> {
    let content = read_file("/proc/uptime")?;
    let duration = parse(&content).context("parse /proc/uptime")?;
    Ok(Uptime {
        duration,
        booted_at: Utc::now() - duration,
    })
}

/// Parse the uptime duration from `/proc/uptime` content.
fn parse(content: &str) -> Option<TimeDelta> {
    let seconds = content.split_whitespace().next()?;
    let whole = seconds.split_once('.').map_or(seconds, |(whole, _)| whole);
    TimeDelta::try_seconds(whole.parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uptime_duration() {
        let cases = [
            ("typical", "270553.72 986453.11\n", Some(270_553_i64)),
            ("zero", "0.00 0.00\n", Some(0)),
            ("garbage", "nope\n", None),
        ];
        for (label, content, expected) in cases {
            let got = parse(content).map(|d| d.num_seconds());
            assert_eq!(got, expected, "case: {label}");
        }
    }
}
