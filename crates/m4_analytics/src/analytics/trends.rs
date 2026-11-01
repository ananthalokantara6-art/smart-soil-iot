//! Anggota 2: Analisis tren dan laju pengeringan.

use chrono::{DateTime, Utc};
use soil_contracts::{Confidence, Parameter, Trend, TrendDirection};

/// Hitung kemiringan regresi linear sederhana (satuan per jam).
pub fn slope_per_hour(points: &[(DateTime<Utc>, f32)]) -> Option<f32> {
    if points.len() < 2 {
        return None;
    }
    let t0 = points[0].0;
    let xs: Vec<f32> = points
        .iter()
        .map(|(t, _)| (*t - t0).num_seconds() as f32 / 3600.0)
        .collect();
    let n = points.len() as f32;
    let mean_x = xs.iter().sum::<f32>() / n;
    let mean_y = points.iter().map(|(_, y)| *y).sum::<f32>() / n;
    let sxx: f32 = xs.iter().map(|x| (x - mean_x).powi(2)).sum();
    if sxx == 0.0 {
        return None;
    }
    let sxy: f32 = xs
        .iter()
        .zip(points)
        .map(|(x, (_, y))| (x - mean_x) * (y - mean_y))
        .sum();
    Some(sxy / sxx)
}

/// Evaluasi tren parameter tertentu.
pub fn evaluate_trend(
    param: Parameter,
    points: &[(DateTime<Utc>, f32)],
    confidence: Confidence,
) -> Option<Trend> {
    let slope = slope_per_hour(points)?;
    let direction = if slope > 0.5 {
        TrendDirection::Rising
    } else if slope < -0.5 {
        TrendDirection::Falling
    } else {
        TrendDirection::Stable
    };

    Some(Trend {
        parameter: param,
        direction,
        slope_per_hour: slope,
        confidence,
    })
}