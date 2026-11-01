//! Contoh eksekusi demonstrasi mandiri M4 (Soil Analytics)
//! Menggunakan mock generator data sensor lahan gambut untuk simulasi Progress 2.

use chrono::{Duration, Utc};
use m4_analytics::ThresholdAnalyzer;
use soil_contracts::mock::{FakeSensorGenerator, InMemoryReadingStore};
use soil_contracts::{Analyzer, DeviceId, PlotId, ReadingSink, ReadingSource};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================================");
    println!("     DEMO MODUL 4: SOIL ANALYTICS & DECISION SUPPORT    ");
    println!("========================================================");

    let store = InMemoryReadingStore::new();
    let dev = DeviceId::new();
    let plot = PlotId::new();
    let mut gen = FakeSensorGenerator::new(2026);
    let t0 = Utc::now() - Duration::hours(5);

    println!("\n[1] Menghasilkan 30 sampel telemetri sensor gambut (Mock M3)...");
    for i in 0..30 {
        let reading = gen.reading(dev, plot, t0 + Duration::minutes(i * 10));
        store.ingest(reading).await?;
    }

    let window = store.window(&plot, t0, Utc::now()).await?;
    println!(
        "    Berhasil memuat {} pembacaan tervalidasi.",
        window.readings.len()
    );

    println!("\n[2] Mengeksekusi Pipeline Analitik Fungsional M4...");
    let analyzer = ThresholdAnalyzer;
    let result = analyzer.analyze(&window).await?;

    println!("\n[3] Hasil Evaluasi Mutu & Risiko Tanah:");
    println!("    Plot ID       : {}", result.plot_id);
    println!(
        "    Rentang Waktu : {} s/d {}",
        result.window_from, result.window_to
    );
    println!("    Jumlah Sampel : {}", result.sample_count);

    println!("\n--- Skor Risiko Lingkungan ---");
    for r in &result.risks {
        println!(
            "  * {:?}: Skor = {:.2}, Tingkat = {:?}, Kepercayaan = {:.2}",
            r.kind,
            r.score,
            r.level,
            r.confidence.value()
        );
    }

    println!("\n--- Evaluasi Tren Deret Waktu ---");
    for t in &result.trends {
        println!(
            "  * Parameter {:?}: Arah = {:?}, Kemiringan/jam = {:.4}",
            t.parameter, t.direction, t.slope_per_hour
        );
    }

    println!("\n--- Deteksi Anomali ---");
    if result.anomalies.is_empty() {
        println!("  * Tidak ada anomali terdeteksi (kondisi tanah normal).");
    } else {
        for a in &result.anomalies {
            println!("  * [{:?}] {}: {}", a.severity, a.ts, a.description);
        }
    }

    println!("\n--- Provenance Keputusan (Audibilitas FP) ---");
    println!(
        "  * Sumber Modul : {} (v{})",
        result.provenance.source, result.provenance.version
    );
    println!("  * Metode       : {}", result.provenance.method);
    for inp in &result.provenance.inputs {
        println!("    - {}", inp);
    }

    println!("\n[OK] Demonstrasi simulasi M4 selesai dengan sukses!");
    Ok(())
}
