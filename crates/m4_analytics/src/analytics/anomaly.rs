//! Anggota 4: Deteksi anomali agroekologi dan validasi fisik sensor.

use chrono::{DateTime, Duration, Utc};

use soil_contracts::{
    Anomaly, Confidence, Parameter, ReadingWindow, RiskKind, RiskLevel, RiskScore,
};

type Series = Vec<(DateTime<Utc>, f32)>;

// ---------- validasi domain fisik ----------

/// Validasi rentang nilai fisik: kelembapan 0-100%, pH 0-14, EC non-negatif.
/// NaN dan infinity ditolak.
pub fn is_physically_valid(moisture: f32, ph: f32, ec: f32) -> bool {
    (0.0..=100.0).contains(&moisture) && (0.0..=14.0).contains(&ph) && ec.is_finite() && ec >= 0.0
}

// ---------- konfigurasi ambang (bisa diubah, bukan angka tertanam) ----------

#[derive(Debug, Clone)]
pub struct WaterloggingConfig {
    pub moisture_threshold_pct: f32,
    pub min_duration: Duration,
}

impl Default for WaterloggingConfig {
    fn default() -> Self {
        Self {
            moisture_threshold_pct: 85.0,
            min_duration: Duration::minutes(30),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AcidShockConfig {
    /// Penurunan pH minimum antar dua sampel berurutan.
    pub min_drop: f32,
    /// Jarak waktu maksimum antar dua sampel agar dianggap "mendadak".
    pub max_interval: Duration,
}

impl Default for AcidShockConfig {
    fn default() -> Self {
        Self {
            min_drop: 0.8,
            max_interval: Duration::minutes(60),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SalinityConfig {
    pub ec_threshold_ms_cm: f32,
    pub min_consecutive: usize,
}

impl Default for SalinityConfig {
    fn default() -> Self {
        Self {
            ec_threshold_ms_cm: 2.0,
            min_consecutive: 2,
        }
    }
}

// ---------- helper ----------

/// Deret satu parameter: hanya data usable, nilai valid, terurut waktu.
fn series_of(window: &ReadingWindow, p: Parameter) -> Series {
    let mut s: Series = window
        .series(p)
        .into_iter()
        .filter(|(_, v)| v.is_finite())
        .collect();
    s.sort_by_key(|(ts, _)| *ts);
    s
}

#[derive(Debug, Clone, Copy)]
struct Run {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    count: usize,
    peak: f32,
}

/// Cari rentetan sampel berurutan yang memenuhi syarat.
fn find_runs(series: &Series, pred: impl Fn(f32) -> bool) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut cur: Option<Run> = None;
    for &(ts, v) in series {
        if pred(v) {
            cur = Some(match cur {
                Some(r) => Run {
                    end: ts,
                    count: r.count + 1,
                    peak: r.peak.max(v),
                    ..r
                },
                None => Run {
                    start: ts,
                    end: ts,
                    count: 1,
                    peak: v,
                },
            });
        } else if let Some(r) = cur.take() {
            runs.push(r);
        }
    }
    if let Some(r) = cur {
        runs.push(r);
    }
    runs
}

// ---------- tiga detektor ----------

/// Kelembapan tinggi kontinu selama minimal `min_duration`.
pub fn detect_waterlogging(window: &ReadingWindow, cfg: &WaterloggingConfig) -> Vec<Anomaly> {
    let series = series_of(window, Parameter::Moisture);
    find_runs(&series, |v| v >= cfg.moisture_threshold_pct)
        .into_iter()
        .filter(|r| r.end - r.start >= cfg.min_duration)
        .map(|r| {
            let dur = r.end - r.start;
            let severity = if dur >= cfg.min_duration * 2 {
                RiskLevel::High
            } else {
                RiskLevel::Medium
            };
            Anomaly {
                ts: r.end,
                parameter: Parameter::Moisture,
                description: format!(
                    "Genangan: kelembapan >= {:.0}% selama {} menit (puncak {:.1}%)",
                    cfg.moisture_threshold_pct,
                    dur.num_minutes(),
                    r.peak
                ),
                severity,
            }
        })
        .collect()
}

/// Lonjakan keasaman mendadak: pH turun tajam antar dua sampel berdekatan.
pub fn detect_acid_shock(window: &ReadingWindow, cfg: &AcidShockConfig) -> Vec<Anomaly> {
    let series = series_of(window, Parameter::Ph);
    series
        .windows(2)
        .filter_map(|pair| {
            let (t0, p0) = pair[0];
            let (t1, p1) = pair[1];
            let drop = p0 - p1;
            if drop >= cfg.min_drop && t1 - t0 <= cfg.max_interval {
                let severity = if drop >= cfg.min_drop * 2.0 {
                    RiskLevel::Critical
                } else {
                    RiskLevel::High
                };
                Some(Anomaly {
                    ts: t1,
                    parameter: Parameter::Ph,
                    description: format!(
                        "Acid shock: pH turun {drop:.2} (dari {p0:.2} ke {p1:.2})"
                    ),
                    severity,
                })
            } else {
                None
            }
        })
        .collect()
}

/// EC di atas ambang toleransi selama minimal `min_consecutive` sampel berturut-turut.
pub fn detect_salinity_stress(window: &ReadingWindow, cfg: &SalinityConfig) -> Vec<Anomaly> {
    let series = series_of(window, Parameter::Ec);
    find_runs(&series, |v| v > cfg.ec_threshold_ms_cm)
        .into_iter()
        .filter(|r| r.count >= cfg.min_consecutive)
        .map(|r| {
            let severity = if r.peak >= cfg.ec_threshold_ms_cm * 1.5 {
                RiskLevel::High
            } else {
                RiskLevel::Medium
            };
            Anomaly {
                ts: r.end,
                parameter: Parameter::Ec,
                description: format!(
                    "Stres salinitas: EC > {:.1} mS/cm pada {} sampel berturut-turut (puncak {:.2})",
                    cfg.ec_threshold_ms_cm, r.count, r.peak
                ),
                severity,
            }
        })
        .collect()
}

/// Deteksi semua anomali pada jendela data (dipakai oleh pipeline).
pub fn detect_anomalies(window: &ReadingWindow) -> Vec<Anomaly> {
    let mut all = Vec::new();
    all.extend(detect_waterlogging(window, &WaterloggingConfig::default()));
    all.extend(detect_acid_shock(window, &AcidShockConfig::default()));
    all.extend(detect_salinity_stress(window, &SalinityConfig::default()));
    all.sort_by_key(|a| a.ts);
    all
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use soil_contracts::{DeviceId, PlotId, QualityFlag, SoilReading};

    /// Setiap tuple = (kelembapan, pH, EC), satu sampel per 10 menit.
    fn window(vals: &[(f32, f32, f32)]) -> ReadingWindow {
        let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let (dev, plot) = (DeviceId::new(), PlotId::new());
        let readings = vals
            .iter()
            .enumerate()
            .map(|(i, &(m, ph, ec))| SoilReading {
                device_id: dev,
                plot_id: plot,
                ts: t0 + Duration::minutes(10 * i as i64),
                moisture_pct: m,
                temperature_c: 27.0,
                ph,
                ec_ms_cm: ec,
                quality: QualityFlag::Good,
            })
            .collect();
        ReadingWindow {
            plot_id: plot,
            from: t0,
            to: t0 + Duration::hours(2),
            readings,
        }
    }

    #[test]
    fn physical_validation_rejects_impossible_values() {
        assert!(is_physically_valid(50.0, 4.0, 0.5));
        assert!(!is_physically_valid(101.0, 4.0, 0.5));
        assert!(!is_physically_valid(-1.0, 4.0, 0.5));
        assert!(!is_physically_valid(50.0, 15.0, 0.5));
        assert!(!is_physically_valid(50.0, 4.0, -0.1));
        assert!(!is_physically_valid(f32::NAN, 4.0, 0.5));
        assert!(!is_physically_valid(50.0, 4.0, f32::INFINITY));
    }

    #[test]
    fn sustained_high_moisture_is_waterlogging() {
        let w = window(&[(90.0, 4.0, 0.5); 6]); // 50 menit
        assert_eq!(
            detect_waterlogging(&w, &WaterloggingConfig::default()).len(),
            1
        );
    }

    #[test]
    fn short_moisture_spike_is_not_waterlogging() {
        let mut v = vec![(60.0, 4.0, 0.5); 6];
        v[2] = (95.0, 4.0, 0.5);
        assert!(detect_waterlogging(&window(&v), &WaterloggingConfig::default()).is_empty());
    }

    #[test]
    fn sudden_ph_drop_is_acid_shock() {
        let w = window(&[(60.0, 4.0, 0.5), (60.0, 4.0, 0.5), (60.0, 2.9, 0.5)]);
        let found = detect_acid_shock(&w, &AcidShockConfig::default());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].parameter, Parameter::Ph);
    }

    #[test]
    fn stable_ph_is_not_acid_shock() {
        let w = window(&[(60.0, 4.0, 0.5), (60.0, 3.9, 0.5), (60.0, 4.1, 0.5)]);
        assert!(detect_acid_shock(&w, &AcidShockConfig::default()).is_empty());
    }

    #[test]
    fn high_ec_is_salinity_stress() {
        let w = window(&[(60.0, 4.0, 0.5), (60.0, 4.0, 3.0), (60.0, 4.0, 3.2)]);
        assert_eq!(
            detect_salinity_stress(&w, &SalinityConfig::default()).len(),
            1
        );
    }

    #[test]
    fn single_ec_spike_is_not_salinity_stress() {
        let w = window(&[(60.0, 4.0, 0.5), (60.0, 4.0, 3.0), (60.0, 4.0, 0.5)]);
        assert!(detect_salinity_stress(&w, &SalinityConfig::default()).is_empty());
    }
}
