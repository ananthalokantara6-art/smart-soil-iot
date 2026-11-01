\# Spesifikasi Desain \& Perencanaan Teknis: Module 4

\## Soil Analytics \& Decision Support System

\*\*Proyek:\*\* Smart Soil IoT  

\*\*Target Crate:\*\* `module\_4\_soil\_analytics\_and\_decision\_support`  

\*\*Mata Kuliah:\*\* Pemrograman Fungsional  

\*\*Bahasa Pemrograman:\*\* Rust (Edition 2021)  



\---



\## 1. Pendahuluan \& Ruang Lingkup Sistem



Module 4 (\*\*Soil Analytics \& Decision Support\*\*) bertindak sebagai pusat komputasi analitik dan mesin inferensi preskriptif di dalam ekosistem \*\*Smart Soil IoT\*\*. Modul ini menjembatani data telemetri fisik tanah yang dihimpun oleh perangkat IoT dengan kebutuhan agrikultur nyata di lapangan, mengubah deret angka mentah menjadi evaluasi mutu kesuburan serta rekomendasi tindakan terukur (irigasi, pemupukan hara makro, dan perbaikan kimiawi tanah).



Seluruh arsitektur modul dirancang secara \*\*fungsional murni (\*pure functional core\*)\*\*, di mana seluruh logika perhitungan deterministik, bebas efek samping (\*side-effect free\*), dan menjamin integritas data melalui struktur data yang tidak dapat diubah (\*immutable\*).



\### 1.1 Matriks Batas Tanggung Jawab Modul



Untuk menjaga kohesi sistem yang tinggi dan ketergantungan antar-komponen yang longgar (\*loose coupling\*), batas ruang lingkup Module 4 didefinisikan secara tegas:



| Bidang Tanggung Jawab | Di Dalam Lingkup Module 4 (In Scope) | Di Luar Lingkup Module 4 (Out of Scope) |

| :--- | :--- | :--- |

| \*\*Akuisisi Data\*\* | Menerima data telemetri yang telah tervalidasi dalam bentuk struktur data memori. | Pembacaan sinyal ADC sensor, interfacing kabel fisik, dan kalibrasi tegangan listrik (\*Tanggung jawab Module 1\*). |

| \*\*Komunikasi Jaringan\*\* | Menyediakan interface serialisasi/deserialisasi payload JSON standar. | Manajemen koneksi MQTT, HTTP client/server, otentikasi pengguna, dan edge caching (\*Tanggung jawab Module 2\*). |

| \*\*Penyimpanan \& UI\*\* | Menghasilkan struktur laporan analitik dan preskripsi terstruktur (`RecommendationReport`). | Penyimpanan persisten basis data time-series dan rendering grafik visual dashboard (\*Tanggung jawab Module 3\*). |

| \*\*Agronomi Tanaman\*\* | Menggunakan parameter batas agronomi sebagai acuan komputasi. | Penelitian biologi tanaman dan pemeliharaan database varietas bibit (\*Tanggung jawab Module 5\*). |

| \*\*Aktuasi Lapangan\*\* | Menghitung volume air ($L/m^2$), durasi menit penyiraman, dan gram pupuk. | Pensaklaran relay fisik, buka-tutup katup solenoid pompa (\*Tanggung jawab Orchestrator / Aktuator\*). |



\---



\## 2. Pemodelan Domain Berbasis Algebraic Data Types (ADTs)



Modul ini memanfaatkan sistem tipe data aljabar Rust untuk merepresentasikan status tanah, kategori kualitas, dan tindakan keputusan secara presisi tanpa celah keadaan yang tidak valid (\*invalid state unrepresentable\*).



\### 2.1 Model Telemetri dan Batas Agronomi Tanaman



