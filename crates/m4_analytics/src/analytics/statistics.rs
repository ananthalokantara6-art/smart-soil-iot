//! Anggota 2: Fungsi statistik murni tanpa side-effect.

/// Hitung rata-rata menggunakan combinator `.fold()`.
pub fn mean(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let sum = values.iter().copied().fold(0.0, |acc, x| acc + x);
    Some(sum / values.len() as f32)
}

/// Hitung varians sampel.
pub fn variance(values: &[f32]) -> Option<f32> {
    let m = mean(values)?;
    if values.len() < 2 {
        return None;
    }
    let sum_sq_diff = values
        .iter()
        .copied()
        .fold(0.0, |acc, x| acc + (x - m).powi(2));
    Some(sum_sq_diff / (values.len() - 1) as f32)
}

/// Hitung standar deviasi.
pub fn standard_deviation(values: &[f32]) -> Option<f32> {
    variance(values).map(|v| v.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        let vals = [10.0, 20.0, 30.0];
        assert_eq!(mean(&vals), Some(20.0));
        assert_eq!(mean(&[]), None);
    }
}