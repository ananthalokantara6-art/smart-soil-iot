use crate::aggregate::{
    auto_interval_secs, bucketize, period_change, rank_by_mean, sensor_health, summarize, Bucket,
    Change, Health, ParamSummary,
};
use crate::alerts::{filter_and_dedupe, AlertFilter};
use crate::overview::{build_plot_overview, OverviewWindow, PlotOverview};
use crate::plots::{NewPlot, PlotMeta};
use crate::MemoryPlatform;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use soil_contracts::{
    Alert, ContractError, DeviceId, Parameter, PlotId, ReadingSource, SoilReading,
};
use std::sync::Arc;

const DASHBOARD_HTML: &str = include_str!("../../../web/index.html");
const DASHBOARD_CSS: &str = include_str!("../../../web/style.css");
const DASHBOARD_JS: &str = include_str!("../../../web/script.js");

const DEFAULT_EXPECTED_SECS: i64 = 600;
const MAX_RANGE_HOURS: i64 = 24 * 90;
const DEFAULT_RANGE_HOURS: i64 = 24;
const DEFAULT_SLOTS: usize = 24;
const MAX_SLOTS: usize = 240;
const MAX_COMPARE_PLOTS: usize = 20;
const OVERVIEW_ALERT_COOLDOWN_SECS: i64 = 1800;
const OVERVIEW_ALERT_LIMIT: usize = 200;

#[derive(Clone)]
struct AppState {
    platform: Arc<MemoryPlatform>,
    ingest_token: Option<String>, // kata sandi (boleh kosong)
}

/// Router tanpa autentikasi (untuk demo dan tes).
pub fn router(platform: Arc<MemoryPlatform>) -> Router {
    router_with_token(platform, None)
}

/// Router dengan token pengiriman data (dipakai M2 lewat HTTP).
pub fn router_with_token(platform: Arc<MemoryPlatform>, ingest_token: Option<String>) -> Router {
    let state: AppState = AppState {
        platform,
        ingest_token,
    };
    Router::new()
        .route("/", get(dashboard))
        .route("/style.css", get(style_css))
        .route("/script.js", get(script_js))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/plots", get(list_plots).post(create_plot))
        .route("/api/v1/plots/{id}/latest", get(latest))
        .route("/api/v1/plots/{id}/series", get(series))
        .route("/api/v1/plots/{id}/health", get(health))
        .route("/api/v1/alerts", get(alerts))
        .route("/api/v1/compare", get(compare))
        .route("/api/v1/readings", post(post_reading))
        .route("/api/v1/readings/batch", post(post_batch))
        .with_state(state)
}

#[derive(Debug)]
struct ApiError(ContractError);

impl From<ContractError> for ApiError {
    fn from(e: ContractError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            ContractError::Validation(_) => StatusCode::BAD_REQUEST,
            ContractError::NotFound(_) => StatusCode::NOT_FOUND,
            ContractError::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({ "error": self.0.to_string() });
        (status, Json(body)).into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

fn bad_request(msg: &str) -> ApiError {
    ApiError(ContractError::Validation(msg.to_string()))
}

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn style_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        DASHBOARD_CSS,
    )
}

async fn script_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        DASHBOARD_JS,
    )
}

async fn list_plots(State(s): State<AppState>) -> ApiResult<Vec<PlotMeta>> {
    Ok(Json(s.platform.list_plots()?))
}

async fn create_plot(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(new): Json<NewPlot>,
) -> Result<(StatusCode, Json<PlotMeta>), ApiError> {
    check_token(&s, &headers)?;
    let meta = PlotMeta::from_new(new)?;
    s.platform.register_plot(meta.clone())?;
    Ok((StatusCode::CREATED, Json(meta)))
}

async fn latest(State(s): State<AppState>, Path(id): Path<PlotId>) -> ApiResult<SoilReading> {
    let reading = s
        .platform
        .latest(&id)
        .await?
        .ok_or_else(|| ContractError::NotFound(format!("belum ada data untuk petak {id}")))?;
    Ok(Json(reading))
}

