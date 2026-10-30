# m4_analytics

**M4 - Analitik tanah dan decision support (tanpa deep learning)**. Milik kelompok 4.

Trait yang diimplementasikan: `Analyzer (sebagai Plugin)`

## Status

Implementasi awal sudah ada di `src/lib.rs` agar pipeline end-to-end berjalan. Baca TODO di
kepala file tersebut, lalu ganti/kembangkan tanpa mengubah tanda tangan trait di `soil_contracts`.

## Aturan

- Hanya bergantung pada `soil_contracts` (jangan import crate modul lain).
- Tiap fitur harus punya tes dan dicatat di `docs/verification-matrix.md`.
- Perubahan kontrak: ajukan PR ke `crates/soil_contracts`, review oleh lead.
- Rencana modul: `docs/modules/`.

## Menjalankan tes

```bash
cargo test -p m4_analytics
```
