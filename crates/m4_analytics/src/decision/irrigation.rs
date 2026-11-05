//! Anggota 3: Sistem Pendukung Keputusan (DSS) Irigasi.
//!
//! Defisit air dihitung dari selisih kelembapan terhadap target (kapasitas lapang)
//! dikali kedalaman zona perakaran:
//!
//! ```text
//! defisit (L/m²) = (target% − aktual%) / 100 × kedalaman_perakaran (mm)
//! ```
//!
//! (1 mm air di atas 1 m² = 1 liter, sehingga hasilnya langsung dalam L/m².)

use crate::error::{Result, SoilError};

/// Parameter irigasi. Semua nilai bisa diganti per petak/komoditas (bukan angka tertanam).
#[derive(Debug, Clone, PartialEq)]
pub struct IrrigationConfig {
    /// Kelembapan tujuan (kapasitas lapang), % VWC.
    pub target_moisture_pct: f32,
    /// Kedalaman zona perakaran, cm.
    pub root_depth_cm: f32,
    /// Debit emitter per luas petak, L/jam per m².
    pub emitter_flow_lph_per_m2: f32,
}

impl Default for IrrigationConfig {
    /// Nilai contoh dari dokumen spesifikasi M4 (target 70%, akar 25 cm, emitter 25 L/jam/m²).
    fn default() -> Self {
        Self {
            target_moisture_pct: 70.0,
            root_depth_cm: 25.0,
            emitter_flow_lph_per_m2: 25.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrrigationRecommendation {
    pub water_volume_liters_per_m2: f32,
    pub recommended_duration_minutes: u32,
    pub moisture_deficit_pct: f32,
    pub reason: String,
}

/// Defisit air (L/m²) menuju kelembapan target. Bernilai 0 jika tanah sudah cukup basah.
pub fn calculate_water_deficit(
    current_moisture_pct: f32,
    target_moisture_pct: f32,
    root_depth_cm: f32,
) -> f32 {
    let deficit_pct = (target_moisture_pct - current_moisture_pct).max(0.0);
    // urutan perkalian dijaga agar hasil bulat (mis. 32 × 250 / 100 = 80) tetap presisi di f32
    deficit_pct * (root_depth_cm * 10.0) / 100.0
}

fn validate(current_moisture_pct: f32, cfg: &IrrigationConfig) -> Result<()> {
    if !current_moisture_pct.is_finite() || !(0.0..=100.0).contains(&current_moisture_pct) {
        return Err(SoilError::InvalidPhysicalRange(format!(
            "kelembapan {current_moisture_pct} di luar 0-100%"
        )));
    }
    if !cfg.target_moisture_pct.is_finite()
        || cfg.target_moisture_pct <= 0.0
        || cfg.target_moisture_pct > 100.0
    {
        return Err(SoilError::InvalidPhysicalRange(format!(
            "target kelembapan {} di luar (0, 100]%",
            cfg.target_moisture_pct
        )));
    }
    if !cfg.root_depth_cm.is_finite() || cfg.root_depth_cm <= 0.0 {
        return Err(SoilError::ComputationError(
            "kedalaman perakaran harus > 0 cm".into(),
        ));
    }
    if !cfg.emitter_flow_lph_per_m2.is_finite() || cfg.emitter_flow_lph_per_m2 <= 0.0 {
        return Err(SoilError::ComputationError(
            "debit emitter harus > 0 L/jam/m²".into(),
        ));
    }
    Ok(())
}

/// Hitung rekomendasi irigasi.
///
/// * `Ok(None)`  : kelembapan sudah mencapai target, tidak perlu menyiram.
/// * `Ok(Some)`  : volume (L/m²) dan durasi (menit, dibulatkan ke atas).
/// * `Err(_)`    : masukan di luar batas fisik atau konfigurasi tidak valid.
pub fn calculate_irrigation(
    current_moisture_pct: f32,
    cfg: &IrrigationConfig,
) -> Result<Option<IrrigationRecommendation>> {
    validate(current_moisture_pct, cfg)?;

    if current_moisture_pct >= cfg.target_moisture_pct {
        return Ok(None);
    }

    let deficit_pct = cfg.target_moisture_pct - current_moisture_pct;
    let volume = calculate_water_deficit(
        current_moisture_pct,
        cfg.target_moisture_pct,
        cfg.root_depth_cm,
    );

    // volume × 60 / debit dihitung dulu baru dibulatkan; toleransi kecil menahan
    // noise floating-point agar 192.0000x tidak menjadi 193 menit.
    let minutes = volume * 60.0 / cfg.emitter_flow_lph_per_m2;
    let duration = (minutes - 1e-3).ceil().max(1.0) as u32;

    Ok(Some(IrrigationRecommendation {
        water_volume_liters_per_m2: volume,
        recommended_duration_minutes: duration,
        moisture_deficit_pct: deficit_pct,
        reason: format!(
            "Kelembapan ({:.1}%) di bawah target ({:.1}%). Defisit {:.1}% setara {:.1} L/m² pada akar {:.0} cm.",
            current_moisture_pct, cfg.target_moisture_pct, deficit_pct, volume, cfg.root_depth_cm
        ),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn matches_spec_example_80_liters_192_minutes() {
        // Contoh di dokumen M4: kelembapan 38% -> target 70% = defisit 32%.
        let r = calculate_irrigation(38.0, &IrrigationConfig::default())
            .unwrap()
            .unwrap();
        assert!(close(r.water_volume_liters_per_m2, 80.0));
        assert_eq!(r.recommended_duration_minutes, 192);
        assert!(close(r.moisture_deficit_pct, 32.0));
    }

    #[test]
    fn no_irrigation_when_at_or_above_target() {
        let cfg = IrrigationConfig::default();
        assert_eq!(calculate_irrigation(70.0, &cfg).unwrap(), None);
        assert_eq!(calculate_irrigation(85.0, &cfg).unwrap(), None);
    }

    #[test]
    fn deeper_roots_need_more_water() {
        let shallow = IrrigationConfig {
            root_depth_cm: 10.0,
            ..Default::default()
        };
        let deep = IrrigationConfig {
            root_depth_cm: 40.0,
            ..Default::default()
        };
        let a = calculate_irrigation(50.0, &shallow).unwrap().unwrap();
        let b = calculate_irrigation(50.0, &deep).unwrap().unwrap();
        assert!(close(a.water_volume_liters_per_m2, 20.0));
        assert!(close(b.water_volume_liters_per_m2, 80.0));
    }

    #[test]
    fn duration_rounds_up() {
        // 1% defisit pada akar 25 cm = 2.5 L/m²; emitter 25 L/jam => 6 menit tepat.
        // 1.1% => 2.75 L/m² => 6.6 menit => 7.
        let cfg = IrrigationConfig::default();
        let exact = calculate_irrigation(69.0, &cfg).unwrap().unwrap();
        assert_eq!(exact.recommended_duration_minutes, 6);
        let up = calculate_irrigation(68.9, &cfg).unwrap().unwrap();
        assert_eq!(up.recommended_duration_minutes, 7);
    }

    #[test]
    fn rejects_physically_invalid_input() {
        let cfg = IrrigationConfig::default();
        assert!(calculate_irrigation(-1.0, &cfg).is_err());
        assert!(calculate_irrigation(101.0, &cfg).is_err());
        assert!(calculate_irrigation(f32::NAN, &cfg).is_err());
    }

    #[test]
    fn rejects_invalid_config() {
        let no_flow = IrrigationConfig {
            emitter_flow_lph_per_m2: 0.0,
            ..Default::default()
        };
        assert!(calculate_irrigation(40.0, &no_flow).is_err());
        let no_root = IrrigationConfig {
            root_depth_cm: 0.0,
            ..Default::default()
        };
        assert!(calculate_irrigation(40.0, &no_root).is_err());
    }

    #[test]
    fn water_deficit_is_zero_when_wet_enough() {
        assert_eq!(calculate_water_deficit(80.0, 70.0, 25.0), 0.0);
    }
}
