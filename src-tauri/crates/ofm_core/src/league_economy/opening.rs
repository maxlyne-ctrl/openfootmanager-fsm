//! Opening a new world on the league economy.

use crate::game::Game;

use super::{EconomyModel, club_economies, ground_capacity_for};

/// A league plays at least this many home games a season (the smallest real top flights play
/// 15 to 17), so a small test league is not sized for two games a year.
const FEWEST_HOME_GAMES: u32 = 15;

/// Put a newly generated world on the league economy: every ground rebuilt to the size its
/// club's matchday money supports, every squad to the rating its money buys, and every price and
/// budget to the market's.
pub fn open_league_economy(game: &mut Game) {
    game.economy = EconomyModel::Leagues;
    let economies = club_economies(game);
    let league_sizes: std::collections::HashMap<&str, u32> = game
        .competitions
        .iter()
        .map(|league| {
            let clubs = u32::try_from(league.participant_ids.len()).unwrap_or(u32::MAX);
            (league.id.as_str(), clubs)
        })
        .collect();
    let mut targets: Vec<(usize, u32)> = Vec::new();
    for (index, team) in game.teams.iter().enumerate() {
        let Some(economy) = economies.get(&team.id) else {
            continue;
        };
        let clubs = league_sizes
            .get(economy.league_id.as_str())
            .copied()
            .unwrap_or(0);
        let home_games = clubs.saturating_sub(1).max(FEWEST_HOME_GAMES);
        targets.push((index, ground_capacity_for(economy, home_games)));
    }
    for (index, capacity) in targets {
        if let Some(team) = game.teams.get_mut(index) {
            team.stadium_capacity = capacity;
            // Rebuilt below to the new size, from the club's own seed.
            team.stadium_layout = None;
        }
    }
    crate::stadium::backfill_stadium_layouts(&mut game.teams);
    super::squads::fit_squads_to_money(game, &economies);
}
