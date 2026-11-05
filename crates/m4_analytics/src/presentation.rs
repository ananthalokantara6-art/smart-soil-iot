//! Anggota 3: Presentasi CLI/ASCII untuk hasil M4 dan DSS.
//!
//! Semua fungsi murni: menerima data hasil pemrosesan M4 (`AnalysisResult`,
//! `Prescription`, `ReadingWindow`) dan mengembalikan `String`. Tidak ada data
//! tertanam, sehingga tampilan selalu mengikuti hasil pipeline. Hanya karakter
//! ASCII (+ warna ANSI opsional) agar aman di terminal mana pun.

use std::fmt::Write;

use soil_contracts::{
    AnalysisResult, Parameter, ReadingWindow, RiskKind, RiskLevel, TrendDirection,
};

use crate::decision::{AmendmentAction, Prescription};

/// Semua yang dibutuhkan untuk menggambar satu layar dashboard.
pub struct DashboardView<'a> {
    pub window: &'a ReadingWindow,
    pub analysis: &'a AnalysisResult,
    pub prescription: &'a Prescription,
    /// Skor SQI 0-100 bila modul SQI sudah tersedia; `None` menampilkan placeholder.
    pub sqi: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub color: bool,
    pub chart_width: usize,
    pub chart_height: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            color: true,
            chart_width: 60,
            chart_height: 8,
        }
    }
}

const RULE: &str = "============================================================================";
const THIN: &str = "----------------------------------------------------------------------------";

/// Batang proporsi, mis. `[#####-----]`. `fraction` dipotong ke 0..=1.
pub fn bar(fraction: f32, width: usize) -> String {
    let f = if fraction.is_nan() {
        0.0
    } else {
        fraction.clamp(0.0, 1.0)
    };
    let filled = (f * width as f32).round() as usize;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(width - filled))
}

/// Grafik garis ASCII. `reference` (mis. target kelembapan) digambar sebagai garis titik-titik.
pub fn ascii_chart(values: &[f32], width: usize, height: usize, reference: Option<f32>) -> String {
    let finite: Vec<f32> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() || width == 0 || height < 2 {
        return "(tidak ada data untuk digambar)\n".to_string();
    }

    let cols = width.min(finite.len());
    let sampled: Vec<f32> = (0..cols)
        .map(|i| {
            let idx = if cols == 1 {
                0
            } else {
                i * (finite.len() - 1) / (cols - 1)
            };
            finite[idx]
        })
        .collect();

    let mut lo = sampled.iter().copied().fold(f32::INFINITY, f32::min);
    let mut hi = sampled.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    if let Some(r) = reference.filter(|r| r.is_finite()) {
        lo = lo.min(r);
        hi = hi.max(r);
    }
    if (hi - lo).abs() < 1e-6 {
        lo -= 1.0;
        hi += 1.0;
    }

    let row_of = |v: f32| -> usize {
        let r = ((v - lo) / (hi - lo) * (height - 1) as f32).round() as usize;
        (height - 1) - r.min(height - 1)
    };

    let mut grid = vec![vec![' '; cols]; height];
    if let Some(r) = reference.filter(|r| r.is_finite()) {
        let row = row_of(r);
        for cell in grid[row].iter_mut() {
            *cell = '.';
        }
    }
    for (x, v) in sampled.iter().enumerate() {
        grid[row_of(*v)][x] = '*';
    }

    let mut out = String::new();
    for (y, row) in grid.iter().enumerate() {
        let label = if y == 0 {
            format!("{hi:>7.1}")
        } else if y == height - 1 {
            format!("{lo:>7.1}")
        } else {
            " ".repeat(7)
        };
        let _ = writeln!(out, "{label} |{}", row.iter().collect::<String>());
    }
    let _ = writeln!(out, "{} +{}", " ".repeat(7), "-".repeat(cols));
    out
}

fn level_label(level: RiskLevel) -> &'static str {
    match level {
        RiskLevel::Low => "AMAN",
        RiskLevel::Medium => "WASPADA",
        RiskLevel::High => "TINGGI",
        RiskLevel::Critical => "KRITIS",
    }
}

fn level_color(level: RiskLevel) -> &'static str {
    match level {
        RiskLevel::Low => "32",
        RiskLevel::Medium => "33",
        RiskLevel::High => "31",
        RiskLevel::Critical => "1;31",
    }
}

fn paint(text: &str, code: &str, color: bool) -> String {
    if color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn risk_name(kind: RiskKind) -> &'static str {
    match kind {
        RiskKind::Waterlogging => "Genangan (waterlogging)",
        RiskKind::Dryness => "Kekeringan (dryness)",
        RiskKind::PhShift => "Pergeseran pH",
        RiskKind::EcAnomaly => "Anomali EC/salinitas",
    }
}

