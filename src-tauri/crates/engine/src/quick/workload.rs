//! The work nobody watches: passes, tackles and interceptions.
//!
//! The full engine logs every one as an event. Here they are drawn as counts
//! per player, from per-90 rates by position, straight into the report — a
//! few hundred events a match that nobody reads one at a time, but which the
//! match ratings and the season stats do read in total.

use rand::Rng;

use super::model::{PositionRates, QuickModel};
use super::sample::{binomial, poisson};
use super::side::SideState;
use crate::report::{MatchReport, TeamStats};
use crate::types::{PlayerData, Position};

fn rates(model: &QuickModel, position: Position) -> &PositionRates {
    let table = &model.positions;
    match position {
        Position::Goalkeeper => &table.goalkeeper,
        Position::Defender => &table.defender,
        Position::Midfielder => &table.midfielder,
        Position::Forward => &table.forward,
    }
}

fn add_u8(value: &mut u8, add: u32) {
    *value = value.saturating_add(u8::try_from(add).unwrap_or(u8::MAX));
}

fn add_u16(value: &mut u16, add: u32) {
    *value = value.saturating_add(u16::try_from(add).unwrap_or(u16::MAX));
}

/// Add each appearing player's passes, tackles and interceptions to his own
/// line of the report and to his side's totals.
pub(super) fn add_workload<R: Rng>(
    report: &mut MatchReport,
    side: &SideState<'_>,
    team_stats_is_home: bool,
    model: &QuickModel,
    rng: &mut R,
) {
    let mut totals = (0u32, 0u32, 0u32, 0u32);
    for index in (0..side.squad.len()).filter(|i| *i < side.starters || side.used_bench.contains(i))
    {
        let player: &PlayerData = side.player(index);
        let Some(stats) = report.player_stats.get_mut(&player.id) else {
            continue;
        };
        let share = f64::from(stats.minutes_played) / 90.0;
        if share <= 0.0 {
            continue;
        }
        let r = rates(model, player.position);
        let attempted = poisson(rng, r.passes_attempted * share);
        let accuracy = if r.passes_attempted > 0.0 {
            (r.passes_completed / r.passes_attempted).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let completed = binomial(rng, attempted, accuracy);
        let tackling = (0.005 * (f64::from(player.tackling) - 70.0)).exp();
        let tackles = poisson(rng, r.tackles * share * tackling);
        let interceptions = poisson(rng, r.interceptions * share);

        add_u8(&mut stats.passes_attempted, attempted);
        add_u8(&mut stats.passes_completed, completed);
        add_u8(&mut stats.tackles_won, tackles);
        add_u8(&mut stats.interceptions, interceptions);
        totals.0 += completed;
        totals.1 += attempted - completed;
        totals.2 += tackles;
        totals.3 += interceptions;
    }
    let team: &mut TeamStats = if team_stats_is_home {
        &mut report.home_stats
    } else {
        &mut report.away_stats
    };
    add_u16(&mut team.passes_completed, totals.0);
    add_u16(&mut team.passes_intercepted, totals.1);
    add_u16(&mut team.tackles, totals.2);
    add_u16(&mut team.interceptions, totals.3);
}
