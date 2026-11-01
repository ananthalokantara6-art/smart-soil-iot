# Matriks Verifikasi

Setiap kebutuhan dari dokumen project harus punya tes yang membuktikannya.
Status: `todo` / `partial` / `done`.

| ID | Modul | Kebutuhan | Tes / bukti | Status |
|----|-------|-----------|-------------|--------|
| M1-01 | M1 | Data kelembapan, suhu, pH, EC divalidasi | `m1_sensing::tests` | partial |
| M1-02 | M1 | Outlier ditandai `Bad` | `m1_sensing::tests::outlier_is_flagged_bad` | done |
| M1-03 | M1 | Kalibrasi dan rekalibrasi otomatis | - | todo |
| M2-01 | M2 | Perangkat tak dikenal ditolak | `m2_gateway::tests::unknown_device_is_rejected` | done |
| M2-02 | M2 | Buffering offline dan kirim ulang | - | todo |
| M2-03 | M2 | Autentikasi dan RBAC | - | todo |
| M3-01 | M3 | Ingest lalu query jendela waktu | `m3_platform::tests` | done |
| M3-02 | M3 | REST API dan dashboard | - | partial |
| M3-03 | M3 | Riwayat peringatan | `m3_platform::tests::alerts_are_stored` | done |
| M4-01 | M4 | Skor risiko waterlogging dan kekeringan | `m4_analytics::tests` | partial |
| M4-02 | M4 | Confidence dan provenance pada hasil | `m4_analytics::tests::result_has_provenance_and_confidence` | done |
| M4-03 | M4 | Deteksi anomali pH dan EC | - | todo |
| M5-01 | M5 | Rekomendasi khusus tanaman dan fase tumbuh | `m5_expert::tests` | partial |
| M5-02 | M5 | Rekomendasi dapat ditelusuri (aturan yang menyala) | `m5_expert::tests::fired_rules_are_recorded` | done |
| M6-01 | M6 | Plugin tidak kompatibel ditolak | `m6_plugin::tests::incompatible_plugin_is_rejected` | done |
| M6-02 | M6 | Enable/disable plugin | `m6_plugin::tests::disabled_plugin_is_hidden` | done |
| E2E-01 | Semua | Sensor -> validasi -> simpan -> analitik -> rekomendasi | `apps/soil_server/tests/e2e.rs` | done |

Tambahkan baris baru setiap kali fitur ditambah. Kelompok mengisi baris modulnya sendiri.
