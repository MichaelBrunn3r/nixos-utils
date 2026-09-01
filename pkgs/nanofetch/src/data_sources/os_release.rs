use anyhow::Context;

use super::read_file;

/// Collect the pretty OS name from `/etc/os-release`.
pub fn collect() -> anyhow::Result<String> {
    let content = read_file("/etc/os-release")?;
    parse(&content).context("parse /etc/os-release")
}

/// Extract the OS name from `/etc/os-release` content: `PRETTY_NAME`, falling
/// back to `NAME` + `VERSION` (e.g. `NixOS 26.11 (Zokor)`).
fn parse(content: &str) -> Option<String> {
    let mut pretty_name = None;
    let mut name = None;
    let mut version = None;
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        if value.is_empty() {
            continue;
        }
        match key {
            "PRETTY_NAME" => pretty_name = Some(value.to_owned()),
            "NAME" => name = Some(value.to_owned()),
            "VERSION" => version = Some(value.to_owned()),
            _ => {}
        }
    }
    if let Some(pretty) = pretty_name {
        return Some(pretty);
    }
    let name = name?;
    Some(match version {
        Some(version) => format!("{name} {version}"),
        None => name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_os_release_pretty_name() {
        let cases = [
            (
                "pretty name preferred",
                "\
NAME=NixOS
PRETTY_NAME=\"NixOS 26.11 (Zokor)\"
VERSION=\"26.11 (Zokor)\"
",
                Some("NixOS 26.11 (Zokor)"),
            ),
            (
                "fallback name + version",
                "\
NAME=NixOS
VERSION=\"26.11 (Zokor)\"
",
                Some("NixOS 26.11 (Zokor)"),
            ),
            (
                "fallback name only",
                "\
NAME=NixOS
ID=nixos
",
                Some("NixOS"),
            ),
            (
                "empty pretty name falls back",
                "\
PRETTY_NAME=\"\"
NAME=NixOS
VERSION=\"26.11 (Zokor)\"
",
                Some("NixOS 26.11 (Zokor)"),
            ),
            ("unquoted pretty name", "PRETTY_NAME=NixOS\n", Some("NixOS")),
            ("nothing", "ID=nixos\n", None),
        ];
        for (label, content, expected) in cases {
            assert_eq!(parse(content).as_deref(), expected, "case: {label}");
        }
    }
}