```rust

use serde::{Deserialize, Serialize};



/// Klasifikasi fisik tekstur tanah yang memengaruhi retensi air dan kapur.

\#\[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]

pub enum SoilTexture {

&#x20;   Sandy,

&#x20;   Loamy,

&#x20;   Clayey,

}



/// Snapshot telemetri kondisi tanah pada satu titik waktu tertentu.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct SoilReading {

&#x20;   pub timestamp: u64,

&#x20;   pub moisture\_percentage: f64,     // 0.0 - 100.0%

&#x20;   pub ph\_level: f64,                // 0.0 - 14.0

&#x20;   pub temperature\_celsius: f64,     // °C

&#x20;   pub nitrogen\_ppm: f64,            // mg/kg

&#x20;   pub phosphorus\_ppm: f64,          // mg/kg

&#x20;   pub potassium\_ppm: f64,           // mg/kg

&#x20;   pub electrical\_conductivity: f64, // dS/m (Salinitas)

}



/// Konfigurasi kebutuhan optimal tanaman yang menjadi acuan diagnosis.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct SoilThresholds {

&#x20;   pub min\_moisture: f64,

&#x20;   pub target\_moisture: f64,             // Kapasitas Lapang

&#x20;   pub permanent\_wilting\_point: f64,     // Titik Layu Permanen

&#x20;   pub min\_ph: f64,

&#x20;   pub max\_ph: f64,

&#x20;   pub target\_nitrogen: f64,

&#x20;   pub target\_phosphorus: f64,

&#x20;   pub target\_potassium: f64,

&#x20;   pub max\_safe\_ec: f64,

&#x20;   pub soil\_texture: SoilTexture,

}

```



\### 2.2 Model Diagnosis Mutu Tanah dan Tren



```rust

/// Klasifikasi mutu kesuburan tanah berdasarkan skor komposit SQI.

\#\[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]

pub enum SoilGrade {

&#x20;   Excellent,

&#x20;   Good,

&#x20;   Moderate,

&#x20;   Degraded,

&#x20;   Critical,

}



/// Dinamika perubahan kadar air tanah terhadap waktu.

\#\[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]

pub enum TrendDirection {

&#x20;   RapidlyDecreasing,

&#x20;   Decreasing,

&#x20;   Stable,

&#x20;   Increasing,

&#x20;   RapidlyIncreasing,

}



/// Indeks Kualitas Tanah (Soil Quality Index - SQI) terbobot.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct SoilQualityIndex {

&#x20;   pub overall\_score: f64,

&#x20;   pub grade: SoilGrade,

&#x20;   pub sub\_scores: QualitySubScores,

}



/// Rincian skor individual multi-parameter kualitas tanah.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct QualitySubScores {

&#x20;   pub moisture\_score: f64,

&#x20;   pub ph\_score: f64,

&#x20;   pub nutrient\_score: f64,

&#x20;   pub salinity\_score: f64,

}



/// Metrik agregasi statistik deret waktu sensor.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct SoilStatistics {

&#x20;   pub mean\_moisture: f64,

&#x20;   pub variance\_moisture: f64,

&#x20;   pub std\_dev\_moisture: f64,

&#x20;   pub mean\_ph: f64,

&#x20;   pub mean\_temperature: f64,

&#x20;   pub moisture\_depletion\_rate\_per\_hour: Option<f64>,

&#x20;   pub moisture\_trend: TrendDirection,

&#x20;   pub estimated\_hours\_to\_wilting: Option<f64>,

}

```



\### 2.3 Model Preskripsi Keputusan dan Tindakan Lapangan



