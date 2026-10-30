//! M5 - Plant-Specific Expert System.
//!
//! Implementasi awal: aturan tetap yang menerjemahkan skor risiko menjadi tindakan,
//! dengan penjelasan dan daftar aturan yang menyala.
//! TODO kelompok 5: definisi ambang per tanaman dan fase tumbuh, antarmuka penulisan/validasi
//! aturan oleh pakar, mesin inferensi, versi aturan.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use soil_contracts::{
    AnalysisResult, Confidence, GrowthStage, PlantId, Plugin, PluginHost, PluginKind, PluginMeta,
    Priority, Provenance, Recommendation, Result, RiskKind, RiskLevel, RuleEvaluator, RuleRef,
};

pub const ID: &str = "m5.basic_rules";
pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Default)]
pub struct BasicRuleEvaluator;

fn level_of(analysis: &AnalysisResult, kind: RiskKind) -> RiskLevel {
    analysis
        .risks
        .iter()
        .find(|r| r.kind == kind)
        .map(|r| r.level)
        .unwrap_or(RiskLevel::Low)
}

fn min_confidence(analysis: &AnalysisResult) -> Confidence {
    let min = analysis
        .risks
        .iter()
        .map(|r| r.confidence.value())
        .fold(1.0_f32, f32::min);
    Confidence::new(min)
}

#[async_trait]
impl RuleEvaluator for BasicRuleEvaluator {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            id: ID.into(),
            name: "Basic plant rules".into(),
            version: VERSION.into(),
            kind: PluginKind::ExpertRules,
            description: "Aturan dasar risiko tanah menjadi tindakan, dengan penjelasan".into(),
            requires_contracts_major: 0,
        }
    }

    async fn evaluate(
        &self,
        plant: &PlantId,
        stage: GrowthStage,
        analysis: &AnalysisResult,
    ) -> Result<Recommendation> {
        let wet = level_of(analysis, RiskKind::Waterlogging);
        let dry = level_of(analysis, RiskKind::Dryness);

        let mut explanation = vec![format!(
            "Tanaman {plant}, fase {stage:?}; risiko waterlogging={wet:?}, kekeringan={dry:?}."
        )];
        let mut fired = Vec::new();

        let (action, priority) = if wet >= RiskLevel::High {
            fired.push(RuleRef {
                rule_id: "R-WET-01".into(),
                rule_version: VERSION.into(),
            });
            explanation.push("R-WET-01: risiko waterlogging tinggi, drainase perlu dibuka.".into());
            (
                "Buka/perbaiki drainase dan tunda penyiraman".to_string(),
                if wet == RiskLevel::Critical {
                    Priority::Urgent
                } else {
                    Priority::Normal
                },
            )
        } else if dry >= RiskLevel::High {
            fired.push(RuleRef {
                rule_id: "R-DRY-01".into(),
                rule_version: VERSION.into(),
            });
            explanation.push("R-DRY-01: risiko kekeringan tinggi, lahan perlu dibasahi.".into());
            (
                "Lakukan irigasi/pembasahan lahan".to_string(),
                if dry == RiskLevel::Critical {
                    Priority::Urgent
                } else {
                    Priority::Normal
                },
            )
        } else {
            fired.push(RuleRef {
                rule_id: "R-OK-01".into(),
                rule_version: VERSION.into(),
            });
            explanation.push("R-OK-01: tidak ada risiko tinggi, kondisi dipertahankan.".into());
            ("Pertahankan kondisi saat ini".to_string(), Priority::Low)
        };

        Ok(Recommendation {
            plot_id: analysis.plot_id,
            plant_id: plant.clone(),
            stage,
            action,
            priority,
            confidence: min_confidence(analysis),
            explanation,
            fired_rules: fired,
            provenance: Provenance {
                source: ID.into(),
                version: VERSION.into(),
                method: "forward_chaining_fixed_rules".into(),
                inputs: vec![
                    format!("analysis_source={}", analysis.provenance.source),
                    format!("analysis_version={}", analysis.provenance.version),
                    format!("samples={}", analysis.sample_count),
                ],
                generated_at: Utc::now(),
            },
        })
    }
}

impl Plugin for BasicRuleEvaluator {
    fn meta(&self) -> PluginMeta {
        RuleEvaluator::meta(self)
    }

    fn register(&self, host: &mut dyn PluginHost) -> Result<()> {
        host.register_rule_evaluator(Arc::new(self.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soil_contracts::{PlotId, RiskScore};

    fn analysis(wet: RiskLevel, dry: RiskLevel) -> AnalysisResult {
        let now = Utc::now();
        let score = |kind, level| RiskScore {
            kind,
            score: 0.5,
            level,
            confidence: Confidence::new(0.8),
        };
        AnalysisResult {
            plot_id: PlotId::new(),
            window_from: now,
            window_to: now,
            risks: vec![
                score(RiskKind::Waterlogging, wet),
                score(RiskKind::Dryness, dry),
            ],
            anomalies: vec![],
            trends: vec![],
            sample_count: 24,
            provenance: Provenance {
                source: "test".into(),
                version: "0".into(),
                method: "manual".into(),
                inputs: vec![],
                generated_at: now,
            },
        }
    }

    #[tokio::test]
    async fn wet_soil_recommends_drainage() {
        let rec = BasicRuleEvaluator
            .evaluate(
                &PlantId::new("nanas"),
                GrowthStage::Vegetative,
                &analysis(RiskLevel::Critical, RiskLevel::Low),
            )
            .await
            .unwrap();
        assert!(rec.action.contains("drainase"));
        assert_eq!(rec.priority, Priority::Urgent);
    }

    #[tokio::test]
    async fn fired_rules_are_recorded() {
        let rec = BasicRuleEvaluator
            .evaluate(
                &PlantId::new("nanas"),
                GrowthStage::Fruiting,
                &analysis(RiskLevel::Low, RiskLevel::High),
            )
            .await
            .unwrap();
        assert_eq!(rec.fired_rules[0].rule_id, "R-DRY-01");
        assert!(!rec.explanation.is_empty());
        assert_eq!(rec.provenance.source, ID);
        assert!((rec.confidence.value() - 0.8).abs() < 1e-6);
    }
}
