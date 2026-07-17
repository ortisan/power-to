use std::{sync::Arc, time::Instant};

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, MatchedPath, Path, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::SecondsFormat;
use opentelemetry::{KeyValue, global, metrics::Histogram};
use opentelemetry_http::HeaderExtractor;
use powerto_application::{
    health::ReadinessProbe,
    issues::{
        GetOwnIssueError, IdempotencyKey, IssueService, SubmissionDisposition, SubmitIssueCommand,
        SubmitIssueError,
    },
};
use powerto_domain::{AccountId, GeometrySource, Issue, IssueReference};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    trace::TraceLayer,
};
use tracing_opentelemetry::OpenTelemetrySpanExt as _;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

const LOCAL_ACCOUNT_HEADER: &str = "x-powerto-local-account-id";
const IDEMPOTENCY_HEADER: &str = "idempotency-key";
const IDEMPOTENCY_REPLAYED_HEADER: &str = "idempotency-replayed";
const ISSUE_BODY_LIMIT_BYTES: usize = 64 * 1024;

/// Controls the deliberately temporary local actor adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalActorHeaderMode {
    /// Reject issue routes until a real authenticated actor resolver exists.
    Disabled,
    /// Trust a local account UUID header. The binary only permits this mode in
    /// the `local` environment while bound to a loopback address.
    InsecureLoopbackOnly,
}

/// Shared state for the HTTP inbound adapter.
#[derive(Clone)]
pub struct ApiState {
    readiness: Arc<dyn ReadinessProbe>,
    issues: IssueService,
    local_actor_header_mode: LocalActorHeaderMode,
    metrics: ApiMetrics,
}

impl ApiState {
    /// Creates API state from inward-facing application ports.
    #[must_use]
    pub fn new(
        readiness: Arc<dyn ReadinessProbe>,
        issues: IssueService,
        local_actor_header_mode: LocalActorHeaderMode,
    ) -> Self {
        let meter = global::meter("powerto-api");
        let request_duration = meter
            .f64_histogram("http.server.request.duration")
            .with_description("Duration of inbound HTTP requests")
            .with_unit("s")
            .build();

        Self {
            readiness,
            issues,
            local_actor_header_mode,
            metrics: ApiMetrics { request_duration },
        }
    }
}

#[derive(Clone, Debug)]
struct ApiMetrics {
    request_duration: Histogram<f64>,
}

/// Creates the complete HTTP router without binding a socket.
pub fn router(state: ApiState) -> Router {
    let metrics_state = state.clone();
    let http_middleware = ServiceBuilder::new()
        .layer(SetSensitiveRequestHeadersLayer::new([
            header::AUTHORIZATION,
            HeaderName::from_static(LOCAL_ACCOUNT_HEADER),
            HeaderName::from_static(IDEMPOTENCY_HEADER),
        ]))
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(http_trace_layer())
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )));

    Router::new()
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/api/openapi.json", get(openapi_document))
        .route("/api/v1/me/issues", post(submit_issue))
        .route("/api/v1/me/issues/{issue_ref}", get(get_own_issue))
        .with_state(state)
        .layer(DefaultBodyLimit::max(ISSUE_BODY_LIMIT_BYTES))
        .layer(from_fn_with_state(metrics_state, record_http_metrics))
        .layer(http_middleware)
}

async fn record_http_metrics(
    State(state): State<ApiState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str)
        .to_owned();

    if route.starts_with("/health/") {
        return next.run(request).await;
    }

    let method = request.method().as_str().to_owned();
    let started_at = Instant::now();
    let response = next.run(request).await;
    state.metrics.request_duration.record(
        started_at.elapsed().as_secs_f64(),
        &[
            KeyValue::new("http.request.method", method),
            KeyValue::new("http.route", route),
            KeyValue::new(
                "http.response.status_code",
                i64::from(response.status().as_u16()),
            ),
        ],
    );

    response
}

fn http_trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    impl Fn(&Request<Body>) -> tracing::Span + Clone,
> {
    TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
        let route = request
            .extensions()
            .get::<MatchedPath>()
            .map_or("unmatched", MatchedPath::as_str);
        if route.starts_with("/health/") {
            return tracing::Span::none();
        }
        let span = tracing::info_span!(
            "http.server.request",
            http.request.method = %request.method(),
            http.route = %route,
        );
        let parent = global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderExtractor(request.headers()))
        });
        if let Err(error) = span.set_parent(parent) {
            tracing::debug!(error = %error, "could not attach remote trace context");
        }
        span
    })
}