#[derive(Deserialize)]
struct SeriesQuery {
    param: Parameter,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    interval_secs: Option<i64>,
}

#[derive(Serialize)]
struct SeriesResponse {
    plot_id: PlotId,
    parameter: Parameter,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    interval_secs: i64,
    buckets: Vec<Bucket>,
    change: Option<Change>,
}

async fn series(
    State(s): State<AppState>,
    Path(id): Path<PlotId>,
    Query(q): Query<SeriesQuery>,
) -> ApiResult<SeriesResponse> {
    let to = q.to.unwrap_or_else(Utc::now);
    let from = q.from.unwrap_or(to - Duration::hours(DEFAULT_RANGE_HOURS));
    if from >= to {
        return Err(bad_request("'from' harus lebih awal dari 'to'"));
    }
    let interval = q
        .interval_secs
        .unwrap_or_else(|| auto_interval_secs((to - from).num_seconds()));
    if interval <= 0 {
        return Err(bad_request("interval_secs harus > 0"));
    }

    let window = s.platform.window(&id, from, to).await?;
    let points: Vec<_> = window.series(q.param);
    let buckets = bucketize(&points, interval);
    let change = period_change(&buckets);
    Ok(Json(SeriesResponse {
        plot_id: id,
        parameter: q.param,
        from,
        to,
        interval_secs: interval,
        buckets,
        change,
    }))
}

#[derive(Deserialize)]
struct HealthQuery {
    expected_interval_secs: Option<i64>,
}

#[derive(Serialize)]
struct HealthResponse {
    plot_id: PlotId,
    device_id: Option<DeviceId>,
    #[serde(flatten)]
    health: Health,
}

async fn health(
    State(s): State<AppState>,
    Path(id): Path<PlotId>,
    Query(q): Query<HealthQuery>,
) -> ApiResult<HealthResponse> {
    let now = Utc::now();
    let expected = q.expected_interval_secs.unwrap_or(DEFAULT_EXPECTED_SECS);
    let mut readings = s
        .platform
        .window(&id, now - Duration::hours(DEFAULT_RANGE_HOURS), now)
        .await?
        .readings;
    if readings.is_empty() {
        readings.extend(s.platform.latest(&id).await?);
    }
    let device_id = readings.last().map(|r| r.device_id);
    Ok(Json(HealthResponse {
        plot_id: id,
        device_id,
        health: sensor_health(&readings, now, expected),
    }))
}

async fn alerts(State(s): State<AppState>, Query(f): Query<AlertFilter>) -> ApiResult<Vec<Alert>> {
    let all = s.platform.all_alerts()?;
    Ok(Json(filter_and_dedupe(&all, &f)))
}

#[derive(Deserialize)]
struct CompareQuery {
    plots: String,
    param: Parameter,
    hours: Option<i64>,
}

fn parse_plot_ids(raw: &str) -> Result<Vec<PlotId>, ApiError> {
    let ids = raw
        .split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(|part| {
            part.parse::<uuid::Uuid>()
                .map(PlotId)
                .map_err(|_| bad_request("id petak tidak valid"))
        })
        .collect::<Result<Vec<PlotId>, ApiError>>()?;
    if ids.is_empty() {
        return Err(bad_request("parameter 'plots' tidak boleh kosong"));
    }
    if ids.len() > MAX_COMPARE_PLOTS {
        return Err(bad_request("terlalu banyak petak untuk dibandingkan"));
    }
    Ok(ids)
}

async fn compare(
    State(s): State<AppState>,
    Query(q): Query<CompareQuery>,
) -> ApiResult<Vec<ParamSummary>> {
    let hours = q
        .hours
        .unwrap_or(DEFAULT_RANGE_HOURS)
        .clamp(1, MAX_RANGE_HOURS);
    let ids = parse_plot_ids(&q.plots)?;
    let now = Utc::now();
    let mut summaries = Vec::new();
    for id in ids {
        let window = s
            .platform
            .window(&id, now - Duration::hours(hours), now)
            .await?;
        summaries.extend(summarize(id, &window.readings, q.param));
    }
    Ok(Json(rank_by_mean(summaries)))
}

