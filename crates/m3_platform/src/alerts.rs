use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use soil_contracts::{Alert, AlertLevel, PlotId};
use std::collections::HashMap;

/// Batas atas jendela deduplikasi yang boleh diminta lewat API (7 hari).
const MAX_DEDUPE_SECS: i64 = 7 * 24 * 3600;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AlertFilter {
    #[serde(alias = "plot")]
    pub plot_id: Option<PlotId>,
    #[serde(alias = "level")]
    pub min_level: Option<AlertLevel>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub dedupe_secs: Option<i64>,
}

pub fn filter_alerts(alerts: &[Alert], f: &AlertFilter) -> Vec<Alert> {
    let mut out: Vec<Alert> = alerts
        .iter()
        .filter(|a| f.plot_id.is_none_or(|p| a.plot_id == p))
        .filter(|a| f.min_level.is_none_or(|l| a.level >= l))
        .filter(|a| f.from.is_none_or(|t| a.ts >= t))
        .filter(|a| f.to.is_none_or(|t| a.ts <= t))
        .cloned()
        .collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.ts));
    out
}

pub fn dedupe_alerts(alerts: &[Alert], cooldown: Duration) -> Vec<Alert> {
    let mut sorted: Vec<&Alert> = alerts.iter().collect();
    sorted.sort_by_key(|a| a.ts);
    let mut last_kept: HashMap<(PlotId, String), DateTime<Utc>> = HashMap::new();
    let mut out = Vec::new();
    for a in sorted {
        let key = (a.plot_id, format!("{:?}|{}", a.level, a.message));
        let keep = last_kept
            .get(&key)
            .is_none_or(|prev| a.ts - *prev >= cooldown);
        if keep {
            last_kept.insert(key, a.ts);
            out.push(a.clone());
        }
    }
    out
}

pub fn filter_and_dedupe(alerts: &[Alert], f: &AlertFilter) -> Vec<Alert> {
    let filtered = filter_alerts(alerts, f);
    match f.dedupe_secs.filter(|s| *s > 0) {
        None => filtered,
        Some(secs) => {
            let mut kept = dedupe_alerts(&filtered, Duration::seconds(secs.min(MAX_DEDUPE_SECS)));
            kept.sort_by_key(|a| std::cmp::Reverse(a.ts));
            kept
        }
    }
}

#[cfg(test)]
#[path = "tests/alerts.rs"]
mod tests;