#[utoipa::path(
    get,
    path = "/health/live",
    responses((status = 204, description = "The process event loop is alive")),
    tag = "health"
)]
async fn liveness() -> Response {
    no_store(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 204, description = "Required dependencies are available"),
        (status = 503, description = "A required dependency is unavailable", body = ProblemDetails)
    ),
    tag = "health"
)]
async fn readiness(State(state): State<ApiState>) -> Response {
    match state.readiness.check().await {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(_) => problem_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "urn:powerto:problem:dependency-unavailable",
            "Service unavailable",
            "DEPENDENCY_UNAVAILABLE",
            "A required dependency is unavailable.",
        ),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/me/issues",
    params(
        ("Idempotency-Key" = String, Header, description = "High-entropy UUID for safe retries"),
        ("x-powerto-local-account-id" = String, Header, description = "Local loopback-only development actor; not authentication")
    ),
    request_body = SubmitIssueRequest,
    responses(
        (status = 201, description = "Issue submitted", body = OwnIssueResponse),
        (status = 200, description = "Existing issue returned for an exact command replay", body = OwnIssueResponse),
        (status = 400, description = "Malformed request or idempotency key", body = ProblemDetails),
        (status = 401, description = "Authenticated actor unavailable", body = ProblemDetails),
        (status = 409, description = "Idempotency or policy conflict", body = ProblemDetails),
        (status = 413, description = "Request body exceeds the limit", body = ProblemDetails),
        (status = 422, description = "Issue input violates domain policy", body = ProblemDetails),
        (status = 500, description = "Stored data violates an invariant", body = ProblemDetails),
        (status = 503, description = "Persistence unavailable", body = ProblemDetails)
    ),
    tag = "own issues"
)]
async fn submit_issue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    payload: Result<Json<SubmitIssueRequest>, JsonRejection>,
) -> Response {
    let account_id = match local_account(&state, &headers) {
        Ok(account_id) => account_id,
        Err(error) => return local_account_error(error),
    };
    let idempotency_key = match idempotency_key(&headers) {
        Ok(key) => key,
        Err(error) => return idempotency_key_error(error),
    };
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return json_rejection(rejection),
    };

    let result = state
        .issues
        .submit(account_id, payload.into_command(idempotency_key))
        .await;
    match result {
        Ok(result) => {
            let status = match result.disposition {
                SubmissionDisposition::Created => StatusCode::CREATED,
                SubmissionDisposition::Replayed => StatusCode::OK,
            };
            let reference = result.issue.reference().into_uuid().to_string();
            let mut response =
                (status, Json(OwnIssueResponse::from_issue(&result.issue))).into_response();
            if let Ok(location) = HeaderValue::from_str(&format!("/api/v1/me/issues/{reference}")) {
                response.headers_mut().insert(header::LOCATION, location);
            }
            if result.disposition == SubmissionDisposition::Replayed {
                response.headers_mut().insert(
                    HeaderName::from_static(IDEMPOTENCY_REPLAYED_HEADER),
                    HeaderValue::from_static("true"),
                );
            }
            private_no_store(response)
        }
        Err(error) => submit_issue_error(error),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/me/issues/{issue_ref}",
    params(
        ("issue_ref" = String, Path, description = "Opaque random issue reference"),
        ("x-powerto-local-account-id" = String, Header, description = "Local loopback-only development actor; not authentication")
    ),
    responses(
        (status = 200, description = "Owner-scoped issue", body = OwnIssueResponse),
        (status = 401, description = "Authenticated actor unavailable", body = ProblemDetails),
        (status = 404, description = "Issue absent or owned by another account", body = ProblemDetails),
        (status = 500, description = "Stored data violates an invariant", body = ProblemDetails),
        (status = 503, description = "Persistence unavailable", body = ProblemDetails)
    ),
    tag = "own issues"
)]
async fn get_own_issue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(issue_ref): Path<String>,
) -> Response {
    let account_id = match local_account(&state, &headers) {
        Ok(account_id) => account_id,
        Err(error) => return local_account_error(error),
    };
    let reference = match Uuid::parse_str(&issue_ref) {
        Ok(reference) => IssueReference::from_uuid(reference),
        Err(_) => return issue_not_found(),
    };

    match state.issues.get_owned(account_id, reference).await {
        Ok(Some(issue)) => {
            private_no_store(Json(OwnIssueResponse::from_issue(&issue)).into_response())
        }
        Ok(None) => issue_not_found(),
        Err(GetOwnIssueError::Unavailable) => persistence_unavailable(),
        Err(GetOwnIssueError::Internal) => internal_invariant_error(),
    }
}

