use anyhow::Context;
use serde::Serialize;

use super::read_file;

/// CPU brand, logical-core count, and max clock frequency (`/proc/cpuinfo`).
#[derive(Debug, Default, Serialize)]
pub struct CpuInfo {
    /// CPU model name, e.g. `AMD Ryzen 7 5800X`.
    pub brand: String,
    /// Number of logical cores.
    pub count: u32,
    /// Maximum core frequency in GHz.
    pub ghz: f64,
}

/// Collect the CPU brand, core count, and max clock frequency from
/// `/proc/cpuinfo`.
pub fn collect() -> anyhow::Result<CpuInfo> {
    let content = read_file("/proc/cpuinfo")?;
    let (brand, count, ghz) = parse(&content).context("parse /proc/cpuinfo")?;
    Ok(CpuInfo { brand, count, ghz })
}

/// Parse CPU brand, logical-core count, and max clock frequency (GHz, rounded
/// to 2 decimals) from `/proc/cpuinfo` content.
fn parse(content: &str) -> Option<(String, u32, f64)> {
    let mut brand = None;
    let mut count = 0_u32;
    let mut max_mhz = 0.0_f64;
    for line in content.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "processor" => count += 1,
            "model name" if brand.is_none() => brand = Some(value.to_owned()),
            "cpu MHz" => {
                let Ok(mhz) = value.parse::<f64>() else {
                    continue;
                };
                max_mhz = max_mhz.max(mhz);
            }
            _ => {}
        }
    }
    Some((brand?, count, round2(max_mhz / 1000.0)))
}

/// Round a frequency to 2 decimal places.
fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cpuinfo_fields() {
        let cases = [
            (
                "multi-core max freq",
                "\
processor       : 0
model name      : AMD Ryzen 7 5800X 8-Core Processor
cpu MHz         : 4700.000

processor       : 1
model name      : AMD Ryzen 7 5800X 8-Core Processor
cpu MHz         : 2199.996
",
                Some((
                    "AMD Ryzen 7 5800X 8-Core Processor".to_owned(),
                    2_u32,
                    4.7_f64,
                )),
            ),
            (
                "single core",
                "\
processor       : 2
model name      : Intel(R) N150
cpu MHz         : 1694.503
",
                Some(("Intel(R) N150".to_owned(), 1_u32, 1.69_f64)),
            ),
            (
                "no model name",
                "\
processor       : 0
cpu MHz         : 3400.000
",
                None,
            ),
        ];
        for (label, content, expected) in cases {
            assert_eq!(parse(content), expected, "case: {label}");
        }
    }
}
