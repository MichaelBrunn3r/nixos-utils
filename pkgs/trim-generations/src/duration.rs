use std::str::FromStr;

use anyhow::{Context, bail};
use chrono::{Datelike, Months, NaiveDate, NaiveDateTime, TimeDelta, Timelike};

//region NominalDuration
/// A duration expressed as a unit and a value.
///
/// "Nominal", because the units are calendar-aware: `1M` is one calendar month
/// rather than a fixed 30 days.
///
/// ## Order
/// Ordered fine -> coarse by [`Unit`], then by `value` (a smaller `value` is finer, `1w < 2w`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NominalDuration {
    pub unit: Unit,
    pub value: u32,
}

// Represent e.g. `{unit: Week, value: 2}` as "2w"
impl std::fmt::Display for NominalDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.value, self.unit)
    }
}

impl NominalDuration {
    /// Subtract this duration from a timestamp.
    pub fn before(&self, time: NaiveDateTime) -> anyhow::Result<NaiveDateTime> {
        match self.unit {
            Unit::Hour => time.checked_sub_signed(TimeDelta::hours(self.value as i64)),
            Unit::Day => time.checked_sub_signed(TimeDelta::days(self.value as i64)),
            Unit::Week => time.checked_sub_signed(TimeDelta::weeks(self.value as i64)),
            Unit::Month => time.checked_sub_months(Months::new(self.value)),
            Unit::Year => time.checked_sub_months(Months::new(self.value.saturating_mul(12))),
        }
        .with_context(|| format!("duration {self:?} overflows chrono's date range"))
    }

    /// Add this duration to a timestamp.
    pub fn after(&self, time: NaiveDateTime) -> anyhow::Result<NaiveDateTime> {
        match self.unit {
            Unit::Hour => time.checked_add_signed(TimeDelta::hours(self.value as i64)),
            Unit::Day => time.checked_add_signed(TimeDelta::days(self.value as i64)),
            Unit::Week => time.checked_add_signed(TimeDelta::weeks(self.value as i64)),
            Unit::Month => time.checked_add_months(Months::new(self.value)),
            Unit::Year => time.checked_add_months(Months::new(self.value.saturating_mul(12))),
        }
        .with_context(|| format!("duration {self:?} overflows chrono's date range"))
    }

    /// Truncate a timestamp to the start of this duration's unit (e.g. days -> 00:00, weeks -> Monday).
    pub fn floor_to_unit(&self, time: NaiveDateTime) -> Option<NaiveDateTime> {
        match self.unit {
            Unit::Hour => time.with_minute(0)?.with_second(0)?.with_nanosecond(0),
            Unit::Day => time.date().and_hms_opt(0, 0, 0),
            Unit::Week => {
                let since_monday = u64::from(time.weekday().num_days_from_monday());
                time.date()
                    .checked_sub_days(chrono::Days::new(since_monday))?
                    .and_hms_opt(0, 0, 0)
            }
            Unit::Month => time.date().with_day(1)?.and_hms_opt(0, 0, 0),
            Unit::Year => NaiveDate::from_ymd_opt(time.year(), 1, 1)?.and_hms_opt(0, 0, 0),
        }
    }
}

impl FromStr for NominalDuration {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut chars = s.chars();
        let unit = chars.next_back().context("empty duration")?;
        let value: u32 = chars.as_str().parse().with_context(|| {
            format!("invalid duration '{s:?}': expected <number><unit> like 3d")
        })?;
        if value == 0 {
            bail!("invalid duration '{s:?}': number must be positive");
        }
        let unit = match unit {
            'h' => Unit::Hour,
            'd' => Unit::Day,
            'w' => Unit::Week,
            'M' => Unit::Month,
            'y' => Unit::Year,
            _ => {
                bail!("invalid duration '{s:?}': unknown unit '{unit}' (expected h, d, w, M, y)")
            }
        };
        Ok(Self { value, unit })
    }
}

//endregion NominalDuration

//region Unit
/// A calendar-aware unit of time that a retention policy can be expressed in.
///
/// ## Order
/// Ordered fine -> coarse: `Hour < Day < Week < Month < Year`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unit {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Unit::Hour => 'h',
                Unit::Day => 'd',
                Unit::Week => 'w',
                Unit::Month => 'M',
                Unit::Year => 'y',
            }
        )
    }
}
//endregion Unit

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `NaiveDateTime` from its parts.
    fn date(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn before() {
        let wed_0826_1530 = date(2026, 8, 26, 15, 30, 0);
        let tue_0331_1200 = date(2026, 3, 31, 12, 0, 0);

        for (raw, time, expected) in [
            ("2h", wed_0826_1530, date(2026, 8, 26, 13, 30, 0)),
            ("3d", wed_0826_1530, date(2026, 8, 23, 15, 30, 0)),
            ("2w", wed_0826_1530, date(2026, 8, 12, 15, 30, 0)),
            ("1M", tue_0331_1200, date(2026, 2, 28, 12, 0, 0)),
            ("1y", wed_0826_1530, date(2025, 8, 26, 15, 30, 0)),
        ] {
            let dur: NominalDuration = raw.parse().unwrap();
            assert_eq!(dur.before(time).unwrap(), expected, "for {dur:?}");
        }
    }

    #[test]
    fn after() {
        let wed_0826_1530 = date(2026, 8, 26, 15, 30, 0);
        for (s, time, expected) in [
            ("2h", wed_0826_1530, date(2026, 8, 26, 17, 30, 0)),
            ("3d", wed_0826_1530, date(2026, 8, 29, 15, 30, 0)),
            ("1M", wed_0826_1530, date(2026, 9, 26, 15, 30, 0)),
            ("1y", wed_0826_1530, date(2027, 8, 26, 15, 30, 0)),
        ] {
            let dur: NominalDuration = s.parse().unwrap();
            assert_eq!(dur.after(time).unwrap(), expected, "for {dur:?}");
        }
    }

    #[test]
    fn order() {
        let mut durations: Vec<NominalDuration> =
            ["1M", "2h", "1d", "1w", "1y", "1h", "2d", "6M", "2w"]
                .into_iter()
                .map(|s| s.parse().unwrap())
                .collect();
        durations.sort();
        let expected: Vec<NominalDuration> = ["1h", "2h", "1d", "2d", "1w", "2w", "1M", "6M", "1y"]
            .into_iter()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(durations, expected);
    }
}
