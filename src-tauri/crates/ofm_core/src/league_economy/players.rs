//! What a player is worth and paid on the league economy, and how good a club's squad is.
//!
//! One curve for every club, so a player costs the same wherever he is; leagues differ because
//! their players do. The knots follow the real market: Haaland and Mbappe (rated 91 in EA FC 26)
//! are valued near EUR 180m and paid about EUR 35m a year; a League Two regular (about 59) earns
//! about EUR 50k.

/// Market value at a player's peak age, euros, by rating.
const VALUE_KNOTS: [(u8, f64); 8] = [
    (40, 10_000.0),
    (50, 50_000.0),
    (59, 300_000.0),
    (68, 3_000_000.0),
    (77, 20_000_000.0),
    (85, 60_000_000.0),
    (92, 180_000_000.0),
    (99, 300_000_000.0),
];

/// Wage per year, euros, by rating. Fitted on the shipped world so each league's wage bill lands
/// near its share of revenue (run finance_audit): the middle of the curve is shallow, so a
/// mid-table European league pays 55-75% of its money in wages, and it steepens past 74, where
/// the Premier League's squads sit.
const WAGE_KNOTS: [(u8, f64); 9] = [
    (40, 7_500.0),
    (50, 19_000.0),
    (59, 50_000.0),
    (68, 300_000.0),
    (74, 2_300_000.0),
    (77, 7_500_000.0),
    (85, 20_000_000.0),
    (92, 35_000_000.0),
    (99, 45_000_000.0),
];

/// A squad's average rating by its club's revenue, euros a season. Anchored on the league
/// averages agreed for the world: the Premier League about 77, the Championship 68, League Two
/// 59, the Welsh and Northern Irish leagues about 50, a tenth division about 40.
const SQUAD_RATING_KNOTS: [(f64, f64); 12] = [
    (50_000.0, 40.0),
    (400_000.0, 47.0),
    (1_300_000.0, 50.0),
    (3_000_000.0, 55.0),
    (9_000_000.0, 59.0),
    (12_000_000.0, 61.0),
    (30_000_000.0, 66.0),
    (45_000_000.0, 69.0),
    (120_000_000.0, 73.0),
    (200_000_000.0, 75.5),
    (380_000_000.0, 77.0),
    (900_000_000.0, 80.0),
];

/// Interpolate `y` at `x` on a log scale of the `y` values, between knots sorted by `x`; the
/// nearest knot holds outside them.
fn log_interpolate(knots: &[(f64, f64)], x: f64) -> f64 {
    let (Some(first), Some(last)) = (knots.first(), knots.last()) else {
        return 0.0;
    };
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    knots
        .windows(2)
        .find(|pair| x <= pair[1].0)
        .map_or(last.1, |pair| {
            let along = (x - pair[0].0) / (pair[1].0 - pair[0].0);
            (pair[0].1.ln() * (1.0 - along) + pair[1].1.ln() * along).exp()
        })
}

fn by_rating(knots: &[(u8, f64)], rating: u8) -> f64 {
    let knots: Vec<(f64, f64)> = knots.iter().map(|(r, v)| (f64::from(*r), *v)).collect();
    log_interpolate(&knots, f64::from(rating))
}

/// How much more or less than at his peak a player of `age` is worth: the young carry their
/// future, the old are near the end of theirs.
fn age_multiple(age: u32) -> f64 {
    match age {
        0..=21 => 1.4,
        22..=24 => 1.25,
        25..=28 => 1.0,
        29..=30 => 0.8,
        31..=32 => 0.55,
        _ => 0.3,
    }
}

/// A player's market value, euros: his rating's value at peak age, moved by his age.
pub fn market_value_for(rating: u8, age: u32) -> u64 {
    (by_rating(&VALUE_KNOTS, rating) * age_multiple(age)).round() as u64
}

/// A player's weekly wage, euros, for his rating.
pub fn weekly_wage_for(rating: u8) -> u32 {
    let weekly = by_rating(&WAGE_KNOTS, rating) / 52.0;
    weekly.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

/// The average rating a club taking `revenue_target` a season fields.
pub fn squad_rating_target(revenue_target: i64) -> f64 {
    let knots: Vec<(f64, f64)> = SQUAD_RATING_KNOTS.to_vec();
    // Ratings interpolate on revenue's log scale, not their own: swap the axes for the call.
    let swapped: Vec<(f64, f64)> = knots
        .iter()
        .map(|(rev, rating)| (rev.ln(), *rating))
        .collect();
    let x = (revenue_target.max(1) as f64).ln();
    let (first, last) = (swapped[0], swapped[swapped.len() - 1]);
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    swapped
        .windows(2)
        .find(|pair| x <= pair[1].0)
        .map_or(last.1, |pair| {
            let along = (x - pair[0].0) / (pair[1].0 - pair[0].0);
            pair[0].1 + (pair[1].1 - pair[0].1) * along
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given the best players in the world, a Premier League regular and a League Two one
    /// When their values at peak age are read
    /// Then they are the market's: about EUR 180m, 20m and 300k.
    #[test]
    fn a_players_value_is_the_markets_for_his_rating() {
        assert_eq!(market_value_for(92, 26), 180_000_000);
        assert_eq!(market_value_for(77, 26), 20_000_000);
        assert_eq!(market_value_for(59, 26), 300_000);
    }

    /// Given players of one rating at 20, 26 and 34
    /// When they are valued
    /// Then the young one is worth most and the old one least.
    #[test]
    fn youth_adds_value_and_age_takes_it_away() {
        let (young, peak, old) = (
            market_value_for(80, 20),
            market_value_for(80, 26),
            market_value_for(80, 34),
        );

        assert!(young > peak && peak > old, "{young} {peak} {old}");
    }

    /// Given players from a tenth division to the world's best
    /// When their wages are read
    /// Then a 92 earns about EUR 35m a year, a 59 about 50k, a 40 about 7,500, and a better
    ///      player never earns less.
    #[test]
    fn a_players_wage_follows_his_rating_up_the_curve() {
        assert_eq!(
            weekly_wage_for(92),
            (35_000_000.0_f64 / 52.0).round() as u32
        );
        assert_eq!(weekly_wage_for(59), (50_000.0_f64 / 52.0).round() as u32);
        assert_eq!(weekly_wage_for(40), (7_500.0_f64 / 52.0).round() as u32);
        assert!((30..99).all(|rating| weekly_wage_for(rating) <= weekly_wage_for(rating + 1)));
    }

    /// Given clubs taking what an average Premier League, League Two and Welsh club takes
    /// When their squads' target ratings are read
    /// Then they are about 77, 59 and 50, and a richer club never fields a weaker squad.
    #[test]
    fn a_richer_club_fields_a_better_squad() {
        assert!((squad_rating_target(380_000_000) - 77.0).abs() < 0.01);
        assert!((squad_rating_target(9_000_000) - 59.0).abs() < 0.01);
        assert!((squad_rating_target(1_200_000) - 50.0).abs() < 0.5);
        let mut previous = 0.0;
        for revenue in [
            10_000,
            100_000,
            1_000_000,
            10_000_000,
            100_000_000,
            1_000_000_000,
        ] {
            let rating = squad_rating_target(revenue);
            assert!(rating >= previous, "{revenue}: {rating} after {previous}");
            previous = rating;
        }
    }
}
