//! Anggota 3: Sistem Pendukung Keputusan (DSS) Irigasi.

#[derive(Debug, Clone, PartialEq)]
pub struct IrrigationRecommendation {
    pub water_volume_liters_per_m2: f32,
    pub recommended_duration_minutes: u32,
    pub reason: String,
}

/// Hitung rekomendasi irigasi berdasarkan defisit kadar air.
pub fn calculate_irrigation(
    current_moisture: f32,
    target_moisture: f32,
    emitter_flow_rate_lpm: f32,
) -> Option<IrrigationRecommendation> {
    if current_moisture >= target_moisture {
        return None;
    }

    let deficit_pct = target_moisture - current_moisture;
    // Asumsi sederhana: 1% defisit kelembapan di kedalaman perakaran setara 2.5 L/m2
    let water_volume = deficit_pct * 2.5;
    let duration = if emitter_flow_rate_lpm > 0.0 {
        (water_volume / emitter_flow_rate_lpm).ceil() as u32
    } else {
        0
    };

    Some(IrrigationRecommendation {
        water_volume_liters_per_m2: water_volume,
        recommended_duration_minutes: duration,
        reason: format!(
            "Kelembapan ({:.1}%) di bawah target ({:.1}%). Defisit {:.1}%.",
            current_moisture, target_moisture, deficit_pct
        ),
    })
}