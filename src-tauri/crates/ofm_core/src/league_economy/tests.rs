use super::*;
use crate::clock::GameClock;
use chrono::{TimeZone, Utc};
use domain::league::League;
use domain::manager::Manager;
use domain::team::Team;

/// Given England's top flight
/// When its economy is read
/// Then it is the table's row: EUR 380m a club, 65% on wages.
#[test]
fn a_listed_league_reads_its_row_of_the_table() {
    let economy = league_economy("ENG", 0, 750);

    assert_eq!(economy.revenue_per_club, 380_000_000);
    assert_eq!(economy.wage_ratio_percent, 65);
    assert_eq!(economy.source, EconomySource::Table);
}

/// Given England's second division
/// When its economy is read
/// Then it earns a fraction of the top flight and spends nearly all of it on wages, as the
///      Championship does.
#[test]
fn the_championship_spends_almost_everything_on_wages() {
    let economy = league_economy("ENG", 1, 530);

    assert_eq!(economy.revenue_per_club, 47_000_000);
    assert_eq!(economy.wage_ratio_percent, 96);
}

/// Given a country the table does not list
/// When the economy of leagues of rising reputation is read
/// Then it comes from reputation, and a more famous league is never poorer.
#[test]
fn an_unlisted_league_is_valued_by_reputation_and_fame_never_costs_money() {
    let mut previous = 0;
    for reputation in (100..=950).step_by(25) {
        let economy = league_economy("XX", 0, reputation);
        assert_eq!(economy.source, EconomySource::Reputation);
        assert!(
            economy.revenue_per_club >= previous,
            "reputation {reputation}: {} after {previous}",
            economy.revenue_per_club
        );
        previous = economy.revenue_per_club;
    }
}

/// Given a reputation halfway between two knots of the fallback
/// When its economy is read
/// Then the figure is their geometric mean: money is interpolated on a log scale.
#[test]
fn the_fallback_interpolates_on_a_log_scale() {
    // Knots (320, 1.5m) and (420, 30m): halfway is sqrt(1.5m * 30m), about 6.7m.
    let economy = league_economy("XX", 0, 370);

    assert!(
        (6_600_000..=6_800_000).contains(&economy.revenue_per_club),
        "{}",
        economy.revenue_per_club
    );
}

/// Given a listed country's division below the rows the table has
/// When its economy is read
/// Then it comes from reputation, but is never richer than the lowest listed division above it.
#[test]
fn a_division_below_the_table_is_never_richer_than_the_one_above() {
    let economy = league_economy("NL", 1, 530);

    assert_eq!(economy.source, EconomySource::Reputation);
    assert!(economy.revenue_per_club <= 42_000_000);
}

/// Given leagues from the richest to the poorest
/// When their revenue is split
/// Then the shares sum to 100; the rich live on broadcasting and the poor on the gate.
#[test]
fn rich_leagues_live_on_television_and_poor_ones_on_the_gate() {
    let rich = league_economy("ENG", 0, 750).split;
    let poor = league_economy("WAL", 0, 316).split;

    for split in [rich, poor] {
        assert_eq!(
            split.broadcast_percent + split.commercial_percent + split.matchday_percent,
            100
        );
    }
    assert!(rich.broadcast_percent > rich.matchday_percent);
    assert!(poor.matchday_percent > poor.broadcast_percent);
}

/// Given a league of twenty clubs
/// When its revenue is spread
/// Then the richest takes five times the poorest, every place takes less than the one above,
///      and the multiples average one, so the league's total is kept.
#[test]
fn a_leagues_money_spreads_five_to_one_and_keeps_its_total() {
    let spread = revenue_spread(20);

    assert_eq!(spread.len(), 20);
    assert!((spread[0] / spread[19] - 5.0).abs() < 1e-9);
    assert!(spread.windows(2).all(|pair| pair[0] > pair[1]));
    let mean = spread.iter().sum::<f64>() / 20.0;
    assert!((mean - 1.0).abs() < 1e-9, "mean {mean}");
}

/// Given a league of one club, and of none
/// When its revenue is spread
/// Then the one club takes the average, and an empty league has no places.
#[test]
fn a_league_of_one_takes_the_average() {
    assert_eq!(revenue_spread(1), vec![1.0]);
    assert!(revenue_spread(0).is_empty());
}

fn club(id: &str, reputation: u32) -> Team {
    let mut team = Team::new(
        id.to_string(),
        id.to_string(),
        id[..3].to_string(),
        "England".to_string(),
        "Testville".to_string(),
        "Ground".to_string(),
        20_000,
    );
    team.reputation = reputation;
    team
}

fn division(id: &str, priority: u32, clubs: &[&str]) -> League {
    let ids: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
    let mut league = League::new(id.to_string(), id.to_string(), 2026, &ids);
    league.country_id = Some("ENG".to_string());
    league.priority = priority;
    league
}

/// England with a top flight of three clubs and a second division of two, and a club in no league.
fn english_game() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "mgr".to_string(),
        "Alex".to_string(),
        "Boss".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut game = Game::new(
        clock,
        manager,
        vec![
            club("aaa-small", 700),
            club("bbb-giant", 800),
            club("ccc-middle", 750),
            club("ddd-second", 530),
            club("eee-second", 520),
            club("fff-nowhere", 500),
        ],
        vec![],
        vec![],
        vec![],
    );
    game.competitions = vec![
        division("top", 1, &["aaa-small", "bbb-giant", "ccc-middle"]),
        division("second", 2, &["ddd-second", "eee-second"]),
    ];
    game
}

