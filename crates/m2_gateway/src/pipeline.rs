//! Pipeline Fungsional Murni Modul 2 (100% Pure Function Composition).
//!
//! File ini menggabungkan seluruh fungsi murni dari:
//! - [`crate::device`] & [`crate::user`] & [`crate::rbac`] (Identitas Perangkat, Kepemilikan, Petak `PlotMeta`, RBAC)
//! - [`crate::anomaly`] (Validasi Sensor Tanah Gambut RAB M1, Deteksi Anomali Edge, Interval Sampling, Buzzer)
//! - [`crate::retry`] (Offline Buffering & Reconnect Batching)
//! - [`crate::contracts`] (Transformasi `RawReading` -> `SoilReading` untuk `ReadingSink` Modul 3)
//!
//! **Penting (Sesuai Rubrik Pemrograman Fungsional)**:
//! Tidak ada `Mutex`, tidak ada operasi jaringan/database, dan tidak ada mutasi langsung di sini.
//! Setiap transisi menerima `GatewaySnapshot` lama dan menghasilkan `(GatewaySnapshot, PipelineOutcome)` baru!

use crate::anomaly::{
    check_anomaly, check_interval, should_trigger_buzzer, validate_and_transform,
};
use crate::contracts::{M3IngestEnvelope, PlotMeta, RawReading};
use crate::device::{Device, Registry};
use crate::retry::{enqueue_offline, take_reconnect_batch, BufferedItem};
use crate::user::Kepemilikan;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Snapshot imutabel dari seluruh state Modul 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySnapshot {
    pub registry: Registry,
    pub kepemilikan: Kepemilikan,
    /// Status jaringan modem SIM7000E saat ini (`true` = Online, `false` = Offline).
    pub network_online: bool,
    /// Antrean *Offline Buffer* ketika sinyal di kebun gambut terputus.
    pub offline_buffer: Vec<BufferedItem>,
    /// Daftar `event_id` yang sudah berhasil dikirim ke Modul 3 (idempotency).
    pub processed_events: HashSet<String>,
    /// Catatan waktu (`timestamp_sec`) pengiriman terakhir per `device_id`.
    pub last_sampling_time: HashMap<String, u64>,
    /// Data tervalidasi (`SoilReading` + `PlotMeta`) yang sudah masuk ke `ReadingSink` Modul 3.
    pub m3_sink_records: Vec<M3IngestEnvelope>,
}

/// Hasil eksekusi murni dari satu event pengiriman data sensor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PipelineOutcome {
    /// Berhasil divalidasi dan langsung diteruskan ke `ReadingSink` Modul 3.
    ForwardedToM3 {
        envelope: M3IngestEnvelope,
        message: String,
    },
    /// Jaringan sedang Offline -> disimpan dengan aman ke dalam *Offline Buffer*.
    BufferedOffline {
        buffered_count: usize,
        message: String,
    },
    /// Ditolak karena duplikat `event_id`, perangkat tidak terdaftar, atau interval sampling belum tercapai.
    Rejected { code: u16, reason: String },
}

impl GatewaySnapshot {
    /// Fungsi murni: Membuat state awal beserta data bawaan sesuai **RAB Modul 1** dan **Kontrak Modul 3**.
    pub fn initial_seed() -> Self {
        let dev1 = Device {
            id: "ESP32-GAMBUT-01".to_string(),
            nama: "Node Sensor Gambut Blok A (Kalimantan)".to_string(),
            tipe: "ESP32-DOIT + SIM7000E + ADS1115 + DS18B20 + DFRobot Moisture/pH/EC".to_string(),
            sampling_interval_sec: 10,
        };
        let dev2 = Device {
            id: "ESP32-GAMBUT-02".to_string(),
            nama: "Node Sensor Gambut Blok B (Kalimantan)".to_string(),
            tipe: "ESP32-DOIT + SIM7000E + ADS1115 + DS18B20 + DFRobot Moisture/pH/EC".to_string(),
            sampling_interval_sec: 15,
        };

        let registry = Registry::new()
            .registrasi_murni(dev1)
            .and_then(|r| r.registrasi_murni(dev2))
            .unwrap_or_default();

        let plot1 = PlotMeta {
            plot_id: "PLOT-KALTENG-01".to_string(),
            name: "Petak Nanas Gambut Blok A".to_string(),
            latitude: -2.2083,
            longitude: 113.9167,
            area_ha: 1.5,
            plant: "nanas".to_string(),
        };
        let plot2 = PlotMeta {
            plot_id: "PLOT-KALTENG-02".to_string(),
            name: "Petak Padi Rawa Gambut Blok B".to_string(),
            latitude: -2.2150,
            longitude: 113.9220,
            area_ha: 2.2,
            plant: "padi-gambut".to_string(),
        };

        let kepemilikan = Kepemilikan::new()
            .tetapkan_lengkap_murni("ESP32-GAMBUT-01", "petani_budi", plot1)
            .tetapkan_lengkap_murni("ESP32-GAMBUT-02", "petani_siti", plot2);

        Self {
            registry,
            kepemilikan,
            network_online: true,
            offline_buffer: Vec::new(),
            processed_events: HashSet::new(),
            last_sampling_time: HashMap::new(),
            m3_sink_records: Vec::new(),
        }
    }
}