#[derive(Clone, Copy)]
enum LocalAccountError {
    Disabled,
    MissingOrInvalid,
}

fn local_account(state: &ApiState, headers: &HeaderMap) -> Result<AccountId, LocalAccountError> {
    if state.local_actor_header_mode == LocalActorHeaderMode::Disabled {
        return Err(LocalAccountError::Disabled);
    }

    let account_id = headers
        .get(HeaderName::from_static(LOCAL_ACCOUNT_HEADER))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok());
    match account_id {
        Some(account_id) => Ok(AccountId::from_uuid(account_id)),
        None => Err(LocalAccountError::MissingOrInvalid),
    }
}

fn local_account_error(error: LocalAccountError) -> Response {
    let detail = match error {
        LocalAccountError::Disabled => "OIDC actor resolution is not available in this deployment.",
        LocalAccountError::MissingOrInvalid => "A valid local development actor is required.",
    };
    problem_response(
        StatusCode::UNAUTHORIZED,
        "urn:powerto:problem:authentication-required",
        "Authentication required",
        "AUTHENTICATION_REQUIRED",
        detail,
    )
}

#[derive(Clone, Copy)]
enum IdempotencyHeaderError {
    Missing,
    Invalid,
}

fn idempotency_key(headers: &HeaderMap) -> Result<IdempotencyKey, IdempotencyHeaderError> {
    let value = headers
        .get(HeaderName::from_static(IDEMPOTENCY_HEADER))
        .and_then(|value| value.to_str().ok());
    let Some(value) = value else {
        return Err(IdempotencyHeaderError::Missing);
    };
    Uuid::parse_str(value)
        .map(IdempotencyKey::from_uuid)
        .map_err(|_| IdempotencyHeaderError::Invalid)
}

fn idempotency_key_error(error: IdempotencyHeaderError) -> Response {
    let (problem_type, title, code) = match error {
        IdempotencyHeaderError::Missing => (
            "urn:powerto:problem:idempotency-key-required",
            "Idempotency key required",
            "IDEMPOTENCY_KEY_REQUIRED",
        ),
        IdempotencyHeaderError::Invalid => (
            "urn:powerto:problem:invalid-idempotency-key",
            "Invalid idempotency key",
            "INVALID_IDEMPOTENCY_KEY",
        ),
    };
    problem_response(
        StatusCode::BAD_REQUEST,
        problem_type,
        title,
        code,
        "Idempotency-Key must contain a UUID.",
    )
}

fn json_rejection(rejection: JsonRejection) -> Response {
    if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return problem_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "urn:powerto:problem:payload-too-large",
            "Payload too large",
            "PAYLOAD_TOO_LARGE",
            "The issue request exceeds the 64 KiB limit.",
        );
    }
    problem_response(
        StatusCode::BAD_REQUEST,
        "urn:powerto:problem:invalid-json",
        "Invalid JSON request",
        "INVALID_JSON",
        "The request body must match the documented JSON schema.",
    )
}

fn submit_issue_error(error: SubmitIssueError) -> Response {
    match error {
        SubmitIssueError::InvalidInput(_) => problem_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "urn:powerto:problem:invalid-issue",
            "Invalid issue",
            "INVALID_ISSUE",
            "One or more issue fields violate the submission policy.",
        ),
        SubmitIssueError::PrivacyNoticeOutdated => problem_response(
            StatusCode::CONFLICT,
            "urn:powerto:problem:privacy-notice-outdated",
            "Privacy notice changed",
            "PRIVACY_NOTICE_OUTDATED",
            "Accept the current privacy notice before creating a new issue.",
        ),
        SubmitIssueError::IdempotencyConflict => problem_response(
            StatusCode::CONFLICT,
            "urn:powerto:problem:idempotency-key-reused",
            "Idempotency key reused",
            "IDEMPOTENCY_KEY_REUSED",
            "This idempotency key was used for a different command.",
        ),
        SubmitIssueError::Unavailable => persistence_unavailable(),
        SubmitIssueError::Internal => internal_invariant_error(),
    }
}

