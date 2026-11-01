use super::*;
use chrono::TimeZone;
use soil_contracts::Provenance;

fn t0() -> DateTime<Utc> {
    Utc.timestamp_opt(1_800_000_000, 0).single().unwrap()
}

fn alert(plot: PlotId, level: AlertLevel, message: &str, offset_min: i64) -> Alert {
    let ts = t0() + Duration::minutes(offset_min);
    Alert {
        plot_id: plot,
        ts,
        level,
        message: message.into(),
        provenance: Provenance {
            source: "test".into(),
            version: "0".into(),
            method: "manual".into(),
            inputs: vec![],
            generated_at: ts,
        },
    }
}

#[test]
fn filter_by_plot_level_and_time() {
    let (a, b) = (PlotId::new(), PlotId::new());
    let all = vec![
        alert(a, AlertLevel::Info, "i", 0),
        alert(a, AlertLevel::Warning, "w", 10),
        alert(a, AlertLevel::Critical, "c", 20),
        alert(b, AlertLevel::Critical, "c-b", 30),
    ];

    let by_plot = filter_alerts(
        &all,
        &AlertFilter {
            plot_id: Some(a),
            ..AlertFilter::default()
        },
    );
    assert_eq!(by_plot.len(), 3);

    let by_level = filter_alerts(
        &all,
        &AlertFilter {
            min_level: Some(AlertLevel::Warning),
            ..AlertFilter::default()
        },
    );
    assert_eq!(by_level.len(), 3);

    let by_time = filter_alerts(
        &all,
        &AlertFilter {
            from: Some(t0() + Duration::minutes(10)),
            to: Some(t0() + Duration::minutes(20)),
            ..AlertFilter::default()
        },
    );
    assert_eq!(by_time.len(), 2);
}

#[test]
fn filter_returns_newest_first_and_keeps_input() {
    let p = PlotId::new();
    let all = vec![
        alert(p, AlertLevel::Info, "lama", 0),
        alert(p, AlertLevel::Info, "baru", 5),
    ];
    let out = filter_alerts(&all, &AlertFilter::default());
    assert_eq!(out[0].message, "baru");
    assert_eq!(all[0].message, "lama");
}

#[test]
fn dedupe_drops_repeats_inside_cooldown_only() {
    let p = PlotId::new();
    let all = vec![
        alert(p, AlertLevel::Warning, "kering", 0),
        alert(p, AlertLevel::Warning, "kering", 5),
        alert(p, AlertLevel::Warning, "kering", 31),
        alert(p, AlertLevel::Warning, "pH", 6),
    ];
    let out = dedupe_alerts(&all, Duration::minutes(30));
    let messages: Vec<&str> = out.iter().map(|a| a.message.as_str()).collect();
    assert_eq!(messages, vec!["kering", "pH", "kering"]);
}

#[test]
fn dedupe_treats_other_plot_or_level_as_different() {
    let (a, b) = (PlotId::new(), PlotId::new());
    let all = vec![
        alert(a, AlertLevel::Warning, "x", 0),
        alert(b, AlertLevel::Warning, "x", 1),
        alert(a, AlertLevel::Critical, "x", 2),
    ];
    assert_eq!(dedupe_alerts(&all, Duration::hours(1)).len(), 3);
}

#[test]
fn filter_and_dedupe_is_opt_in() {
    let p = PlotId::new();
    let all = vec![
        alert(p, AlertLevel::Warning, "x", 0),
        alert(p, AlertLevel::Warning, "x", 1),
    ];
    assert_eq!(filter_and_dedupe(&all, &AlertFilter::default()).len(), 2);

    let with_dedupe = AlertFilter {
        dedupe_secs: Some(600),
        ..AlertFilter::default()
    };
    assert_eq!(filter_and_dedupe(&all, &with_dedupe).len(), 1);

    let non_positive = AlertFilter {
        dedupe_secs: Some(0),
        ..AlertFilter::default()
    };
    assert_eq!(filter_and_dedupe(&all, &non_positive).len(), 2);
}

#[test]
fn empty_input_gives_empty_output() {
    assert!(filter_alerts(&[], &AlertFilter::default()).is_empty());
    assert!(dedupe_alerts(&[], Duration::minutes(1)).is_empty());
}

