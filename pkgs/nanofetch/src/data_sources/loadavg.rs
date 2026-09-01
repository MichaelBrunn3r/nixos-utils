use anyhow::Context;
use serde::Serialize;

use super::read_file;

/// 1/5/15-minute load averages (`/proc/loadavg`).
#[derive(Debug, Default, Serialize)]
pub struct Load {
    /// 1-minute load average.
    pub one: f64,
    /// 5-minute load average.
    pub five: f64,
    /// 15-minute load average.
    pub fifteen: f64,
}

/// Collect the 1/5/15-minute load averages from `/proc/loadavg`.
pub fn collect() -> anyhow::Result<Load> {
    let content: String = read_file("/proc/loadavg")?;
    parse(&content).context("parse /proc/loadavg")
}

/// Parse 1/5/15-minute load averages from `/proc/loadavg` content.
fn parse(content: &str) -> Option<Load> {
    let mut fields = content.split_whitespace();
    Some(Load {
        one: fields.next()?.parse().ok()?,
        five: fields.next()?.parse().ok()?,
        fifteen: fields.next()?.parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_loadavg_fields() {
        let cases = [
            (
                "typical",
                "3.45 3.60 3.80 5/482 12345\n",
                Some((3.45_f64, 3.60, 3.80)),
            ),
            ("garbage", "garbage\n", None),
        ];
        for (label, content, expected) in cases {
            let got = parse(content).map(|l| (l.one, l.five, l.fifteen));
            assert_eq!(got, expected, "case: {label}");
        }
    }
}
