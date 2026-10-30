# Smart Soil IoT

Sistem pemantauan tanah untuk pertanian lahan gambut dengan dukungan keputusan cerdas.
Satu workspace Rust, enam modul, enam kelompok.

## Alur data

```
M1 validasi -> M2 transport -> M3 simpan -> M4 analitik -> M5 pakar tanaman -> M3 tampil
                                              ^                 ^
                                              +---- M6: M4 dan M5 didaftarkan sebagai plugin
```

| Modul | Crate | Peran | Kelompok |
|-------|-------|-------|----------|
| M1 | `m1_sensing` | Akuisisi sensor, kalibrasi, filter noise, deteksi outlier | 1 |
| M2 | `m2_gateway` | Komunikasi IoT, identitas perangkat, autentikasi, edge | 2 |
| M3 | `m3_platform` | REST API, penyimpanan time-series, dashboard, alert | 3 |
| M4 | `m4_analytics` | Skor risiko, anomali, tren (tanpa deep learning) | 4 |
| M5 | `m5_expert` | Aturan khusus tanaman, penjelasan, provenance | 5 |
| M6 | `m6_plugin` | Registry, lifecycle, kompatibilitas plugin | 6 |
| Kontrak | `soil_contracts` | Tipe dan trait bersama (milik lead) | Lead |
| Orchestrator | `soil_server` | Menyambungkan semua modul | Lead |

## Aturan emas

1. Modul **hanya** bergantung pada `soil_contracts`. Tidak boleh saling import.
2. Penyambungan dilakukan di `apps/soil_server` lewat `Arc<dyn Trait>`.
3. Setiap `AnalysisResult` dan `Recommendation` wajib berisi `Provenance` dan `Confidence`.
4. Dependensi bersama versinya dikunci di `[workspace.dependencies]` pada `Cargo.toml` root.

## Menjalankan

```bash
cargo run -p soil_server        # demo pipeline end-to-end dengan data palsu
cargo test --workspace          # semua tes, termasuk e2e
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

Rust stable terbaru (lihat `rust-toolchain.toml`).

## Struktur

```
crates/       soil_contracts + m1..m6
apps/         soil_server (orchestrator)
firmware/     kode ESP32 (M1/M2)
web/          frontend dashboard (M3), jika terpisah
docs/         arsitektur, kontrak, matriks verifikasi, catatan riset
```

Mulai dari [`docs/architecture.md`](docs/architecture.md) dan [`CONTRIBUTING.md`](CONTRIBUTING.md).
