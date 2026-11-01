//! Verifikasi integration test untuk statistik deskriptif dan tren M4.
//!
//! Kelompok tes ini memeriksa hasil numerik, batas input, perilaku IEEE 754,
//! sifat fungsi murni, penggunaan ulang closure, serta regresi laju pengeringan.

use chrono::{Duration, TimeZone, Utc};
use m4_analytics::analytics::statistics::{
    exponential_moving_average, mean, simple_moving_average, standard_deviation, variance,
};
use m4_analytics::analytics::trends::drying_rate_regression;

/// Menguji slope kelembapan linear dalam satuan persentase per jam.
#[test]
fn drying_rate_regression_returns_moisture_slope_per_hour() {
    let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let points = [
        (start, 60.0),
        (start + Duration::hours(1), 50.0),
        (start + Duration::hours(2), 40.0),
    ];

    assert_eq!(drying_rate_regression(&points), Some(-10.0));
}

/// Menguji penolakan data terlalu sedikit atau timestamp yang sama.
#[test]
fn drying_rate_regression_requires_two_distinct_timestamps() {
    let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();

    assert_eq!(drying_rate_regression(&[]), None);
    assert_eq!(drying_rate_regression(&[(start, 50.0)]), None);
    assert_eq!(
        drying_rate_regression(&[(start, 50.0), (start, 40.0)]),
        None
    );
}

/// Menguji laju konstan dan laju dengan interval waktu pecahan.
#[test]
fn drying_rate_regression_handles_constant_and_fractional_rates() {
    let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    assert_eq!(
        drying_rate_regression(&[
            (start, 50.0),
            (start + Duration::hours(1), 50.0),
            (start + Duration::hours(2), 50.0),
        ]),
        Some(0.0)
    );

    let rate = drying_rate_regression(&[
        (start, 60.0),
        (start + Duration::minutes(30), 58.0),
        (start + Duration::hours(1), 56.0),
    ])
    .expect("regresi valid");
    assert!((rate + 4.0).abs() < 1e-12);
}

/// Memastikan regresi tidak memutasi input dan menolak kelembapan non-finite.
#[test]
fn drying_rate_regression_preserves_input_and_rejects_non_finite_moisture() {
    let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let points = [(start, 60.0), (start + Duration::hours(1), 50.0)];
    let original = points;

    assert_eq!(drying_rate_regression(&points), Some(-10.0));
    assert_eq!(points, original);
    assert_eq!(
        drying_rate_regression(&[(start, f64::NAN), (start + Duration::hours(1), 50.0)]),
        None
    );
    assert_eq!(
        drying_rate_regression(&[(start, f64::INFINITY), (start + Duration::hours(1), 50.0)]),
        None
    );
}

/// Menguji pembaruan EMA secara rekursif menggunakan hasil sebelumnya.
#[test]
fn exponential_moving_average_applies_smoothing_recursively() {
    let ema = exponential_moving_average(0.5);

    assert_eq!(
        ema(&[10.0, 20.0, 30.0]),
        vec![Some(10.0), Some(15.0), Some(22.5)]
    );
}

/// Menguji input kosong serta batas `alpha` 0 dan 1.
#[test]
fn exponential_moving_average_handles_alpha_boundaries_and_empty_input() {
    assert!(exponential_moving_average(0.5)(&[]).is_empty());
    assert_eq!(
        exponential_moving_average(1.0)(&[2.0, 4.0, 8.0]),
        vec![Some(2.0), Some(4.0), Some(8.0)]
    );
    assert_eq!(
        exponential_moving_average(0.0)(&[2.0, 4.0, 8.0]),
        vec![Some(2.0), Some(2.0), Some(2.0)]
    );
}

/// Menguji penolakan `alpha` di luar rentang 0 sampai 1.
#[test]
fn exponential_moving_average_rejects_invalid_alpha() {
    for alpha in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert_eq!(
            exponential_moving_average(alpha)(&[1.0, 2.0]),
            vec![None, None]
        );
    }
}

/// Memastikan closure EMA dapat dipakai ulang tanpa mengubah input.
#[test]
fn exponential_moving_average_is_reusable_and_does_not_mutate_input() {
    let ema = exponential_moving_average(0.25);
    let values = [4.0, 8.0, 12.0];
    let original = values;

    assert_eq!(ema(&values), vec![Some(4.0), Some(5.0), Some(6.75)]);
    assert_eq!(ema(&[10.0, 20.0]), vec![Some(10.0), Some(12.5)]);
    assert_eq!(values, original);
}

/// Menguji propagasi hasil non-finite pada perhitungan EMA.
#[test]
fn exponential_moving_average_preserves_non_finite_sample_results() {
    assert!(exponential_moving_average(0.5)(&[1.0, f64::NAN])[1]
        .unwrap()
        .is_nan());
    assert_eq!(
        exponential_moving_average(0.5)(&[1.0, f64::INFINITY])[1],
        Some(f64::INFINITY)
    );
}

/// Menguji output SMA yang aligned setelah jendela pertama lengkap.
#[test]
fn simple_moving_average_returns_aligned_full_windows() {
    let sma = simple_moving_average(3);

    assert_eq!(
        sma(&[1.0, 2.0, 3.0, 4.0, 5.0]),
        vec![None, None, Some(2.0), Some(3.0), Some(4.0),]
    );
}

/// Menguji input kosong, window terlalu besar, dan window berukuran nol.
#[test]
fn simple_moving_average_handles_window_boundaries_and_empty_input() {
    let empty = simple_moving_average(3);
    assert!(empty(&[]).is_empty());

    let oversized = simple_moving_average(4);
    assert_eq!(oversized(&[1.0, 2.0, 3.0]), vec![None, None, None]);

    let zero = simple_moving_average(0);
    assert_eq!(zero(&[1.0, 2.0]), vec![None, None]);
}

