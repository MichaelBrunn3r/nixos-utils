use std::collections::BTreeMap;

use serde::Serialize;

use super::read_file;

pub type NetIfacesStats = BTreeMap<String, NetIfaceStats>;

/// Cumulative byte counters for one network interface, keyed by interface
/// name.
#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct NetIfaceStats {
    /// Cumulative bytes received.
    pub rx: u64,
    /// Cumulative bytes transmitted.
    pub tx: u64,
}

/// Collect the cumulative per-interface byte counters from `/proc/net/dev`,
/// keyed by interface name.
pub fn collect() -> anyhow::Result<NetIfacesStats> {
    let content = read_file("/proc/net/dev")?;
    Ok(parse(&content))
}

/// Parse cumulative per-interface byte counters from `/proc/net/dev`.
///
/// The receive column block precedes the transmit block: rx is the first
/// numeric column and tx the ninth.
fn parse(content: &str) -> NetIfacesStats {
    let mut out = BTreeMap::new();
    for line in content.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let nums: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|field| field.parse().ok())
            .collect();
        if nums.len() >= 9 {
            out.insert(
                name.to_owned(),
                NetIfaceStats {
                    rx: nums[0],
                    tx: nums[8],
                },
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_netdev_bytes() {
        const CONTENT: &str = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1129545    7806    0    0    0     0          0         0  1129545    7806    0    0    0     0       0          0
wlp166s0: 366413358 3042833    0 1827098    0     0          0         0 112625303  175748    0  121    0     0       0          0
";
        let got = parse(CONTENT);
        assert_eq!(
            got.get("lo"),
            Some(&NetIfaceStats {
                rx: 1_129_545,
                tx: 1_129_545
            })
        );
        assert_eq!(
            got.get("wlp166s0"),
            Some(&NetIfaceStats {
                rx: 366_413_358,
                tx: 112_625_303
            })
        );
    }

    #[test]
    fn parse_netdev_garbage() {
        assert!(parse("not dev output\n").is_empty());
    }
}
