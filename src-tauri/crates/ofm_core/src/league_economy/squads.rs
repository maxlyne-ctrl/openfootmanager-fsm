//! Bringing a new world's squads, prices and budgets into line with its leagues' money.

use std::collections::HashMap;

use crate::game::Game;

use super::{ClubEconomy, market_value_for, squad_rating_target, weekly_wage_for};

/// Where the unattached pool's ratings centre: a player no club has signed is mostly a
/// lower-league journeyman. At 52 the pool sat above the weakest leagues, whose clubs signed it
/// cheaply and rose four points in six seasons (finance_audit, shipped world).
const FREE_AGENT_RATING: f64 = 46.0;

/// Squads open this far under the rating their money buys, because they settle upward: a new
/// world's squads are young and grow into their peak, and over five seasons every league rose
/// about two to three points before levelling off, on the old economy as on this one
/// (finance_audit, shipped world). Opening under the target lands them on it.
pub(crate) const SETTLING_ALLOWANCE: f64 = 2.0;

/// No rating is moved outside this range: a squad is moved, not invented.
const LOWEST_RATING: u8 = 25;
const HIGHEST_RATING: u8 = 95;

/// A club keeps this much of its wage budget free above what its squad costs.
const WAGE_BUDGET_HEADROOM_PERCENT: i64 = 5;
/// Opening cash: a quarter of a season's revenue.
const OPENING_CASH_PERCENT: i64 = 25;
/// The transfer budget: a seventh of a season's revenue.
const TRANSFER_BUDGET_PERCENT: i64 = 15;
/// The share of cash banked beyond the reserve that a new season adds to the transfer budget.
const BANKED_SPEND_PERCENT: i64 = 50;

/// Move every squad to the rating its money buys, price every player on the market's curves,
/// and set each club's wage budget, transfer budget and cash from its revenue.
pub(super) fn fit_squads_to_money(game: &mut Game, economies: &HashMap<String, ClubEconomy>) {
    let today = game.clock.current_date.date_naive();
    move_squads_to_their_standard(game, economies, today);
    for player in &mut game.players {
        let age = domain::age::playing_age(today, &player.date_of_birth);
        player.market_value = market_value_for(player.ovr, age);
        player.stage_wage(weekly_wage_for(player.ovr));
    }
    pay_each_squad_its_leagues_share(game, economies);
    set_budgets(game, economies);
}

/// Scale each club's wages so its bill is its league's share of its revenue. The curve says how
/// pay differs between players; the club's money says how much there is, which is why the same
/// player costs more at a rich club, as he does in life. Without this a squad of mixed quality
/// pays its few good players far over its means, because the curve is steepest at the top.
fn pay_each_squad_its_leagues_share(game: &mut Game, economies: &HashMap<String, ClubEconomy>) {
    let mut bills: HashMap<String, i64> = HashMap::new();
    for player in &game.players {
        if let Some(club) = &player.team_id {
            *bills.entry(club.clone()).or_default() += i64::from(player.wage());
        }
    }
    let factors: HashMap<String, f64> = bills
        .into_iter()
        .filter_map(|(club, bill)| {
            let economy = economies.get(&club)?;
            let share = economy
                .revenue_target
                .saturating_mul(i64::from(economy.league.wage_ratio_percent))
                / 100
                / 52;
            (bill > 0).then(|| (club, share as f64 / bill as f64))
        })
        .collect();
    for player in &mut game.players {
        let Some(factor) = player.team_id.as_ref().and_then(|club| factors.get(club)) else {
            continue;
        };
        let scaled = (f64::from(player.wage()) * factor).round();
        player.stage_wage(scaled.clamp(1.0, f64::from(u32::MAX)) as u32);
    }
}

fn move_squads_to_their_standard(
    game: &mut Game,
    economies: &HashMap<String, ClubEconomy>,
    today: chrono::NaiveDate,
) {
    let mut ratings: HashMap<Option<&str>, (f64, usize)> = HashMap::new();
    for player in &game.players {
        let entry = ratings.entry(player.team_id.as_deref()).or_default();
        entry.0 += f64::from(player.ovr);
        entry.1 += 1;
    }
    let shifts: HashMap<Option<String>, i32> = ratings
        .into_iter()
        .filter_map(|(club, (sum, count))| {
            let target = match club {
                None => FREE_AGENT_RATING,
                Some(id) => {
                    squad_rating_target(economies.get(id)?.revenue_target) - SETTLING_ALLOWANCE
                }
            };
            let shift = (target - sum / count as f64).round() as i32;
            Some((club.map(str::to_string), shift))
        })
        .collect();
    for player in &mut game.players {
        let Some(shift) = shifts.get(&player.team_id).copied().filter(|s| *s != 0) else {
            continue;
        };
        let moved = |rating: u8| {
            (i32::from(rating) + shift).clamp(i32::from(LOWEST_RATING), i32::from(HIGHEST_RATING))
                as u8
        };
        let (rating, headroom) = (player.ovr, player.potential.saturating_sub(player.ovr));
        let position = player.natural_position.clone();
        crate::player_rating::move_attributes_to(&mut player.attributes, &position, moved(rating));
        // The ceiling moves with the player, so a youngster keeps the room to grow he had.
        let new_rating = crate::player_rating::natural_rating_from(player, &player.attributes);
        player.potential = new_rating.saturating_add(headroom).min(99);
        crate::player_rating::refresh_player_derived(player, today);
    }
}

fn set_budgets(game: &mut Game, economies: &HashMap<String, ClubEconomy>) {
    let mut bills: HashMap<&str, i64> = HashMap::new();
    for player in &game.players {
        if let Some(club) = player.team_id.as_deref() {
            *bills.entry(club).or_default() += i64::from(player.wage());
        }
    }
    let bills: HashMap<String, i64> = bills.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    for team in &mut game.teams {
        let Some(economy) = economies.get(&team.id) else {
            continue;
        };
        let revenue = economy.revenue_target;
        let bill = bills.get(&team.id).copied().unwrap_or(0);
        let affordable =
            revenue.saturating_mul(i64::from(economy.league.wage_ratio_percent)) / 100 / 52;
        team.wage_budget = affordable.max(bill * (100 + WAGE_BUDGET_HEADROOM_PERCENT) / 100);
        team.transfer_budget = revenue * TRANSFER_BUDGET_PERCENT / 100;
        team.finance = revenue * OPENING_CASH_PERCENT / 100;
    }
}

/// Each club's budgets for a new season on the league economy, from its revenue now: promoted
/// and relegated clubs spend at their new level, and money a club has banked beyond a reserve of
/// a quarter-season becomes half again of a transfer budget. Without this the opening budgets
/// stood for a whole career, and clubs that could not spend them piled up cash.
pub fn refresh_budgets(game: &mut Game) {
    if game.economy != super::EconomyModel::Leagues {
        return;
    }
    let economies = super::club_economies(game);
    for team in &mut game.teams {
        let Some(economy) = economies.get(&team.id) else {
            continue;
        };
        let revenue = economy.revenue_target;
        let share = revenue.saturating_mul(i64::from(economy.league.wage_ratio_percent)) / 100 / 52;
        team.wage_budget = share * (100 + WAGE_BUDGET_HEADROOM_PERCENT) / 100;
        let reserve = revenue * OPENING_CASH_PERCENT / 100;
        let banked = (team.finance - reserve).max(0);
        let budget = revenue * TRANSFER_BUDGET_PERCENT / 100 + banked * BANKED_SPEND_PERCENT / 100;
        team.transfer_budget = budget.min(team.finance.max(0));
    }
}
