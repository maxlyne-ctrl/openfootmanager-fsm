//! Small sampling helpers the quick resolver needs and `rand` does not ship
//! without `rand_distr`.

use rand::{Rng, RngExt};

/// A Poisson draw. Knuth's product method: exact, and cheap for the small
/// means a match produces (the largest is shots, in the teens).
pub(super) fn poisson<R: Rng>(rng: &mut R, mean: f64) -> u32 {
    if mean.is_nan() || mean <= 0.0 {
        return 0;
    }
    // Split very large means so `exp(-mean)` never underflows.
    if mean > 60.0 {
        let half = mean / 2.0;
        return poisson(rng, half) + poisson(rng, mean - half);
    }
    let limit = (-mean).exp();
    let mut product = 1.0;
    let mut count = 0u32;
    loop {
        product *= rng.random_range(0.0..1.0f64);
        if product <= limit {
            return count;
        }
        count += 1;
    }
}

/// How many of `trials` succeed with probability `p` each.
pub(super) fn binomial<R: Rng>(rng: &mut R, trials: u32, p: f64) -> u32 {
    (0..trials).filter(|_| chance(rng, p)).count() as u32
}

pub(super) fn chance<R: Rng>(rng: &mut R, p: f64) -> bool {
    rng.random_range(0.0..1.0f64) < p
}

/// A normal draw (Box–Muller).
pub(super) fn normal<R: Rng>(rng: &mut R, mean: f64, sd: f64) -> f64 {
    let u1 = rng.random_range(f64::MIN_POSITIVE..1.0f64);
    let u2 = rng.random_range(0.0..1.0f64);
    mean + sd * (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// An index drawn from unnormalised weights.
pub(super) fn categorical<R: Rng>(rng: &mut R, weights: &[f64]) -> usize {
    let total: f64 = weights.iter().sum();
    let mut roll = rng.random_range(0.0..total.max(f64::MIN_POSITIVE));
    for (index, &w) in weights.iter().enumerate() {
        if roll < w {
            return index;
        }
        roll -= w;
    }
    weights.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Given many draws at a mean of 2.3, when they are averaged, then the
    /// mean and variance both sit near 2.3, as a Poisson's must.
    #[test]
    fn poisson_draws_have_the_asked_for_mean_and_variance() {
        let mut rng = StdRng::seed_from_u64(7);
        let draws: Vec<f64> = (0..200_000)
            .map(|_| f64::from(poisson(&mut rng, 2.3)))
            .collect();
        let mean = draws.iter().sum::<f64>() / draws.len() as f64;
        let var = draws.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / draws.len() as f64;
        assert!((mean - 2.3).abs() < 0.02, "mean {mean}");
        assert!((var - 2.3).abs() < 0.05, "variance {var}");
    }

    /// Given a mean of zero or a NaN, when drawn, then the count is zero
    /// rather than a hang.
    #[test]
    fn a_degenerate_mean_draws_zero() {
        let mut rng = StdRng::seed_from_u64(7);
        assert_eq!(poisson(&mut rng, 0.0), 0);
        assert_eq!(poisson(&mut rng, f64::NAN), 0);
        assert_eq!(poisson(&mut rng, -1.0), 0);
    }
}
