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
/// Then a League Two crowd pays about EUR 20, a Premier League one EUR 90, a tiny club EUR 8,
///      and richer never means cheaper.
#[test]
fn the_price_of_a_ticket_rises_with_the_clubs_money() {
    assert_eq!(matchday::ticket_price(9_000_000), 20);
    assert_eq!(matchday::ticket_price(380_000_000), 90);
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

    open_league_economy(&mut game);

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

fn squad_player(id: &str, club: Option<&str>, rating: u8, potential: u8) -> domain::player::Player {
    let mut player = domain::player::Player::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        "2000-01-01".to_string(),
        "ENG".to_string(),
        domain::player::Position::Midfielder,
        crate::test_support::uniform_attributes(rating),
    );
    player.team_id = club.map(str::to_string);
    crate::player_rating::refresh_player_derived(
        &mut player,
        chrono::NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
    );
    player.potential = potential.max(player.ovr);
    player
}

/// England's two divisions with three players a club, all rated as one, and a free agent.
fn english_game_with_squads() -> Game {
    let mut game = english_game();
    let clubs: Vec<String> = game.teams.iter().map(|team| team.id.clone()).collect();
    for club in &clubs {
        for n in 0..3 {
            let rating = game.players.len() as u8 % 3 + 64;
            game.players
                .push(squad_player(&format!("{club}-{n}"), Some(club), rating, 80));
        }
    }
    game.players.push(squad_player("free-agent", None, 66, 66));
    game
}

fn squad_average(game: &Game, club: &str) -> f64 {
    let ratings: Vec<f64> = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(club))
        .map(|player| f64::from(player.ovr))
        .collect();
    ratings.iter().sum::<f64>() / ratings.len() as f64
}

/// Given England's two divisions, every squad rated about the same
/// When the world is opened on the league economy
/// Then each squad opens at the rating its club's money buys, less the points a new world's
///      squads grow in their first seasons, so the most famous top-flight club fields a far
///      better side than a second-division one.
#[test]
fn opening_a_world_rates_every_squad_by_its_money() {
    let mut game = english_game_with_squads();

    open_league_economy(&mut game);

    let economies = club_economies(&game);
    for club in ["bbb-giant", "ddd-second", "fff-nowhere"] {
        let Some(economy) = economies.get(club) else {
            continue;
        };
        let target = squad_rating_target(economy.revenue_target) - squads::SETTLING_ALLOWANCE;
        let average = squad_average(&game, club);
        assert!(
            (average - target).abs() <= 1.5,
            "{club}: {average} for {target}"
        );
    }
    assert!(squad_average(&game, "bbb-giant") > squad_average(&game, "ddd-second") + 6.0);
}

/// Given a young player with room to grow
/// When his squad is moved to its new standard
/// Then his ceiling moves with him: he keeps the headroom he had.
#[test]
fn a_player_keeps_his_headroom_when_his_squad_is_moved() {
    let mut game = english_game_with_squads();
    let before = game
        .players
        .iter()
        .find(|p| p.id == "ddd-second-0")
        .cloned()
        .unwrap();

    open_league_economy(&mut game);

    let after = game
        .players
        .iter()
        .find(|p| p.id == "ddd-second-0")
        .unwrap();
    let headroom_before = i32::from(before.potential) - i32::from(before.ovr);
    let headroom_after = i32::from(after.potential) - i32::from(after.ovr);
    assert!(
        (headroom_after - headroom_before).abs() <= 2,
        "{headroom_before} -> {headroom_after}"
    );
}

/// Given the same world
/// When it is opened on the league economy
/// Then every player is valued on the market's curve for his new rating and age, and a free
///      agent, whom no club pays, asks the curve's wage.
#[test]
fn opening_a_world_values_every_player_on_the_market_curve() {
    let mut game = english_game_with_squads();

    open_league_economy(&mut game);

    for player in &game.players {
        assert_eq!(
            player.market_value,
            market_value_for(player.ovr, 26),
            "{}",
            player.id
        );
    }
    let free = game.players.iter().find(|p| p.id == "free-agent").unwrap();
    assert_eq!(free.wage(), weekly_wage_for(free.ovr));
}

/// Given the same world
/// When it is opened on the league economy
/// Then each club's wage bill is its league's share of its revenue, and inside a club a better
///      player is never paid less than a worse one.
#[test]
fn opening_a_world_pays_each_squad_its_leagues_share_of_its_money() {
    let mut game = english_game_with_squads();

    open_league_economy(&mut game);

    let economies = club_economies(&game);
    for (club, economy) in &economies {
        let mut squad: Vec<(u8, u32)> = game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(club.as_str()))
            .map(|player| (player.ovr, player.wage()))
            .collect();
        let bill: i64 = squad.iter().map(|(_, wage)| i64::from(*wage)).sum();
        let share =
            economy.revenue_target * i64::from(economy.league.wage_ratio_percent) / 100 / 52;
        assert!(
            (bill - share).abs() <= squad.len() as i64,
            "{club}: {bill} for {share}"
        );
        squad.sort();
        assert!(
            squad.windows(2).all(|pair| pair[0].1 <= pair[1].1),
            "{club}: {squad:?}"
        );
    }
}

