//! Modul Edge Processing: Validasi Sensor Tanah Gambut, Deteksi Anomali, & Kontrol Interval Sampling.
//!
//! Seluruh fungsi di dalam modul ini adalah **100% Pure Functions (Fungsi Murni)**:
//! - Tidak memiliki efek samping (*no side-effects*, tidak mengubah variabel global/eksternal).
//! - Bersifat *immutable* dan deterministik (input sama selalu menghasilkan output sama).
//! - Menerapkan *Higher-Order Functions* (`filter_map`, `all`, `any`, serta fungsi yang menerima validator)
//!   dan *Function Composition* (`Result::and_then`).
//!
//! Spesifikasi sensor mengacu pada **RAB Modul 1**:
//! 1. **Kelembapan (`moisture_pct`)**: DFRobot Gravity Waterproof Capacitive Soil Moisture + ADC ADS1115 16-bit.
//! 2. **Suhu Tanah (`temperature_c`)**: DS18B20 Waterproof Temperature Probe.
//! 3. **pH Tanah (`ph`)**: Sensor pH Tanah Analog + DMS (Data Measurement System).
//! 4. **Konduktivitas (`ec_ms_cm`)**: DFRobot Gravity Analog TDS/EC Sensor V1.0.
//! 5. **Alarm Fisik**: Buzzer Putih Active 5V (menyala jika terdeteksi sensor putus / nilai di luar batas fisik).

use crate::contracts::{PlotMeta, QualityFlag, RawReading, SoilReading};
use serde::{Deserialize, Serialize};

/// Kategori status hasil pemeriksaan satu parameter sensor di sisi Edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorStatus {
    /// Nilai berada dalam rentang normal lahan gambut.
    Normal,
    /// Nilai berada di luar kondisi ideal gambut namun masih dalam batas fisik sensor (`Suspect`).
    Warning,
    /// Nilai di luar batas fisik sensor atau indikasi kabel sensor terputus (`Bad` -> aktifkan Buzzer).
    CriticalAnomaly,
}

/// Fungsi murni: Mengecek apakah interval waktu sampling sudah terpenuhi.
///
/// Digunakan untuk menghemat daya baterai Li-Ion 18650 (3x) dan kuota modem SIM7000E.
pub fn check_interval(current_time: u64, last_time: u64, interval: u64) -> bool {
    current_time.saturating_sub(last_time) >= interval
}

/// Fungsi murni: Memeriksa kelembapan tanah (`moisture_pct` dalam % VWC).
///
/// - Batas fisik DFRobot Capacitive Sensor: `0.0% ..= 100.0%`
/// - Rentang normal lahan gambut: `30.0% ..= 90.0%`
pub fn check_moisture(moisture_pct: f32) -> &'static str {
    match classify_moisture(moisture_pct) {
        SensorStatus::Normal => "NORMAL",
        SensorStatus::Warning | SensorStatus::CriticalAnomaly => "ANOMALY",
    }
}

/// Fungsi murni: Klasifikasi detail kelembapan tanah.
pub fn classify_moisture(moisture_pct: f32) -> SensorStatus {
    if !moisture_pct.is_finite() || !(0.0..=100.0).contains(&moisture_pct) {
        SensorStatus::CriticalAnomaly
    } else if !(30.0..=90.0).contains(&moisture_pct) {
        SensorStatus::Warning
    } else {
        SensorStatus::Normal
    }
}

/// Fungsi murni: Memeriksa suhu tanah (`temperature_c` dalam °C).
///
/// - Batas fisik DS18B20 Waterproof Probe: `-10.0°C ..= 80.0°C`
/// - Rentang normal lahan gambut tropis: `18.0°C ..= 40.0°C`
pub fn check_temperature(temperature_c: f32) -> &'static str {
    match classify_temperature(temperature_c) {
        SensorStatus::Normal => "NORMAL",
        SensorStatus::Warning | SensorStatus::CriticalAnomaly => "ANOMALY",
    }
}