/// Fungsi murni utama (Pipeline Komposisi):
/// Memproses satu [`RawReading`] dari perangkat IoT tanpa efek samping.
///
/// Mengembalikan pasangan `(GatewaySnapshot_Baru, PipelineOutcome)`.
pub fn process_sensor_event_pure(
    snapshot: &GatewaySnapshot,
    raw: RawReading,
) -> (GatewaySnapshot, PipelineOutcome) {
    // 1. Cek jika jaringan sedang Offline -> langsung simpan ke Offline Buffer (Immutable)
    if !snapshot.network_online {
        let next_buffer = enqueue_offline(
            &snapshot.offline_buffer,
            raw.clone(),
            "Sinyal modem SIM7000E terputus (Mode Offline)",
        );
        let count = next_buffer.len();
        let next_snapshot = GatewaySnapshot {
            offline_buffer: next_buffer,
            ..snapshot.clone()
        };
        return (
            next_snapshot,
            PipelineOutcome::BufferedOffline {
                buffered_count: count,
                message: format!(
                    "Jaringan OFFLINE: Event '{}' dari '{}' disimpan ke Offline Buffer (Total antrean: {})",
                    raw.event_id, raw.device_id, count
                ),
            },
        );
    }

    // 2. Cek duplikasi event_id (Idempotency)
    if snapshot.processed_events.contains(&raw.event_id) {
        return (
            snapshot.clone(),
            PipelineOutcome::Rejected {
                code: 409,
                reason: format!("Event '{}' sudah pernah diproses sebelumnya", raw.event_id),
            },
        );
    }

    // 3. Cek apakah perangkat terdaftar di Registry
    let device = match snapshot.registry.cari(&raw.device_id) {
        Some(d) => d,
        None => {
            return (
                snapshot.clone(),
                PipelineOutcome::Rejected {
                    code: 401,
                    reason: format!(
                        "Perangkat '{}' tidak terdaftar di DeviceDirectory Modul 2",
                        raw.device_id
                    ),
                },
            );
        }
    };

    // 4. Cek pemetaan pemilik & PlotMeta untuk kontrak Modul 3
    let (owner, plot_meta) = match (
        snapshot.kepemilikan.pemilik(&raw.device_id),
        snapshot.kepemilikan.plot_of(&raw.device_id),
    ) {
        (Some(o), Some(p)) => (o.clone(), p.clone()),
        _ => {
            return (
                snapshot.clone(),
                PipelineOutcome::Rejected {
                    code: 403,
                    reason: format!(
                        "Perangkat '{}' belum dipetakan ke pemilik atau PlotMeta",
                        raw.device_id
                    ),
                },
            );
        }
    };

    // 5. Cek Interval Sampling (Sampling Interval Control)
    if let Some(&last_ts) = snapshot.last_sampling_time.get(&raw.device_id)
        && !check_interval(raw.timestamp_sec, last_ts, device.sampling_interval_sec)
    {
        let selisih = raw.timestamp_sec.saturating_sub(last_ts);
        return (
            snapshot.clone(),
            PipelineOutcome::Rejected {
                code: 429,
                reason: format!(
                    "Sampling terlalu cepat ({}s < interval {}s). Menghemat baterai 18650 & kuota SIM7000E.",
                    selisih, device.sampling_interval_sec
                ),
            },
        );
    }

    // 6. Validasi & Transformasi Fungsional (RawReading -> SoilReading) + Deteksi Anomali Edge
    let soil_reading = match validate_and_transform(&raw, &plot_meta) {
        Ok(r) => r,
        Err(err_msg) => {
            return (
                snapshot.clone(),
                PipelineOutcome::Rejected {
                    code: 400,
                    reason: err_msg,
                },
            );
        }
    };

    let anomalies = check_anomaly(&raw);
    let buzzer_alarm_active = should_trigger_buzzer(soil_reading.quality);

    let envelope = M3IngestEnvelope {
        reading: soil_reading,
        plot_meta,
        owner_username: owner,
        anomalies,
        buzzer_alarm_active,
    };

    // 7. Bangun Snapshot Baru secara Immutable (Tanpa memutasi snapshot lama)
    let next_events: HashSet<String> = snapshot
        .processed_events
        .iter()
        .cloned()
        .chain(std::iter::once(raw.event_id.clone()))
        .collect();

    let next_last_time: HashMap<String, u64> = snapshot
        .last_sampling_time
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .chain(std::iter::once((raw.device_id.clone(), raw.timestamp_sec)))
        .collect();

    let next_m3_records: Vec<M3IngestEnvelope> = snapshot
        .m3_sink_records
        .iter()
        .cloned()
        .chain(std::iter::once(envelope.clone()))
        .collect();

    let next_snapshot = GatewaySnapshot {
        processed_events: next_events,
        last_sampling_time: next_last_time,
        m3_sink_records: next_m3_records,
        ..snapshot.clone()
    };

    let msg = if envelope.buzzer_alarm_active {
        format!(
            "Diterima dengan ALARM BUZZER AKTIF (Quality: {:?}) -> Diteruskan ke ReadingSink M3",
            envelope.reading.quality
        )
    } else {
        format!(
            "Validasi sukses (Quality: {:?}) -> SoilReading & PlotMeta diteruskan ke ReadingSink M3",
            envelope.reading.quality
        )
    };

    (
        next_snapshot,
        PipelineOutcome::ForwardedToM3 {
            envelope,
            message: msg,
        },
    )
}