```rust

/// Tingkat urgensi eksekusi tindakan agronomi.

\#\[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]

pub enum UrgencyLevel {

&#x20;   Normal,

&#x20;   Advisory,

&#x20;   Urgent,

&#x20;   Critical,

}



/// Rekomendasi volume air dan durasi sistem penyiraman.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct IrrigationAction {

&#x20;   pub water\_volume\_liters\_per\_m2: f64,

&#x20;   pub recommended\_duration\_minutes: u32,

&#x20;   pub moisture\_deficit\_percentage: f64,

&#x20;   pub reason: String,

}



/// Defisit unsur hara makro tanah terhadap ambang kebutuhan tanaman.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]

pub struct NutrientDeficit {

&#x20;   pub nitrogen\_deficit\_ppm: f64,

&#x20;   pub phosphorus\_deficit\_ppm: f64,

&#x20;   pub potassium\_deficit\_ppm: f64,

}



/// Rekomendasi takaran pupuk komersial spesifik.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct FertilizerAction {

&#x20;   pub urea\_grams\_per\_m2: f64,

&#x20;   pub sp36\_grams\_per\_m2: f64,

&#x20;   pub kcl\_grams\_per\_m2: f64,

&#x20;   pub alternative\_npk\_15\_15\_15\_grams\_per\_m2: Option<f64>,

&#x20;   pub application\_notes: String,

}



/// Rekomendasi tindakan perbaikan karakteristik tanah.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub enum AmendmentAction {

&#x20;   ApplyAgriculturalLime {

&#x20;       grams\_per\_m2: f64,

&#x20;       target\_ph: f64,

&#x20;       rationale: String,

&#x20;   },

&#x20;   ApplyElementalSulfur {

&#x20;       grams\_per\_m2: f64,

&#x20;       target\_ph: f64,

&#x20;       rationale: String,

&#x20;   },

&#x20;   FlushSaltsWithLeaching {

&#x20;       excess\_ec: f64,

&#x20;       recommended\_flush\_volume\_liters\_per\_m2: f64,

&#x20;       rationale: String,

&#x20;   },

}



/// Laporan rekomendasi akhir terintegrasi hasil pemrosesan Module 4.

\#\[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct RecommendationReport {

&#x20;   pub generated\_at\_timestamp: u64,

&#x20;   pub soil\_quality\_index: SoilQualityIndex,

&#x20;   pub statistics: SoilStatistics,

&#x20;   pub anomalies: AnomalyAlerts,

&#x20;   pub irrigation\_action: Option<IrrigationAction>,

&#x20;   pub fertilizer\_action: Option<FertilizerAction>,

&#x20;   pub soil\_amendment\_action: Option<AmendmentAction>,

&#x20;   pub overall\_urgency: UrgencyLevel,

}

```



\### 2.4 Penanganan Galat Fungsional (Monadic Error Type)



Tidak ada penggunaan runtime `panic!` atau pengecualian tidak tertangani. Kesalahan didefinisikan secara deklaratif:



```rust

\#\[derive(Debug, Clone, PartialEq)]

pub enum SoilError {

&#x20;   EmptyDataset { operation: \&'static str },

&#x20;   InsufficientData { required: usize, found: usize },

&#x20;   InvalidPhysicalRange { parameter: \&'static str, value: f64, valid\_min: f64, valid\_max: f64 },

&#x20;   InvalidWeightConfiguration { total\_weight: f64 },

&#x20;   ZeroDivisionRisk { context: \&'static str },

}

```



\---



\## 3. Spesifikasi Fungsi: Mesin Analitik Murni (Pure Analytics Engine)



Seluruh fungsi analitik dirancang tanpa mutasi keadaan luar, deterministik, dan memanfaatkan fitur ekspresif Rust seperti \*closure\*, \*iterator combinators\*, dan fungsi tingkat tinggi (\*Higher-Order Functions\*).



\### 3.1 Sub-Domain: Statistik Deskriptif \& Pemulusan Deret Waktu



| Nama Fungsi | Definisi Tanda Tangan (Signature) | Deskripsi Algoritma \& Karakteristik Fungsional |

| :--- | :--- | :--- |

| `mean` | `fn mean(values: \&\[f64]) -> Option<f64>` | Menghitung rata-rata aritmetika menggunakan `Iterator::fold`. Mengembalikan `None` apabila deret masukan kosong tanpa menimbulkan pembagian dengan nol. |

| `variance` | `fn variance(values: \&\[f64]) -> Option<f64>` | Menghitung varians sampel secara murni dengan memetakan kuadrat deviasi tiap elemen terhadap rata-rata menggunakan kombinasi `.map()` dan `.fold()`. |

| `standard\_deviation` | `fn standard\_deviation(values: \&\[f64]) -> Option<f64>` | Menghitung standar deviasi melalui komposisi monadik `variance(values).map(f64::sqrt)`. |