fn issue_not_found() -> Response {
    problem_response(
        StatusCode::NOT_FOUND,
        "urn:powerto:problem:issue-not-found",
        "Issue not found",
        "ISSUE_NOT_FOUND",
        "The issue does not exist in this account scope.",
    )
}

fn persistence_unavailable() -> Response {
    let mut response = problem_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "urn:powerto:problem:persistence-unavailable",
        "Service unavailable",
        "PERSISTENCE_UNAVAILABLE",
        "Issue persistence is temporarily unavailable.",
    );
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    response
}

fn internal_invariant_error() -> Response {
    problem_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "urn:powerto:problem:internal-invariant",
        "Internal server error",
        "INTERNAL_INVARIANT",
        "Stored data could not be represented safely.",
    )
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn private_no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response
}

fn problem_response(
    status: StatusCode,
    problem_type: &'static str,
    title: &'static str,
    code: &'static str,
    detail: &'static str,
) -> Response {
    let problem = ProblemDetails {
        r#type: problem_type,
        title,
        status: status.as_u16(),
        code,
        detail,
    };
    let mut response = (status, Json(problem)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    private_no_store(response)
}

async fn openapi_document() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Stable public error representation. Internal errors and personal data must
/// never be included in this DTO.
#[derive(Serialize, ToSchema)]
struct ProblemDetails {
    r#type: &'static str,
    title: &'static str,
    status: u16,
    code: &'static str,
    detail: &'static str,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct SubmitIssueRequest {
    title: String,
    category_id: String,
    summary: String,
    problem_statement: String,
    affected_community: String,
    desired_outcome: String,
    location: ConfirmedPointRequest,
    geometry_source: GeometrySourceRequest,
    location_confirmed: bool,
    location_label: Option<String>,
    public_attribution: bool,
    privacy_notice_version: String,
    privacy_notice_accepted: bool,
}

impl SubmitIssueRequest {
    fn into_command(self, idempotency_key: IdempotencyKey) -> SubmitIssueCommand {
        SubmitIssueCommand {
            idempotency_key,
            title: self.title,
            category_id: self.category_id,
            summary: self.summary,
            problem_statement: self.problem_statement,
            affected_community: self.affected_community,
            desired_outcome: self.desired_outcome,
            longitude: self.location.longitude,
            latitude: self.location.latitude,
            geometry_source: self.geometry_source.into_domain(),
            location_confirmed: self.location_confirmed,
            location_label: self.location_label,
            public_attribution: self.public_attribution,
            privacy_notice_version: self.privacy_notice_version,
            privacy_notice_accepted: self.privacy_notice_accepted,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct ConfirmedPointRequest {
    longitude: f64,
    latitude: f64,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum GeometrySourceRequest {
    MapSelection,
    DeviceLocation,
    GeocodedSearch,
}

impl GeometrySourceRequest {
    const fn into_domain(self) -> GeometrySource {
        match self {
            Self::MapSelection => GeometrySource::MapSelection,
            Self::DeviceLocation => GeometrySource::DeviceLocation,
            Self::GeocodedSearch => GeometrySource::GeocodedSearch,
        }
    }
}

#[derive(Serialize, ToSchema)]
struct OwnIssueResponse {
    reference: String,
    version: u64,
    status: &'static str,
    category_id: String,
    submission_policy_version: String,
    title: String,
    summary: String,
    problem_statement: String,
    affected_community: String,
    desired_outcome: String,
    location: ConfirmedPointResponse,
    geometry_source: &'static str,
    location_label: Option<String>,
    public_attribution: bool,
    privacy_notice_version: String,
    submitted_at: String,
}

impl OwnIssueResponse {
    fn from_issue(issue: &Issue) -> Self {
        Self {
            reference: issue.reference().into_uuid().to_string(),
            version: issue.version(),
            status: issue.status().as_str(),
            category_id: issue.category_id().to_owned(),
            submission_policy_version: issue.submission_policy_version().to_owned(),
            title: issue.title().to_owned(),
            summary: issue.summary().to_owned(),
            problem_statement: issue.problem_statement().to_owned(),
            affected_community: issue.affected_community().to_owned(),
            desired_outcome: issue.desired_outcome().to_owned(),
            location: ConfirmedPointResponse {
                longitude: issue.point().longitude(),
                latitude: issue.point().latitude(),
            },
            geometry_source: issue.geometry_source().as_str(),
            location_label: issue.location_label().map(str::to_owned),
            public_attribution: issue.public_attribution(),
            privacy_notice_version: issue.privacy_notice_version().to_owned(),
            submitted_at: issue
                .submitted_at()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Serialize, ToSchema)]
struct ConfirmedPointResponse {
    longitude: f64,
    latitude: f64,
}

#[derive(OpenApi)]
#[openapi(
    paths(liveness, readiness, submit_issue, get_own_issue),
    components(schemas(
        ProblemDetails,
        SubmitIssueRequest,
        ConfirmedPointRequest,
        GeometrySourceRequest,
        OwnIssueResponse,
        ConfirmedPointResponse
    )),
    tags(
        (name = "health", description = "Process and dependency health"),
        (name = "own issues", description = "Private issue intake and owner-scoped reads")
    )
)]
struct ApiDoc;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request},
    };
    use powerto_application::issues::IssueService;
    use powerto_test_support::{FixedReadiness, InMemoryIssueStore};
    use serde_json::{Value, json};
    use tower::ServiceExt as _;
    use utoipa::OpenApi as _;
    use uuid::Uuid;

    use super::{ApiDoc, ApiState, LocalActorHeaderMode, router};

    const ACTOR_HEADER: &str = "x-powerto-local-account-id";

    fn state(mode: LocalActorHeaderMode) -> ApiState {
        ApiState::new(
            Arc::new(FixedReadiness::ready()),
            IssueService::new(Arc::new(InMemoryIssueStore::default()), "privacy-v1"),
            mode,
        )
    }

    fn valid_payload() -> Value {
        json!({
            "title": "Deep pothole",
            "category_id": "road-surface",
            "summary": "Buses avoid the damaged lane.",
            "problem_statement": "The depression remains after repeated observations.",
            "affected_community": "Passengers, cyclists, and drivers.",
            "desired_outcome": "Restore a level and safe road surface.",
            "location": {"longitude": -46.633308, "latitude": -23.55052},
            "geometry_source": "map_selection",
            "location_confirmed": true,
            "location_label": "Eastbound bus lane",
            "public_attribution": false,
            "privacy_notice_version": "privacy-v1",
            "privacy_notice_accepted": true
        })
    }

    #[tokio::test]
    async fn liveness_does_not_depend_on_postgres() {
        let app = router(ApiState::new(
            Arc::new(FixedReadiness::unavailable()),
            IssueService::new(Arc::new(InMemoryIssueStore::default()), "privacy-v1"),
            LocalActorHeaderMode::Disabled,
        ));
        let response = send(&app, Method::GET, "/health/live", None, None, None).await;

        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
        assert_eq!(
            response.headers().get(axum::http::header::CACHE_CONTROL),
            Some(&axum::http::HeaderValue::from_static("no-store"))
        );
    }

    #[tokio::test]
    async fn readiness_reports_an_unavailable_dependency() {
        let app = router(ApiState::new(
            Arc::new(FixedReadiness::unavailable()),
            IssueService::new(Arc::new(InMemoryIssueStore::default()), "privacy-v1"),
            LocalActorHeaderMode::Disabled,
        ));
        let response = send(&app, Method::GET, "/health/ready", None, None, None).await;

        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            response.headers().get(axum::http::header::CONTENT_TYPE),
            Some(&axum::http::HeaderValue::from_static(
                "application/problem+json"
            ))
        );
    }

    #[tokio::test]
    async fn submit_replay_and_owner_scope_are_enforced() {
        let app = router(state(LocalActorHeaderMode::InsecureLoopbackOnly));
        let actor = Uuid::new_v4().to_string();
        let other_actor = Uuid::new_v4().to_string();
        let key = Uuid::new_v4().to_string();
        let payload = valid_payload().to_string();

        let created = send(
            &app,
            Method::POST,
            "/api/v1/me/issues",
            Some(&actor),
            Some(&key),
            Some(payload.clone()),
        )
        .await;
        assert_eq!(created.status(), axum::http::StatusCode::CREATED);
        assert_eq!(
            created.headers().get(axum::http::header::CACHE_CONTROL),
            Some(&axum::http::HeaderValue::from_static("private, no-store"))
        );
        let location = created
            .headers()
            .get(axum::http::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let created_body = response_json(created).await;
        assert!(created_body.get("reference").is_some());
        assert!(created_body.get("issue_id").is_none());

        let replayed = send(
            &app,
            Method::POST,
            "/api/v1/me/issues",
            Some(&actor),
            Some(&key),
            Some(payload),
        )
        .await;
        assert_eq!(replayed.status(), axum::http::StatusCode::OK);
        assert_eq!(
            replayed.headers().get("idempotency-replayed"),
            Some(&axum::http::HeaderValue::from_static("true"))
        );
        assert_eq!(response_json(replayed).await, created_body);

        let location = match location {
            Some(location) => location,
            None => panic!("created issue did not return a location header"),
        };
        let owner_view = send(&app, Method::GET, &location, Some(&actor), None, None).await;
        assert_eq!(owner_view.status(), axum::http::StatusCode::OK);

        let other_view = send(&app, Method::GET, &location, Some(&other_actor), None, None).await;
        assert_eq!(other_view.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn same_key_with_changed_command_returns_conflict() {
        let app = router(state(LocalActorHeaderMode::InsecureLoopbackOnly));
        let actor = Uuid::new_v4().to_string();
        let key = Uuid::new_v4().to_string();
        let original = valid_payload();
        let first = send(
            &app,
            Method::POST,
            "/api/v1/me/issues",
            Some(&actor),
            Some(&key),
            Some(original.to_string()),
        )
        .await;
        assert_eq!(first.status(), axum::http::StatusCode::CREATED);

        let mut changed = original;
        changed["title"] = Value::String("Different problem".to_owned());
        let conflict = send(
            &app,
            Method::POST,
            "/api/v1/me/issues",
            Some(&actor),
            Some(&key),
            Some(changed.to_string()),
        )
        .await;

        assert_eq!(conflict.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn issue_routes_are_closed_when_local_actor_mode_is_disabled() {
        let app = router(state(LocalActorHeaderMode::Disabled));
        let response = send(
            &app,
            Method::POST,
            "/api/v1/me/issues",
            Some(&Uuid::new_v4().to_string()),
            Some(&Uuid::new_v4().to_string()),
            Some(valid_payload().to_string()),
        )
        .await;

        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn openapi_contains_health_and_issue_contracts() {
        let serialized = serde_json::to_value(ApiDoc::openapi());

        match serialized {
            Ok(document) => {
                assert!(document["paths"]["/health/live"].is_object());
                assert!(document["paths"]["/health/ready"].is_object());
                assert!(document["paths"]["/api/v1/me/issues"].is_object());
                assert!(document["paths"]["/api/v1/me/issues/{issue_ref}"].is_object());
            }
            Err(error) => panic!("OpenAPI document was not serializable: {error}"),
        }
    }

    async fn send(
        app: &axum::Router,
        method: Method,
        path: &str,
        actor: Option<&str>,
        idempotency_key: Option<&str>,
        body: Option<String>,
    ) -> axum::response::Response {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(actor) = actor {
            builder = builder.header(ACTOR_HEADER, actor);
        }
        if let Some(idempotency_key) = idempotency_key {
            builder = builder.header("idempotency-key", idempotency_key);
        }
        let request_body = match body {
            Some(body) => {
                builder = builder.header(axum::http::header::CONTENT_TYPE, "application/json");
                Body::from(body)
            }
            None => Body::empty(),
        };
        let request = match builder.body(request_body) {
            Ok(request) => request,
            Err(error) => panic!("test request could not be built: {error}"),
        };

        match app.clone().oneshot(request).await {
            Ok(response) => response,
            Err(error) => match error {},
        }
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = match to_bytes(response.into_body(), 128 * 1024).await {
            Ok(bytes) => bytes,
            Err(error) => panic!("response body could not be read: {error}"),
        };
        match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(error) => panic!("response body was not JSON: {error}"),
        }
    }
}
