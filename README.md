# Smart Soil IoT

Sistem pemantauan tanah untuk pertanian lahan gambut dengan dukungan keputusan cerdas.
Satu workspace Rust, enam modul, enam kelompok.
<img width="736" height="682" alt="image" src="https://github.com/user-attachments/assets/981ce59c-85a7-43f2-a4b5-e93601f0e36a" />

## 👥 Team Members & Contributors

<table>
  <tr>
    <td align="center">
      <a href="https://github.com/ananthalokantara6-art">
        <img src="https://github.com/ananthalokantara6-art.png" width="100px;" alt="Anantha"/><br />
        <sub><b>Anantha Lokantara</b></sub>
      </a><br />
      <sub>Project Lead</sub>
    </td>
    <td align="center">
      <a href="https://github.com/nab4bil">
        <img src="https://github.com/nab4bil.png" width="100px;" alt="Nama Teman 1"/><br />
        <sub><b>nab4bil</b></sub>
      </a><br />
      <sub>Module 1 Lead</sub>
    </td>
    <td align="center">
      <a href="https://github.com/TiaraSaparang">
        <img src="https://github.com/TiaraSaparang.png" width="100px;" alt="Nama Teman 2"/><br />
        <sub><b>TiaraSaparang</b></sub>
      </a><br />
      <sub>Module 2 Lead</sub>
    </td>
    <td align="center">
      <a href="https://github.com/lisaaly9">
        <img src="https://github.com/lisaaly9.png" width="100px;" alt="Nama Teman 2"/><br />
        <sub><b>lisaaly9</b></sub>
      </a><br />
      <sub>Module 3 Lead</sub>
    </td>
   <td align="center">
      <a href="https://github.com/elowgelo">
        <img src="https://github.com/elowgelo.png" width="100px;" alt="elgurX"/><br />
        <sub><b>elowgelo</b></sub>
      </a><br />
      <sub>Module 4 Lead</sub>
    </td>
    <td align="center">
      <a href="https://github.com/JUANRESQ">
        <img src="https://github.com/JUANRESQ.png" width="100px;" alt="Nama Teman 2"/><br />
        <sub><b>JUANRESQ</b></sub>
      </a><br />
      <sub>Module 5 Lead</sub>
    </td>
    <td align="center">
      <a href="https://github.com/alcanary">
        <img src="https://github.com/alcanary.png" width="100px;" alt="Nama Teman 2"/><br />
        <sub><b>alcanary</b></sub>
      </a><br />
      <sub>Module 6 Lead</sub>
    </td>
  </tr>
</table>

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

## Aturan 

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
