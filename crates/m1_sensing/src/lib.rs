//! M1 - Soil Sensor Acquisition and Validation.
//!
//! Implementasi awal: kalibrasi linear (offset + gain) dan pemeriksaan rentang fisik.
//! TODO kelompok 1: filter noise (median/EMA), deteksi outlier statistik, rekalibrasi otomatis.

use soil_contracts::{
    ContractError, PlotId, QualityFlag, RawReading, ReadingValidator, Result, SoilReading,
};

/// Kalibrasi linear: `nilai = raw * gain + offset`.
#[derive(Debug, Clone, Copy)]
pub struct Linear {
    pub gain: f32,
    pub offset: f32,
}

impl Linear {
    pub const IDENTITY: Linear = Linear {
        gain: 1.0,
        offset: 0.0,
    };

    fn apply(self, raw: f32) -> f32 {
        raw * self.gain + self.offset
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    pub moisture: Linear,
    pub temperature: Linear,
    pub ph: Linear,
    pub ec: Linear,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            moisture: Linear::IDENTITY,
            temperature: Linear::IDENTITY,
            ph: Linear::IDENTITY,
            ec: Linear::IDENTITY,
        }
    }
}

/// Validator berbasis kalibrasi dan rentang fisik sensor.
#[derive(Debug, Clone, Default)]
pub struct RangeValidator {
    pub calibration: Calibration,
}

impl RangeValidator {
    pub fn new(calibration: Calibration) -> Self {
        Self { calibration }
    }
}

impl ReadingValidator for RangeValidator {
    fn validate(&self, raw: &RawReading, plot_id: PlotId) -> Result<SoilReading> {
        let values = [
            raw.moisture_raw,
            raw.temperature_raw,
            raw.ph_raw,
            raw.ec_raw,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(ContractError::Validation(
                "nilai sensor NaN atau tak hingga".into(),
            ));
        }

        let c = &self.calibration;
        let moisture = c.moisture.apply(raw.moisture_raw);
        let temperature = c.temperature.apply(raw.temperature_raw);
        let ph = c.ph.apply(raw.ph_raw);
        let ec = c.ec.apply(raw.ec_raw);

        let in_range = (0.0..=100.0).contains(&moisture)
            && (-10.0..=80.0).contains(&temperature)
            && (0.0..=14.0).contains(&ph)
            && (0.0..=20.0).contains(&ec);

        Ok(SoilReading {
            device_id: raw.device_id,
            plot_id,
            ts: raw.ts,
            moisture_pct: moisture,
            temperature_c: temperature,
            ph,
            ec_ms_cm: ec,
            quality: if in_range {
                QualityFlag::Good
            } else {
                QualityFlag::Bad
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use soil_contracts::DeviceId;

    fn raw(moisture: f32, ph: f32) -> RawReading {
        RawReading {
            device_id: DeviceId::new(),
            ts: Utc::now(),
            moisture_raw: moisture,
            temperature_raw: 27.0,
            ph_raw: ph,
            ec_raw: 0.5,
        }
    }

    #[test]
    fn normal_reading_is_good() {
        let v = RangeValidator::default();
        let r = v.validate(&raw(60.0, 4.0), PlotId::new()).unwrap();
        assert_eq!(r.quality, QualityFlag::Good);
    }

    #[test]
    fn outlier_is_flagged_bad() {
        let v = RangeValidator::default();
        let r = v.validate(&raw(140.0, 4.0), PlotId::new()).unwrap();
        assert_eq!(r.quality, QualityFlag::Bad);
        assert!(!r.is_usable());
    }

    #[test]
    fn nan_is_rejected() {
        let v = RangeValidator::default();
        assert!(v.validate(&raw(f32::NAN, 4.0), PlotId::new()).is_err());
    }

    #[test]
    fn calibration_is_applied() {
        let v = RangeValidator::new(Calibration {
            ph: Linear {
                gain: 1.0,
                offset: 0.5,
            },
            ..Calibration::default()
        });
        let r = v.validate(&raw(60.0, 4.0), PlotId::new()).unwrap();
        assert!((r.ph - 4.5).abs() < 1e-6);
    }
}
