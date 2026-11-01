//! Statistik deskriptif tanpa mutasi input atau efek samping.

/// Menghitung rata-rata aritmetika; satuan hasil sama dengan satuan input.
///
/// Penjumlahan menggunakan `fold` tanpa mengubah data asal.
/// Mengembalikan `None` jika input kosong, karena rata-rata tidak terdefinisi.
/// Nilai non-finite dan overflow mengikuti aritmetika IEEE 754 pada `f64`;
/// fungsi ini tidak menyaring atau memvalidasi pembacaan sensor.
pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let sum = values.iter().fold(0.0, |acc, value| acc + value);
    Some(sum / values.len() as f64)
}

/// Menghitung varians sampel dengan pembagi `n - 1` (koreksi Bessel).
///
/// Mengembalikan `None` jika jumlah sampel kurang dari dua.
/// Satuan hasil adalah kuadrat satuan input. Perhitungan dua lintasan memakai
/// `mean` lalu `fold` atas kuadrat deviasi, tanpa mengubah input atau I/O.
/// Nilai non-finite dan overflow mengikuti IEEE 754; input tidak disaring.
pub fn variance(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }

    let average = mean(values)?;
    let squared_deviations = values
        .iter()
        .fold(0.0, |acc, value| acc + (value - average).powi(2));
    Some(squared_deviations / (values.len() - 1) as f64)
}

/// Menghitung simpangan baku sampel sebagai akar dari [`variance`].
///
/// Satuan hasil sama dengan input. Mengembalikan `None` jika sampel kurang
/// dari dua. Fungsi murni ini meneruskan perilaku non-finite dan overflow
/// dari `variance`, tanpa menyaring atau mengubah input.
pub fn standard_deviation(values: &[f64]) -> Option<f64> {
    variance(values).map(f64::sqrt)
}

/// Membuat closure *simple moving average* (SMA) untuk `window_size` sampel.
///
/// Closure menerima deret waktu dalam urutan kronologis dan menghasilkan satu
/// nilai untuk setiap input. Posisi sebelum jendela lengkap berisi `None`;
/// posisi berikutnya berisi rata-rata jendela yang berakhir di posisi tersebut.
/// `window_size == 0` menghasilkan `None` pada setiap posisi tanpa panic.
/// Closure tidak mengubah input dan dapat dipakai ulang pada beberapa deret.
pub fn simple_moving_average(window_size: usize) -> impl Fn(&[f64]) -> Vec<Option<f64>> {
    move |values| {
        values
            .iter()
            .enumerate()
            .map(|(index, _)| {
                if window_size == 0 || index + 1 < window_size {
                    None
                } else {
                    mean(&values[index + 1 - window_size..=index])
                }
            })
            .collect()
    }
}

/// Membuat closure *exponential moving average* (EMA) dengan faktor `alpha`.
///
/// EMA dimulai dari sampel pertama, lalu diperbarui secara rekursif dengan
/// rumus `ema_t = alpha * x_t + (1 - alpha) * ema_(t-1)`. Nilai `alpha` yang
/// valid berada pada rentang `0.0..=1.0`; faktor di luar rentang menghasilkan
/// `None` untuk setiap posisi. Input kosong menghasilkan output kosong.
/// Closure tidak memutasi input dan dapat digunakan ulang.
pub fn exponential_moving_average(alpha: f64) -> impl Fn(&[f64]) -> Vec<Option<f64>> {
    move |values| {
        if !(0.0..=1.0).contains(&alpha) {
            return values.iter().map(|_| None).collect();
        }

        values
            .iter()
            .scan(None, |previous, value| {
                let current = match *previous {
                    None => *value,
                    Some(previous) => alpha * *value + (1.0 - alpha) * previous,
                };
                *previous = Some(current);
                Some(Some(current))
            })
            .collect()
    }
}
