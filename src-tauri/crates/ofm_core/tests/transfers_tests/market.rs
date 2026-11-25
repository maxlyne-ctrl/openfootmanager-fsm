use super::*;

/// A day of the AI market with one player at team-3 that only AI clubs can buy: team-2 has the
/// money for him and the user (team-1) takes no part.
fn ai_market_game(mut player: Player) -> Game {
    player.team_id = Some("team-3".to_string());
    let mut game = make_game_with_player(player, vec![], 5_000_000, 2_000_000);
    game.teams
        .push(make_ai_team("team-3", "Seller FC", 3_000_000, 1_000_000));
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    give_every_club_squad_depth(&mut game);
    attach_transfer_log_league(&mut game);
    game
}

fn club_after_a_market_day(mut game: Game, player_id: &str) -> Option<String> {
    evaluate_transfer_market(&mut game);
    find_player(&game, player_id).team_id.clone()
}

/// Given a listed player at team-3 whose contract ends within two months: interest score 90
/// When the AI market runs for a day
/// Then team-2 buys him. This is the control for the two tests below: the same day, the same
///      clubs and the same money do complete a sale.
#[test]
fn an_ai_club_buys_a_player_who_is_very_attractive() {
    let mut player = make_player("player-very-attractive");
    player.stage_contract_end(Some("2026-09-01".to_string()));
    player.market_value = 1_200_000;
    player.transfer_listed = true;

    let club = club_after_a_market_day(ai_market_game(player), "player-very-attractive");

    assert_eq!(club.as_deref(), Some("team-2"));
}

/// Given a listed player at team-3 valued 400,000 with 300 days left: interest score 40, enough
///       to be watched (35) but not the more than 60 an AI-to-AI deal needs
/// When the AI market runs for a day
/// Then he stays at team-3.
#[test]
fn an_ai_club_does_not_buy_a_listed_player_who_is_only_moderately_attractive() {
    let mut player = make_player("player-moderate");
    player.stage_contract_end(Some("2027-05-28".to_string()));
    player.market_value = 400_000;
    player.transfer_listed = true;

    let club = club_after_a_market_day(ai_market_game(player), "player-moderate");

    assert_eq!(club.as_deref(), Some("team-3"));
}

/// Given an unlisted team-3 player valued 600,000 on a long contract who leads the scoring
///       charts: 10 for his value plus the 25 an award leaderboard adds, 35 in all
/// When the AI market runs for a day
/// Then he stays at team-3: the leaderboard bonus is added once, not multiplied.
#[test]
fn an_award_leaderboard_player_is_not_bought_on_the_bonus_alone() {
    let mut player = make_player("player-top-scorer");
    player.market_value = 600_000;
    player.stats.appearances = 6;
    player.stats.goals = 19;

    let club = club_after_a_market_day(ai_market_game(player), "player-top-scorer");

    assert_eq!(club.as_deref(), Some("team-3"));
}

/// Given a listed player at team-3 valued 1,200,000 with 300 days left: interest score exactly
///       60 (30 listed, 10 contract, 20 value)
/// When the AI market runs for a day
/// Then he stays at team-3: an AI-to-AI deal needs more than 60
#[test]
fn an_ai_club_does_not_buy_a_player_scoring_exactly_sixty() {
    let mut player = make_player("player-sixty");
    player.stage_contract_end(Some("2027-05-28".to_string()));
    player.market_value = 1_200_000;
    player.transfer_listed = true;

    let club = club_after_a_market_day(ai_market_game(player), "player-sixty");

    assert_eq!(club.as_deref(), Some("team-3"));
}

/// Given a listed player at team-3 valued 600,000 with four months left: interest score 65
///       (30 listed, 25 contract, 10 value)
/// When the AI market runs for a day
/// Then team-2 buys him: 65 is over the bar
#[test]
fn an_ai_club_buys_a_player_scoring_sixty_five() {
    let mut player = make_player("player-sixty-five");
    player.stage_contract_end(Some("2026-12-01".to_string()));
    player.market_value = 600_000;
    player.transfer_listed = true;

    let club = club_after_a_market_day(ai_market_game(player), "player-sixty-five");

    assert_eq!(club.as_deref(), Some("team-2"));
}

fn attractive_player(id: &str, club: &str) -> Player {
    let mut player = make_player(id);
    player.team_id = Some(club.to_string());
    player.stage_contract_end(Some("2026-09-01".to_string()));
    player.market_value = 1_200_000;
    player.transfer_listed = true;
    player
}

fn domestic_league(id: &str, country: &str, clubs: &[&str]) -> League {
    let ids: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
    let mut league = League::new(id.to_string(), id.to_string(), 2026, &ids);
    league.country_id = Some(country.to_string());
    league
}

/// England (the user's team-1, buyers team-2, team-6 and team-7, and team-3 selling three very
/// attractive players), listed first as the shipped world lists it, and Germany after it (buyer
/// team-4, and team-5 selling one).
fn two_country_market() -> Game {
    let mut game = make_game_with_player(
        attractive_player("eng-a", "team-3"),
        vec![],
        5_000_000,
        2_000_000,
    );
    game.players.push(attractive_player("eng-b", "team-3"));
    game.players.push(attractive_player("eng-c", "team-3"));
    game.players.push(attractive_player("de-a", "team-5"));
    for (id, name) in [
        ("team-3", "Seller FC"),
        ("team-6", "Buyer Six"),
        ("team-7", "Buyer Seven"),
        ("team-4", "Kaufer FC"),
        ("team-5", "Verkaufer FC"),
    ] {
        game.teams
            .push(make_ai_team(id, name, 6_000_000, 3_000_000));
    }
    game.teams[1].finance = 6_000_000;
    game.teams[1].transfer_budget = 3_000_000;
    give_every_club_squad_depth(&mut game);
    attach_transfer_log_league(&mut game);
    game.competitions = vec![
        domestic_league(
            "eng",
            "ENG",
            &["team-1", "team-2", "team-3", "team-6", "team-7"],
        ),
        domestic_league("de", "DE", &["team-4", "team-5"]),
    ];
    game
}

/// Given an English league with more buyers than a day's deals, listed before a German one with a
///       buyer and a player for him
/// When the AI market runs for a day
/// Then the German buyer still signs someone, and England's clubs complete no more than their
///      own day's deals: each league's clubs have their own, so the clubs listed first cannot
///      take the whole world's.
#[test]
fn every_leagues_clubs_get_a_turn_in_the_days_market() {
    let mut game = two_country_market();

    evaluate_transfer_market(&mut game);

    let targets = ["eng-a", "eng-b", "eng-c", "de-a"];
    let bought_by = |club: &str| {
        targets
            .iter()
            .filter(|id| find_player(&game, id).team_id.as_deref() == Some(club))
            .count()
    };
    assert_eq!(bought_by("team-4"), 1, "the German buyer signs someone");
    let english_buys = ["team-2", "team-3", "team-6", "team-7"]
        .iter()
        .map(|club| bought_by(club))
        .sum::<usize>()
        // team-3 still holds what it did not sell.
        - ["eng-a", "eng-b", "eng-c"]
            .iter()
            .filter(|id| find_player(&game, id).team_id.as_deref() == Some("team-3"))
            .count();
    assert!(
        english_buys <= 2,
        "England's clubs completed {english_buys} deals in a day"
    );
}
