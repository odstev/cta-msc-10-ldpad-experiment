//! Statistical reporting utilities.

use rand::seq::SliceRandom;
use rand::thread_rng;

pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() { return 0.0; }
    xs.iter().sum::<f64>() / xs.len() as f64
}

pub fn std_dev(xs: &[f64]) -> f64 {
    if xs.len() < 2 { return 0.0; }
    let m = mean(xs);
    let var = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (xs.len() as f64 - 1.0);
    var.sqrt()
}

pub fn bootstrap_ci_95(samples: &[f64], iterations: usize) -> (f64, f64) {
    if samples.is_empty() { return (0.0, 0.0); }
    let mut rng = thread_rng();
    let mut means = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let resample: Vec<f64> = (0..samples.len())
            .map(|_| *samples.choose(&mut rng).unwrap())
            .collect();
        means.push(mean(&resample));
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = means[(iterations as f64 * 0.025) as usize];
    let hi = means[(iterations as f64 * 0.975) as usize];
    (lo, hi)
}

pub fn cohen_d(a: &[f64], b: &[f64]) -> f64 {
    let ma = mean(a);
    let mb = mean(b);
    let sa = std_dev(a);
    let sb = std_dev(b);
    let pooled = (((sa.powi(2) + sb.powi(2)) / 2.0)).sqrt();
    if pooled.abs() < 1e-12 { return 0.0; }
    (ma - mb) / pooled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_of_uniform() {
        let xs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((mean(&xs) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn ci_brackets_mean() {
        let xs: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let (lo, hi) = bootstrap_ci_95(&xs, 500);
        assert!(lo < mean(&xs));
        assert!(hi > mean(&xs));
    }
}