/// Given the same world
/// When it is opened on the league economy
/// Then every club in a league can pay its squad, holds a quarter of a season's revenue in cash,
///      and the most famous club has the most to spend.
#[test]
fn opening_a_world_sets_every_clubs_budgets_from_its_money() {
    let mut game = english_game_with_squads();

    open_league_economy(&mut game);

    let economies = club_economies(&game);
    for team in game
        .teams
        .iter()
        .filter(|team| economies.contains_key(&team.id))
    {
        let bill: i64 = game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(team.id.as_str()))
            .map(|player| i64::from(player.wage()))
            .sum();
        assert!(
            team.wage_budget >= bill,
            "{}: budget {} for {bill}",
            team.id,
            team.wage_budget
        );
        assert!(
            team.finance >= economies[&team.id].revenue_target / 4,
            "{}",
            team.id
        );
    }
    let budget = |id: &str| {
        game.teams
            .iter()
            .find(|team| team.id == id)
            .unwrap()
            .transfer_budget
    };
    assert!(budget("bbb-giant") > budget("ddd-second"));
}

/// Given a sixteen-year-old recruit of the most famous top-flight club, and one of a
///      second-division club, each drawn at 45 with a ceiling of 70
/// When each arrives in a league economy career
/// Then the famous club's is moved up and the other's down, the famous club's with more room
///      to grow, and both are valued and paid on the market's curves.
#[test]
fn a_recruit_is_brought_to_his_clubs_standard_and_priced_on_the_curves() {
    let mut game = english_game();
    game.economy = EconomyModel::Leagues;
    let mut giant = squad_player("giant-kid", Some("bbb-giant"), 45, 70);
    let mut second = squad_player("second-kid", Some("eee-second"), 45, 70);
    let (giant_before, second_before) = (giant.ovr, second.ovr);

    fit_newcomer(&game, &mut giant);
    fit_newcomer(&game, &mut second);

    assert!(
        giant.ovr > giant_before + 3,
        "{giant_before} -> {}",
        giant.ovr
    );
    assert!(
        second.ovr <= second_before + 1,
        "{second_before} -> {}",
        second.ovr
    );
    for player in [&giant, &second] {
        assert_eq!(player.wage(), weekly_wage_for(player.ovr));
    }
    // The ceiling moves by the whole gap to the club's standard and the rating by half of it,
    // so a rich academy's recruit has more room to grow than a poor one's.
    let headroom = |p: &domain::player::Player| i32::from(p.potential) - i32::from(p.ovr);
    assert!(headroom(&giant) > 25, "giant headroom {}", headroom(&giant));
    assert!(headroom(&giant) > headroom(&second));
}

/// Given a free agent generated for a national pool
/// When he arrives in a league economy career
/// Then his rating is his own, and he is valued and paid on the market's curves.
#[test]
fn a_free_agent_keeps_his_rating_and_is_priced_on_the_curves() {
    let mut game = english_game();
    game.economy = EconomyModel::Leagues;
    let mut free = squad_player("pool", None, 67, 67);
    free.market_value = 1;
    let rating = free.ovr;

    fit_newcomer(&game, &mut free);

    assert_eq!(free.ovr, rating);
    assert_eq!(free.wage(), weekly_wage_for(rating));
    assert!(free.market_value > 1);
}

/// Given a recruit in a career on the old economy
/// When he arrives
/// Then he is left exactly as generated.
#[test]
fn a_newcomer_on_the_old_economy_is_left_as_generated() {
    let game = english_game();
    let mut recruit = squad_player("kid", Some("bbb-giant"), 45, 70);
    let before = (
        recruit.ovr,
        recruit.potential,
        recruit.market_value,
        recruit.wage(),
    );

    fit_newcomer(&game, &mut recruit);

    assert_eq!(
        (
            recruit.ovr,
            recruit.potential,
            recruit.market_value,
            recruit.wage()
        ),
        before
    );
}

/// Given a prospect a scout found for the most famous top-flight club, belonging to no club yet
/// When he is fitted to the league economy
/// Then he is drawn to that club's standard, as its own recruit would be.
#[test]
fn a_scouted_prospect_is_drawn_to_the_standard_of_the_club_that_found_him() {
    let mut game = english_game();
    game.economy = EconomyModel::Leagues;
    let mut recruit = squad_player("kid", Some("bbb-giant"), 45, 70);
    let mut prospect = squad_player("found", None, 45, 70);

    fit_newcomer(&game, &mut recruit);
    fit_prospect(&game, "bbb-giant", &mut prospect);

    assert_eq!(prospect.ovr, recruit.ovr);
    assert!(prospect.team_id.is_none());
}