fn param_name(p: Parameter) -> &'static str {
    match p {
        Parameter::Moisture => "Kelembapan",
        Parameter::Temperature => "Suhu",
        Parameter::Ph => "pH",
        Parameter::Ec => "EC",
    }
}

fn param_unit(p: Parameter) -> &'static str {
    match p {
        Parameter::Moisture => "%",
        Parameter::Temperature => "C",
        Parameter::Ph => "",
        Parameter::Ec => "mS/cm",
    }
}

fn direction_name(d: TrendDirection) -> &'static str {
    match d {
        TrendDirection::Rising => "naik",
        TrendDirection::Falling => "turun",
        TrendDirection::Stable => "stabil",
    }
}

/// Gambar satu layar dashboard lengkap sebagai teks.
pub fn render_dashboard(view: &DashboardView<'_>, opts: &RenderOptions) -> String {
    let (a, p) = (view.analysis, view.prescription);
    let mut o = String::new();

    let _ = writeln!(o, "{RULE}");
    let _ = writeln!(o, " SMART SOIL IoT - DASHBOARD ANALITIK & KEPUTUSAN (M4)");
    let _ = writeln!(o, "{RULE}");
    let _ = writeln!(o, " Petak    : {}", a.plot_id);
    let _ = writeln!(
        o,
        " Rentang  : {} s/d {}",
        a.window_from.format("%Y-%m-%d %H:%M"),
        a.window_to.format("%Y-%m-%d %H:%M")
    );
    let _ = writeln!(
        o,
        " Sampel   : {} dipakai analitik dari {} pembacaan",
        a.sample_count,
        view.window.readings.len()
    );
    let _ = writeln!(
        o,
        " Terakhir : kelembapan {:.1} %   |   pH rata-rata {:.2}",
        p.current_moisture_pct, p.mean_ph
    );
    match view.sqi {
        Some(s) => {
            let _ = writeln!(o, " SQI      : {:>5.1}/100 {}", s, bar(s / 100.0, 20));
        }
        None => {
            let _ = writeln!(o, " SQI      : belum tersedia (menunggu modul SQI)");
        }
    }

    let _ = writeln!(
        o,
        "\n{THIN}\n KURVA KELEMBAPAN (%)   . = target irigasi\n{THIN}"
    );
    let moisture: Vec<f32> = view
        .window
        .series(Parameter::Moisture)
        .iter()
        .map(|(_, v)| *v)
        .collect();
    let target = p
        .irrigation
        .as_ref()
        .map(|r| p.current_moisture_pct + r.moisture_deficit_pct);
    o.push_str(&ascii_chart(
        &moisture,
        opts.chart_width,
        opts.chart_height,
        target,
    ));

    let _ = writeln!(o, "\n{THIN}\n SKOR RISIKO\n{THIN}");
    for r in &a.risks {
        let tag = paint(
            &format!("{:<8}", level_label(r.level)),
            level_color(r.level),
            opts.color,
        );
        let _ = writeln!(
            o,
            " {:<24} {} {:.2} {} (kepercayaan {:.2})",
            risk_name(r.kind),
            bar(r.score, 20),
            r.score,
            tag,
            r.confidence.value()
        );
    }

    let _ = writeln!(o, "\n{THIN}\n TREN\n{THIN}");
    if a.trends.is_empty() {
        let _ = writeln!(o, " (data belum cukup untuk menghitung tren)");
    }
    for t in &a.trends {
        let _ = writeln!(
            o,
            " {:<11}: {} ({:+.2} {}/jam)",
            param_name(t.parameter),
            direction_name(t.direction),
            t.slope_per_hour,
            param_unit(t.parameter)
        );
    }

    let _ = writeln!(
        o,
        "\n{THIN}\n PERINGATAN / ANOMALI ({})\n{THIN}",
        a.anomalies.len()
    );
    if a.anomalies.is_empty() {
        let _ = writeln!(o, " Tidak ada anomali terdeteksi.");
    }
    for an in a.anomalies.iter().take(5) {
        let tag = paint(
            &format!("[{}]", level_label(an.severity)),
            level_color(an.severity),
            opts.color,
        );
        let _ = writeln!(o, " {} {} {}", tag, an.ts.format("%H:%M"), an.description);
    }
    if a.anomalies.len() > 5 {
        let _ = writeln!(o, " ... dan {} anomali lain", a.anomalies.len() - 5);
    }

    let _ = writeln!(o, "\n{THIN}\n REKOMENDASI TINDAKAN\n{THIN}");
    match &p.irrigation {
        Some(r) => {
            let _ = writeln!(
                o,
                " [IRIGASI]    {:.1} L/m2 selama {} menit",
                r.water_volume_liters_per_m2, r.recommended_duration_minutes
            );
            let _ = writeln!(o, "              {}", r.reason);
        }
        None => {
            let _ = writeln!(
                o,
                " [IRIGASI]    Tidak perlu: kelembapan sudah mencapai target."
            );
        }
    }
    match &p.amendment {
        Some(act) => {
            let name = match act {
                AmendmentAction::ApplyAgriculturalLime { .. } => "kapur dolomit",
                AmendmentAction::ApplyElementalSulfur { .. } => "belerang elemental",
            };
            let _ = writeln!(o, " [AMELIORASI] {} {:.0} g/m2", name, act.grams_per_m2());
            let _ = writeln!(o, "              {}", act.rationale());
        }
        None => {
            let _ = writeln!(o, " [AMELIORASI] Tidak perlu: pH dalam rentang optimal.");
        }
    }

    let _ = writeln!(o, "\n{THIN}");
    let _ = writeln!(
        o,
        " Sumber: {} v{} | metode: {}",
        a.provenance.source, a.provenance.version, a.provenance.method
    );
    let _ = writeln!(o, "{RULE}");
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{prescribe, DecisionConfig};
    use crate::pipeline::process_soil_pipeline;
    use chrono::{Duration, Utc};
    use soil_contracts::{DeviceId, PlotId, QualityFlag, SoilReading};

    fn window(moisture: f32, ph: f32) -> ReadingWindow {
        let (dev, plot) = (DeviceId::new(), PlotId::new());
        let t0 = Utc::now();
        ReadingWindow {
            plot_id: plot,
            from: t0,
            to: t0 + Duration::minutes(30),
            readings: (0..30)
                .map(|i| SoilReading {
                    device_id: dev,
                    plot_id: plot,
                    ts: t0 + Duration::minutes(i),
                    moisture_pct: moisture,
                    temperature_c: 27.0,
                    ph,
                    ec_ms_cm: 0.5,
                    quality: QualityFlag::Good,
                })
                .collect(),
        }
    }

    fn render(moisture: f32, ph: f32, sqi: Option<f32>, color: bool) -> String {
        let w = window(moisture, ph);
        let a = process_soil_pipeline(&w).unwrap();
        let p = prescribe(&w, &DecisionConfig::default()).unwrap();
        render_dashboard(
            &DashboardView {
                window: &w,
                analysis: &a,
                prescription: &p,
                sqi,
            },
            &RenderOptions {
                color,
                ..Default::default()
            },
        )
    }

    #[test]
    fn bar_has_fixed_width_and_clamps() {
        assert_eq!(bar(0.5, 10), "[#####-----]");
        assert_eq!(bar(-3.0, 4), "[----]");
        assert_eq!(bar(9.0, 4), "[####]");
        assert_eq!(bar(f32::NAN, 4), "[----]");
    }

    #[test]
    fn chart_handles_empty_constant_and_single_point() {
        assert!(ascii_chart(&[], 40, 6, None).contains("tidak ada data"));
        let flat = ascii_chart(&[50.0; 20], 40, 6, None);
        assert_eq!(flat.matches('*').count(), 20);
        let one = ascii_chart(&[42.0], 40, 6, None);
        assert_eq!(one.matches('*').count(), 1);
    }

    #[test]
    fn chart_puts_peak_on_top_row_and_valley_on_bottom() {
        let out = ascii_chart(&[0.0, 10.0, 0.0], 3, 5, None);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].contains('*')); // puncak
        assert!(lines[4].contains('*')); // lembah
    }

    #[test]
    fn chart_draws_reference_line() {
        let out = ascii_chart(&[30.0, 31.0, 32.0], 3, 6, Some(70.0));
        assert!(out.contains("..."));
    }

    #[test]
    fn dry_soil_dashboard_shows_irrigation_and_data_from_pipeline() {
        let s = render(38.0, 6.4, None, false);
        assert!(s.contains("80.0 L/m2 selama 192 menit"));
        assert!(s.contains("belum tersedia"));
        assert!(s.contains("m4.threshold_risk"));
        assert!(!s.contains('\x1b'));
    }

    #[test]
    fn acidic_soil_dashboard_shows_lime() {
        let s = render(75.0, 5.3, Some(64.0), false);
        assert!(s.contains("kapur dolomit 275 g/m2"));
        assert!(s.contains("64.0/100"));
        assert!(s.contains("Tidak perlu: kelembapan"));
    }

    #[test]
    fn color_mode_emits_ansi() {
        let s = render(95.0, 6.4, None, true);
        assert!(s.contains('\x1b'));
    }
}
