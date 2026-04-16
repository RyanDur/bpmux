pub(in crate::terminal) fn fmt_opt_hours(h: Option<f64>) -> String {
    match h {
        Some(v) if v < 1.0 => format!("{:.0}min", v * 60.0),
        Some(v) => format!("{v:.1}h"),
        None => "—".to_string(),
    }
}

pub(in crate::terminal) fn fmt_opt_rate(r: Option<f64>) -> String {
    match r {
        Some(v) => format!("{:.0}%", v * 100.0),
        None => "—".to_string(),
    }
}
