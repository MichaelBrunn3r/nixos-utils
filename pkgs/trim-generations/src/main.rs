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
    println!();
    trim(&args.profile, &plan, args.apply)?;
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

/// Delete generations a plan marked for removal.
fn trim(profile: &Path, decisions: &[Decision], apply: bool) -> anyhow::Result<()> {
    let ids_to_remove: Vec<_> = decisions
        .iter()
        .filter(|d| matches!(d.action, Action::Remove(_)))
        .map(|d| d.id)
        .collect();

    if ids_to_remove.is_empty() {
        println!("nothing to trim");
        return Ok(());
    }

    // Build the command arguments once; reuse them for both display and execution.
    let mut args: Vec<String> = vec![
        "nix-env".into(),
        "--profile".into(),
        profile.display().to_string(),
        "--delete-generations".into(),
    ];
    args.extend(ids_to_remove.iter().map(ToString::to_string));

    // Run the command
    println!("{}", gray(format!("> {}", args.join(" "))));
    if !apply {
        println!("dry run: not executing (pass --apply to trim)");
        return Ok(());
    }

    let output = std::process::Command::new(&args[0])
        .args(&args[1..])
        .output()
        .context("failed to run nix-env")?;

    if !output.status.success() {
        bail!(
            "failed to delete generations: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    println!("deleted {} generation(s)", ids_to_remove.len());
    Ok(())
}

fn gray(s: impl std::fmt::Display) -> String {
    if console_supports_color() {
        format!("\x1b[90m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn console_supports_color() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").map_or(true, |t| t != "dumb")
}
