use anyhow::{Context, bail};
use chrono::NaiveDateTime;

/// All generations of a profile, split by their position relative to the current one.
#[derive(Debug)]
pub struct Generations {
    pub generations: Vec<Generation>, // all generations, sorted by id ascending
    pub current: usize,               // index of the current generation in `generations`
}

impl Generations {
    /// Parse `nix-env --list-generations` output and split by the current generation.
    ///
    /// Generations are expected to be listed in creation order: ids strictly increase (gaps allowed)
    /// and timestamps stay in the same, non-decreasing order.
    ///
    /// # Example
    /// ```text
    /// 1   2024-01-01 10:00:00
    /// 2   2025-01-01 12:00:00   (current)
    /// 3   2026-08-01 10:00:21
    /// ```
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        let mut generations: Vec<Generation> = Vec::new();
        let mut current_id: Option<GenerationId> = None;

        for line in raw.lines() {
            // Split row into cells
            let cells: Vec<&str> = line.trim().split("   ").collect();

            // Skip blank lines
            if cells.is_empty() || cells[0].is_empty() {
                continue;
            }

            let gen_id = GenerationId::new(
                cells[0]
                    .trim()
                    .parse()
                    .context("failed to parse generation id")?,
            );
            let created = NaiveDateTime::parse_from_str(cells[1].trim(), "%Y-%m-%d %H:%M:%S")
                .with_context(|| {
                    format!("failed to parse creation timestamp of generation {gen_id}")
                })?;

            if cells.len() > 2 && !cells[2].trim().is_empty() {
                if current_id.is_some() {
                    bail!("found multiple current generations");
                }
                current_id = Some(gen_id);
            }

            generations.push(Generation::new(gen_id, created));
        }

        let current_id = match current_id {
            Some(id) => id,
            None => bail!("found no current generation"),
        };

        if !is_sorted_chronologically(&generations) {
            bail!("generations are not in chronological order");
        }
        let current = generations
            .binary_search_by(|g| g.id.cmp(&current_id))
            .expect("current generation is present in the parsed list");

        Ok(Self {
            generations,
            current,
        })
    }

    /// The current generation.
    pub fn current(&self) -> &Generation {
        &self.generations[self.current]
    }

    /// Generations with an id below the current one.
    pub fn older(&self) -> &[Generation] {
        &self.generations[..self.current]
    }

    /// Generations with an id above the current one (e.g. after a rollback).
    pub fn newer(&self) -> &[Generation] {
        &self.generations[self.current + 1..]
    }
}

//region Generation
/// A profile generation.
#[derive(Debug)]
pub struct Generation {
    pub id: GenerationId,
    pub created: NaiveDateTime,
}

impl Generation {
    pub fn new(id: GenerationId, created: NaiveDateTime) -> Self {
        Self { id, created }
    }
}

/// Checks if generations are in chronological order: ids strictly increase (gaps
/// allowed) and timestamps are non-decreasing in the same order.
pub(crate) fn is_sorted_chronologically(generations: &[Generation]) -> bool {
    generations
        .windows(2)
        .all(|w| w[0].id < w[1].id && w[0].created <= w[1].created)
}
//endregion Generation

//region GenerationId
/// ID of a Nix profile generation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenerationId(u32);

impl GenerationId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for GenerationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for GenerationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

//endregion GenerationId