/// Fungsi murni: Klasifikasi detail suhu tanah.
pub fn classify_temperature(temperature_c: f32) -> SensorStatus {
    if !temperature_c.is_finite() || !(-10.0..=80.0).contains(&temperature_c) {
        SensorStatus::CriticalAnomaly
    } else if !(18.0..=40.0).contains(&temperature_c) {
        SensorStatus::Warning
    } else {
        SensorStatus::Normal
    }
}

/// Fungsi murni: Memeriksa pH tanah (`ph` skala 0-14).
///
/// - Batas fisik Sensor pH Analog + DMS: `0.0 ..= 14.0`
/// - Rentang wajar lahan gambut (alami asam s/d pasca-kapur): `3.0 ..= 7.0`
pub fn check_ph(ph: f32) -> &'static str {
    match classify_ph(ph) {
        SensorStatus::Normal => "NORMAL",
        SensorStatus::Warning | SensorStatus::CriticalAnomaly => "ANOMALY",
    }
}

/// Fungsi murni: Klasifikasi detail pH tanah gambut.
pub fn classify_ph(ph: f32) -> SensorStatus {
    if !ph.is_finite() || !(0.0..=14.0).contains(&ph) {
        SensorStatus::CriticalAnomaly
    } else if !(3.0..=7.0).contains(&ph) {
        SensorStatus::Warning
    } else {
        SensorStatus::Normal
    }
}

/// Fungsi murni: Memeriksa konduktivitas listrik (`ec_ms_cm` dalam mS/cm).
///
/// - Batas fisik DFRobot Gravity TDS/EC Sensor: `0.0 ..= 20.0 mS/cm`
/// - Rentang wajar lahan gambut: `0.05 ..= 4.0 mS/cm`
pub fn check_ec(ec_ms_cm: f32) -> &'static str {
    match classify_ec(ec_ms_cm) {
        SensorStatus::Normal => "NORMAL",
        SensorStatus::Warning | SensorStatus::CriticalAnomaly => "ANOMALY",
    }
}

/// Fungsi murni: Klasifikasi detail EC tanah.
pub fn classify_ec(ec_ms_cm: f32) -> SensorStatus {
    if !ec_ms_cm.is_finite() || !(0.0..=20.0).contains(&ec_ms_cm) {
        SensorStatus::CriticalAnomaly
    } else if !(0.05..=4.0).contains(&ec_ms_cm) {
        SensorStatus::Warning
    } else {
        SensorStatus::Normal
    }
}

/// Tipe fungsi validator (digunakan untuk *Higher-Order Function* [`evaluate_with_rules`]).
pub type RuleFn = fn(&RawReading) -> Option<String>;

/// Higher-Order Function: Menerima daftar fungsi aturan (` &[RuleFn] `) dan mengeksekusinya
/// secara imutabel menggunakan `.iter().filter_map(...)` (tanpa variabel `mut Vec`).
pub fn evaluate_with_rules(raw: &RawReading, rules: &[RuleFn]) -> Vec<String> {
    rules.iter().filter_map(|rule_fn| rule_fn(raw)).collect()
}

/// Aturan 1: Cek anomali kelembapan tanah (DFRobot Capacitive Moisture).
pub fn rule_moisture(raw: &RawReading) -> Option<String> {
    match classify_moisture(raw.moisture_raw) {
        SensorStatus::Normal => None,
        SensorStatus::Warning => Some(format!(
            "Anomali Kelembapan Gambut: {:.1}% di luar rentang ideal (30-90%)",
            raw.moisture_raw
        )),
        SensorStatus::CriticalAnomaly => Some(format!(
            "KRITIS Sensor Moisture (DFRobot/ADS1115): nilai {:.1}% tidak valid / indikasi kabel putus!",
            raw.moisture_raw
        )),
    }
}

