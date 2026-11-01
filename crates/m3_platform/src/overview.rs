use chrono::{DateTime, Utc};
use serde::Serialize;
use soil_contracts::{Parameter, SoilReading};

use crate::aggregate::{sensor_health, Health};
use crate::plots::PlotMeta;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParamSeries {
    pub moisture: Vec<Option<f32>>,
    pub temperature: Vec<Option<f32>>,
    pub ph: Vec<Option<f32>>,
    pub ec: Vec<Option<f32>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlotOverview {
    pub plot: PlotMeta,
    pub latest: Option<SoilReading>,
    pub series: ParamSeries,
    pub health: Health,
}

#[derive(Debug, Clone, Copy)]
pub struct OverviewWindow {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub slots: usize,
    pub expected_secs: i64,
}

pub fn slot_means(
    points: &[(DateTime<Utc>, f32)],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    slots: usize,
) -> Vec<Option<f32>> {
    if to <= from {
        return vec![None; slots.max(1)];
    }
    let n = slots.max(1);
    let span_ms = i128::from((to - from).num_milliseconds().max(1));
    let (sums, counts) = points
        .iter()
        .filter(|p| p.1.is_finite() && p.0 >= from && p.0 <= to)
        .fold(
            (vec![0.0_f64; n], vec![0_usize; n]),
            |(mut sums, mut counts), p| {
                let offset_ms = i128::from((p.0 - from).num_milliseconds());
                let idx = ((offset_ms * n as i128) / span_ms) as usize;
                let idx = idx.min(n - 1);
                sums[idx] += f64::from(p.1);
                counts[idx] += 1;
                (sums, counts)
            },
        );
    sums.into_iter()
        .zip(counts)
        .map(|(sum, count)| (count > 0).then(|| (sum / count as f64) as f32))
        .collect()
}

fn usable_points(readings: &[SoilReading], parameter: Parameter) -> Vec<(DateTime<Utc>, f32)> {
    readings
        .iter()
        .filter(|r| r.is_usable())
        .map(|r| (r.ts, r.value(parameter)))
        .collect()
}

pub fn pick_latest(readings: &[SoilReading], fallback: Option<SoilReading>) -> Option<SoilReading> {
    readings
        .iter()
        .filter(|r| r.is_usable())
        .max_by_key(|r| r.ts)
        .cloned()
        .or(fallback)
}

pub fn build_plot_overview(
    meta: PlotMeta,
    readings: &[SoilReading],
    latest: Option<SoilReading>,
    w: OverviewWindow,
) -> PlotOverview {
    let series_of = |p: Parameter| slot_means(&usable_points(readings, p), w.from, w.to, w.slots);
    let series = ParamSeries {
        moisture: series_of(Parameter::Moisture),
        temperature: series_of(Parameter::Temperature),
        ph: series_of(Parameter::Ph),
        ec: series_of(Parameter::Ec),
    };

    let health_input: &[SoilReading] = match (readings.is_empty(), latest.as_ref()) {
        (true, Some(last)) => std::slice::from_ref(last),
        _ => readings,
    };
    let health = sensor_health(health_input, w.to, w.expected_secs);

    PlotOverview {
        plot: meta,
        latest: pick_latest(readings, latest),
        series,
        health,
    }
}

#[cfg(test)]
#[path = "tests/overview.rs"]
mod tests;
