use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{Context, bail};
use chrono::NaiveDateTime;

use crate::duration::NominalDuration;
use crate::generation::{Generation, GenerationId, Generations};

/// Policy controlling which generations to trim.
#[derive(Debug)]
pub struct Policy {
    /// The maximum age cutoff.
    pub max_age: NominalDuration,
    /// How many of the newest old gens are always kept, even when older than `max_age`.
    pub keep_last: u32,
    /// The normalized retention rules.
    pub rules: Vec<RetentionRule>,
}

impl Policy {
    pub fn new(max_age: NominalDuration, keep_last: u32, rules: Vec<RetentionRule>) -> Self {
        Self {
            max_age,
            keep_last,
            rules: normalize_rules(rules),
        }
    }

    /// Plan the retention actions for some generations at a specified current time.
    ///
    /// ## Returns
    ///
    /// One [`Decision`] per input generation, in the same order. Each generation is either
    /// kept ([`Action::Keep`], with the [`KeepReason`]s that selected it) or removed
    /// ([`Action::Remove`], with the reason it was trimmed). An error is returned if `now` is
    /// before `epoch` and `max_age` or any rule cannot be computed.
    ///
    pub fn plan(
        &self,
        now: NaiveDateTime,
        generations: &Generations,
    ) -> anyhow::Result<Vec<Decision>> {
        let cutoff = self.max_age.before(now)?;

        // Collections of reasons to keep gens
        let mut reasons_keep: HashMap<GenerationId, Vec<KeepReason>> = HashMap::new();

        // Keep the newest N old gens
        generations
            .older()
            .iter()
            .rev()
            .take(self.keep_last as usize)
            .map(|g| (g.id, vec![KeepReason::KeepLastN(self.keep_last)]))
            .collect_into(&mut reasons_keep);

        // Gens older than >= max_age are always removed (except keep_last).
        // Gens younger than current remain untouched.
        // Thus rules can only work on `current < eligible <= max_age`.
        let eligible: Vec<&Generation> = generations
            .older()
            .iter()
            .filter(|g| g.created >= cutoff)
            .collect();

        for rule in &self.rules {
            // Example: duration=1h, repeat=3, max_age=2h
            //
            //                          bucket(now)
            //         -2h       -1h        0h       +1h        // time offsets from bucket(now)
            // past <---|----v----|---------|----v----|--->     // timeline
            //             cutoff               now
            //
            //               [______eligible____)               // rules only work on gens in this range
            //          [___W2___)[___W1___)[___W0___)          // Buckets [start, end)
            for (start, end) in rule.past_buckets_from(now)?.take(rule.repeat as usize) {
                // Stop if the timespan is older than the cutoff (start <= end <= cutoff)
                if end <= cutoff {
                    break;
                }

                // Keep the newest generation in the timespan, if any.
                if let Some(newest) = eligible
                    .iter()
                    .copied()
                    .rfind(|g| g.created >= start && g.created < end)
                {
                    reasons_keep
                        .entry(newest.id)
                        .or_default()
                        .push(KeepReason::Rule(*rule));
                }
            }
        }

        let plan = generations
            .older()
            .iter()
            .map(|g| {
                let action = match reasons_keep.remove(&g.id) {
                    Some(reasons) => Action::Keep(reasons),
                    None if g.created < cutoff => Action::Remove(RemoveReason::OlderThanMaxAge),
                    None => Action::Remove(RemoveReason::NoRule),
                };
                Decision { id: g.id, action }
            })
            .collect();

        Ok(plan)
    }
}

//region RetentionRule
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionRule {
    pub duration: NominalDuration,
    pub repeat: u32,
}

// Represent `{duration: {unit: Week, value: 2}, repeat: 10}` as "2w*10"
impl std::fmt::Display for RetentionRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}*{}", self.duration, self.repeat)
    }
}

