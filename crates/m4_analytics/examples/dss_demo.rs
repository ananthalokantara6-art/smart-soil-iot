//! Demo CLI Progress 2: dashboard ASCII + rekomendasi DSS (Anggota 3).
//!
//! Interaktif:   cargo run -p m4_analytics --example dss_demo
//! Satu skenario: cargo run -p m4_analytics --example dss_demo -- 2
//! Tanpa warna:   cargo run -p m4_analytics --example dss_demo -- 2 --no-color
//!
//! Data tiap skenario dibangun sebagai `ReadingWindow`, lalu DILEWATKAN ke pipeline M4
//! (`process_soil_pipeline`) dan DSS (`prescribe`). Layar menampilkan hasil
//! perhitungan itu, bukan teks yang ditulis tangan.

use std::io::{self, BufRead, Write};

use chrono::{Duration, Utc};
use m4_analytics::decision::{
    prescribe, AmendmentConfig, DecisionConfig, IrrigationConfig, SoilTexture,
};
use m4_analytics::pipeline::process_soil_pipeline;
use m4_analytics::presentation::{render_dashboard, DashboardView, RenderOptions};
use soil_contracts::mock::FakeSensorGenerator;
use soil_contracts::{DeviceId, PlotId, QualityFlag, ReadingWindow, SoilReading};

/// ASUMSI DEMO untuk lahan gambut. Ganti dengan angka dari rekomendasi agronomis
/// komoditas yang dipakai. Faktor dolomit memakai nilai tekstur lempung dari dokumen M4.
fn peat_demo_config() -> DecisionConfig {
    DecisionConfig {
        irrigation: IrrigationConfig {
            target_moisture_pct: 70.0,
            root_depth_cm: 25.0,
            emitter_flow_lph_per_m2: 25.0,
        },
        amendment: AmendmentConfig::for_texture(SoilTexture::Loamy, 4.5, 5.5),
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    Normal,
    Dry,
    Waterlogged,
    Acidic,
    Random,
}

impl Scenario {
    fn from_key(k: &str) -> Option<Self> {
        match k {
            "1" => Some(Self::Normal),
            "2" => Some(Self::Dry),
            "3" => Some(Self::Waterlogged),
            "4" => Some(Self::Acidic),
            "5" => Some(Self::Random),
            _ => None,
        }
    }
}

/// Bangun 36 sampel (tiap 10 menit): kelembapan dan pH bergerak linear dari `start` ke `end`
/// dengan goyangan kecil deterministik supaya grafik terlihat seperti data sensor.
fn trajectory(m: (f32, f32), ph: (f32, f32)) -> ReadingWindow {
    const N: usize = 36;
    let (dev, plot) = (DeviceId::new(), PlotId::new());
    let t0 = Utc::now() - Duration::minutes(10 * N as i64);
    let lerp = |a: f32, b: f32, i: usize| a + (b - a) * i as f32 / (N - 1) as f32;
    let readings: Vec<SoilReading> = (0..N)
        .map(|i| {
            let wobble = ((i * 37 % 11) as f32 - 5.0) * 0.25;
            SoilReading {
                device_id: dev,
                plot_id: plot,
                ts: t0 + Duration::minutes(10 * i as i64),
                moisture_pct: lerp(m.0, m.1, i) + wobble,
                temperature_c: 27.0,
                ph: lerp(ph.0, ph.1, i) + wobble * 0.02,
                ec_ms_cm: 0.5,
                quality: QualityFlag::Good,
            }
        })
        .collect();
    ReadingWindow {
        plot_id: plot,
        from: t0,
        to: t0 + Duration::minutes(10 * N as i64),
        readings,
    }
}

fn random_window(seed: u64) -> ReadingWindow {
    let (dev, plot) = (DeviceId::new(), PlotId::new());
    let mut gen = FakeSensorGenerator::new(seed);
    let t0 = Utc::now() - Duration::hours(6);
    let readings: Vec<SoilReading> = (0..36)
        .map(|i| gen.reading(dev, plot, t0 + Duration::minutes(10 * i)))
        .collect();
    ReadingWindow {
        plot_id: plot,
        from: t0,
        to: Utc::now(),
        readings,
    }
}

fn build_window(s: Scenario, seed: u64) -> (&'static str, ReadingWindow) {
    match s {
        Scenario::Normal => ("Kondisi normal", trajectory((72.0, 73.0), (5.0, 5.0))),
        Scenario::Dry => ("Tanah mengering", trajectory((46.0, 18.0), (4.9, 4.9))),
        Scenario::Waterlogged => ("Genangan air", trajectory((72.0, 94.0), (5.0, 4.9))),
        Scenario::Acidic => (
            "Tanah gambut sangat masam",
            trajectory((74.0, 72.0), (4.2, 3.5)),
        ),
        Scenario::Random => (
            "Data acak sensor (FakeSensorGenerator)",
            random_window(seed),
        ),
    }
}

fn show(s: Scenario, seed: u64, opts: &RenderOptions) {
    let (title, window) = build_window(s, seed);
    println!("\n>>> Skenario: {title}\n");
    let analysis = match process_soil_pipeline(&window) {
        Ok(a) => a,
        Err(e) => return println!("Pipeline M4 gagal: {e}"),
    };
    let prescription = match prescribe(&window, &peat_demo_config()) {
        Ok(p) => p,
        Err(e) => return println!("DSS gagal: {e}"),
    };
    let view = DashboardView {
        window: &window,
        analysis: &analysis,
        prescription: &prescription,
        sqi: None, // diisi setelah modul SQI (Anggota 2) tersedia
    };
    print!("{}", render_dashboard(&view, opts));
}

fn print_menu() {
    println!("\n=== MENU DEMO DSS (pilih skenario) ===");
    println!(" 1) Kondisi normal");
    println!(" 2) Tanah mengering   -> rekomendasi irigasi");
    println!(" 3) Genangan air      -> peringatan waterlogging");
    println!(" 4) Gambut masam      -> rekomendasi kapur dolomit");
    println!(" 5) Data acak sensor  (angka berbeda tiap dipilih)");
    println!(" q) Keluar");
    print!("Pilihan: ");
    let _ = io::stdout().flush();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = RenderOptions {
        color: !args.iter().any(|a| a == "--no-color"),
        ..Default::default()
    };

    // Mode sekali jalan: argumen angka skenario (berguna untuk tes dan tangkapan layar).
    if let Some(s) = args.iter().find_map(|a| Scenario::from_key(a)) {
        show(s, 2026, &opts);
        return;
    }

    let mut seed = 2026u64;
    let stdin = io::stdin();
    loop {
        print_menu();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let key = line.trim();
        if key.eq_ignore_ascii_case("q") {
            break;
        }
        match Scenario::from_key(key) {
            Some(s) => {
                show(s, seed, &opts);
                seed += 1;
            }
            None => println!("Pilihan tidak dikenal: '{key}'"),
        }
    }
    println!("Selesai.");
}
