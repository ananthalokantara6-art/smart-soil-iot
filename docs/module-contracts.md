# Kontrak Antar Modul

Sumber kebenaran: kode di `crates/soil_contracts`. Dokumen ini ringkasannya.

| Trait | Diimplementasikan | Dipanggil oleh |
|-------|-------------------|----------------|
| `ReadingValidator` | M1 | M2 |
| `DeviceDirectory` | M2 | M2 (gateway), M3, orchestrator |
| `ReadingSink` | M3 | M2 |
| `ReadingSource` | M3 | M4, dashboard |
| `AlertSink` | M3 | orchestrator, M4, M5 |
| `Analyzer` | M4 (sebagai plugin) | orchestrator |
| `RuleEvaluator` | M5 (sebagai plugin) | orchestrator |
| `Plugin` / `PluginHost` | M6 (host), M4 dan M5 (plugin) | orchestrator |

## Satuan

- Kelembapan: % VWC (0-100)
- Suhu: derajat Celsius
- pH: skala pH (0-14)
- EC: mS/cm

## Provenance dan uncertainty

Wajib pada `AnalysisResult`, `Alert`, dan `Recommendation`: sumber, versi, metode, masukan,
waktu pembuatan, serta `Confidence` 0.0-1.0.

## Versi

`CONTRACTS_VERSION` mengikuti versi crate. Perubahan MAJOR = breaking. Plugin menyatakan
`requires_contracts_major`, dan M6 menolak plugin yang tidak cocok.

## Asumsi (perlu disepakati, lalu bekukan v0.1)

- [ ] Satuan di atas disetujui M1 dan M4
- [ ] Rentang `Confidence` 0-1
- [ ] Satu perangkat terhubung ke satu petak (`plot_id`)
- [ ] Format transport M2 ke M3 cukup lewat trait (tanpa HTTP internal)

## Aturan perubahan

PR ke `soil_contracts` wajib menjelaskan dampak ke tiap modul dan di-review lead.
