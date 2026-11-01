//! Anggota 1: Pipeline orkestrasi fungsional monadik.

use chrono::Utc;
use soil_contracts::{
    AnalysisResult, Confidence, Parameter, Provenance, ReadingWindow,
};

use crate::analytics::{anomaly, statistics, trends};
use crate::error::{Result, SoilError};
use crate::{ID, VERSION};

/// Jalankan pipeline analitik fungsional secara monadik.
pub fn process_soil_pipeline(window: &ReadingWindow) -> Result<AnalysisResult> {
    let moisture = window.series(Parameter::Moisture);
    if moisture.is_empty() {
        return Err(SoilError::EmptyDataset);
    }

    let values: Vec<f32> = moisture.iter().map(|(_, v)| *v).collect();
    let mean_val = statistics::mean(&values).ok_or(SoilError::EmptyDataset)?;

    let n = moisture.len() as f32;
    let confidence = Confidence::new(n / 30.0);

    let risks = anomaly::evaluate_risks(mean_val, confidence);
    let anomalies = anomaly::detect_anomalies(window);

    let trends = trends::evaluate_trend(Parameter::Moisture, &moisture, confidence)
        .map(|t| vec![t])
        .unwrap_or_default();

    Ok(AnalysisResult {
        plot_id: window.plot_id,
        window_from: window.from,
        window_to: window.to,
        risks,
        anomalies,
        trends,
        sample_count: moisture.len(),
        provenance: Provenance {
            source: ID.into(),
            version: VERSION.into(),
            method: "functional_pipeline_m4".into(),
            inputs: vec![
                format!("samples={}", moisture.len()),
                format!("mean_moisture_pct={mean_val:.2}"),
            ],
            generated_at: Utc::now(),
        },
    })
}