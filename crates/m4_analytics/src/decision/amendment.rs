//! Anggota 3: Rekomendasi pembenah tanah (ameliorasi) berdasarkan deviasi pH.
//!
//! * pH di bawah batas minimum  -> kapur dolomit (CaCO₃/Mg), menaikkan pH.
//! * pH di atas batas maksimum  -> belerang elemental (S), menurunkan pH.
//!
//! Dosis = faktor penyangga × selisih pH ke titik tengah rentang optimal.
//!
//! CATATAN AGRONOMI: faktor dolomit mengikuti dokumen spesifikasi M4
//! (pasir 150, lempung 250, liat 350 g/m² per kenaikan 1.0 pH). Dokumen tidak memberi
//! angka untuk belerang, jadi faktor belerang di sini adalah nilai awal perkiraan
//! dan harus diverifikasi ke sumber agronomi. Tanah gambut punya daya sangga
//! berbeda dari tanah mineral; ganti faktor lewat `AmendmentConfig` bila sudah ada
//! angka khusus gambut.

use crate::error::{Result, SoilError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoilTexture {
    Sandy,
    Loamy,
    Clayey,
}

impl SoilTexture {
    /// Dolomit (g/m²) untuk menaikkan pH sebesar 1.0. Sumber: dokumen M4.
    pub fn lime_g_m2_per_ph(self) -> f32 {
        match self {
            SoilTexture::Sandy => 150.0,
            SoilTexture::Loamy => 250.0,
            SoilTexture::Clayey => 350.0,
        }
    }

    /// Belerang (g/m²) untuk menurunkan pH sebesar 1.0. Nilai awal perkiraan, perlu verifikasi.
    pub fn sulfur_g_m2_per_ph(self) -> f32 {
        match self {
            SoilTexture::Sandy => 30.0,
            SoilTexture::Loamy => 50.0,
            SoilTexture::Clayey => 70.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AmendmentConfig {
    pub min_ph: f32,
    pub max_ph: f32,
    pub lime_g_m2_per_ph: f32,
    pub sulfur_g_m2_per_ph: f32,
}

impl AmendmentConfig {
    pub fn for_texture(texture: SoilTexture, min_ph: f32, max_ph: f32) -> Self {
        Self {
            min_ph,
            max_ph,
            lime_g_m2_per_ph: texture.lime_g_m2_per_ph(),
            sulfur_g_m2_per_ph: texture.sulfur_g_m2_per_ph(),
        }
    }

    /// Titik pH sasaran = tengah rentang optimal.
    pub fn target_ph(&self) -> f32 {
        (self.min_ph + self.max_ph) / 2.0
    }
}

impl Default for AmendmentConfig {
    /// Contoh dari dokumen M4: rentang 6.0-6.8 (target 6.4), tekstur lempung.
    fn default() -> Self {
        Self::for_texture(SoilTexture::Loamy, 6.0, 6.8)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AmendmentAction {
    ApplyAgriculturalLime {
        grams_per_m2: f32,
        target_ph: f32,
        rationale: String,
    },
    ApplyElementalSulfur {
        grams_per_m2: f32,
        target_ph: f32,
        rationale: String,
    },
}

impl AmendmentAction {
    pub fn grams_per_m2(&self) -> f32 {
        match self {
            Self::ApplyAgriculturalLime { grams_per_m2, .. }
            | Self::ApplyElementalSulfur { grams_per_m2, .. } => *grams_per_m2,
        }
    }

    pub fn rationale(&self) -> &str {
        match self {
            Self::ApplyAgriculturalLime { rationale, .. }
            | Self::ApplyElementalSulfur { rationale, .. } => rationale,
        }
    }
}

fn validate(current_ph: f32, cfg: &AmendmentConfig) -> Result<()> {
    if !current_ph.is_finite() || !(0.0..=14.0).contains(&current_ph) {
        return Err(SoilError::InvalidPhysicalRange(format!(
            "pH {current_ph} di luar 0-14"
        )));
    }
    let range_ok = cfg.min_ph.is_finite()
        && cfg.max_ph.is_finite()
        && cfg.min_ph >= 0.0
        && cfg.max_ph <= 14.0
        && cfg.min_ph < cfg.max_ph;
    if !range_ok {
        return Err(SoilError::ComputationError(
            "rentang pH optimal tidak valid (harus 0 <= min < max <= 14)".into(),
        ));
    }
    if cfg.lime_g_m2_per_ph <= 0.0 || cfg.sulfur_g_m2_per_ph <= 0.0 {
        return Err(SoilError::ComputationError(
            "faktor penyangga harus > 0".into(),
        ));
    }
    Ok(())
}

/// Rekomendasi pembenah tanah dari pH terukur.
///
/// * `Ok(None)` jika pH berada di dalam rentang optimal.
pub fn recommend_soil_amendment(
    current_ph: f32,
    cfg: &AmendmentConfig,
) -> Result<Option<AmendmentAction>> {
    validate(current_ph, cfg)?;
    let target = cfg.target_ph();

    if current_ph < cfg.min_ph {
        let grams = (target - current_ph) * cfg.lime_g_m2_per_ph;
        return Ok(Some(AmendmentAction::ApplyAgriculturalLime {
            grams_per_m2: grams,
            target_ph: target,
            rationale: format!(
                "pH tanah ({:.1}) terlalu masam (batas minimum {:.1}). Berikan kapur dolomit sebesar {:.1} g/m².",
                current_ph, cfg.min_ph, grams
            ),
        }));
    }

    if current_ph > cfg.max_ph {
        let grams = (current_ph - target) * cfg.sulfur_g_m2_per_ph;
        return Ok(Some(AmendmentAction::ApplyElementalSulfur {
            grams_per_m2: grams,
            target_ph: target,
            rationale: format!(
                "pH tanah ({:.1}) terlalu basa (batas maksimum {:.1}). Berikan belerang elemental sebesar {:.1} g/m².",
                current_ph, cfg.max_ph, grams
            ),
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn matches_spec_example_275_g_lime() {
        // Dokumen M4: pH 5.3, lempung, target 6.4 => 275 g/m².
        match recommend_soil_amendment(5.3, &AmendmentConfig::default())
            .unwrap()
            .unwrap()
        {
            AmendmentAction::ApplyAgriculturalLime {
                grams_per_m2,
                target_ph,
                ..
            } => {
                assert!(close(grams_per_m2, 275.0));
                assert!(close(target_ph, 6.4));
            }
            other => panic!("harusnya kapur, dapat {other:?}"),
        }
    }

    #[test]
    fn lime_dose_scales_with_texture() {
        let dose = |t| {
            let cfg = AmendmentConfig::for_texture(t, 6.0, 7.0); // target 6.5
            recommend_soil_amendment(5.5, &cfg)
                .unwrap()
                .unwrap()
                .grams_per_m2()
        };
        assert!(close(dose(SoilTexture::Sandy), 150.0));
        assert!(close(dose(SoilTexture::Loamy), 250.0));
        assert!(close(dose(SoilTexture::Clayey), 350.0));
    }

    #[test]
    fn alkaline_soil_gets_sulfur() {
        let cfg = AmendmentConfig::default(); // target 6.4, belerang 50 g/m² per pH
        match recommend_soil_amendment(7.4, &cfg).unwrap().unwrap() {
            AmendmentAction::ApplyElementalSulfur { grams_per_m2, .. } => {
                assert!(close(grams_per_m2, 50.0));
            }
            other => panic!("harusnya belerang, dapat {other:?}"),
        }
    }

    #[test]
    fn no_action_inside_optimal_range() {
        let cfg = AmendmentConfig::default();
        assert_eq!(recommend_soil_amendment(6.0, &cfg).unwrap(), None);
        assert_eq!(recommend_soil_amendment(6.4, &cfg).unwrap(), None);
        assert_eq!(recommend_soil_amendment(6.8, &cfg).unwrap(), None);
    }

    #[test]
    fn dose_grows_with_deviation() {
        let cfg = AmendmentConfig::default();
        let mild = recommend_soil_amendment(5.8, &cfg).unwrap().unwrap();
        let severe = recommend_soil_amendment(4.0, &cfg).unwrap().unwrap();
        assert!(severe.grams_per_m2() > mild.grams_per_m2());
    }

    #[test]
    fn rejects_physically_invalid_ph() {
        let cfg = AmendmentConfig::default();
        assert!(recommend_soil_amendment(16.0, &cfg).is_err());
        assert!(recommend_soil_amendment(-0.5, &cfg).is_err());
        assert!(recommend_soil_amendment(f32::NAN, &cfg).is_err());
    }

    #[test]
    fn rejects_invalid_range() {
        let cfg = AmendmentConfig {
            min_ph: 7.0,
            max_ph: 6.0,
            ..Default::default()
        };
        assert!(recommend_soil_amendment(5.0, &cfg).is_err());
    }
}