/// Fungsi murni **Fold / Komposisi**: Saat koneksi kembali *Online* (*Upload-on-Reconnect*),
/// memproses seluruh item di dalam *Offline Buffer* secara berurutan menggunakan `.into_iter().fold(...)`.
pub fn flush_offline_buffer_pure(
    snapshot: &GatewaySnapshot,
) -> (GatewaySnapshot, Vec<PipelineOutcome>) {
    let online_snapshot = GatewaySnapshot {
        network_online: true,
        offline_buffer: Vec::new(),
        ..snapshot.clone()
    };

    let (batch_to_send, _) =
        take_reconnect_batch(&snapshot.offline_buffer, snapshot.offline_buffer.len());

    batch_to_send.into_iter().fold(
        (online_snapshot, Vec::new()),
        |(current_snap, outcomes), item| {
            let (next_snap, outcome) = process_sensor_event_pure(&current_snap, item.raw);
            let next_outcomes = outcomes
                .into_iter()
                .chain(std::iter::once(outcome))
                .collect();
            (next_snap, next_outcomes)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::QualityFlag;

    #[test]
    fn test_end_to_end_pure_pipeline_and_offline_reconnect() {
        let s0 = GatewaySnapshot::initial_seed();

        let raw1 = RawReading {
            event_id: "evt-100".to_string(),
            device_id: "ESP32-GAMBUT-01".to_string(),
            ts: "2026-11-01T10:00:00Z".to_string(),
            timestamp_sec: 1000,
            moisture_raw: 65.0,
            temperature_raw: 28.5,
            ph_raw: 4.4,
            ec_raw: 1.1,
        };

        // 1. Kirim saat Online -> Masuk ke M3 Sink dengan Quality Good
        let (s1, out1) = process_sensor_event_pure(&s0, raw1);
        assert!(matches!(out1, PipelineOutcome::ForwardedToM3 { .. }));
        assert_eq!(s1.m3_sink_records.len(), 1);
        assert_eq!(s1.m3_sink_records[0].reading.quality, QualityFlag::Good);

        // 2. Matikan jaringan (Offline) -> Kirim 2 data -> Masuk ke Offline Buffer
        let s_offline = GatewaySnapshot {
            network_online: false,
            ..s1
        };
        let raw2 = RawReading {
            event_id: "evt-101".to_string(),
            device_id: "ESP32-GAMBUT-01".to_string(),
            ts: "2026-11-01T10:00:20Z".to_string(),
            timestamp_sec: 1020,
            moisture_raw: 62.0,
            temperature_raw: 29.0,
            ph_raw: 4.3,
            ec_raw: 1.2,
        };
        let (s2, out2) = process_sensor_event_pure(&s_offline, raw2);
        assert!(matches!(out2, PipelineOutcome::BufferedOffline { .. }));
        assert_eq!(s2.offline_buffer.len(), 1);
        assert_eq!(s2.m3_sink_records.len(), 1); // Belum bertambah di M3

        // 3. Reconnect (Upload-on-Reconnect dengan fold murni) -> Buffer terkirim ke M3!
        let (s3, flush_outcomes) = flush_offline_buffer_pure(&s2);
        assert_eq!(flush_outcomes.len(), 1);
        assert!(s3.offline_buffer.is_empty());
        assert_eq!(s3.m3_sink_records.len(), 2);
    }
}