/// Aturan 2: Cek anomali suhu tanah (DS18B20 Waterproof Probe).
pub fn rule_temperature(raw: &RawReading) -> Option<String> {
    match classify_temperature(raw.temperature_raw) {
        SensorStatus::Normal => None,
        SensorStatus::Warning => Some(format!(
            "Anomali Suhu Tanah: {:.1}°C di luar rentang normal gambut (18-40°C)",
            raw.temperature_raw
        )),
        SensorStatus::CriticalAnomaly => Some(format!(
            "KRITIS Sensor Suhu (DS18B20): nilai {:.1}°C di luar batas fisik (-10..80°C)!",
            raw.temperature_raw
        )),
    }
}

/// Aturan 3: Cek anomali pH tanah (Sensor pH Analog + DMS).
pub fn rule_ph(raw: &RawReading) -> Option<String> {
    match classify_ph(raw.ph_raw) {
        SensorStatus::Normal => None,
        SensorStatus::Warning => Some(format!(
            "Anomali pH Gambut: {:.2} di luar rentang wajar (3.0-7.0)",
            raw.ph_raw
        )),
        SensorStatus::CriticalAnomaly => Some(format!(
            "KRITIS Sensor pH (+DMS): nilai {:.2} di luar skala pH (0-14)!",
            raw.ph_raw
        )),
    }
}

/// Aturan 4: Cek anomali konduktivitas listrik (DFRobot Gravity EC/TDS).
pub fn rule_ec(raw: &RawReading) -> Option<String> {
    match classify_ec(raw.ec_raw) {
        SensorStatus::Normal => None,
        SensorStatus::Warning => Some(format!(
            "Anomali EC Tanah: {:.2} mS/cm di luar rentang wajar gambut (0.05-4.0 mS/cm)",
            raw.ec_raw
        )),
        SensorStatus::CriticalAnomaly => Some(format!(
            "KRITIS Sensor EC (DFRobot): nilai {:.2} mS/cm di luar batas fisik (0-20 mS/cm)!",
            raw.ec_raw
        )),
    }
}

/// Daftar aturan standar untuk 4 parameter wajib tanah gambut.
pub const STANDARD_PEATLAND_RULES: [RuleFn; 4] = [
    rule_moisture,
    rule_temperature,
    rule_ph,
    rule_ec,
];

/// Fungsi murni: Memeriksa seluruh parameter sensor pada [`RawReading`] menggunakan
/// *Higher-Order Function* [`evaluate_with_rules`].
pub fn check_anomaly(raw: &RawReading) -> Vec<String> {
    evaluate_with_rules(raw, &STANDARD_PEATLAND_RULES)
}

/// Fungsi murni: Menentukan [`QualityFlag`] dari pembacaan mentah berdasarkan status 4 sensor.
pub fn determine_quality(raw: &RawReading) -> QualityFlag {
    let statuses = [
        classify_moisture(raw.moisture_raw),
        classify_temperature(raw.temperature_raw),
        classify_ph(raw.ph_raw),
        classify_ec(raw.ec_raw),
    ];

    if statuses.iter().any(|s| *s == SensorStatus::CriticalAnomaly) {
        QualityFlag::Bad
    } else if statuses.iter().any(|s| *s == SensorStatus::Warning) {
        QualityFlag::Suspect
    } else {
        QualityFlag::Good
    }
}

/// Fungsi murni: Menentukan apakah Buzzer Active 5V (RAB Modul 1 No. 9) harus berbunyi.
///
/// Buzzer berbunyi jika kualitas data `Bad` (indikasi sensor dicuri/terputus atau rusak).
pub fn should_trigger_buzzer(quality: QualityFlag) -> bool {
    matches!(quality, QualityFlag::Bad)
}

/// Fungsi murni: Validasi identitas dasar pada payload (`event_id` dan `device_id`).
pub fn validate_identity_fields(raw: &RawReading) -> Result<&RawReading, String> {
    if raw.event_id.trim().is_empty() {
        Err("event_id tidak boleh kosong".to_string())
    } else if raw.device_id.trim().is_empty() {
        Err("device_id tidak boleh kosong".to_string())
    } else {
        Ok(raw)
    }
}

