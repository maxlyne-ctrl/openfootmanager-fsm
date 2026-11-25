//! How a league's money divides between its clubs.

/// Richest to poorest in a league of real size: the Premier League's top club takes about five
/// times its bottom one (GBP 719.5m against GBP 132.3m in 2023/24, Swiss Ramble).
pub(super) const RICHEST_TO_POOREST: f64 = 5.0;

/// Each place's multiple of the league average, richest first, for a league of `clubs`. The
/// multiples fall geometrically from richest to poorest and average exactly 1, so the spread
/// moves money between clubs without making any.
pub fn revenue_spread(clubs: usize) -> Vec<f64> {
    if clubs <= 1 {
        return vec![1.0; clubs];
    }
    let steps = (clubs - 1) as f64;
    let raw: Vec<f64> = (0..clubs)
        .map(|place| RICHEST_TO_POOREST.powf(-(place as f64) / steps))
        .collect();
    let mean = raw.iter().sum::<f64>() / clubs as f64;
    raw.into_iter().map(|multiple| multiple / mean).collect()
}
