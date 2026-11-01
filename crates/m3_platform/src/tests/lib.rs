use super::*;
use chrono::Duration;
use soil_contracts::{AlertLevel, DeviceId, Provenance, QualityFlag};

fn reading(plot: PlotId, ts: DateTime<Utc>) -> SoilReading {
    SoilReading {
        device_id: DeviceId::new(),
        plot_id: plot,
        ts,
        moisture_pct: 60.0,
        temperature_c: 27.0,
        ph: 4.0,
        ec_ms_cm: 0.5,
        quality: QualityFlag::Good,
    }
}

#[tokio::test]
async fn ingest_then_query_window() {
    let p = MemoryPlatform::new();
    let plot = PlotId::new();
    let t0 = Utc::now();
    for i in 0..6 {
        p.ingest(reading(plot, t0 + Duration::minutes(i)))
            .await
            .unwrap();
    }
    p.ingest(reading(PlotId::new(), t0)).await.unwrap();

    let w = p
        .window(&plot, t0, t0 + Duration::minutes(2))
        .await
        .unwrap();
    assert_eq!(w.readings.len(), 3);
    let latest = p.latest(&plot).await.unwrap().unwrap();
    assert_eq!(latest.ts, t0 + Duration::minutes(5));
}

#[tokio::test]
async fn duplicate_reading_is_not_stored_twice() {
    let p = MemoryPlatform::new();
    let plot = PlotId::new();
    let r = reading(plot, Utc::now() - Duration::minutes(1));
    assert!(p.ingest_checked(r.clone()).unwrap());
    assert!(!p.ingest_checked(r.clone()).unwrap());
    // lewat trait pun tidak error dan tidak menggandakan data
    p.ingest(r).await.unwrap();
    let w = p
        .window(&plot, Utc::now() - Duration::hours(1), Utc::now())
        .await
        .unwrap();
    assert_eq!(w.readings.len(), 1);
}

#[test]
fn invalid_readings_are_rejected_before_storage() {
    let p = MemoryPlatform::new();
    let plot = PlotId::new();

    let mut nan = reading(plot, Utc::now());
    nan.ph = f32::NAN;
    assert!(matches!(
        p.ingest_checked(nan),
        Err(ContractError::Validation(_))
    ));

    let future = reading(plot, Utc::now() + Duration::days(365));
    assert!(matches!(
        p.ingest_checked(future),
        Err(ContractError::Validation(_))
    ));
}

#[tokio::test]
async fn empty_platform_returns_empty_results() {
    let p = MemoryPlatform::new();
    let plot = PlotId::new();
    assert!(p.latest(&plot).await.unwrap().is_none());
    let w = p
        .window(&plot, Utc::now() - Duration::hours(1), Utc::now())
        .await
        .unwrap();
    assert!(w.readings.is_empty());
    assert!(p.list_plots().unwrap().is_empty());
    assert!(p.all_alerts().unwrap().is_empty());
}

#[tokio::test]
async fn window_is_sorted_and_bounds_are_inclusive() {
    let p = MemoryPlatform::new();
    let plot = PlotId::new();
    let t0 = Utc::now() - Duration::hours(1);
    for i in [3, 1, 2, 0] {
        p.ingest(reading(plot, t0 + Duration::minutes(i)))
            .await
            .unwrap();
    }
    let w = p
        .window(&plot, t0, t0 + Duration::minutes(3))
        .await
        .unwrap();
    let stamps: Vec<_> = w.readings.iter().map(|r| r.ts).collect();
    let mut sorted = stamps.clone();
    sorted.sort();
    assert_eq!(stamps, sorted);
    assert_eq!(stamps.len(), 4);
}

#[test]
fn plots_are_listed_by_name() {
    use crate::plots::NewPlot;
    let p = MemoryPlatform::new();
    for name in ["Petak B", "Petak A"] {
        let meta = PlotMeta::from_new(NewPlot {
            name: name.into(),
            latitude: None,
            longitude: None,
            area_ha: None,
            plant: None,
        })
        .unwrap();
        p.register_plot(meta).unwrap();
    }
    let names: Vec<String> = p
        .list_plots()
        .unwrap()
        .into_iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(names, vec!["Petak A", "Petak B"]);
}

#[tokio::test]
async fn alerts_are_stored() {
    let p = MemoryPlatform::new();
    let plot = PlotId::new();
    p.raise(Alert {
        plot_id: plot,
        ts: Utc::now(),
        level: AlertLevel::Warning,
        message: "uji".into(),
        provenance: Provenance {
            source: "test".into(),
            version: "0".into(),
            method: "manual".into(),
            inputs: vec![],
            generated_at: Utc::now(),
        },
    })
    .await
    .unwrap();
    assert_eq!(p.alerts_for(&plot).unwrap().len(), 1);
    assert!(p.alerts_for(&PlotId::new()).unwrap().is_empty());
}