/// Given a league economy world just opened, so each club pays its squad its league's share
/// When a squad player's market wage at his own club is read
/// Then it is the wage the club pays him now: the opening already set every wage to the market.
#[test]
fn a_players_market_wage_is_what_his_club_pays_for_his_rating() {
    let mut game = english_game_with_squads();
    open_league_economy(&mut game);

    let economies = club_economies(&game);
    for player in game.players.iter().filter(|p| p.team_id.is_some()) {
        let club = player.team_id.as_deref().unwrap();
        if !economies.contains_key(club) {
            continue;
        }
        let market = market_wage(&game, player, club).expect("a club in a league has a market");
        let paid = i64::from(player.wage());
        assert!(
            (i64::from(market) - paid).abs() <= 1,
            "{}: market {market}, paid {paid}",
            player.id
        );
    }
}

/// Given a player whose rating has fallen since his wage was set
/// When his market wage is read
/// Then it is below what he is paid: the market pays for what he is now, so his next contract
///      can be cheaper.
#[test]
fn a_fading_players_market_wage_falls_with_his_rating() {
    let mut game = english_game_with_squads();
    open_league_economy(&mut game);
    let index = game
        .players
        .iter()
        .position(|p| p.id == "bbb-giant-0")
        .unwrap();
    let paid = game.players[index].wage();
    let position = game.players[index].natural_position.clone();
    let lower = game.players[index].ovr - 6;
    crate::player_rating::move_attributes_to(&mut game.players[index].attributes, &position, lower);
    let today = game.clock.current_date.date_naive();
    crate::player_rating::refresh_player_derived(&mut game.players[index], today);

    let market = market_wage(&game, &game.players[index], "bbb-giant").unwrap();

    assert!(market < paid, "market {market}, paid {paid}");
}

/// Given a career on the old economy, and a club in no league
/// When a market wage is asked for
/// Then there is none, and the old rules stand.
#[test]
fn there_is_no_market_wage_off_the_league_economy() {
    let game = english_game_with_squads();
    let player = game.players.iter().find(|p| p.id == "bbb-giant-0").unwrap();

    assert_eq!(market_wage(&game, player, "bbb-giant"), None);
    let mut on = english_game_with_squads();
    on.economy = EconomyModel::Leagues;
    assert_eq!(market_wage(&on, player, "fff-nowhere"), None);
}

/// Given a league economy world a season on, the most famous club sitting on a hoard of cash
/// When budgets are set for the new season
/// Then its wage budget is its league's share of its revenue, and its transfer budget a seventh
///      of its revenue plus half the cash it holds over a reserve of a quarter-season: money in
///      the bank becomes money to spend, as it does at a real club.
#[test]
fn a_new_season_turns_a_hoard_into_a_transfer_budget() {
    let mut game = english_game_with_squads();
    open_league_economy(&mut game);
    let giant = club_economy(&game, "bbb-giant").unwrap();
    let revenue = giant.revenue_target;
    let index = game.teams.iter().position(|t| t.id == "bbb-giant").unwrap();
    game.teams[index].finance = revenue * 2;

    refresh_budgets(&mut game);

    let team = &game.teams[index];
    let share = revenue * i64::from(giant.league.wage_ratio_percent) / 100 / 52;
    assert_eq!(team.wage_budget, share * 105 / 100);
    let reserve = revenue / 4;
    assert_eq!(
        team.transfer_budget,
        revenue * 15 / 100 + (revenue * 2 - reserve) / 2
    );
}

/// Given a club short of cash
/// When budgets are set for the new season
/// Then its transfer budget is no more than it holds: a club cannot spend money it does not
///      have.
#[test]
fn a_club_short_of_cash_has_no_more_to_spend_than_it_holds() {
    let mut game = english_game_with_squads();
    open_league_economy(&mut game);
    let index = game
        .teams
        .iter()
        .position(|t| t.id == "ddd-second")
        .unwrap();
    game.teams[index].finance = 1_000;

    refresh_budgets(&mut game);

    assert_eq!(game.teams[index].transfer_budget, 1_000);
}

/// Given a career on the old economy
/// When budgets are set for the new season
/// Then nothing changes: the old economy never refreshed them.
#[test]
fn budgets_are_left_alone_on_the_old_economy() {
    let mut game = english_game_with_squads();
    let before: Vec<(i64, i64)> = game
        .teams
        .iter()
        .map(|t| (t.wage_budget, t.transfer_budget))
        .collect();

    refresh_budgets(&mut game);

    let after: Vec<(i64, i64)> = game
        .teams
        .iter()
        .map(|t| (t.wage_budget, t.transfer_budget))
        .collect();
    assert_eq!(after, before);
}
