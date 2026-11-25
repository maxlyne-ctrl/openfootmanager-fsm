//! What a club on the league economy is paid by its league: television money and commercial
//! income each week, and prize money by position at the season's end.

use crate::league_economy::{ClubEconomy, LeagueEconomy};

/// Television and commercial money are paid in equal weekly instalments over the year.
pub(super) const WEEKS_PER_YEAR: i64 = 52;

/// The share of a league's television money held back and paid by final position. The Premier
/// League pays about a quarter of its central distribution on merit.
pub(crate) const MERIT_SHARE_PERCENT: i64 = 25;

/// A week's television and commercial money for `club`: its revenue target's broadcast share,
/// less the merit share paid at the end of the season, and its commercial share.
pub(super) fn weekly_league_income(club: &ClubEconomy) -> (i64, i64) {
    let share = |percent: u32| club.revenue_target.saturating_mul(i64::from(percent)) / 100;
    let broadcast = share(club.league.split.broadcast_percent) * (100 - MERIT_SHARE_PERCENT)
        / 100
        / WEEKS_PER_YEAR;
    let commercial = share(club.league.split.commercial_percent) / WEEKS_PER_YEAR;
    (broadcast, commercial)
}

/// What a club keeps of its revenue after wages and running costs, for the transfer market and
/// the bank: UEFA's top divisions spend 47% of revenue on player wages and 36% on other running
/// costs (ECFIL 2025), which leaves about a tenth once the rest of the wage bill is counted.
pub(crate) const MARGIN_PERCENT: i64 = 10;

/// A week of `club`'s running costs: the stadium, the staff who are not players, travel,
/// administration.
pub(super) fn weekly_running_costs(club: &ClubEconomy) -> i64 {
    let share = 100 - i64::from(club.league.wage_ratio_percent) - MARGIN_PERCENT;
    club.revenue_target.saturating_mul(share.max(0)) / 100 / WEEKS_PER_YEAR
}

/// Prize money for finishing `position` (1 is first) of `clubs` in a league with this economy.
/// The league's merit pot is split in steps from first to last: first takes `clubs` shares,
/// last takes one.
pub(crate) fn league_prize_money(league: &LeagueEconomy, clubs: u32, position: u32) -> i64 {
    if clubs == 0 || position == 0 || position > clubs {
        return 0;
    }
    let pot = league
        .revenue_per_club
        .saturating_mul(i64::from(clubs))
        .saturating_mul(i64::from(league.split.broadcast_percent))
        / 100
        * MERIT_SHARE_PERCENT
        / 100;
    let shares = i64::from(clubs) * (i64::from(clubs) + 1) / 2;
    let own_shares = i64::from(clubs - position + 1);
    pot.saturating_mul(own_shares) / shares
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::game::Game;
    use crate::league_economy::{EconomyModel, league_economy};
    use chrono::{TimeZone, Utc};
    use domain::finance::CashKind;
    use domain::league::League;
    use domain::manager::Manager;
    use domain::team::Team;

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
        team.finance = 10_000_000;
        team
    }

    /// A Monday in England's season; a top flight of two, and the user at the smaller club.
    fn english_game(economy: EconomyModel) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 7, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("aaa-small".to_string());
        let mut game = Game::new(
            clock,
            manager,
            vec![club("aaa-small", 700), club("bbb-giant", 800)],
            vec![],
            vec![],
            vec![],
        );
        let ids = vec!["aaa-small".to_string(), "bbb-giant".to_string()];
        let mut league = League::new("top".to_string(), "Top".to_string(), 2026, &ids);
        league.country_id = Some("ENG".to_string());
        game.league = Some(league.clone());
        game.competitions = vec![league];
        game.economy = economy;
        game
    }

    fn paid(game: &Game, club: &str, kind: CashKind) -> i64 {
        game.cash_journal
            .posts_for(club)
            .filter(|post| post.kind == kind)
            .map(|post| post.amount)
            .sum()
    }

    /// Given a career on the league economy, in England's top flight
    /// When a week's finances are processed
    /// Then every club is paid its week of television and commercial money, the famous club
    ///      more than the other, and the amounts are a 52nd of its targets' shares.
    #[test]
    fn a_week_on_the_league_economy_pays_television_and_commercial_money() {
        let mut game = english_game(EconomyModel::Leagues);
        let giant = crate::league_economy::club_economy(&game, "bbb-giant").unwrap();

        crate::finances::process_weekly_finances(&mut game);

        let (broadcast, commercial) = weekly_league_income(&giant);
        assert!(broadcast > 1_000_000, "{broadcast}");
        assert_eq!(paid(&game, "bbb-giant", CashKind::Broadcasting), broadcast);
        assert_eq!(paid(&game, "bbb-giant", CashKind::Commercial), commercial);
        assert!(
            paid(&game, "bbb-giant", CashKind::Broadcasting)
                > paid(&game, "aaa-small", CashKind::Broadcasting)
        );
    }

    /// Given a career on the league economy, in England's top flight
    /// When a week's finances are processed
    /// Then each club pays a week of running costs: what is left of its revenue target once its
    ///      league's wage share and a tenth for the transfer market are set aside.
    #[test]
    fn a_week_on_the_league_economy_pays_the_running_costs() {
        let mut game = english_game(EconomyModel::Leagues);
        let giant = crate::league_economy::club_economy(&game, "bbb-giant").unwrap();

        crate::finances::process_weekly_finances(&mut game);

        let expected = giant.revenue_target
            * (100 - i64::from(giant.league.wage_ratio_percent) - MARGIN_PERCENT)
            / 100
            / WEEKS_PER_YEAR;
        assert_eq!(weekly_running_costs(&giant), expected);
        assert!(paid(&game, "bbb-giant", CashKind::Upkeep) <= -expected);
    }

    /// Given a career on the old economy
    /// When a week's finances are processed
    /// Then no television or commercial money is paid: the old economy is left as it was.
    #[test]
    fn a_week_on_the_old_economy_pays_no_league_money() {
        let mut game = english_game(EconomyModel::Legacy);

        crate::finances::process_weekly_finances(&mut game);

        for club in ["aaa-small", "bbb-giant"] {
            assert_eq!(paid(&game, club, CashKind::Broadcasting), 0);
            assert_eq!(paid(&game, club, CashKind::Commercial), 0);
        }
    }

    /// Given England's top flight of twenty
    /// When the prize for each position is read
    /// Then first takes twenty times last, every place takes less than the one above, and the
    ///      prizes add up to the league's merit pot.
    #[test]
    fn prize_money_falls_from_first_to_last_and_pays_out_the_pot() {
        let league = league_economy("ENG", 0, 750);
        let prizes: Vec<i64> = (1..=20)
            .map(|p| league_prize_money(&league, 20, p))
            .collect();

        assert_eq!(prizes[0] / prizes[19], 20);
        assert!(prizes.windows(2).all(|pair| pair[0] > pair[1]));
        let pot = 380_000_000_i64 * 20 * 50 / 100 * MERIT_SHARE_PERCENT / 100;
        let total: i64 = prizes.iter().sum();
        assert!((pot - total).abs() <= 20, "pot {pot}, paid {total}");
        assert_eq!(league_prize_money(&league, 20, 21), 0);
    }
}
