# M4 — Analitik matematika dan statistik deskriptif

## Implementasi saat ini

- API: `m4_analytics::analytics::statistics::mean(&[f64]) -> Option<f64>`.
- Lokasi: `crates/m4_analytics/src/analytics/statistics.rs`.
- Penjumlahan menggunakan iterator `.fold()`; hasil dibagi jumlah elemen.
- Fungsi deterministik, tanpa I/O atau mutasi input; satuan hasil mengikuti input.
- Input kosong menghasilkan `None`; satu elemen menghasilkan nilai elemen itu.
- `variance(&[f64]) -> Option<f64>` menghitung varians **sampel** dengan
  pembagi `n - 1` (koreksi Bessel), bukan varians populasi. Menggunakan `mean`
  dan `.fold()` atas kuadrat deviasi; satuan hasil adalah kuadrat satuan input.
- `standard_deviation(&[f64]) -> Option<f64>` menggunakan
  `variance(values).map(f64::sqrt)`; satuan hasil sama dengan input.
- Kedua fungsi dispersi mengembalikan `None` jika sampel kurang dari dua,
  tanpa mutasi input atau I/O. Tidak ada perubahan pada kontrak bersama.
- `simple_moving_average(window_size)` adalah higher-order function yang
  mengembalikan closure `Fn(&[f64]) -> Vec<Option<f64>>`. Output sejajar dengan
  input: `None` sebelum jendela penuh, lalu rata-rata jendela kronologis.
- Ukuran jendela nol menghasilkan `None` untuk setiap posisi; input kosong
  menghasilkan output kosong. Closure dapat dipakai ulang dan tidak memutasi
  deret input.
- `exponential_moving_average(alpha)` mengembalikan closure EMA dengan rumus
  `ema_t = alpha * x_t + (1 - alpha) * ema_(t-1)`, dimulai dari sampel pertama.
  `alpha` harus berada pada `0.0..=1.0`; nilai di luar rentang menghasilkan
  `None` pada semua posisi.
- `analytics::trends::drying_rate_regression` menghitung kemiringan regresi
  linear kelembapan terhadap waktu dalam `%/jam`. Nilai negatif menunjukkan
  pengeringan. Fungsi membutuhkan minimal dua timestamp berbeda dan menolak
  kelembapan non-finite tanpa mengubah input.
- NaN, infinity, dan overflow mengikuti IEEE 754. Fungsi tidak memvalidasi sensor
  atau menghapus sampel invalid; pemanggil bertanggung jawab atas validasi input.
- Kontrak bersama tetap. Pipeline menggunakan mean f64 melalui konversi data
  sensor f32, lalu mengembalikan hasil ke f32 untuk evaluasi risiko.
- EMA menggunakan fold per prefiks tanpa assignment state lokal. Trade-off:
  waktu O(n²), sehingga ditujukan untuk jendela data terbatas.

## Verifikasi

`cargo test --lib analytics::statistics`

Unit test berada di `src/analytics/statistics_tests.rs`, dimuat sebagai
`analytics::statistics::tests`. Integration test tetap tersedia melalui
`cargo test -p m4_analytics --test analytics_tests`.

Tes mencakup rata-rata, input tidak berubah, determinisme, input kosong,
satu elemen, nilai negatif/nol, pecahan, nilai non-finite, dan overflow.
Tes dispersi juga mencakup pembagi sampel, batas dua sampel, data konstan,
akar varians, nilai negatif/pecahan, determinisme, dan propagasi IEEE 754.
Tes SMA mencakup alignment, batas jendela, penggunaan ulang closure,
immutability, input kosong, ukuran jendela nol, dan nilai non-finite.
Tes EMA mencakup smoothing rekursif, batas alpha, input kosong, validasi alpha,
penggunaan ulang closure, immutability, dan nilai non-finite.
Tes regresi tren mencakup laju negatif/konstan/pecahan, batas jumlah titik,
timestamp identik, immutability, dan kelembapan non-finite.
Lihat kebutuhan M4-04 sampai M4-09 pada `docs/verification-matrix.md`.

## Ruang lingkup berikutnya

Prediksi tren lanjutan belum ditambahkan oleh perubahan ini.
