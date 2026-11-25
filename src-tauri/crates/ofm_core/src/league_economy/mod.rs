//! How much money a league is worth, and each club's share of it.
//!
//! The figures are real-world averages per club (see `data/league_economy.json`, which names the
//! source of each), so a top-flight English club earns hundreds of times a Welsh one, as it does.
//! Leagues the table does not list fall back to the average reputation of their clubs, which
//! already carries the nation's strength and the division's tier.
//!
//! This module only states targets. The finance, wage and generation code read them; nothing
//! here moves money.

mod spread;
mod table;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::game::Game;

pub use spread::revenue_spread;

/// Where a league's figure came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconomySource {
    /// A row of the real-world table.
    Table,
    /// Read off the average reputation of the league's clubs.
    Reputation,
}

/// How a league's revenue divides, in whole percent that sum to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevenueSplit {
    pub broadcast_percent: u32,
    pub commercial_percent: u32,
    pub matchday_percent: u32,
}

/// What an average club in one league takes in a season, and spends on wages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeagueEconomy {
    /// Euros per club per season.
    pub revenue_per_club: i64,
    pub split: RevenueSplit,
    /// Share of revenue paid in wages, in whole percent.
    pub wage_ratio_percent: u32,
    pub source: EconomySource,
}

/// One club's place in its league's economy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubEconomy {
    pub league_id: String,
    pub division: u32,
    /// Euros per season the club should take: its league's average times its place in the spread.
    pub revenue_target: i64,
    pub league: LeagueEconomy,
}

/// The economy of division `division` (0 is the top flight) of `country`, whose clubs average
/// `mean_reputation`.
pub fn league_economy(country: &str, division: u32, mean_reputation: u32) -> LeagueEconomy {
    table::league_economy(country, division, mean_reputation)
}

/// Every club in a domestic league, keyed by id, with its revenue target. Clubs are ranked by
/// reputation inside their league: the most famous takes the top of the spread, a promoted club
/// the bottom. A club in no league is absent.
pub fn club_economies(game: &Game) -> HashMap<String, ClubEconomy> {
    let reputation: HashMap<&str, u32> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), team.reputation))
        .collect();
    let mut economies = HashMap::new();
    for (division, league) in crate::end_of_season::ladder_rungs(game) {
        let mut clubs: Vec<(&str, u32)> = league
            .participant_ids
            .iter()
            .filter_map(|id| reputation.get(id.as_str()).map(|rep| (id.as_str(), *rep)))
            .collect();
        if clubs.is_empty() {
            continue;
        }
        // Most famous first; the id breaks a tie so the order cannot change between runs.
        clubs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        let mean_reputation =
            clubs.iter().map(|(_, rep)| u64::from(*rep)).sum::<u64>() / clubs.len() as u64;
        let country = league.country_id.as_deref().unwrap_or_default();
        let economy = league_economy(
            country,
            division,
            u32::try_from(mean_reputation).unwrap_or(u32::MAX),
        );
        for ((club_id, _), multiple) in clubs.iter().zip(revenue_spread(clubs.len())) {
            economies.insert(
                (*club_id).to_string(),
                ClubEconomy {
                    league_id: league.id.clone(),
                    division,
                    revenue_target: (economy.revenue_per_club as f64 * multiple).round() as i64,
                    league: economy,
                },
            );
        }
    }
    economies
}