| `simple\_moving\_average` | `fn simple\_moving\_average(window\_size: usize) -> impl Fn(\&\[f64]) -> Vec<f64>` | \*\*Higher-Order Function (HOF)\*\* yang menerima ukuran jendela pengamatan dan mengembalikan closure pemulus deret waktu menggunakan `.windows().filter\_map(mean)`. |

| `exponential\_moving\_average` | `fn exponential\_moving\_average(alpha: f64, values: \&\[f64]) -> Vec<f64>` | Menerapkan pemulusan eksponensial ($EMA$) secara fungsional melalui penumpukan state perataan dalam akumulator `Iterator::fold`. |



\### 3.2 Sub-Domain: Penilaian Kualitas Tanah (Soil Quality Index - SQI)



Indeks mutu tanah dihitung dengan formula pembobotan multi-kriteria:

$$SQI = (W\_m \\cdot S\_m) + (W\_{ph} \\cdot S\_{ph}) + (W\_n \\cdot S\_n) + (W\_{ec} \\cdot S\_{ec})$$

dengan batas normalisasi $\\sum W = 1.0$ dan skor individual ternormalisasi pada rentang $\[0.0, 100.0]$.



| Nama Fungsi | Definisi Tanda Tangan (Signature) | Deskripsi Algoritma \& Karakteristik Fungsional |

| :--- | :--- | :--- |

| `calculate\_moisture\_subscore` | `fn calculate\_moisture\_subscore(moisture: f64, thresholds: \&SoilThresholds) -> f64` | Menghitung skor kecukupan hidrologis tanah berdasarkan posisi kelembapan aktual di antara titik layu permanen ($PWP$) dan target kapasitas lapang. |

| `calculate\_ph\_subscore` | `fn calculate\_ph\_subscore(ph: f64, thresholds: \&SoilThresholds) -> f64` | Menghitung skor kimiawi keasaman tanah berbasis kurva linear deviasi terhadap batas toleransi optimal tanaman. |

| `calculate\_nutrient\_subscore` | `fn calculate\_nutrient\_subscore(reading: \&SoilReading, thresholds: \&SoilThresholds) -> f64` | Menghitung rasio pemenuhan hara makro (N, P, K) menggunakan reduksi iterator fungsional dan memberikan penalti pada pemupukan berlebih. |

| `calculate\_salinity\_subscore` | `fn calculate\_salinity\_subscore(ec: f64, max\_safe\_ec: f64) -> f64` | Menghitung skor toleransi salinitas berdasarkan ambang aman konduktivitas listrik ($EC$). |

| `classify\_soil\_grade` | `fn classify\_soil\_grade(score: f64) -> SoilGrade` | Mengklasifikasikan skor numerik ke dalam varian enum `SoilGrade` menggunakan \*pattern matching guard\*. |

| `calculate\_sqi` | `fn calculate\_sqi(reading: \&SoilReading, thresholds: \&SoilThresholds, weights: \&SqiWeights) -> Result<SoilQualityIndex, SoilError>` | Mengagregasikan seluruh sub-skor secara terbobot dan mengembalikan evaluasi mutu kesuburan tanah lengkap. |



\### 3.3 Sub-Domain: Tren Temporal \& Prediksi Kekeringan



| Nama Fungsi | Definisi Tanda Tangan (Signature) | Deskripsi Algoritma \& Karakteristik Fungsional |

| :--- | :--- | :--- |

| `compute\_moisture\_depletion\_rate` | `fn compute\_moisture\_depletion\_rate(readings: \&\[SoilReading]) -> Option<f64>` | Menghitung laju pengeringan tanah (% penurunan kelembapan per jam) melalui formula regresi linear satu dimensi yang dihitung secara fungsional via `.fold()`. |

| `detect\_trend\_direction` | `fn detect\_trend\_direction(depletion\_rate\_per\_hour: f64) -> TrendDirection` | Mengkategorikan gradien penurunan kelembapan ke dalam enum arah pergerakan tren secara deklaratif. |