/// Fungsi murni: Validasi angka numerik agar bukan `NaN` atau `Infinity`.
pub fn validate_finite_numbers(raw: &RawReading) -> Result<&RawReading, String> {
    let values = [
        raw.moisture_raw,
        raw.temperature_raw,
        raw.ph_raw,
        raw.ec_raw,
    ];
    if values.iter().all(|v| v.is_finite()) {
        Ok(raw)
    } else {
        Err("Nilai sensor mengandung NaN atau Infinity (kabel sensor terputus)".to_string())
    }
}

/// Komposisi Fungsi (*Function Composition* dengan `.and_then`):
/// Memvalidasi [`RawReading`] lalu mentransformasikannya menjadi [`SoilReading`] siap kirim ke Modul 3.
pub fn validate_and_transform(raw: &RawReading, plot: &PlotMeta) -> Result<SoilReading, String> {
    validate_identity_fields(raw)
        .and_then(validate_finite_numbers)
        .map(|valid_raw| SoilReading {
            device_id: valid_raw.device_id.clone(),
            plot_id: plot.plot_id.clone(),
            ts: valid_raw.ts.clone(),
            moisture_pct: valid_raw.moisture_raw,
            temperature_c: valid_raw.temperature_raw,
            ph: valid_raw.ph_raw,
            ec_ms_cm: valid_raw.ec_raw,
            quality: determine_quality(valid_raw),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_raw(moisture: f32, temp: f32, ph: f32, ec: f32) -> RawReading {
        RawReading {
            event_id: "evt-001".to_string(),
            device_id: "ESP32-GAMBUT-01".to_string(),
            ts: "2026-11-01T10:00:00Z".to_string(),
            timestamp_sec: 100,
            moisture_raw: moisture,
            temperature_raw: temp,
            ph_raw: ph,
            ec_raw: ec,
        }
    }

    fn sample_plot() -> PlotMeta {
        PlotMeta {
            plot_id: "PLOT-KALIMANTAN-01".to_string(),
            name: "Petak Nanas Gambut Blok A".to_string(),
            latitude: -2.2083,
            longitude: 113.9167,
            area_ha: 1.5,
            plant: "nanas".to_string(),
        }
    }

    #[test]
    fn test_all_peatland_sensors_normal() {
        let raw = sample_raw(68.5, 28.4, 4.5, 1.2);
        let anomalies = check_anomaly(&raw);
        assert!(anomalies.is_empty());
        assert_eq!(determine_quality(&raw), QualityFlag::Good);
        assert!(!should_trigger_buzzer(determine_quality(&raw)));
    }

    #[test]
    fn test_warning_creates_suspect_quality() {
        // Kelembapan 15% (terlalu kering untuk gambut, tapi masih dalam batas fisik 0-100%)
        let raw = sample_raw(15.0, 28.4, 4.5, 1.2);
        let anomalies = check_anomaly(&raw);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(determine_quality(&raw), QualityFlag::Suspect);
        assert!(!should_trigger_buzzer(determine_quality(&raw)));
    }

    #[test]
    fn test_critical_sensor_triggers_bad_quality_and_buzzer() {
        // Kelembapan 145% dan pH -2.0 -> di luar batas fisik sensor -> Buzzer aktif!
        let raw = sample_raw(145.0, 28.4, -2.0, 1.2);
        let reading = validate_and_transform(&raw, &sample_plot()).unwrap();
        assert_eq!(reading.quality, QualityFlag::Bad);
        assert!(should_trigger_buzzer(reading.quality));
    }

    #[test]
    fn test_interval_check_pure() {
        assert!(check_interval(300, 240, 60));
        assert!(!check_interval(280, 240, 60));
    }
}
