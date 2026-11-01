use super::*;
use crate::aggregate::HealthStatus;
use crate::plots::NewPlot;
use chrono::{Duration, TimeZone};
use soil_contracts::{DeviceId, PlotId, QualityFlag};

fn t0() -> DateTime<Utc> {
    Utc.timestamp_opt(1_800_000_000, 0).single().unwrap()
}

fn meta() -> PlotMeta {
    PlotMeta::from_new(NewPlot {
        name: "Petak Uji".into(),
        latitude: None,
        longitude: None,
        area_ha: None,
        plant: None,
    })
    .unwrap()
}

fn reading(ts: DateTime<Utc>, moisture: f32, quality: QualityFlag) -> SoilReading {
    SoilReading {
        device_id: DeviceId::new(),
        plot_id: PlotId::new(),
        ts,
        moisture_pct: moisture,
        temperature_c: 27.0,
        ph: 4.0,
        ec_ms_cm: 0.5,
        quality,
    }
}

fn window(hours: i64, slots: usize) -> OverviewWindow {
    OverviewWindow {
        from: t0(),
        to: t0() + Duration::hours(hours),
        slots,
        expected_secs: 600,
    }
}

#[test]
fn slot_means_averages_inside_each_slot() {
    let to = t0() + Duration::hours(4);
    let points = vec![
        (t0() + Duration::minutes(10), 10.0),
        (t0() + Duration::minutes(20), 20.0),
        (t0() + Duration::minutes(150), 40.0),
        (to, 80.0),
    ];
    let out = slot_means(&points, t0(), to, 4);
    assert_eq!(out, vec![Some(15.0), None, Some(40.0), Some(80.0)]);
}

#[test]
fn slot_means_ignores_invalid_and_out_of_range_points() {
    let to = t0() + Duration::hours(1);
    let points = vec![
        (t0() - Duration::minutes(1), 1.0),
        (t0() + Duration::minutes(5), f32::NAN),
        (t0() + Duration::minutes(5), f32::INFINITY),
        (to + Duration::minutes(1), 3.0),
    ];
    assert_eq!(slot_means(&points, t0(), to, 3), vec![None, None, None]);
}

#[test]
fn slot_means_handles_empty_input_and_degenerate_window() {
    assert_eq!(slot_means(&[], t0(), t0(), 5), vec![None; 5]);
    assert_eq!(slot_means(&[], t0(), t0() + Duration::hours(1), 0).len(), 1);
}

#[test]
fn empty_plot_has_no_data_status_and_full_length_series() {
    let o = build_plot_overview(meta(), &[], None, window(24, 24));
    assert!(o.latest.is_none());
    assert_eq!(o.health.status, HealthStatus::NoData);
    assert_eq!(o.series.moisture.len(), 24);
    assert!(o.series.ec.iter().all(Option::is_none));
}

#[test]
fn series_uses_only_usable_readings() {
    let w = window(1, 2);
    let readings = vec![
        reading(t0() + Duration::minutes(5), 50.0, QualityFlag::Good),
        reading(t0() + Duration::minutes(6), 999.0, QualityFlag::Bad),
        reading(t0() + Duration::minutes(40), 70.0, QualityFlag::Suspect),
    ];
    let o = build_plot_overview(meta(), &readings, None, w);
    assert_eq!(o.series.moisture, vec![Some(50.0), Some(70.0)]);
}

#[test]
fn latest_prefers_usable_over_newer_bad_reading() {
    let readings = vec![
        reading(t0() + Duration::minutes(5), 50.0, QualityFlag::Good),
        reading(t0() + Duration::minutes(9), 999.0, QualityFlag::Bad),
    ];
    let latest = pick_latest(&readings, None).unwrap();
    assert!((latest.moisture_pct - 50.0).abs() < 1e-4);

    let only_bad = vec![reading(t0(), 999.0, QualityFlag::Bad)];
    let fallback = reading(t0(), 1.0, QualityFlag::Bad);
    assert_eq!(
        pick_latest(&only_bad, Some(fallback.clone())),
        Some(fallback)
    );
}

#[test]
fn health_falls_back_to_latest_reading_when_window_is_empty() {
    let w = window(1, 4);
    let old = reading(w.to - Duration::hours(3), 50.0, QualityFlag::Good);
    let o = build_plot_overview(meta(), &[], Some(old), w);
    assert_eq!(o.health.status, HealthStatus::Offline);
    assert!(o.latest.is_some());
}