impl FromStr for RetentionRule {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let (len, rep) = s.split_once('*').with_context(|| {
            format!("invalid rule '{s:?}': expected <duration>*<repeat>, e.g. 2w*10")
        })?;
        let length = len.parse()?;
        let repetition: u32 = rep
            .parse()
            .with_context(|| format!("invalid rule '{s:?}': unable to parse repeat"))?;
        if repetition == 0 {
            bail!("invalid rule '{s:?}': repeat must be positive");
        }
        Ok(Self {
            duration: length,
            repeat: repetition,
        })
    }
}

impl RetentionRule {
    /// Create an infinite iterator of timespan buckets of length `duration` walking from `now`
    /// into the past. The first bucket is the one containing `now`; because `now` can sit
    /// anywhere within a bucket, that first bucket often extends into the future (its end may
    /// be after `now`). Every subsequent bucket lies entirely in the past.
    pub fn past_buckets_from(
        &self,
        now: NaiveDateTime,
    ) -> anyhow::Result<impl Iterator<Item = (NaiveDateTime, NaiveDateTime)>> {
        let duration = self.duration;

        let mut start = duration.start_of_bucket_containing(now)?;

        let first_end = duration.after(start)?;
        Ok(gen move {
            let mut end = first_end;
            loop {
                yield (start, end);
                end = start;
                let Some(next) = duration.before(start).ok() else {
                    break;
                };
                start = next;
            }
        })
    }
}

/// Sort rules by coarseness (duration of unit) fine -> coarse and drop duplicates that share an
/// identical duration, keeping the one with the larger repeat.
fn normalize_rules(rules: Vec<RetentionRule>) -> Vec<RetentionRule> {
    let mut rules = rules;
    rules.sort_by(|a, b| {
        a.duration
            .cmp(&b.duration)
            .then_with(|| b.repeat.cmp(&a.repeat))
    });
    rules.dedup_by(|a, b| a.duration == b.duration);
    rules
}
//endregion RetentionRule

//region Decisions
/// The decision for a single generation, in the same order as the input generations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub id: GenerationId,
    pub action: Action,
}

/// The action to take for one generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Keep it, for these reasons (keep_last and/or one entry per rule that selected it).
    Keep(Vec<KeepReason>),
    /// Remove it, for this reason.
    Remove(RemoveReason),
}

//region KeepReason
/// Why a generation is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepReason {
    /// Kept because it is among the newest N old generations.
    KeepLastN(u32),
    /// Kept because it is the newest generation in one of this rule's buckets.
    Rule(RetentionRule),
}

impl std::fmt::Display for KeepReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeepReason::KeepLastN(n) => write!(f, "keep_last {n}"),
            KeepReason::Rule(rule) => write!(f, "{rule}"),
        }
    }
}
//endregion KeepReason

//region RemoveReason
/// Why a generation is removed. Exactly one applies per generation: `max_age` excludes
/// everything below the cutoff from the rules' domain, so the two never co-occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveReason {
    /// Removed because it is older than the `max_age` cutoff.
    OlderThanMaxAge,
    /// Removed because it is within `max_age` and no rule (nor keep_last) selected it.
    NoRule,
}

