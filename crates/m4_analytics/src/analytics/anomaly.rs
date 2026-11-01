//! Anggota 4: Deteksi anomali agroekologi dan validasi fisik sensor.

use soil_contracts::{Anomaly, Parameter, ReadingWindow, RiskKind, RiskLevel, RiskScore, Confidence};

/// Validasi rentang nilai fisik pembacaan sensor.
pub fn is_physically_valid(moisture: f32, ph: f32, ec: f32) -> bool {
    (0.0..=100.0).contains(&moisture) && (0.0..=14.0).contains(&ph) && ec >= 0.0
}

/// Deteksi anomali pada jendela data sensor.
pub fn detect_anomalies(window: &ReadingWindow) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();

    // Contoh: periksa lonjakan pH atau nilai ekstrem
    for r in window.readings.iter().filter(|r| r.is_usable()) {
        if r.ph < 3.0 {
            anomalies.push(Anomaly {
                ts: r.ts,
                parameter: Parameter::Ph,
                description: format!("Acid shock terdeteksi: pH sangat rendah ({:.2})", r.ph),
                severity: RiskLevel::Critical,
            });
        }
        if r.ec_ms_cm > 4.0 {
            anomalies.push(Anomaly {
                ts: r.ts,
                parameter: Parameter::Ec,
                description: format!("Stres salinitas terdeteksi: EC tinggi ({:.2} mS/cm)", r.ec_ms_cm),
                severity: RiskLevel::High,
            });
        }
    }

    anomalies
}

/// Evaluasi skor risiko genangan air dan kekeringan.
pub fn evaluate_risks(mean_moisture: f32, confidence: Confidence) -> Vec<RiskScore> {
    let wet_score = ((mean_moisture - 75.0) / 20.0).clamp(0.0, 1.0);
    let dry_score = (-(mean_moisture - 45.0) / 25.0).clamp(0.0, 1.0);

    let level = |s: f32| match s {
        s if s >= 0.85 => RiskLevel::Critical,
        s if s >= 0.6 => RiskLevel::High,
        s if s >= 0.3 => RiskLevel::Medium,
        _ => RiskLevel::Low,
    };

    vec![
        RiskScore {
            kind: RiskKind::Waterlogging,
            score: wet_score,
            level: level(wet_score),
            confidence,
        },
        RiskScore {
            kind: RiskKind::Dryness,
            score: dry_score,
            level: level(dry_score),
            confidence,
        },
    ]
}