//! What a club pays a player on the league economy, now: the curve's wage for his rating,
//! scaled by what the club can afford for a squad like his.

use domain::player::Player;

use crate::game::Game;

use super::{EconomyModel, club_economy, weekly_wage_for};

/// The weekly wage `club_id` would pay `player` for his rating today, or `None` off the league
/// economy or for a club in no league. Contracts ask from this rather than from what the player
/// was paid before, so wages follow ratings both ways instead of ratcheting up at each renewal.
///
/// The club's pay factor is its league's wage share of its revenue over what the curve would
/// pay its squad as it stands; the opening set every squad's bill to that share, so at the
/// opening this is exactly each player's wage.
pub fn market_wage(game: &Game, player: &Player, club_id: &str) -> Option<u32> {
    if game.economy != EconomyModel::Leagues {
        return None;
    }
    let club = club_economy(game, club_id)?;
    let share = club
        .revenue_target
        .saturating_mul(i64::from(club.league.wage_ratio_percent))
        / 100
        / 52;
    let curve_bill: i64 = game
        .players
        .iter()
        .filter(|squad_player| squad_player.team_id.as_deref() == Some(club_id))
        .map(|squad_player| i64::from(weekly_wage_for(squad_player.ovr)))
        .sum();
    let curve_wage = f64::from(weekly_wage_for(player.ovr));
    if curve_bill <= 0 {
        return Some(curve_wage.round() as u32);
    }
    let factor = share as f64 / curve_bill as f64;
    Some(
        (curve_wage * factor)
            .round()
            .clamp(1.0, f64::from(u32::MAX)) as u32,
    )
}
