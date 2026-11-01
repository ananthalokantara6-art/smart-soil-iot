//Server demo M3 dengan data palsu, agar dashboard bisa dilihat tanpa M1/M2.

use std::sync::Arc;
use chrono::{Duration, Utc};
use m3_platform::plots::{NewPlot, PlotMeta};
use m3_platform::{api, MemoryPlatform};
use soil_contracts::mock::FakeSensorGenerator;
use soil_contracts::{Alert, AlertLevel, AlertSink, DeviceId, PlotId, Provenance, QualityFlag};

fn plot(name: &str, lat: f64, lon: f64, ha: f64, plant: &str) -> PlotMeta {
    PlotMeta::from_new(NewPlot {
        name: name.into(),
        latitude: Some(lat),
        longitude: Some(lon),
        area_ha: Some(ha),
        plant: Some(plant.into()),
    })
    .expect("data demo valid")
}

fn seed(
    platform: &MemoryPlatform,
    plot_id: PlotId,
    seed: u64,
    moisture_shift: f32,
    silent_minutes: i64,
) {
    let mut generator = FakeSensorGenerator::new(seed);
    let device = DeviceId::new();
    let now = Utc::now();
    for i in (silent_minutes / 10..=288).rev() {
        let ts = now - Duration::minutes(i * 10);
        let mut r = generator.reading(device, plot_id, ts);
        // Variasi harian sederhana supaya grafik tidak datar.
        r.moisture_pct += moisture_shift + 6.0 * ((i as f32) / 20.0).sin();
        if i > 0 && i % 40 == 0 {
            r.quality = QualityFlag::Suspect;
        }
        if i > 0 && i % 97 == 0 {
            r.quality = QualityFlag::Bad;
        }
        let _ = platform.ingest_checked(r);
    }
}

fn alert(plot_id: PlotId, level: AlertLevel, message: &str, minutes_ago: i64) -> Alert {
    let ts = Utc::now() - Duration::minutes(minutes_ago);
    Alert {
        plot_id,
        ts,
        level,
        message: message.into(),
        provenance: Provenance {
            source: "demo".into(),
            version: "0.1".into(),
            method: "data contoh".into(),
            inputs: vec![],
            generated_at: ts,
        },
    }
}

#[tokio::main]
async fn main() {
    let platform = Arc::new(MemoryPlatform::new());
    let a = plot("Petak A (Nanas)", -0.5021, 117.1536, 1.5, "nanas");
    let b = plot("Petak B (Padi)", -0.5068, 117.1612, 2.2, "padi");
    for (meta, seed_no, shift, silent) in [(&a, 1, 0.0, 0), (&b, 2, 22.0, 45)] {
        platform.register_plot(meta.clone()).expect("simpan petak");
        seed(&platform, meta.plot_id, seed_no, shift, silent);
    }
    for al in [
        alert(
            b.plot_id,
            AlertLevel::Critical,
            "Risiko waterlogging tinggi (kelembapan > 80%)",
            180,
        ),
        alert(
            b.plot_id,
            AlertLevel::Warning,
            "Perangkat tidak mengirim data lebih dari 30 menit",
            30,
        ),
        alert(a.plot_id, AlertLevel::Info, "Kalibrasi pH selesai", 600),
    ] {
        platform.raise(al).await.expect("simpan alert");
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("port 3000 dipakai program lain?");
    println!("Dashboard demo: http://127.0.0.1:3000");
    axum::serve(listener, api::router(platform))
        .await
        .expect("server berhenti");
}
