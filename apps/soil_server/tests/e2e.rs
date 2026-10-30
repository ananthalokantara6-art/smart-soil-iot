//! E2E-01: sensor -> M2/M1 -> M3 -> M4 -> M5, dengan M4 dan M5 dipasang lewat M6.

use chrono::{Duration, Utc};
use soil_contracts::{
    AlertLevel, ContractError, DeviceId, GrowthStage, PlantId, PlotId, Priority, RawReading, UserId,
};
use soil_server::App;

fn raw(device: DeviceId, ts: chrono::DateTime<Utc>, moisture: f32) -> RawReading {
    RawReading {
        device_id: device,
        ts,
        moisture_raw: moisture,
        temperature_raw: 27.0,
        ph_raw: 4.0,
        ec_raw: 0.5,
    }
}

async fn feed(app: &App, device: DeviceId, moisture: f32, count: i64) -> chrono::DateTime<Utc> {
    let start = Utc::now() - Duration::minutes(count);
    for i in 0..count {
        app.ingest(raw(device, start + Duration::minutes(i), moisture))
            .await
            .unwrap();
    }
    start
}

#[tokio::test]
async fn waterlogged_plot_gets_drainage_recommendation_and_alert() {
    let (device, plot) = (DeviceId::new(), PlotId::new());
    let app = App::new(device, UserId::new(), plot).unwrap();
    let start = feed(&app, device, 93.0, 30).await;

    let (analysis, rec) = app
        .assess(
            &PlantId::new("nanas"),
            GrowthStage::Vegetative,
            start,
            Utc::now(),
        )
        .await
        .unwrap();

    assert_eq!(analysis.sample_count, 30);
    assert!(rec.action.contains("drainase"));
    assert!(rec.priority >= Priority::Normal);
    assert!(!rec.fired_rules.is_empty());
    assert_eq!(rec.provenance.source, "m5.basic_rules");
    assert_eq!(analysis.provenance.source, "m4.threshold_risk");

    let alerts = app.platform.alerts_for(&plot).unwrap();
    assert!(!alerts.is_empty());
    assert!(alerts.iter().any(|a| a.level >= AlertLevel::Warning));
}

#[tokio::test]
async fn healthy_plot_has_no_alert() {
    let (device, plot) = (DeviceId::new(), PlotId::new());
    let app = App::new(device, UserId::new(), plot).unwrap();
    let start = feed(&app, device, 60.0, 30).await;

    let (_, rec) = app
        .assess(
            &PlantId::new("nanas"),
            GrowthStage::Fruiting,
            start,
            Utc::now(),
        )
        .await
        .unwrap();

    assert_eq!(rec.priority, Priority::Low);
    assert!(app.platform.alerts_for(&plot).unwrap().is_empty());
}

#[tokio::test]
async fn unknown_device_is_rejected_end_to_end() {
    let (device, plot) = (DeviceId::new(), PlotId::new());
    let app = App::new(device, UserId::new(), plot).unwrap();

    let err = app
        .ingest(raw(DeviceId::new(), Utc::now(), 60.0))
        .await
        .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[tokio::test]
async fn disabling_the_analyzer_plugin_stops_assessment() {
    let (device, plot) = (DeviceId::new(), PlotId::new());
    let mut app = App::new(device, UserId::new(), plot).unwrap();
    let start = feed(&app, device, 60.0, 5).await;

    app.registry
        .set_enabled("m4.threshold_risk", false)
        .unwrap();
    let err = app
        .assess(
            &PlantId::new("nanas"),
            GrowthStage::Vegetative,
            start,
            Utc::now(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ContractError::Plugin(_)));
}