| `estimate\_hours\_to\_wilting` | `fn estimate\_hours\_to\_wilting(current\_moisture: f64, permanent\_wilting\_point: f64, depletion\_rate\_per\_hour: f64) -> Option<f64>` | Memprediksi sisa waktu sebelum tanaman menyentuh batas kritis kelayuan permanen jika pola pengeringan berlanjut. Mengembalikan `None` jika tanah tidak sedang mengering. |



\### 3.4 Sub-Domain: Deteksi Anomali \& Risiko Agroekologi



| Nama Fungsi | Definisi Tanda Tangan (Signature) | Deskripsi Algoritma \& Karakteristik Fungsional |

| :--- | :--- | :--- |

| `detect\_waterlogging\_risk` | `fn detect\_waterlogging\_risk(readings: \&\[SoilReading], saturation\_threshold: f64, consecutive\_readings: usize) -> bool` | Mendeteksi kondisi jenuh air berkelanjutan yang berisiko membusukkan akar menggunakan iterator jendela geser `.windows().any(...)`. |

| `detect\_acid\_shock` | `fn detect\_acid\_shock(readings: \&\[SoilReading], max\_drop\_delta: f64) -> Option<f64>` | Mendeteksi penurunan nilai pH tanah secara drastis dalam jendela waktu pengamatan singkat. |

| `detect\_salinity\_stress` | `fn detect\_salinity\_stress(reading: \&SoilReading, max\_safe\_ec: f64) -> bool` | Memeriksa apakah konduktivitas listrik melampaui batas toleransi stres osmotik akar. |

| `evaluate\_all\_anomalies` | `fn evaluate\_all\_anomalies(readings: \&\[SoilReading], thresholds: \&SoilThresholds) -> AnomalyAlerts` | Mengombinasikan seluruh evaluasi risiko menjadi struktur data peringatan anomali yang memuat pesan deskriptif. |



\---



\## 4. Spesifikasi Fungsi: Mesin Pendukung Keputusan (Decision Support System)



Submodul ini mentranslasikan metrik analitik tanah menjadi panduan tindakan preskriptif kuantitatif bagi petani dan aktuator irigasi otomatis.



\### 4.1 Sub-Domain: Preskripsi Irigasi Presisi



| Nama Fungsi | Definisi Tanda Tangan (Signature) | Deskripsi Algoritma \& Formula Agronomi |

| :--- | :--- | :--- |

| `calculate\_water\_deficit` | `fn calculate\_water\_deficit(current\_moisture: f64, target\_moisture: f64, root\_depth\_cm: f64) -> f64` | Menghitung kekurangan volume air volumetrik menuju kapasitas lapang dalam satuan liter per meter persegi ($L/m^2$):<br>$$\\text{Defisit Air } (L/m^2) = \\frac{\\theta\_{\\text{target}} - \\theta\_{\\text{aktual}}}{100} \\times (\\text{Kedalaman Perakaran mm})$$ |

| `recommend\_irrigation` | `fn recommend\_irrigation(reading: \&SoilReading, thresholds: \&SoilThresholds, emitter\_flow\_rate\_lph\_per\_m2: f64) -> Option<IrrigationAction>` | Menghasilkan rekomendasi irigasi lengkap beserta estimasi durasi menit operasional emitter penyiraman jika kadar air berada di bawah batas minimum aman. |



\### 4.2 Sub-Domain: Preskripsi Pemupukan Hara Makro (N, P, K)



| Nama Fungsi | Definisi Tanda Tangan (Signature) | Deskripsi Algoritma \& Formula Agronomi |

| :--- | :--- | :--- |

| `calculate\_nutrient\_deficits` | `fn calculate\_nutrient\_deficits(reading: \&SoilReading, targets: \&SoilThresholds) -> NutrientDeficit` | Menghitung selisih defisit elemen murni Nitrogen, Fosfor, dan Kalium terhadap target tanaman secara independen. |

