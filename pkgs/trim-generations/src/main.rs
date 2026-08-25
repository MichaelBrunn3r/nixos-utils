#![feature(gen_blocks)]
#![feature(iter_collect_into)]

mod duration;
mod generation;
mod policy;

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::Parser;

use duration::NominalDuration;
use generation::Generations;

use crate::generation::Generation;
use crate::policy::{Action, Decision, Policy, RetentionRule};

/// Trim a Nix profile's generations.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the Nix profile.
    #[arg(long, default_value = "/nix/var/nix/profiles/system")]
    profile: PathBuf,

    /// Maximum age of generations to keep (e.g. 30d, 2w, 1M, 1y).
    #[arg(long, default_value = "30d")]
    max_age: NominalDuration,

    /// Always keep the newest N old generations, even when older than max_age.
    #[arg(long, default_value_t = 5)]
    keep_last: u32,

    /// Retention rules, semicolon-separated: <duration>*<repeat>;... e.g. "1d*7;2w*10;1M*12".
    #[arg(long, default_value = "1d*30")]
    rules: String,

    /// Actually trim generations.
    #[arg(long)]
    apply: bool,
}

fn main() -> anyhow::Result<()> {
    //region Parse CLI args
    let args = Args::parse();

    if !args.profile.exists() {
        bail!("profile not found: {}", args.profile.display());
    }
    let rules: Vec<RetentionRule> = args
        .rules
        .split(';')
        .map(|s| s.parse())
        .collect::<anyhow::Result<_>>()?;
    let policy = Policy::new(args.max_age, args.keep_last, rules);
    //endregion Parse CLI args

    let gens = Generations::parse(&read_profile(&args.profile)?)?;
    let now = chrono::Local::now().naive_local();
    let plan = policy.plan(now, &gens)?;

    print_policy(&policy);
    println!();
    print_table(&gens, &plan);
    Ok(())
}

/// Run `nix-env --list-generations` for a profile and return the raw output.
pub fn read_profile(profile_path: &Path) -> anyhow::Result<String> {
    let output = std::process::Command::new("nix-env")
        .arg("--profile")
        .arg(profile_path)
        .arg("--list-generations")
        .output()
        .context("failed to run nix-env")?;

    if !output.status.success() {
        bail!("failed to read profile");
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Print the policy settings, one indented row per setting.
fn print_policy(policy: &Policy) {
    println!("policy:");
    println!("  max_age:   {}", policy.max_age);
    println!("  keep_last: {}", policy.keep_last);
    println!("  rules:");
    for rule in &policy.rules {
        println!("    - {rule}");
    }
}

/// Print one row per generation with the action to take and why.
fn print_table(gens: &Generations, decisions: &[Decision]) {
    fn fmt_time(g: &Generation) -> String {
        g.created.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn print_row(
        id: impl std::fmt::Display,
        time: impl std::fmt::Display,
        action: impl std::fmt::Display,
        reason: impl std::fmt::Display,
    ) {
        println!("{:<4} │ {:<19} │ {:<6} │ {}", id, time, action, reason);
    }

    // Header
    print_row("id", "created", "action", "reason");

    // Rows for old gens
    for (g, d) in gens.older().iter().zip(decisions) {
        let (action, reason) = match &d.action {
            Action::Keep(rs) => (
                "keep",
                rs.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
            Action::Remove(r) => ("remove", r.to_string()),
        };
        print_row(g.id, fmt_time(g), action, reason);
    }

    // Row for current gen
    let current = gens.current();
    print_row(current.id, fmt_time(current), "keep", "current");

    // Rows for newer gens
    for g in gens.newer() {
        print_row(g.id, fmt_time(g), "keep", "newer");
    }
}

fn trim(remove: &[Generation]) {}