#[derive(Deserialize)]
struct OverviewQuery {
    hours: Option<i64>,
    slots: Option<usize>,
    expected_interval_secs: Option<i64>,
}

#[derive(Serialize)]
struct OverviewResponse {
    hours: i64,
    slots: usize,
    generated_at: DateTime<Utc>,
    plots: Vec<PlotOverview>,
    alerts: Vec<Alert>,
}

async fn overview(
    State(s): State<AppState>,
    Query(q): Query<OverviewQuery>,
) -> ApiResult<OverviewResponse> {
    let hours = q
        .hours
        .unwrap_or(DEFAULT_RANGE_HOURS)
        .clamp(1, MAX_RANGE_HOURS);
    let slots = q.slots.unwrap_or(DEFAULT_SLOTS).clamp(2, MAX_SLOTS);
    let expected_secs = q
        .expected_interval_secs
        .unwrap_or(DEFAULT_EXPECTED_SECS)
        .max(1);
    let now = Utc::now();
    let w = OverviewWindow {
        from: now - Duration::hours(hours),
        to: now,
        slots,
        expected_secs,
    };

    let mut plots = Vec::new();
    for meta in s.platform.list_plots()? {
        let window = s.platform.window(&meta.plot_id, w.from, w.to).await?;
        let latest = s.platform.latest(&meta.plot_id).await?;
        plots.push(build_plot_overview(meta, &window.readings, latest, w));
    }

    let filter = AlertFilter {
        dedupe_secs: Some(OVERVIEW_ALERT_COOLDOWN_SECS),
        ..AlertFilter::default()
    };
    let mut alerts = filter_and_dedupe(&s.platform.all_alerts()?, &filter);
    alerts.truncate(OVERVIEW_ALERT_LIMIT);

    Ok(Json(OverviewResponse {
        hours,
        slots,
        generated_at: now,
        plots,
        alerts,
    }))
}

fn tokens_match(expected: &str, given: Option<&str>) -> bool {
    given.is_some_and(|g| {
        let (a, b) = (expected.as_bytes(), g.as_bytes());
        a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
    })
}

fn check_token(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(expected) = &state.ingest_token else {
        return Ok(());
    };
    let given = headers.get("x-api-key").and_then(|v| v.to_str().ok());
    if tokens_match(expected, given) {
        Ok(())
    } else {
        Err(ContractError::Unauthorized.into())
    }
}

#[derive(Serialize)]
struct IngestResult {
    inserted: bool,
}

async fn post_reading(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(reading): Json<SoilReading>,
) -> Result<(StatusCode, Json<IngestResult>), ApiError> {
    check_token(&s, &headers)?;
    let inserted = s.platform.ingest_checked(reading)?;
    let status = if inserted {
        StatusCode::ACCEPTED
    } else {
        StatusCode::CONFLICT
    };
    Ok((status, Json(IngestResult { inserted })))
}

#[derive(Serialize)]
struct BatchResult {
    received: usize,
    inserted: usize,
    duplicates: usize,
    rejected: usize,
}

async fn post_batch(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(readings): Json<Vec<SoilReading>>,
) -> Result<(StatusCode, Json<BatchResult>), ApiError> {
    check_token(&s, &headers)?;

    let mut result = BatchResult {
        received: readings.len(),
        inserted: 0,
        duplicates: 0,
        rejected: 0,
    };

    for reading in readings {
        match s.platform.ingest_checked(reading) {
            Ok(true) => result.inserted += 1,
            Ok(false) => result.duplicates += 1,
            Err(ContractError::Validation(_)) => result.rejected += 1,
            Err(error) => return Err(error.into()),
        }
    }

    Ok((StatusCode::ACCEPTED, Json(result)))
}

#[cfg(test)]
#[path = "tests/api.rs"]
mod tests;
