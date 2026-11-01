use super::*;
use chrono::Duration;
use soil_contracts::DeviceId;

fn t0() -> DateTime<Utc> {
    Utc.timestamp_opt(1_800_000_000, 0).single().unwrap()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn reading(ts: DateTime<Utc>, quality: QualityFlag) -> SoilReading {
    SoilReading {
        device_id: DeviceId::new(),
        plot_id: PlotId::new(),
        ts,
        moisture_pct: 60.0,
        temperature_c: 27.0,
        ph: 4.0,
        ec_ms_cm: 0.5,
        quality,
    }
}

fn bucket(mean: f32) -> Bucket {
    Bucket {
        start: t0(),
        count: 1,
        min: mean,
        max: mean,
        mean,
    }
}

#[test]
fn bucketize_groups_by_interval() {
    let series = vec![
        (t0(), 10.0),
        (t0() + Duration::seconds(30), 20.0),
        (t0() + Duration::seconds(70), 40.0),
    ];
    let out = bucketize(&series, 60);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].count, 2);
    assert!(close(out[0].mean, 15.0));
    assert!(close(out[0].min, 10.0) && close(out[0].max, 20.0));
    assert!(close(out[1].mean, 40.0));
}

#[test]
fn bucketize_handles_empty_and_invalid_values() {
    assert!(bucketize(&[], 60).is_empty());
    let bad = vec![(t0(), f32::NAN), (t0(), f32::INFINITY)];
    assert!(bucketize(&bad, 60).is_empty());
    // interval tidak valid dipaksa menjadi 1 detik, tidak panik
    assert_eq!(bucketize(&[(t0(), 1.0)], 0).len(), 1);
}

#[test]
fn auto_interval_has_floor_of_one_minute() {
    assert_eq!(auto_interval_secs(-5), 60);
    assert_eq!(auto_interval_secs(0), 60);
    assert_eq!(auto_interval_secs(86_400), 720);
}

#[test]
fn moving_average_smooths_and_keeps_length() {
    assert!(moving_average(&[], 3).is_empty());
    let out = moving_average(&[1.0, 2.0, 3.0, 4.0], 2);
    assert_eq!(out.len(), 4);
    assert!(close(out[0], 1.0) && close(out[1], 1.5));
    assert!(close(out[2], 2.5) && close(out[3], 3.5));
    // window 0 diperlakukan sebagai 1 (tidak mengubah nilai)
    assert_eq!(moving_average(&[5.0, 7.0], 0), vec![5.0, 7.0]);
}

#[test]
fn period_change_detects_direction() {
    assert!(period_change(&[]).is_none());

    let stable = period_change(&[bucket(100.0), bucket(101.0)]).unwrap();
    assert_eq!(stable.direction, TrendDirection::Stable);

    let rising = period_change(&[bucket(100.0), bucket(110.0)]).unwrap();
    assert_eq!(rising.direction, TrendDirection::Rising);
    assert!(close(rising.percent.unwrap(), 10.0));

    let falling = period_change(&[bucket(100.0), bucket(50.0)]).unwrap();
    assert_eq!(falling.direction, TrendDirection::Falling);

    let from_zero = period_change(&[bucket(0.0), bucket(5.0)]).unwrap();
    assert_eq!(from_zero.direction, TrendDirection::Rising);
    assert!(from_zero.percent.is_none());
}

#[test]
fn health_no_data() {
    let h = sensor_health(&[], t0(), 600);
    assert_eq!(h.status, HealthStatus::NoData);
    assert!(h.last_seen.is_none());
    assert!(close(h.score, 0.0));
}

#[test]
fn health_status_follows_age_of_last_reading() {
    let at = |age_secs: i64| {
        let r = reading(t0() - Duration::seconds(age_secs), QualityFlag::Good);
        sensor_health(&[r], t0(), 600)
    };
    let online = at(300);
    assert_eq!(online.status, HealthStatus::Online);
    assert!(close(online.score, 1.0));

    let stale = at(1_500);
    assert_eq!(stale.status, HealthStatus::Stale);
    assert!(close(stale.score, 0.5));

    let offline = at(4_000);
    assert_eq!(offline.status, HealthStatus::Offline);
    assert!(close(offline.score, 0.0));
}

