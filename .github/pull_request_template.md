## Modul

<!-- M1 / M2 / M3 / M4 / M5 / M6 / kontrak / orchestrator -->

## Perubahan

<!-- Apa yang berubah dan kenapa -->

## Checklist

- [ ] `cargo fmt --all` dan `cargo clippy --workspace --all-targets` bersih
- [ ] `cargo test --workspace` hijau
- [ ] Hanya menyentuh folder modul sendiri (atau sudah disepakati dengan lead)
- [ ] Tidak meng-import crate modul lain (hanya `soil_contracts`)
- [ ] Kebutuhan yang dibuktikan tes ini sudah dicatat di `docs/verification-matrix.md`
- [ ] Jika mengubah `soil_contracts`: dampak ke modul lain sudah dijelaskan di bawah

## Dampak ke modul lain

<!-- Kosongkan jika tidak ada -->
