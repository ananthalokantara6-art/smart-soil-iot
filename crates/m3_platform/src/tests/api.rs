use super::*;
use soil_contracts::{AlertLevel, AlertSink, Provenance, QualityFlag};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn spawn(app: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// Klien HTTP/1.1 minimal (tanpa dependensi baru): kirim satu permintaan, baca semua balasan.
async fn call(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if !body.is_empty() {
        req.push_str("Content-Type: application/json\r\n");
    }
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8_lossy(&raw).to_string();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status HTTP");
    let body = text
        .split_once("\r\n\r\n")
        .map_or("", |(_, rest)| rest)
        .to_string();
    (status, body)
}

fn json(body: &str) -> serde_json::Value {
    serde_json::from_str(body).expect("balasan harus JSON")
}

fn sample(plot: PlotId, ts: DateTime<Utc>) -> SoilReading {
    SoilReading {
        device_id: DeviceId::new(),
        plot_id: plot,
        ts,
        moisture_pct: 60.0,
        temperature_c: 27.0,
        ph: 4.0,
        ec_ms_cm: 0.5,
        quality: QualityFlag::Good,
    }
}

fn sample_alert(plot: PlotId, message: &str, minutes_ago: i64) -> Alert {
    let ts = Utc::now() - Duration::minutes(minutes_ago);
    Alert {
        plot_id: plot,
        ts,
        level: AlertLevel::Warning,
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

fn register(platform: &MemoryPlatform, name: &str) -> PlotId {
    let meta = PlotMeta::from_new(NewPlot {
        name: name.into(),
        latitude: None,
        longitude: None,
        area_ha: None,
        plant: None,
    })
    .unwrap();
    let id = meta.plot_id;
    platform.register_plot(meta).unwrap();
    id
}

#[test]
fn dashboard_files_reference_each_other_and_overview() {
    assert!(DASHBOARD_JS.contains("/api/v1/overview"));
    assert!(DASHBOARD_HTML.contains("/style.css"));
    assert!(DASHBOARD_HTML.contains("/script.js"));
}

#[test]
fn token_comparison() {
    assert!(tokens_match("rahasia", Some("rahasia")));
    assert!(!tokens_match("rahasia", Some("rahasiA")));
    assert!(!tokens_match("rahasia", Some("rahasia!")));
    assert!(!tokens_match("rahasia", None));
}

#[test]
fn plot_id_list_parsing() {
    let a = PlotId::new();
    let b = PlotId::new();
    let ok = parse_plot_ids(&format!("{a}, {b} ,")).unwrap();
    assert_eq!(ok, vec![a, b]);
    assert!(parse_plot_ids("").is_err());
    assert!(parse_plot_ids(" , ").is_err());
    assert!(parse_plot_ids("bukan-uuid").is_err());
}

#[tokio::test]
async fn dashboard_and_assets_are_served() {
    let addr = spawn(router(Arc::new(MemoryPlatform::new()))).await;
    let (status, body) = call(addr, "GET", "/", &[], "").await;
    assert_eq!(status, 200);
    assert!(body.contains("Smart Soil IoT"));
    assert_eq!(call(addr, "GET", "/style.css", &[], "").await.0, 200);
    assert_eq!(call(addr, "GET", "/script.js", &[], "").await.0, 200);
}

#[tokio::test]
async fn post_reading_then_read_latest_and_reject_duplicate() {
    let addr = spawn(router(Arc::new(MemoryPlatform::new()))).await;
    let plot = PlotId::new();
    let body = serde_json::to_string(&sample(plot, Utc::now() - Duration::minutes(1))).unwrap();

    let (status, _) = call(addr, "POST", "/api/v1/readings", &[], &body).await;
    assert_eq!(status, 202);
    let (status, resp) = call(addr, "POST", "/api/v1/readings", &[], &body).await;
    assert_eq!(status, 409);
    assert_eq!(json(&resp)["inserted"], false);

    let (status, resp) = call(
        addr,
        "GET",
        &format!("/api/v1/plots/{plot}/latest"),
        &[],
        "",
    )
    .await;
    assert_eq!(status, 200);
    assert!(json(&resp)["moisture_pct"].is_number());
}

#[tokio::test]
async fn latest_without_data_is_404_with_json_error() {
    let addr = spawn(router(Arc::new(MemoryPlatform::new()))).await;
    let path = format!("/api/v1/plots/{}/latest", PlotId::new());
    let (status, resp) = call(addr, "GET", &path, &[], "").await;
    assert_eq!(status, 404);
    assert!(json(&resp)["error"].is_string());
}

#[tokio::test]
async fn invalid_readings_are_rejected() {
    let addr = spawn(router(Arc::new(MemoryPlatform::new()))).await;
    let mut bad = sample(PlotId::new(), Utc::now());
    bad.moisture_pct = 150.0;
    let body = serde_json::to_string(&bad).unwrap();
    assert_eq!(
        call(addr, "POST", "/api/v1/readings", &[], &body).await.0,
        400
    );

    let future = sample(PlotId::new(), Utc::now() + Duration::days(30));
    let body = serde_json::to_string(&future).unwrap();
    assert_eq!(
        call(addr, "POST", "/api/v1/readings", &[], &body).await.0,
        400
    );

    // JSON rusak ditolak oleh extractor (bukan 2xx)
    let (status, _) = call(addr, "POST", "/api/v1/readings", &[], "{bukan json").await;
    assert!((400..500).contains(&status));
}

#[tokio::test]
async fn batch_counts_inserted_duplicates_and_rejected() {
    let addr = spawn(router(Arc::new(MemoryPlatform::new()))).await;
    let plot = PlotId::new();
    let good = sample(plot, Utc::now() - Duration::minutes(2));
    let mut bad = sample(plot, Utc::now() - Duration::minutes(1));
    bad.ph = 20.0;
    let future = sample(plot, Utc::now() + Duration::days(1));
    let batch = vec![good.clone(), good, bad, future];

    let body = serde_json::to_string(&batch).unwrap();
    let (status, resp) = call(addr, "POST", "/api/v1/readings/batch", &[], &body).await;
    assert_eq!(status, 202);
    let v = json(&resp);
    assert_eq!(v["received"], 4);
    assert_eq!(v["inserted"], 1);
    assert_eq!(v["duplicates"], 1);
    assert_eq!(v["rejected"], 2);
}

#[tokio::test]
async fn write_endpoints_require_token_when_configured() {
    let app = router_with_token(Arc::new(MemoryPlatform::new()), Some("rahasia".into()));
    let addr = spawn(app).await;
    let body = serde_json::to_string(&sample(PlotId::new(), Utc::now())).unwrap();

    assert_eq!(
        call(addr, "POST", "/api/v1/readings", &[], &body).await.0,
        401
    );
    assert_eq!(
        call(
            addr,
            "POST",
            "/api/v1/readings",
            &[("x-api-key", "salah")],
            &body
        )
        .await
        .0,
        401
    );
    assert_eq!(
        call(
            addr,
            "POST",
            "/api/v1/readings",
            &[("x-api-key", "rahasia")],
            &body
        )
        .await
        .0,
        202
    );
    assert_eq!(
        call(addr, "POST", "/api/v1/plots", &[], r#"{"name":"Petak X"}"#)
            .await
            .0,
        401
    );
    assert_eq!(
        call(
            addr,
            "POST",
            "/api/v1/plots",
            &[("x-api-key", "rahasia")],
            r#"{"name":"Petak X"}"#
        )
        .await
        .0,
        201
    );
    // pembacaan data tetap terbuka untuk dashboard
    assert_eq!(call(addr, "GET", "/api/v1/plots", &[], "").await.0, 200);
}

#[tokio::test]
async fn create_plot_validates_input() {
    let addr = spawn(router(Arc::new(MemoryPlatform::new()))).await;
    let (status, _) = call(addr, "POST", "/api/v1/plots", &[], r#"{"name":"  "}"#).await;
    assert_eq!(status, 400);
    let (status, resp) = call(
        addr,
        "POST",
        "/api/v1/plots",
        &[],
        r#"{"name":"Petak A","latitude":-0.5,"longitude":117.1,"area_ha":1.5}"#,
    )
    .await;
    assert_eq!(status, 201);
    assert!(json(&resp)["plot_id"].is_string());
}

#[tokio::test]
async fn series_validates_range_and_aggregates() {
    let platform = Arc::new(MemoryPlatform::new());
    let plot = register(&platform, "Petak A");
    let now = Utc::now();
    for i in 1..=6 {
        platform
            .ingest_checked(sample(plot, now - Duration::minutes(i * 10)))
            .unwrap();
    }
    let addr = spawn(router(platform)).await;

    let bad_range = format!(
        "/api/v1/plots/{plot}/series?param=ph&from=2026-01-02T00:00:00Z&to=2026-01-01T00:00:00Z"
    );
    assert_eq!(call(addr, "GET", &bad_range, &[], "").await.0, 400);

    let bad_interval = format!("/api/v1/plots/{plot}/series?param=ph&interval_secs=0");
    assert_eq!(call(addr, "GET", &bad_interval, &[], "").await.0, 400);

    let ok = format!("/api/v1/plots/{plot}/series?param=ph&interval_secs=3600");
    let (status, resp) = call(addr, "GET", &ok, &[], "").await;
    assert_eq!(status, 200);
    let v = json(&resp);
    assert!(!v["buckets"].as_array().unwrap().is_empty());
    assert_eq!(v["parameter"], "ph");
}

#[tokio::test]
async fn health_reports_no_data_then_online() {
    let platform = Arc::new(MemoryPlatform::new());
    let plot = PlotId::new();
    let addr = spawn(router(platform.clone())).await;
    let path = format!("/api/v1/plots/{plot}/health");

    let (status, resp) = call(addr, "GET", &path, &[], "").await;
    assert_eq!(status, 200);
    assert_eq!(json(&resp)["status"], "no_data");

    platform
        .ingest_checked(sample(plot, Utc::now() - Duration::minutes(1)))
        .unwrap();
    let (_, resp) = call(addr, "GET", &path, &[], "").await;
    assert_eq!(json(&resp)["status"], "online");
}

#[tokio::test]
async fn compare_validates_input_and_ranks_plots() {
    let platform = Arc::new(MemoryPlatform::new());
    let (a, b) = (register(&platform, "A"), register(&platform, "B"));
    for (plot, moisture) in [(a, 40.0_f32), (b, 80.0_f32)] {
        let mut r = sample(plot, Utc::now() - Duration::minutes(5));
        r.moisture_pct = moisture;
        platform.ingest_checked(r).unwrap();
    }
    let addr = spawn(router(platform)).await;

    assert_eq!(
        call(
            addr,
            "GET",
            "/api/v1/compare?plots=&param=moisture",
            &[],
            ""
        )
        .await
        .0,
        400
    );
    assert_eq!(
        call(
            addr,
            "GET",
            "/api/v1/compare?plots=x&param=moisture",
            &[],
            ""
        )
        .await
        .0,
        400
    );

    let ok = format!("/api/v1/compare?plots={a},{b}&param=moisture");
    let (status, resp) = call(addr, "GET", &ok, &[], "").await;
    assert_eq!(status, 200);
    let v = json(&resp);
    assert_eq!(v.as_array().unwrap().len(), 2);
    assert_eq!(v[0]["plot_id"], b.to_string());
}

#[tokio::test]
async fn alerts_can_be_filtered_and_deduplicated() {
    let platform = Arc::new(MemoryPlatform::new());
    let plot = PlotId::new();
    for minutes_ago in [30, 20, 10] {
        platform
            .raise(sample_alert(plot, "kering", minutes_ago))
            .await
            .unwrap();
    }
    platform
        .raise(sample_alert(PlotId::new(), "lain", 5))
        .await
        .unwrap();
    let addr = spawn(router(platform)).await;

    let (_, all) = call(addr, "GET", "/api/v1/alerts", &[], "").await;
    assert_eq!(json(&all).as_array().unwrap().len(), 4);

    let (_, one_plot) = call(
        addr,
        "GET",
        &format!("/api/v1/alerts?plot_id={plot}"),
        &[],
        "",
    )
    .await;
    assert_eq!(json(&one_plot).as_array().unwrap().len(), 3);

    let (_, deduped) = call(
        addr,
        "GET",
        &format!("/api/v1/alerts?plot_id={plot}&dedupe_secs=3600"),
        &[],
        "",
    )
    .await;
    assert_eq!(json(&deduped).as_array().unwrap().len(), 1);

    let (_, none) = call(addr, "GET", "/api/v1/alerts?level=critical", &[], "").await;
    assert!(json(&none).as_array().unwrap().is_empty());
}

#[tokio::test]
async fn overview_on_empty_platform_is_valid() {
    let addr = spawn(router(Arc::new(MemoryPlatform::new()))).await;
    let (status, resp) = call(addr, "GET", "/api/v1/overview", &[], "").await;
    assert_eq!(status, 200);
    let v = json(&resp);
    assert!(v["plots"].as_array().unwrap().is_empty());
    assert!(v["alerts"].as_array().unwrap().is_empty());
    assert_eq!(v["hours"], 24);
}

#[tokio::test]
async fn overview_returns_actual_data_for_dashboard() {
    let platform = Arc::new(MemoryPlatform::new());
    let with_data = register(&platform, "Petak Aktif");
    let _empty = register(&platform, "Petak Kosong");
    for minutes_ago in [30, 5, 1] {
        platform
            .ingest_checked(sample(
                with_data,
                Utc::now() - Duration::minutes(minutes_ago),
            ))
            .unwrap();
    }
    platform
        .raise(sample_alert(with_data, "kering", 3))
        .await
        .unwrap();
    platform
        .raise(sample_alert(with_data, "kering", 2))
        .await
        .unwrap();
    let addr = spawn(router(platform)).await;

    let (status, resp) = call(addr, "GET", "/api/v1/overview?hours=1&slots=6", &[], "").await;
    assert_eq!(status, 200);
    let v = json(&resp);
    assert_eq!(v["slots"], 6);

    let plots = v["plots"].as_array().unwrap();
    assert_eq!(plots.len(), 2);
    let active = plots
        .iter()
        .find(|p| p["plot"]["name"] == "Petak Aktif")
        .unwrap();
    let empty = plots
        .iter()
        .find(|p| p["plot"]["name"] == "Petak Kosong")
        .unwrap();

    assert!(active["latest"]["moisture_pct"].is_number());
    assert_eq!(active["series"]["moisture"].as_array().unwrap().len(), 6);
    assert!(active["series"]["moisture"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x.is_number()));
    assert_eq!(active["health"]["status"], "online");

    assert!(empty["latest"].is_null());
    assert_eq!(empty["health"]["status"], "no_data");

    // dua alert identik dalam jendela cooldown ditampilkan sekali
    assert_eq!(v["alerts"].as_array().unwrap().len(), 1);
}