#[test]
fn health_counts_gaps_and_unusable_readings() {
    let readings = vec![
        reading(t0() - Duration::seconds(3_600), QualityFlag::Good),
        reading(t0() - Duration::seconds(300), QualityFlag::Good),
        reading(t0() - Duration::seconds(200), QualityFlag::Bad),
    ];
    let h = sensor_health(&readings, t0(), 600);
    assert_eq!(h.gap_count, 1);
    assert_eq!(h.sample_count, 3);
    assert!(close(h.unusable_ratio, 1.0 / 3.0));
    assert_eq!(h.status, HealthStatus::Online);
}

#[test]
fn health_does_not_depend_on_input_order() {
    let mut readings = vec![
        reading(t0() - Duration::seconds(100), QualityFlag::Good),
        reading(t0() - Duration::seconds(5_000), QualityFlag::Good),
    ];
    let a = sensor_health(&readings, t0(), 600);
    readings.reverse();
    let b = sensor_health(&readings, t0(), 600);
    assert_eq!(a, b);
}

#[test]
fn summarize_ignores_unusable_readings() {
    let plot = PlotId::new();
    let mut a = reading(t0(), QualityFlag::Good);
    a.moisture_pct = 50.0;
    let mut b = reading(t0(), QualityFlag::Suspect);
    b.moisture_pct = 70.0;
    let mut bad = reading(t0(), QualityFlag::Bad);
    bad.moisture_pct = 99.0;

    let s = summarize(plot, &[a, b, bad.clone()], Parameter::Moisture).unwrap();
    assert_eq!(s.count, 2);
    assert!(close(s.min, 50.0) && close(s.max, 70.0));
    assert!(close(s.mean, 60.0) && close(s.std_dev, 10.0));

    assert!(summarize(plot, &[bad], Parameter::Moisture).is_none());
    assert!(summarize(plot, &[], Parameter::Ph).is_none());
}

#[test]
fn rank_by_mean_sorts_descending() {
    let mk = |mean: f32| ParamSummary {
        plot_id: PlotId::new(),
        parameter: Parameter::Moisture,
        count: 1,
        min: mean,
        max: mean,
        mean,
        std_dev: 0.0,
    };
    let ranked = rank_by_mean(vec![mk(1.0), mk(3.0), mk(2.0)]);
    let means: Vec<f32> = ranked.iter().map(|s| s.mean).collect();
    assert_eq!(means, vec![3.0, 2.0, 1.0]);
}

#[test]
fn validate_reading_checks_ranges() {
    let ok = reading(t0(), QualityFlag::Good);
    assert!(validate_reading(&ok).is_ok());

    let mut r = ok.clone();
    r.ph = f32::NAN;
    assert!(validate_reading(&r).is_err());

    let mut r = ok.clone();
    r.moisture_pct = 101.0;
    assert!(validate_reading(&r).is_err());

    let mut r = ok;
    r.temperature_c = -30.0;
    assert!(validate_reading(&r).is_err());
}

#[test]
fn validate_integrity_rejects_nan_and_future_timestamps() {
    let ok = reading(t0(), QualityFlag::Good);
    assert!(validate_integrity(&ok, t0()).is_ok());

    let near_future = reading(t0() + Duration::minutes(1), QualityFlag::Good);
    assert!(validate_integrity(&near_future, t0()).is_ok());

    let far_future = reading(t0() + Duration::minutes(10), QualityFlag::Good);
    assert!(validate_integrity(&far_future, t0()).is_err());

    let mut nan = ok;
    nan.ec_ms_cm = f32::NAN;
    assert!(validate_integrity(&nan, t0()).is_err());
}