impl std::fmt::Display for RemoveReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoveReason::OlderThanMaxAge => write!(f, "> max age"),
            RemoveReason::NoRule => write!(f, "no rule"),
        }
    }
}
//endregion RemoveReason
//endregion Decisions

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};

    use super::*;

    /// Build a `NaiveDateTime` from its parts.
    fn date(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    mod rule {
        use super::*;
        use crate::duration::NominalDuration;

        #[test]
        fn parse() {
            for (input, expected, repeat) in [
                ("2h*3", "2h", 3),
                ("3d*7", "3d", 7),
                ("2w*10", "2w", 10),
                ("6M*12", "6M", 12),
                ("1y*1", "1y", 1),
            ] {
                let rule: RetentionRule = input.parse().unwrap();
                let expected: NominalDuration = expected.parse().unwrap();
                assert_eq!(rule.duration, expected, "for {input:?}");
                assert_eq!(rule.repeat, repeat, "for {input:?}");
            }
        }

        #[test]
        fn parse_rejects_invalid() {
            for s in ["", "2w", "2w*", "*10", "2w*0", "2w*abc", "2w*-1", "x*10"] {
                assert!(s.parse::<RetentionRule>().is_err(), "should reject {s:?}");
            }
        }

        #[test]
        fn normalize() {
            let cases: [(&str, &[&str], &[&str]); 4] = [
                (
                    "sorts fine -> coarse",
                    &["1M*12", "1d*7", "1h*3", "2w*10"],
                    &["1h*3", "1d*7", "2w*10", "1M*12"],
                ),
                (
                    "dedups identical durations, keeping the larger repeat",
                    &["1d*3", "1d*7", "1d*7", "1d*4"],
                    &["1d*7"],
                ),
                (
                    "keeps same unit with different durations",
                    &["2d*4", "1d*7"],
                    &["1d*7", "2d*4"],
                ),
                ("keeps empty rules empty", &[], &[]),
            ];

            for (label, input, expected) in cases {
                let policy = Policy::new(
                    "2y".parse().unwrap(),
                    3,
                    input.iter().copied().map(|s| s.parse().unwrap()).collect(),
                );
                let expected: Vec<RetentionRule> = expected
                    .iter()
                    .copied()
                    .map(|s| s.parse().unwrap())
                    .collect();
                assert_eq!(policy.rules, expected, "case: {label}");
            }
        }

        #[test]
        fn buckets() {
            let rule: RetentionRule = "1d*1".parse().unwrap();
            let now = date(2026, 8, 24, 12, 0, 0);
            let buckets: Vec<_> = rule.past_buckets_from(now).unwrap().take(3).collect();
            assert_eq!(
                buckets,
                vec![
                    (date(2026, 8, 24, 0, 0, 0), date(2026, 8, 25, 0, 0, 0)),
                    (date(2026, 8, 23, 0, 0, 0), date(2026, 8, 24, 0, 0, 0)),
                    (date(2026, 8, 22, 0, 0, 0), date(2026, 8, 23, 0, 0, 0)),
                ]
            );
        }

        #[test]
        fn buckets_are_stable_across_runs() {
            let rule: RetentionRule = "2w*5".parse().unwrap();
            let t1 = date(2026, 8, 24, 12, 0, 0); // Monday
            let t2 = t1 + chrono::Duration::days(7); // next Monday
            let w1: Vec<_> = rule.past_buckets_from(t1).unwrap().take(4).collect();
            let w2: Vec<_> = rule.past_buckets_from(t2).unwrap().take(4).collect();
            // All buckets except the newest are identical between the two runs.
            assert_eq!(w1[..3], w2[1..]);
            // The newest bucket starts at its fixed position on the timeline (CE-anchored for this week).
            assert_eq!(
                w1[0],
                (date(2026, 8, 17, 0, 0, 0), date(2026, 8, 31, 0, 0, 0))
            );
        }
    }

    mod plan {
        use super::*;
        use crate::generation::is_sorted_chronologically;

        /// Build a `Generations` value from `(id, created)` pairs, treating them as the older
        /// generations and appending a current generation after them.
        fn gens(items: &[(u32, NaiveDateTime)]) -> Generations {
            let mut generations: Vec<Generation> = items
                .iter()
                .map(|&(id, created)| Generation::new(GenerationId::new(id), created))
                .collect();
            assert!(
                is_sorted_chronologically(&generations),
                "generations must be in chronological order"
            );

            // Append a current generation after the older ones so the fixture is realistic.
            let (last_id, last_created) = items
                .last()
                .copied()
                .unwrap_or((0, date(1970, 1, 1, 0, 0, 0)));
            generations.push(Generation::new(
                GenerationId::new(last_id + 1),
                last_created + chrono::Duration::seconds(1),
            ));
            let current = generations.len() - 1;

            Generations {
                generations,
                current,
            }
        }

        /// Kept generation ids returned by [`Policy::plan`], in input order.
        fn kept_ids(policy: &Policy, now: NaiveDateTime, gens: &Generations) -> Vec<GenerationId> {
            policy
                .plan(now, gens)
                .unwrap()
                .into_iter()
                .filter(|d| matches!(d.action, Action::Keep(_)))
                .map(|d| d.id)
                .collect()
        }

        #[test]
        fn selects_gens_to_keep() {
            let cases: [(&str, NaiveDateTime, &str, u32, Generations, &[u32]); 4] = [
                (
                    "keep_last=5 > 3 -> keeps all gens, even though they are > max age",
                    date(2026, 8, 24, 12, 0, 0),
                    "2y",
                    5,
                    gens(&[
                        (1, date(2023, 1, 1, 0, 0, 0)),
                        (2, date(2023, 1, 1, 0, 0, 0)),
                        (3, date(2023, 1, 1, 0, 0, 0)),
                    ]),
                    &[1, 2, 3],
                ),
                (
                    "0 gens -> nothing to keep",
                    date(2026, 8, 24, 12, 0, 0),
                    "2y",
                    3,
                    gens(&[]),
                    &[],
                ),
                (
                    "no rules: keeps the newest keep_last only, other gens are removed too",
                    date(2026, 8, 24, 12, 0, 0), // cutoff for "2y" = 2024-08-24 12:00
                    "2y",
                    2,
                    gens(&[
                        (1, date(2024, 1, 1, 0, 0, 0)), // older than cutoff -> removed
                        (2, date(2025, 1, 1, 0, 0, 0)), // within max_age, no rule -> removed
                        (3, date(2025, 6, 1, 0, 0, 0)), // within max_age, no rule -> removed
                        (4, date(2026, 1, 1, 0, 0, 0)), // newest
                        (5, date(2026, 8, 1, 0, 0, 0)), // newest
                    ]),
                    &[4, 5],
                ),
                (
                    "keep_last pulls gens older than the cutoff (precedence over max_age)",
                    date(2026, 8, 24, 12, 0, 0), // cutoff for "30d" = 2026-07-25 12:00
                    "30d",
                    3,
                    gens(&[
                        (1, date(2026, 7, 1, 0, 0, 0)),  // older than cutoff
                        (2, date(2026, 7, 20, 0, 0, 0)), // older than cutoff
                        (3, date(2026, 8, 1, 0, 0, 0)),  // within bucket
                    ]),
                    &[1, 2, 3],
                ),
            ];

            for (label, now, max_age, keep_last, gens, expected) in cases {
                let policy = Policy::new(max_age.parse().unwrap(), keep_last, vec![]);
                let expected: Vec<GenerationId> =
                    expected.iter().map(|&id| GenerationId::new(id)).collect();
                assert_eq!(kept_ids(&policy, now, &gens), expected, "case: {label}");
            }
        }

        #[test]
        fn actions_and_reasons() {
            let now = date(2026, 8, 24, 12, 0, 0);
            // cutoff for "5d" = 2026-08-19 12:00
            let policy = Policy::new("5d".parse().unwrap(), 1, ["1d*1".parse().unwrap()].to_vec());
            let gens = gens(&[
                (1, date(2026, 8, 10, 0, 0, 0)), // older than cutoff -> Remove(OlderThanMaxAge)
                (2, date(2026, 8, 20, 0, 0, 0)), // within max age, no rule -> Remove(NoRule)
                (3, date(2026, 8, 24, 9, 0, 0)), // keep_last + newest in the 1d bucket
            ]);
            let decisions = policy.plan(now, &gens).unwrap();
            assert_eq!(decisions[0].id, GenerationId::new(1));
            assert_eq!(
                decisions[0].action,
                Action::Remove(RemoveReason::OlderThanMaxAge)
            );
            assert_eq!(decisions[1].action, Action::Remove(RemoveReason::NoRule));
            assert_eq!(
                decisions[2].action,
                Action::Keep(vec![
                    KeepReason::KeepLastN(1),
                    KeepReason::Rule("1d*1".parse().unwrap()),
                ])
            );
        }

        #[test]
        fn rule_keeps_newest_per_bucket() {
            let now = date(2026, 8, 24, 12, 0, 0);
            let policy = Policy::new(
                "30d".parse().unwrap(),
                0,
                ["1d*2".parse().unwrap()].to_vec(),
            );
            let gens = gens(&[
                (1, date(2026, 8, 20, 10, 0, 0)),
                (2, date(2026, 8, 20, 22, 0, 0)), // newest in its day
                (3, date(2026, 8, 22, 9, 0, 0)),
                (4, date(2026, 8, 23, 10, 0, 0)),
                (5, date(2026, 8, 24, 9, 0, 0)),
            ]);
            // buckets: [Aug 24, Aug 25) and [Aug 23, Aug 24) -> newest of each: 5 and 4
            let keep = kept_ids(&policy, now, &gens);
            assert_eq!(keep, vec![GenerationId::new(4), GenerationId::new(5)]);
        }

        #[test]
        fn rule_bucket_satisfied_by_keep_last() {
            let now = date(2026, 8, 24, 12, 0, 0);
            // keep_last already keeps the newest; the rule's bucket is satisfied by it and must
            // not add the older gen of the same bucket.
            let policy = Policy::new(
                "30d".parse().unwrap(),
                1,
                ["1d*1".parse().unwrap()].to_vec(),
            );
            let gens = gens(&[
                (1, date(2026, 8, 24, 8, 0, 0)),
                (2, date(2026, 8, 24, 9, 0, 0)),
            ]);
            assert_eq!(kept_ids(&policy, now, &gens), vec![GenerationId::new(2)]);
        }

        #[test]
        fn rules_ignore_gens_older_than_max_age() {
            let now = date(2026, 8, 24, 12, 0, 0);
            // cutoff for "5d" = 2026-08-19 12:00; gen 1 is below it, in a bucket the rule examines.
            let policy = Policy::new("5d".parse().unwrap(), 0, ["1d*6".parse().unwrap()].to_vec());
            let gens = gens(&[
                (1, date(2026, 8, 19, 10, 0, 0)), // older than cutoff -> never kept
                (2, date(2026, 8, 19, 18, 0, 0)),
                (3, date(2026, 8, 21, 9, 0, 0)),
            ]);
            assert_eq!(
                kept_ids(&policy, now, &gens),
                vec![GenerationId::new(2), GenerationId::new(3)]
            );
        }

        #[test]
        fn empty_buckets_are_skipped() {
            let now = date(2026, 8, 24, 12, 0, 0);
            let policy = Policy::new(
                "30d".parse().unwrap(),
                0,
                ["1d*2".parse().unwrap()].to_vec(),
            );
            // gen 1 falls in neither examined bucket -> both buckets empty, nothing kept
            let gens = gens(&[(1, date(2026, 8, 20, 10, 0, 0))]);
            assert_eq!(kept_ids(&policy, now, &gens), vec![]);
        }

        #[test]
        fn multi_rule_plan_is_idempotent_across_runs() {
            let policy = Policy::new(
                "2y".parse().unwrap(),
                0,
                [
                    "1d*7".parse().unwrap(),
                    "2w*5".parse().unwrap(),
                    "1M*12".parse().unwrap(),
                ]
                .to_vec(),
            );
            let gens = gens(&[
                (1, date(2026, 5, 10, 0, 0, 0)),
                (2, date(2026, 6, 10, 0, 0, 0)),
                (3, date(2026, 7, 10, 0, 0, 0)),
                (4, date(2026, 8, 10, 0, 0, 0)),
                (5, date(2026, 8, 20, 0, 0, 0)),
            ]);

            let start = date(2026, 8, 24, 12, 0, 0);
            let expected = kept_ids(&policy, start, &gens);

            let end = date(2026, 10, 12, 0, 0, 0);
            let mut now = start;
            while now < end {
                let kept = kept_ids(&policy, now, &gens);
                assert_eq!(
                    kept, expected,
                    "planning the same generations at {now} must keep the same set as at \
                     {start}: expected {expected:?}, got {kept:?}"
                );
                now += chrono::Duration::hours(1);
            }
        }
    }
}
