use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
pub struct DoraMetrics {
    pub deployment_frequency_per_day: f64,
    pub change_failure_rate: Option<f64>,
    pub mttr_hours: Option<f64>,
}

pub fn compute(runs: &[(u64, String, i64)], window_days: i64) -> DoraMetrics {
    let green: Vec<i64> = runs
        .iter()
        .filter(|(_, s, _)| s == "green")
        .map(|(_, _, at)| *at)
        .collect();
    let red: Vec<i64> = runs
        .iter()
        .filter(|(_, s, _)| s == "red")
        .map(|(_, _, at)| *at)
        .collect();
    let concluded = green.len() + red.len();
    let change_failure_rate = if concluded == 0 {
        None
    } else {
        Some(red.len() as f64 / concluded as f64)
    };
    let mut recoveries: Vec<f64> = red
        .iter()
        .filter_map(|r| {
            green
                .iter()
                .filter(|g| *g >= r)
                .min()
                .map(|g| (*g - *r) as f64 / 3600.0)
        })
        .collect();
    recoveries.sort_by(|a, b| a.total_cmp(b));
    let mttr_hours = if recoveries.is_empty() {
        None
    } else {
        Some(recoveries[recoveries.len() / 2])
    };
    DoraMetrics {
        deployment_frequency_per_day: green.len() as f64 / window_days as f64,
        change_failure_rate,
        mttr_hours,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::compute;

    #[test]
    fn dora_derives_from_conclusive_runs() {
        let runs = vec![
            (1, "green".to_string(), 1_000),
            (2, "red".to_string(), 2_000),
            (2, "green".to_string(), 9_200),
            (3, "green".to_string(), 20_000),
        ];
        let m = compute(&runs, 14);
        assert_eq!(m.deployment_frequency_per_day, 3.0 / 14.0);
        assert_eq!(m.change_failure_rate, Some(0.25));
        assert_eq!(m.mttr_hours, Some(2.0));
    }

    #[test]
    fn empty_ledger_yields_nulls_not_zeros() {
        let m = compute(&[], 14);
        assert_eq!(m.deployment_frequency_per_day, 0.0);
        assert!(m.change_failure_rate.is_none());
        assert!(m.mttr_hours.is_none());
    }
}
