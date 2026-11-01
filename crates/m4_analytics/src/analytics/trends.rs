//! Regresi tren deret waktu untuk analitik tanah.

use chrono::{DateTime, Utc};
use soil_contracts::{Confidence, Parameter, Trend, TrendDirection};

/// Menghitung kemiringan regresi linear kelembapan terhadap waktu.
///
/// Hasil dinyatakan dalam persentase kelembapan per jam (`%/jam`). Nilai
/// negatif menunjukkan pengeringan, sedangkan nilai positif menunjukkan
/// peningkatan kelembapan. Titik harus diberikan dalam urutan waktu dan tidak
/// diurutkan atau dimutasi oleh fungsi.
///
/// Mengembalikan `None` jika jumlah titik kurang dari dua, semua timestamp
/// identik, atau ada nilai waktu/kelembapan non-finite.
pub fn drying_rate_regression(points: &[(DateTime<Utc>, f64)]) -> Option<f64> {
    if points.len() < 2 || points.iter().any(|(_, moisture)| !moisture.is_finite()) {
        return None;
    }

    let start = points[0].0;
    let elapsed_hours: Vec<f64> = points
        .iter()
        .map(|(timestamp, _)| {
            timestamp.signed_duration_since(start).num_milliseconds() as f64 / 3_600_000.0
        })
        .collect();
    let n = points.len() as f64;
    let mean_x = elapsed_hours.iter().sum::<f64>() / n;
    let mean_y = points.iter().map(|(_, moisture)| *moisture).sum::<f64>() / n;
    let sum_squared_x = elapsed_hours
        .iter()
        .map(|x| (x - mean_x).powi(2))
        .sum::<f64>();

    if sum_squared_x == 0.0 || !sum_squared_x.is_finite() {
        return None;
    }

    let covariance = elapsed_hours
        .iter()
        .zip(points.iter())
        .map(|(x, (_, moisture))| (x - mean_x) * (moisture - mean_y))
        .sum::<f64>();

    Some(covariance / sum_squared_x)
}

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
