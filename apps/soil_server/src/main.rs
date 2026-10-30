//! Demo pipeline end-to-end dengan data sensor palsu.
//! Jalankan: `cargo run -p soil_server`

use chrono::{Duration, Utc};
use soil_contracts::mock::FakeSensorGenerator;
use soil_contracts::{DeviceId, GrowthStage, PlantId, PlotId, RawReading, UserId};
use soil_server::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, owner, plot) = (DeviceId::new(), UserId::new(), PlotId::new());
    let app = App::new(device, owner, plot)?;

    let mut generator = FakeSensorGenerator::new(2026);
    let start = Utc::now() - Duration::minutes(30);
    for i in 0..30 {
        let r = generator.reading(device, plot, start + Duration::minutes(i));
        app.ingest(RawReading {
            device_id: device,
            ts: r.ts,
            moisture_raw: r.moisture_pct,
            temperature_raw: r.temperature_c,
            ph_raw: r.ph,
            ec_raw: r.ec_ms_cm,
        })
        .await?;
    }

    let (analysis, rec) = app
        .assess(
            &PlantId::new("nanas"),
            GrowthStage::Vegetative,
            start,
            Utc::now(),
        )
        .await?;

    println!("Sampel dianalisis : {}", analysis.sample_count);
    for risk in &analysis.risks {
        println!(
            "Risiko {:?}: skor {:.2} ({:?}), confidence {:.2}",
            risk.kind,
            risk.score,
            risk.level,
            risk.confidence.value()
        );
    }
    println!("Rekomendasi       : {} ({:?})", rec.action, rec.priority);
    for line in &rec.explanation {
        println!("  - {line}");
    }
    println!(
        "Provenance        : {} v{} / {}",
        rec.provenance.source, rec.provenance.version, rec.provenance.method
    );
    println!(
        "Peringatan        : {}",
        app.platform.alerts_for(&plot)?.len()
    );
    Ok(())
}
