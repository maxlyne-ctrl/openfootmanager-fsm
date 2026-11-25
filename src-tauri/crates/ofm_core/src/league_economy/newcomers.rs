//! Players who arrive during a league economy career: academy recruits, scouted prospects and
//! national-pool free agents. The generator draws them on the world's old scale; this brings
//! each onto the league economy's, as the opening did for everyone already there.

use domain::player::Player;

use crate::game::Game;

use super::{EconomyModel, club_economy, market_value_for, squad_rating_target, weekly_wage_for};

/// The squad standard the generator's academy intake is centred on: the world's average squad
/// before the league economy.
const GENERATOR_SQUAD_STANDARD: f64 = 66.0;
/// The share of the gap between a club's standard and the generator's that its academy
/// recruits take. A Premier League academy's sixteen-year-olds rate about 50, the age the
/// best youth prospects carry; a Welsh club's are in the high thirties.
const ACADEMY_SHARE_OF_STANDARD: f64 = 0.5;

/// Bring `player`, just generated, onto the league economy: a recruit to a club is moved toward
/// his club's standard, keeping his headroom, and every newcomer is valued and paid on the
/// market's curves. On the old economy he is left as generated.
pub fn fit_newcomer(game: &Game, player: &mut Player) {
    let club_id = player.team_id.clone();
    fit_for_club(game, club_id.as_deref(), player);
}

/// As [`fit_newcomer`], for a prospect a scout found for `club_id` who belongs to no club yet:
/// he is drawn to that club's standard, as its own recruit would be.
pub fn fit_prospect(game: &Game, club_id: &str, player: &mut Player) {
    fit_for_club(game, Some(club_id), player);
}

fn fit_for_club(game: &Game, club_id: Option<&str>, player: &mut Player) {
    if game.economy != EconomyModel::Leagues {
        return;
    }
    let today = game.clock.current_date.date_naive();
    let club = club_id.and_then(|club_id| club_economy(game, club_id));
    if let Some(club) = club {
        let gap = squad_rating_target(club.revenue_target) - GENERATOR_SQUAD_STANDARD;
        let shift = (gap * ACADEMY_SHARE_OF_STANDARD).round() as i32;
        // The ceiling takes the whole gap: a poor club's academy grows modest players and a rich
        // one's stars. Kept on the world's headroom, the poor leagues' standard crept up five
        // points in five seasons (finance_audit, shipped world).
        let ceiling = (i32::from(player.potential) + gap.round() as i32).clamp(25, 99);
        let target = (i32::from(player.ovr) + shift).clamp(25, 95) as u8;
        let position = player.natural_position.clone();
        crate::player_rating::move_attributes_to(&mut player.attributes, &position, target);
        let rating = crate::player_rating::natural_rating_from(player, &player.attributes);
        player.potential = (ceiling as u8).max(rating);
        crate::player_rating::refresh_player_derived(player, today);
    }
    let age = domain::age::playing_age(today, &player.date_of_birth);
    player.market_value = market_value_for(player.ovr, age);
    player.stage_wage(weekly_wage_for(player.ovr));
}
