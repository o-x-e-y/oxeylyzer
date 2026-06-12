use std::collections::HashMap;
use std::time::Duration;

/// Collected results for one algorithm during a bench run.
pub struct AlgoMetrics {
    #[allow(dead_code)]
    pub name: &'static str,
    pub scores: Vec<i64>,
    /// layout string → occurrence count, for duplicate detection
    pub counts: HashMap<String, usize>,
    /// set once the algorithm finishes its budget
    pub elapsed: Option<Duration>,
}

impl AlgoMetrics {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            scores: Vec::new(),
            counts: HashMap::new(),
            elapsed: None,
        }
    }

    pub fn record(&mut self, score: i64, layout: String) {
        self.scores.push(score);
        *self.counts.entry(layout).or_default() += 1;
    }

    pub fn best(&self) -> Option<i64> {
        self.scores.iter().max().copied()
    }

    pub fn top5_mean(&self) -> Option<f64> {
        if self.scores.is_empty() {
            return None;
        }
        let mut sorted = self.scores.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        let top = &sorted[..sorted.len().min(5)];

        Some(top.iter().sum::<i64>() as f64 / top.len() as f64)
    }

    /// Two layouts are duplicates iff their layout strings are identical.
    pub fn duplicate_pct(&self) -> f64 {
        let n = self.scores.len();
        if n == 0 {
            return 0.0;
        }
        let unique = self.counts.len();

        (n - unique) as f64 / n as f64 * 100.0
    }

    pub fn layouts_per_sec(&self) -> Option<f64> {
        self.elapsed
            .map(|d| self.scores.len() as f64 / d.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled() -> AlgoMetrics {
        let mut m = AlgoMetrics::new("test");
        for (score, layout) in [
            (10, "abc"),
            (50, "def"),
            (30, "abc"),
            (20, "ghi"),
            (40, "jkl"),
            (60, "mno"),
            (50, "def"),
        ] {
            m.record(score, layout.to_string());
        }
        m
    }

    #[test]
    fn best_is_max() {
        assert_eq!(filled().best(), Some(60));
    }

    #[test]
    fn top5_mean_takes_five_highest() {
        // top 5 of [10,50,30,20,40,60,50] = [60,50,50,40,30] → mean 46
        assert_eq!(filled().top5_mean(), Some(46.0));
    }

    #[test]
    fn top5_mean_with_fewer_than_five() {
        let mut m = AlgoMetrics::new("test");
        m.record(10, "a".into());
        m.record(20, "b".into());
        assert_eq!(m.top5_mean(), Some(15.0));
    }

    #[test]
    fn duplicate_pct_counts_repeated_layout_strings() {
        // 7 layouts, 5 unique ("abc" ×2, "def" ×2) → 2/7 duplicates
        let pct = filled().duplicate_pct();
        assert!((pct - (2.0 / 7.0 * 100.0)).abs() < 1e-9);
    }

    #[test]
    fn empty_metrics_are_sane() {
        let m = AlgoMetrics::new("test");
        assert_eq!(m.best(), None);
        assert_eq!(m.top5_mean(), None);
        assert_eq!(m.duplicate_pct(), 0.0);
        assert_eq!(m.layouts_per_sec(), None);
    }
}
