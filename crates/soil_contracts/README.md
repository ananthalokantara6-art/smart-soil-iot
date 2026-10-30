# soil_contracts

Crate kontrak bersama Smart Soil IoT. Dimiliki **project lead**.

## Aturan

1. Modul hanya boleh bergantung pada `soil_contracts`, **tidak boleh saling import**.
2. Penyambungan antar modul dilakukan di `smart_soil_orchestrator` lewat `Arc<dyn Trait>`.
3. Butuh tipe/trait baru atau ingin mengubah yang ada? Buka PR ke crate ini dan minta review lead.
   Perubahan MAJOR versi = breaking change (plugin memeriksanya lewat `PluginMeta::is_compatible`).
4. Untuk tes, aktifkan mock: `soil_contracts = { workspace = true, features = ["mock"] }`.

## Siapa implement apa

| Trait               | Diimplementasikan | Dipanggil oleh        |
|---------------------|-------------------|-----------------------|
| `ReadingValidator`  | M1                | M2                    |
| `DeviceDirectory`   | M2                | M3, orchestrator      |
| `ReadingSink`       | M3                | M2                    |
| `ReadingSource`     | M3                | M4, dashboard         |
| `AlertSink`         | M3                | M4, M5                |
| `Analyzer`          | M4 (via plugin)   | orchestrator          |
| `RuleEvaluator`     | M5 (via plugin)   | orchestrator          |
| `Plugin`/`PluginHost` | M6              | M4, M5, plugin lain   |

## Wajib: provenance dan uncertainty

Setiap `AnalysisResult` dan `Recommendation` harus mengisi `Provenance` dan `Confidence`.
Ini syarat verifiabilitas dari dosen.