/// Given England's top two divisions
/// When every club's economy is read
/// Then each knows its league and division, the most famous club of the top flight takes the
///      top of its spread, and a club in no league has no economy.
#[test]
fn every_club_in_a_league_is_given_its_place_in_that_leagues_economy() {
    let economies = club_economies(&english_game());

    assert_eq!(economies.len(), 5);
    assert!(!economies.contains_key("fff-nowhere"));
    let giant = &economies["bbb-giant"];
    let small = &economies["aaa-small"];
    assert_eq!((giant.league_id.as_str(), giant.division), ("top", 0));
    assert_eq!(economies["ddd-second"].division, 1);
    assert!(giant.revenue_target > economies["ccc-middle"].revenue_target);
    assert!(economies["ccc-middle"].revenue_target > small.revenue_target);
    assert_eq!(giant.league.revenue_per_club, 380_000_000);
}

/// Given the same top flight of three
/// When its clubs' targets are added up
/// Then they come to three times the league's average: the spread moves money between clubs and
///      makes none.
#[test]
fn a_leagues_targets_add_up_to_its_clubs_times_its_average() {
    let economies = club_economies(&english_game());

    let total: i64 = ["aaa-small", "bbb-giant", "ccc-middle"]
        .iter()
        .map(|id| economies[*id].revenue_target)
        .sum();
    assert!((total - 3 * 380_000_000).abs() <= 3, "total {total}");
}

fn economy_in(country: &str, division: u32, multiple: f64) -> ClubEconomy {
    let league = league_economy(country, division, 500);
    ClubEconomy {
        league_id: "l".to_string(),
        division,
        revenue_target: (league.revenue_per_club as f64 * multiple) as i64,
        league,
    }
}

/// Given clubs from League Two to the Premier League
/// When their price per head is read
/// Then a League Two crowd pays about EUR 20, a Premier League one EUR 75, a tiny club EUR 8,
///      and richer never means cheaper.
#[test]
fn the_price_of_a_ticket_rises_with_the_clubs_money() {
    assert_eq!(matchday::ticket_price(9_000_000), 20);
    assert_eq!(matchday::ticket_price(380_000_000), 75);
    assert_eq!(matchday::ticket_price(100_000), 8);
    let mut previous = 0;
    for revenue in [
        1_000_000,
        5_000_000,
        20_000_000,
        90_000_000,
        200_000_000,
        900_000_000,
    ] {
        let price = matchday::ticket_price(revenue);
        assert!(price >= previous, "{revenue}: {price} after {previous}");
        previous = price;
    }
}

/// Given an average Premier League club and an average Welsh one, over 19 home league games
/// When the ground their matchday money supports is sized
/// Then the English club needs a big modern ground and the Welsh one a small one, as in life.
#[test]
fn a_clubs_ground_is_the_size_its_matchday_money_supports() {
    let english = matchday::ground_capacity_for(&economy_in("ENG", 0, 1.0), 19);
    let welsh = matchday::ground_capacity_for(&economy_in("WAL", 0, 1.0), 19);

    assert!((35_000..=65_000).contains(&english), "England {english}");
    assert!((800..=6_000).contains(&welsh), "Wales {welsh}");
}

/// Given the richest club in the richest league, and a club with no money at all
/// When their grounds are sized
/// Then neither leaves the range of grounds there are.
#[test]
fn no_ground_is_bigger_than_the_biggest_or_smaller_than_a_stand() {
    let giant = matchday::ground_capacity_for(&economy_in("ENG", 0, 3.0), 19);
    let pauper = matchday::ground_capacity_for(&economy_in("XX", 0, 0.0), 19);

    assert_eq!(giant, 90_000);
    assert_eq!(pauper, 800);
}

/// Given England's two divisions, on a world just built
/// When it is opened on the league economy with grounds fitted to their leagues
/// Then the career is on the league economy, every club's capacity is what its new ground holds,
///      and the most famous top-flight club plays in a bigger ground than a second-division club.
#[test]
fn opening_a_world_fits_every_ground_to_its_league() {
    let mut game = english_game();

    open_league_economy(&mut game, GroundPolicy::FitToLeague);

    assert_eq!(game.economy, EconomyModel::Leagues);
    let capacity = |id: &str| {
        let team = game.teams.iter().find(|team| team.id == id).unwrap();
        let held = crate::stadium::figures::stadium_figures(team.stadium_layout.as_ref().unwrap());
        assert_eq!(team.stadium_capacity, held.capacity, "{id}");
        team.stadium_capacity
    };
    assert!(capacity("bbb-giant") > capacity("ddd-second"));
    assert!(capacity("ddd-second") > 5_000);
}

/// Given the same world, as an imported package whose authors set its grounds
/// When it is opened on the league economy keeping its grounds
/// Then the career is on the league economy and every capacity is as it was.
#[test]
fn opening_a_package_keeps_the_grounds_its_authors_set() {
    let mut game = english_game();
    let before: Vec<u32> = game
        .teams
        .iter()
        .map(|team| team.stadium_capacity)
        .collect();

    open_league_economy(&mut game, GroundPolicy::KeepAuthored);

    assert_eq!(game.economy, EconomyModel::Leagues);
    let after: Vec<u32> = game
        .teams
        .iter()
        .map(|team| team.stadium_capacity)
        .collect();
    assert_eq!(after, before);
}

/// Given England's two divisions
/// When each club's economy is asked for one at a time
/// Then each answer is the one the whole world's table gives, and a club in no league has none.
#[test]
fn one_clubs_economy_is_the_one_the_whole_table_gives() {
    let game = english_game();
    let all = club_economies(&game);

    for team in &game.teams {
        assert_eq!(
            club_economy(&game, &team.id).as_ref(),
            all.get(&team.id),
            "{}",
            team.id
        );
    }
}