/// Memastikan closure SMA dapat dipakai ulang tanpa memutasi input.
#[test]
fn simple_moving_average_is_reusable_and_does_not_mutate_input() {
    let sma = simple_moving_average(2);
    let values = [2.0, 4.0, 8.0];
    let original = values;

    assert_eq!(sma(&values), vec![None, Some(3.0), Some(6.0)]);
    assert_eq!(sma(&[10.0, 20.0]), vec![None, Some(15.0)]);
    assert_eq!(values, original);
}

/// Menguji propagasi hasil non-finite pada jendela SMA.
#[test]
fn simple_moving_average_preserves_non_finite_window_results() {
    let sma = simple_moving_average(2);
    assert!(sma(&[1.0, f64::NAN])[1].unwrap().is_nan());
    assert_eq!(sma(&[1.0, f64::INFINITY])[1], Some(f64::INFINITY));
}

/// Menguji bahwa simpangan baku membutuhkan minimal dua sampel.
#[test]
fn standard_deviation_requires_at_least_two_samples() {
    assert_eq!(standard_deviation(&[]), None);
    assert_eq!(standard_deviation(&[42.0]), None);
}

/// Menguji variance dan simpangan baku pada jumlah minimum dua sampel.
#[test]
fn dispersion_handles_two_samples() {
    assert_eq!(variance(&[1.0, 3.0]), Some(2.0));
    let actual = standard_deviation(&[1.0, 3.0]).unwrap();
    assert!((actual - std::f64::consts::SQRT_2).abs() < 1e-12);
}

/// Menguji bahwa data konstan memiliki dispersi nol.
#[test]
fn dispersion_is_zero_for_constant_samples() {
    assert_eq!(variance(&[7.0, 7.0, 7.0]), Some(0.0));
    assert_eq!(standard_deviation(&[7.0, 7.0, 7.0]), Some(0.0));
}

/// Menguji data negatif/pecahan, determinisme, dan immutability dispersi.
#[test]
fn dispersion_handles_negative_fractional_samples_without_mutation() {
    let values = [-1.5, -0.5, 0.5];
    let original = values;
    assert_eq!(variance(&values), Some(1.0));
    assert_eq!(standard_deviation(&values), Some(1.0));
    assert_eq!(variance(&values), variance(&values));
    assert_eq!(standard_deviation(&values), standard_deviation(&values));
    assert_eq!(values, original);
}

/// Menguji propagasi NaN, infinity, dan overflow pada dispersi.
#[test]
fn dispersion_preserves_ieee754_non_finite_behavior() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(variance(&[1.0, invalid]).unwrap().is_nan());
        assert!(standard_deviation(&[1.0, invalid]).unwrap().is_nan());
    }
    assert_eq!(variance(&[-f64::MAX, f64::MAX]), Some(f64::INFINITY));
    assert_eq!(
        standard_deviation(&[-f64::MAX, f64::MAX]),
        Some(f64::INFINITY)
    );
}

/// Menguji simpangan baku sebagai akar dari varians sampel.
#[test]
fn standard_deviation_is_square_root_of_sample_variance() {
    assert_eq!(standard_deviation(&[1.0, 3.0, 5.0]), Some(2.0));
}

/// Menguji bahwa variance membutuhkan minimal dua sampel.
#[test]
fn variance_requires_at_least_two_samples() {
    assert_eq!(variance(&[]), None);
    assert_eq!(variance(&[42.0]), None);
}

/// Menguji penggunaan pembagi `n - 1` untuk varians sampel.
#[test]
fn variance_uses_sample_denominator() {
    assert_eq!(variance(&[1.0, 2.0, 3.0]), Some(1.0));
}

/// Menguji mean untuk satu nilai.
#[test]
fn mean_handles_single_value() {
    assert_eq!(mean(&[42.5]), Some(42.5));
}

/// Menguji mean dengan nilai negatif dan nol.
#[test]
fn mean_handles_negative_values_and_zero() {
    assert_eq!(mean(&[-6.0, 0.0, 3.0]), Some(-1.0));
}

/// Menguji mean dengan nilai pecahan floating-point.
#[test]
fn mean_handles_fractions() {
    let actual = mean(&[0.1, 0.2, 0.3]).expect("input tidak kosong");
    assert!((actual - 0.2).abs() < 1e-12);
}

/// Menguji propagasi NaN, infinity, dan overflow pada mean.
#[test]
fn mean_preserves_ieee754_non_finite_behavior() {
    assert!(mean(&[1.0, f64::NAN]).unwrap().is_nan());
    assert_eq!(mean(&[1.0, f64::INFINITY]), Some(f64::INFINITY));
    assert_eq!(mean(&[1.0, f64::NEG_INFINITY]), Some(f64::NEG_INFINITY));
    assert!(mean(&[f64::INFINITY, f64::NEG_INFINITY]).unwrap().is_nan());
    assert_eq!(mean(&[f64::MAX, f64::MAX]), Some(f64::INFINITY));
}

/// Menguji bahwa mean input kosong menghasilkan `None`.
#[test]
fn mean_returns_none_for_empty_input() {
    assert_eq!(mean(&[]), None);
}

/// Menguji hasil mean, determinisme, dan input yang tidak berubah.
#[test]
fn mean_computes_average_without_changing_input() {
    let values = [10.0, 20.0, 30.0];
    let original = values;

    assert_eq!(mean(&values), Some(20.0));
    assert_eq!(values, original);
    assert_eq!(mean(&values), mean(&values));
}