| `recommend\_fertilizers` | `fn recommend\_fertilizers(deficit: \&NutrientDeficit) -> Option<FertilizerAction>` | Mengonversi defisit hara murni (ppm) ke takaran pupuk komersial riil ($g/m^2$) berdasarkan kadar aktif hara:<br>• \*\*Urea:\*\* $46\\% \\text{ N}$<br>• \*\*SP-36:\*\* $36\\% \\text{ P}\_2\\text{O}\_5$<br>• \*\*KCl:\*\* $60\\% \\text{ K}\_2\\text{O}$<br>serta estimasi opsi pupuk majemuk NPK 15-15-15. |



\### 4.3 Sub-Domain: Preskripsi Pembenah Tanah (Soil Amendment)



| Nama Fungsi | Definisi Tanda Tangan (Signature) | Deskripsi Algoritma \& Formula Agronomi |

| :--- | :--- | :--- |

| `recommend\_soil\_amendment` | `fn recommend\_soil\_amendment(reading: \&SoilReading, thresholds: \&SoilThresholds) -> Option<AmendmentAction>` | Menerapkan aturan ameliorasi berbasis tekstur tanah:<br>• \*\*Tanah Masam ($pH < min$):\*\* Dosis kapur dolomit pertanian ($CaCO\_3$) dihitung dengan faktor penyangga tekstur ($150\\text{ g/m}^2$ untuk pasir, $250\\text{ g/m}^2$ untuk lempung, $350\\text{ g/m}^2$ untuk liat per $1.0\\text{ kenaikan pH}$).<br>• \*\*Tanah Basa ($pH > max$):\*\* Dosis bubuk belerang murni ($S$) untuk menormalkan pH.<br>• \*\*Tanah Salin:\*\* Rekomendasi volume air pencucian garam (\*leaching\*). |

| `evaluate\_overall\_urgency` | `fn evaluate\_overall\_urgency(sqi: \&SoilQualityIndex, anomalies: \&AnomalyAlerts, irrigation: Option<\&IrrigationAction>, fertilizer: Option<\&FertilizerAction>, amendment: Option<\&AmendmentAction>) -> UrgencyLevel` | Menentukan prioritas tindakan keseluruhan (`Normal`, `Advisory`, `Urgent`, `Critical`) melalui evaluasi \*pattern matching\* berhirarki. |



\---



\## 5. Integrasi Alur Monadik (Functional Pipeline Orchestration)



Seluruh fungsi analitik dan penentuan keputusan dirangkai ke dalam satu fungsi orkestrasi fungsional murni pada berkas `src/pipeline.rs`:



```rust

pub fn process\_soil\_pipeline(

&#x20;   readings: \&\[SoilReading],

&#x20;   thresholds: \&SoilThresholds,

&#x20;   config: \&PipelineConfig,

) -> Result<RecommendationReport, SoilError>

```



\### 5.1 Diagram Alir Transformasi Data Fungsional



```text

&#x20;              Deret Bacaan Sensor: \&\[SoilReading]

&#x20;                               │

&#x20;                               ▼

&#x20;   \[Tahap 1: Validasi Fungsional] ──── (Gagal) ────► Err(SoilError::InvalidPhysicalRange)

&#x20;                               │

&#x20;                               ▼ (Lolos)

&#x20;   \[Tahap 2: Agregasi Statistik Deskriptif \& HOF SMA]

&#x20;                               │

&#x20;                               ▼

&#x20;   \[Tahap 3 \& 4: Evaluasi SQI \& Estimasi Waktu Titik Layu]

&#x20;                               │

&#x20;                               ▼

&#x20;   \[Tahap 5: Pemeriksaan Anomali Agroekologi]

&#x20;                               │

&#x20;                               ▼

&#x20;   \[Tahap 6: Eksekusi Mesin Keputusan (Irigasi, Hara, Amelioran)]

&#x20;                               │

&#x20;                               ▼

&#x20;                RecommendationReport (Immutable Output)

```



Alur eksekusi ini menjamin bahwa seluruh transformasi data berlangsung linier, tanpa mutasi \*shared memory\*, dan aman dari interupsi kegagalan berkat penanganan error monadik.



\---



\## 6. Spesifikasi Antarmuka Publik \& Protokol Komunikasi Sistem



\### 6.1 Antarmuka Crate Eksternal (`pub fn`)



