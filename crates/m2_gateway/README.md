# m2_gateway

**M2 - Komunikasi IoT, edge, user management**. Milik kelompok 2.

Trait yang diimplementasikan: `DeviceDirectory (dan memakai ReadingValidator, ReadingSink)`

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
cargo test -p m2_gateway
```
