//! What a club charges at the gate, and how big a ground its matchday money supports.

use super::ClubEconomy;

/// Average takings per head at the gate, by club revenue, interpolated on a log scale. A League
/// Two crowd pays about EUR 20 a head (GBP 2.1m matchday over 23 home games at ~5,000, Swiss
/// Ramble 2023/24); the Premier League takes about EUR 75 (GBP 1bn+ matchday over 380 games
/// at ~40,000, Deloitte ARFF 2026), hospitality included.
const PRICE_KNOTS: [(i64, i64); 4] = [
    (500_000, 8),
    (9_000_000, 20),
    (40_000_000, 30),
    (380_000_000, 75),
];

/// The share of the ground an average home league match fills.
pub(super) const EXPECTED_ATTENDANCE_PERCENT: i64 = 76;

/// No club plays in less than this, however small: a club's ground is at least a stand.
pub(super) const SMALLEST_GROUND: u32 = 800;
/// The largest grounds in the world hold about 90,000 (Camp Nou, Wembley).
pub(super) const LARGEST_GROUND: u32 = 90_000;

/// What a club whose season takes `revenue_target` charges, on average, a head.
pub fn ticket_price(revenue_target: i64) -> i64 {
    let (first, last) = (PRICE_KNOTS[0], PRICE_KNOTS[PRICE_KNOTS.len() - 1]);
    if revenue_target <= first.0 {
        return first.1;
    }
    if revenue_target >= last.0 {
        return last.1;
    }
    PRICE_KNOTS
        .windows(2)
        .find(|pair| revenue_target <= pair[1].0)
        .map_or(last.1, |pair| {
            let ((low_rev, low_price), (high_rev, high_price)) = (pair[0], pair[1]);
            let along = ((revenue_target as f64).ln() - (low_rev as f64).ln())
                / ((high_rev as f64).ln() - (low_rev as f64).ln());
            (low_price as f64 + (high_price - low_price) as f64 * along).round() as i64
        })
}

/// The ground `club` needs for its matchday share over `home_games` league games at its price
/// and an average crowd, within the smallest and largest grounds there are.
pub fn ground_capacity_for(club: &ClubEconomy, home_games: u32) -> u32 {
    let matchday =
        club.revenue_target as f64 * f64::from(club.league.split.matchday_percent) / 100.0;
    let per_game = matchday / f64::from(home_games.max(1));
    let per_seat =
        ticket_price(club.revenue_target) as f64 * EXPECTED_ATTENDANCE_PERCENT as f64 / 100.0;
    let seats = (per_game / per_seat.max(1.0)).round();
    (seats.clamp(f64::from(SMALLEST_GROUND), f64::from(LARGEST_GROUND))) as u32
}