Modul 4 mengekspos fungsi publik utama yang dapat dikonsumsi langsung oleh unit `smart\_soil\_orchestrator` atau diuji secara independen:



| Fungsi Publik (`pub fn`) | Konsumen Potensial | Masukan (Input) | Keluaran (Output) | Peran Komunikasi |

| :--- | :--- | :--- | :--- | :--- |

| `process\_soil\_pipeline` | `smart\_soil\_orchestrator` / Modul 3 | `\&\[SoilReading]`, `\&SoilThresholds`, `\&PipelineConfig` | `Result<RecommendationReport, SoilError>` | \*\*Entry Point Utama:\*\* Menghasilkan laporan analitik dan preskripsi terpadu. |

| `calculate\_sqi` | Module 3 (Visualisasi) | `\&SoilReading`, `\&SoilThresholds`, `\&SqiWeights` | `Result<SoilQualityIndex, SoilError>` | Menyediakan metrik kesuburan tanah untuk grafik dashboard. |

| `recommend\_irrigation` | Unit Aktuator Pompa | `\&SoilReading`, `\&SoilThresholds`, `f64` (debit) | `Option<IrrigationAction>` | Menyediakan perintah preskriptif volume air irigasi. |

| `recommend\_fertilizers` | Aplikasi Mobile Petani | `\&NutrientDeficit` | `Option<FertilizerAction>` | Menyediakan takaran dosis pupuk NPK. |

| `recommend\_soil\_amendment` | Aplikasi Mobile Petani | `\&SoilReading`, `\&SoilThresholds` | `Option<AmendmentAction>` | Menyediakan panduan pemberian kapur dolomit / belerang. |



\### 6.2 Contoh Format Pertukaran Data Serialisasi JSON



Hasil pemrosesan Module 4 secara otomatis dapat diserialisasikan ke format JSON standar untuk menjamin kemudahan integrasi dengan modul backend web maupun sistem broker IoT:



```json

{

&#x20; "generated\_at\_timestamp": 1710010800,

&#x20; "soil\_quality\_index": {

&#x20;   "overall\_score": 78.45,

&#x20;   "grade": "Good",

&#x20;   "sub\_scores": {

&#x20;     "moisture\_score": 68.2,

&#x20;     "ph\_score": 82.5,

&#x20;     "nutrient\_score": 81.0,

&#x20;     "salinity\_score": 92.0

&#x20;   }

&#x20; },

&#x20; "statistics": {

&#x20;   "mean\_moisture": 41.75,

&#x20;   "variance\_moisture": 14.25,

&#x20;   "std\_dev\_moisture": 3.77,

&#x20;   "mean\_ph": 5.35,

&#x20;   "mean\_temperature": 27.12,

&#x20;   "moisture\_depletion\_rate\_per\_hour": 3.0,

&#x20;   "moisture\_trend": "RapidlyDecreasing",

&#x20;   "estimated\_hours\_to\_wilting": 5.33

&#x20; },

&#x20; "irrigation\_action": {

&#x20;   "water\_volume\_liters\_per\_m2": 80.0,

&#x20;   "recommended\_duration\_minutes": 192,

&#x20;   "moisture\_deficit\_percentage": 32.0,

&#x20;   "reason": "Kelembapan tanah (38.0%) berada di bawah batas minimum (45.0%). Defisit air sebesar 80.0 L/m²."

&#x20; },

&#x20; "fertilizer\_action": {

&#x20;   "urea\_grams\_per\_m2": 19.56,

&#x20;   "sp36\_grams\_per\_m2": 14.58,

&#x20;   "kcl\_grams\_per\_m2": 17.50,

&#x20;   "alternative\_npk\_15\_15\_15\_grams\_per\_m2": 60.0

&#x20; },

&#x20; "soil\_amendment\_action": {

&#x20;   "ApplyAgriculturalLime": {

&#x20;     "grams\_per\_m2": 275.0,

&#x20;     "target\_ph": 6.4,

&#x20;     "rationale": "pH tanah (5.3) terlalu masam. Berikan kapur dolomit sebesar 275.0 g/m²."

&#x20;   }

&#x20; },

&#x20; "overall\_urgency": "Urgent"

}

```



