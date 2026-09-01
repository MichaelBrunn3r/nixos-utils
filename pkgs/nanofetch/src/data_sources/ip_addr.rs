use std::collections::BTreeMap;
use std::process::Command;

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub type NetIfaces = BTreeMap<String, NetIface>;

/// A network interface with its addresses.
#[derive(Debug, Default, Serialize)]
pub struct NetIface {
    /// MAC address.
    pub mac: Option<String>,
    /// Non-loopback IP addresses.
    pub ips: Vec<String>,
}

/// Collect the network interfaces via `ip -j addr`, keyed by interface name.
pub fn collect() -> anyhow::Result<NetIfaces> {
    let out = Command::new("ip")
        .args(["-j", "addr"])
        .output()
        .context("run `ip -j addr`")?;
    if !out.status.success() {
        let status = out.status;
        anyhow::bail!("`ip -j addr` exited with {status}");
    }
    let Some(ifaces) = parse_ip_addr(&String::from_utf8_lossy(&out.stdout)) else {
        log::debug!("failed to parse `ip -j addr` output");
        anyhow::bail!("failed to parse `ip -j addr` output");
    };
    Ok(ifaces)
}

/// Parse `ip -j addr` JSON into interfaces keyed by name with their
/// non-loopback addresses.
///
/// Loopback addresses (`127.0.0.0/8`, `::1`) are dropped and interfaces left
/// without any non-loopback address are dropped. IPs are sorted by protocol
/// (IPv4 before IPv6) then address. Interfaces without a MAC address (e.g.
/// point-to-point links) keep `mac` as `None`.
fn parse_ip_addr(json: &str) -> Option<NetIfaces> {
    #[derive(Deserialize)]
    struct AddrInfo {
        family: String,
        local: String,
    }

    #[derive(Deserialize)]
    struct IpIf {
        ifname: String,
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        addr_info: Vec<AddrInfo>,
    }

    let ifaces: Vec<IpIf> = serde_json::from_str(json).ok()?;
    let mut out = BTreeMap::new();
    for iface in ifaces {
        let mut ips: Vec<(String, String)> = iface
            .addr_info
            .into_iter()
            .filter(|addr| matches!(addr.family.as_str(), "inet" | "inet6"))
            .filter(|addr| !is_loopback(&addr.local))
            .map(|addr| (addr.family, addr.local))
            .collect();
        ips.sort();
        if ips.is_empty() {
            continue;
        }
        out.insert(
            iface.ifname,
            NetIface {
                mac: iface.address,
                ips: ips.into_iter().map(|(_, address)| address).collect(),
            },
        );
    }
    Some(out)
}

/// Whether an address is in a loopback range (`127.0.0.0/8` or `::1`).
fn is_loopback(address: &str) -> bool {
    address == "::1" || address.starts_with("127.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ip_addr_filters_and_sorts() {
        let json = r#"[
            {"ifname":"wlp166s0","address":"88:d8:2e:99:cf:55","addr_info":[
                {"family":"inet6","local":"fe80::5950:7d94:50df:4475"},
                {"family":"inet","local":"192.168.178.40"},
                {"family":"inet6","local":"2a00:a520:1154:6600:2fa:ac66:2e6b:fce5"}
            ]},
            {"ifname":"lo","address":"00:00:00:00:00:00","addr_info":[
                {"family":"inet","local":"127.0.0.1"},
                {"family":"inet6","local":"::1"}
            ]}
        ]"#;
        let got = parse_ip_addr(json).expect("valid json");
        // `lo` is dropped (only loopback addresses); the other iface keeps its
        // non-loopback IPs sorted by protocol (inet) then address.
        assert_eq!(got.len(), 1);
        let iface = got.get("wlp166s0").expect("iface present");
        assert_eq!(iface.mac.as_deref(), Some("88:d8:2e:99:cf:55"));
        assert_eq!(
            iface.ips,
            [
                "192.168.178.40",
                "2a00:a520:1154:6600:2fa:ac66:2e6b:fce5",
                "fe80::5950:7d94:50df:4475",
            ]
        );
    }

    #[test]
    fn parse_ip_addr_keys_by_name() {
        let json = r#"[
            {"ifname":"wlp166s0","address":"88:d8:2e:99:cf:55","addr_info":[
                {"family":"inet","local":"192.168.178.40"}
            ]},
            {"ifname":"enp3s0","address":"aa:bb:cc:dd:ee:ff","addr_info":[
                {"family":"inet","local":"10.0.0.2"}
            ]}
        ]"#;
        let ifaces = parse_ip_addr(json).expect("valid json");
        let names: Vec<&str> = ifaces.keys().map(String::as_str).collect();
        assert_eq!(names, ["enp3s0", "wlp166s0"]);
    }

    #[test]
    fn parse_ip_addr_drops_loopback_only() {
        let json = r#"[
            {"ifname":"lo","address":"00:00:00:00:00:00","addr_info":[
                {"family":"inet","local":"127.0.0.1"}
            ]},
            {"ifname":"docker0","address":"02:42:0a:00:00:01","addr_info":[]}
        ]"#;
        assert!(parse_ip_addr(json).expect("valid json").is_empty());
    }

    #[test]
    fn parse_ip_addr_iface_without_mac() {
        // `ip -j addr` omits `address` for interfaces without a MAC (e.g. a
        // point-to-point link), which must not fail the whole parse.
        let json = r#"[
            {"ifname":"tailscale0","flags":["POINTOPOINT","UP"],"addr_info":[
                {"family":"inet6","local":"fe80::f343:639d:be07:5a34"}
            ]}
        ]"#;
        let got = parse_ip_addr(json).expect("valid json");
        assert_eq!(got.len(), 1);
        let iface = got.get("tailscale0").expect("iface present");
        assert_eq!(iface.mac, None);
        assert_eq!(iface.ips, ["fe80::f343:639d:be07:5a34"]);
    }

    #[test]
    fn parse_ip_addr_garbage() {
        assert!(parse_ip_addr("not json").is_none());
    }
}
