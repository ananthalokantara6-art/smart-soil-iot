//! Regresi tren deret waktu untuk analitik tanah.

use chrono::{DateTime, Utc};

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