\---



\## 7. Rencana Pengujian Mutu \& Skenario Lapangan



Pengujian dilakukan secara komprehensif tanpa bergantung pada hardware sensor fisik melalui \*unit tests\* dan \*integration tests\* berbasis data tiruan (\*fixture\*) pada direktori `tests/`:



1\. \*\*Skenario Lahan Kekeringan Ekstrem:\*\*

&#x20;  \* Masukan kelembapan 28.0% (ambang minimum 45.0%, target 70.0%).

&#x20;  \* Sistem harus mengaktifkan `IrrigationAction`, menghitung defisit volume air, dan durasi operasional secara presisi.

2\. \*\*Skenario Koreksi Tanah Masam (Acidic Liming):\*\*

&#x20;  \* Masukan pH tanah 4.8 pada lahan bertekstur lempung (\*Loamy\*).

&#x20;  \* Sistem harus menghasilkan `AmendmentAction::ApplyAgriculturalLime` dengan perhitungan dosis dolomit yang tepat sesuai faktor penyangga tekstur.

3\. \*\*Skenario Anomali Genangan Air \& Toksisitas Garam:\*\*

&#x20;  \* Deret kelembapan $\\ge 90.0\\%$ secara kontinu selama lebih dari 3 sampel pengamatan dengan nilai $EC \\ge 3.0\\text{ dS/m}$.

&#x20;  \* Sistem harus menyalakan peringatan waterlogging dan salinitas, menetapkan `UrgencyLevel::Critical`, menonaktifkan aksi irigasi, dan merekomendasikan tindakan pencucian garam (\*leaching\*).

4\. \*\*Skenario Validasi Batas Galat (\*Edge Cases\*):\*\*

&#x20;  \* Masukan dataset kosong (`\&\[]`) harus mengembalikan `Err(SoilError::EmptyDataset)`.

&#x20;  \* Masukan pembacaan anomali (misal pH bernilai 16.0 atau kelembapan negatif) harus ditolak saat proses validasi awal.



\---



\## 8. Pembagian Tanggung Jawab \& Tata Kelola Tim



Pengerjaan proyek dibagi secara modular ke dalam empat peran spesialisasi untuk menjamin kontribusi aktif seluruh anggota tim pada repositori Git:



| Peran Anggota | Fokus Sub-Domain Kode | Berkas Tanggung Jawab |

| :--- | :--- | :--- |

| \*\*Anggota 1 (Project Lead)\*\* | Arsitektur Modul, Pipeline Monadik \& Integrasi Sistem Eksternal | `src/lib.rs`, `src/pipeline.rs`, `src/error.rs`, `examples/demo.rs` |

| \*\*Anggota 2\*\* | Komputasi Matematika, Statistik Deskriptif Murni, HOF SMA \& Evaluasi Tren | `src/analytics/statistics.rs`, `src/analytics/trends.rs`, `src/analytics/sqi.rs` |

| \*\*Anggota 3\*\* | Mesin Keputusan Preskriptif (Irigasi, Hara Makro NPK \& Ameliorasi Kimiawi) | `src/decision/irrigation.rs`, `src/decision/fertilizer.rs`, `src/decision/amendment.rs`, `src/decision/mod.rs` |

| \*\*Anggota 4\*\* | Pemodelan Data Domain, Aturan Anomali Agroekologi \& Skenario Uji Terintegrasi | `src/models/reading.rs`, `src/models/analytics.rs`, `src/analytics/anomaly.rs`, `tests/\*` |



\### Standar Tata Kelola Kode:

\* Setiap anggota bekerja pada cabang fitur masing-masing (`feature/pipeline-core`, `feature/analytics-math`, `feature/decision-support`, `feature/domain-models`).

\* Integrasi ke cabang utama (`main`) wajib melalui mekanisme Pull Request dengan telaah kode (\*code review\*).

\* Seluruh fungsi publik wajib lolos pengujian dokumentasi dengan perintah `cargo test --doc` dan pengujian unit dengan `cargo test`.



