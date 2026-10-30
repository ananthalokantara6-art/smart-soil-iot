# Panduan Kontribusi

## Alur kerja

1. `git pull` dari `main`, lalu buat branch: `kelompok-N/<topik>` (contoh `kelompok-4/risk-scoring`).
2. Kerjakan hanya di folder modul kelompokmu (`crates/mN_*`).
3. Sebelum push: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
4. Buka PR ke `main`. CI harus hijau dan project lead harus me-review.
5. Jangan push langsung ke `main`. Jangan commit `target/`.

## Pesan commit

Format: `modul: ringkasan singkat`, misalnya `m4: tambah skor risiko waterlogging`.

## Aturan teknis

- Hanya boleh bergantung pada `soil_contracts` (dan dependensi yang ada di `[workspace.dependencies]`).
- Butuh dependensi baru? Minta lead menambahkannya di `Cargo.toml` root. Jangan menulis versi sendiri.
- Butuh tipe atau trait baru dari kontrak? Buka issue atau PR ke `crates/soil_contracts` dan jelaskan alasannya.
- Pakai `soil_contracts` dengan feature `mock` di bagian `[dev-dependencies]` untuk tes yang butuh data palsu.
- Tidak ada `unwrap()` di kode non-tes. Kembalikan `soil_contracts::Result`.
- Modul M4 tidak boleh memakai deep learning (permintaan dosen).

## Verifiabilitas

Setiap fitur yang dijanjikan harus punya tes yang membuktikannya. Catat di
[`docs/verification-matrix.md`](docs/verification-matrix.md): kebutuhan, tes, status.

## Kebaruan dan paten

Kalau menemukan pendekatan yang baru (kalibrasi otomatis, skoring risiko gambut, aturan pakar
bergaya provenance, dsb.), catat di [`docs/research-notes.md`](docs/research-notes.md) sejak awal,
lengkap dengan tanggal dan siapa kontributornya.
