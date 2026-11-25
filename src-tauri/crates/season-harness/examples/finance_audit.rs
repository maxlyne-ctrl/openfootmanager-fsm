//! What one season's money looks like across the world: for every league, the average club's
//! income by kind, its wage bill and its opening cash, so a finance model can be judged against
//! real-world budgets by league. Each row also prints the league's target from
//! `ofm_core::league_economy`, so the two can be compared.
//!
//! Usage: `finance_audit [gate|shipped]`

use domain::finance::CashKind;
use domain::league::CompetitionType;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};
use std::collections::HashMap;

struct NoObserver;
impl SeasonObserver for NoObserver {}

#[derive(Default)]
struct LeagueTotals {
    clubs: i64,
    reputation: i64,
    opening_cash: i64,
    capacity: i64,
    rating_sum: f64,
    rated: i64,
    target: i64,
    target_wage_ratio: u32,
    from_table: bool,
    by_kind: HashMap<String, i64>,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let mut spec = match args.get(1).map(String::as_str) {
        Some("shipped") => WorldSpec::shipped(42),
        _ => WorldSpec::gate(42),
    };
    spec.scope = Scope::Everything;
    spec.international_football = false;
    let mut game = spec.build()?;
    let opening: HashMap<String, i64> = game
        .teams
        .iter()
        .map(|team| (team.id.clone(), team.finance))
        .collect();
    // Each club's league, read before the season so promotion does not move it.
    let mut league_of: HashMap<String, (String, String)> = HashMap::new();
    for competition in &game.competitions {
        if competition.kind != CompetitionType::League {
            continue;
        }
        for id in &competition.participant_ids {
            league_of.entry(id.clone()).or_insert((
                competition.country_id.clone().unwrap_or_default(),
                competition.name.clone(),
            ));
        }
    }
    let economies = ofm_core::league_economy::club_economies(&game);
    let mut squad_rating: HashMap<String, (f64, i64)> = HashMap::new();
    for player in &game.players {
        if let Some(team_id) = &player.team_id {
            let entry = squad_rating.entry(team_id.clone()).or_default();
            entry.0 +=
                ofm_core::player_rating::ovr_from_attributes(&player.attributes, &player.position);
            entry.1 += 1;
        }
    }
    run_seasons(&mut game, RunOptions::seasons(1), &mut NoObserver)
        .map_err(|error| format!("{error:?}"))?;

    let mut totals: HashMap<(String, String), LeagueTotals> = HashMap::new();
    for team in &game.teams {
        let Some(league) = league_of.get(&team.id) else {
            continue;
        };
        let entry = totals.entry(league.clone()).or_default();
        if let Some(economy) = economies.get(&team.id) {
            entry.target += economy.revenue_target;
            entry.target_wage_ratio = economy.league.wage_ratio_percent;
            entry.from_table =
                economy.league.source == ofm_core::league_economy::EconomySource::Table;
        }
        entry.clubs += 1;
        entry.reputation += i64::from(team.reputation);
        entry.opening_cash += opening.get(&team.id).copied().unwrap_or(0);
        entry.capacity += i64::from(team.stadium_capacity);
        if let Some((sum, count)) = squad_rating.get(&team.id) {
            entry.rating_sum += sum;
            entry.rated += count;
        }
        for post in game.cash_journal.posts_for(&team.id) {
            if post.kind == CashKind::OpeningBalance {
                continue;
            }
            *entry.by_kind.entry(format!("{:?}", post.kind)).or_default() += post.amount;
        }
    }
    let mut rows: Vec<_> = totals.into_iter().collect();
    rows.sort_by_key(|(_, t)| -(t.reputation / t.clubs.max(1)));
    let kinds = [
        "Broadcasting",
        "Commercial",
        "Matchday",
        "Sponsorship",
        "PrizeMoney",
        "CommercialCampaign",
        "PlayerWages",
        "StaffWages",
        "Upkeep",
        "TransferFeeIn",
        "TransferFeeOut",
    ];
    println!(
        "country\tleague\tclubs\tovr\trep\tcap\topening_cash\t{}\tincome\twages/income\ttarget_income\ttarget_wages/income\tsource",
        kinds.join("\t")
    );
    for ((country, league), t) in rows {
        let n = t.clubs.max(1);
        let avg = |kind: &str| t.by_kind.get(kind).copied().unwrap_or(0) / n;
        let income = avg("Matchday")
            + avg("Sponsorship")
            + avg("PrizeMoney")
            + avg("Broadcasting")
            + avg("Commercial");
        let wages = -(avg("PlayerWages") + avg("StaffWages"));
        let ratio = if income > 0 {
            format!("{:.0}%", 100.0 * wages as f64 / income as f64)
        } else {
            "-".to_string()
        };
        println!(
            "{country}\t{league}\t{}\t{:.1}\t{}\t{}\t{}\t{}\t{income}\t{ratio}\t{}\t{}%\t{}",
            t.clubs,
            t.rating_sum / t.rated.max(1) as f64,
            t.reputation / n,
            t.capacity / n,
            t.opening_cash / n,
            kinds
                .iter()
                .map(|k| avg(k).to_string())
                .collect::<Vec<_>>()
                .join("\t"),
            t.target / n,
            t.target_wage_ratio,
            if t.from_table { "table" } else { "reputation" },
        );
    }
    Ok(())
}
