//! `axum` HTTP routes for `hydrogen serve` (`Phase 3A.3`):
//! - `GET /` — Carbon Warm-Paper Workspace Landing Page (`< 2 ms` via `ledger.json`)
//! - `GET /products/{product}` — Product Sub-Cards Page (`< 2 ms` via `ledger.json`)
//! - `GET /products/{product}/{subject}` — Unified Sub-Product Status Dashboard (`StatusRenderMode::Server`)
//! - `GET /products/{product}/{subject}/files/{*rel_path}` — Strictly sandboxed `inbox/` & `archive/` streamer
//! - `GET /assets/hydrogen-logo.png` & `GET /products/{product}/brand-logo` — PNG brand logos
//! - `GET /health`, `GET /api/overview`, `GET /api/products/{product}`, `GET /api/products/{product}/{subject}`
//! - `POST /api/products/{product}/{subject}/sync` — Explicit Vault -> `ledger.json` + `status.html` sync (Rule A6)

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use memory_core_rs::{CourseCatalogSnapshot, CourseQuestion};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base64_util::{read_png_dimensions, to_data_uri};
use crate::carbon::catalog::mineral_color_for_subject;
use crate::domain::{QuestionPackage, UniversalId};
use crate::error::{HydrogenError, Result};
use crate::ingest::{
    OpenCommentEntry, ProductLedger, ProductStatusReport, SideBySideQuestionRow, SourceStage,
    StatusRenderMode, SubjectStatusCard, TopicCoverageRow, VaultSummary,
    build_subject_status_report, discover_product_subjects, render_status_html_with_mode,
    write_status_html,
};
use crate::server::scanner::{
    human_subject_title, resolve_product_brand_logo_path, scan_product_overview,
    scan_workspace_overview,
};
use crate::server::views::{
    render_error_html, render_product_explorer_html, render_workspace_landing_html,
};
use crate::server::{
    FileAccessError, ServerState, resolve_sandboxed_subject_file, validate_product_slug,
    validate_subject_slug,
};
use crate::vault::VaultManager;

/// Query parameters accepted on HTML & JSON status routes (`?refresh=1`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RefreshQuery {
    #[serde(default)]
    pub refresh: Option<String>,
}

impl RefreshQuery {
    fn is_refresh(&self) -> bool {
        self.refresh
            .as_deref()
            .is_some_and(|v| matches!(v.trim(), "1" | "true" | "yes"))
    }
}

/// Query parameters accepted on `POST /api/products/{product}/{subject}/sync`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SyncQuery {
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

/// JSON response payload returned by `POST /api/products/{product}/{subject}/sync`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncVaultResponse {
    pub status: String,
    pub product: String,
    pub subject: String,
    pub vault_stem: String,
    pub status_html_path: String,
    pub elapsed_ms: u128,
    pub vault_summary: Option<VaultSummary>,
    pub paper_count: usize,
    pub item_count: usize,
    pub open_comment_count: usize,
    pub pending_task_count: usize,
}

/// Construct the complete `axum::Router` for `hydrogen serve`.
pub fn build_router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/health", get(handle_health))
        .route("/assets/hydrogen-logo.png", get(handle_hydrogen_logo))
        .route("/products/{product}/brand-logo", get(handle_product_logo))
        .route("/", get(handle_workspace_landing))
        .route("/products/{product}", get(handle_product_page))
        .route("/products/{product}/{subject}", get(handle_subject_page))
        .route(
            "/products/{product}/{subject}/files",
            get(handle_empty_files_root),
        )
        .route(
            "/products/{product}/{subject}/files/",
            get(handle_empty_files_root),
        )
        .route(
            "/products/{product}/{subject}/files/{*rel_path}",
            get(handle_subject_file),
        )
        .route("/api/overview", get(handle_api_overview))
        .route("/api/products/{product}", get(handle_api_product))
        .route("/api/products/{product}/{subject}", get(handle_api_subject))
        .route(
            "/api/products/{product}/{subject}/sync",
            post(handle_api_subject_sync),
        )
        .with_state(state)
}

async fn handle_health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "hydrogen",
        "version": crate::VERSION,
    }))
}

async fn handle_hydrogen_logo(State(state): State<Arc<ServerState>>) -> Response {
    let logo_path = state.config().hydrogen_logo_path();
    serve_png_file(&logo_path).await
}

async fn handle_product_logo(
    State(state): State<Arc<ServerState>>,
    AxumPath(product): AxumPath<String>,
) -> Response {
    if let Err(err) = validate_product_slug(&product) {
        return json_file_error_response(err);
    }
    let Some(logo_path) = resolve_product_brand_logo_path(state.config(), &product) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("no PNG brand logo found for product '{product}'") })),
        )
            .into_response();
    };
    serve_png_file(&logo_path).await
}

async fn serve_png_file(path: &std::path::Path) -> Response {
    if !path.is_file() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "PNG asset not found" })),
        )
            .into_response();
    }
    match tokio::fs::read(path).await {
        Ok(bytes) if read_png_dimensions(&bytes).is_some() => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                (header::CACHE_CONTROL, "public, max-age=300"),
            ],
            bytes,
        )
            .into_response(),
        Ok(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "brand logo must be a valid PNG image" })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "failed to read PNG asset" })),
        )
            .into_response(),
    }
}

async fn handle_workspace_landing(
    State(state): State<Arc<ServerState>>,
    Query(q): Query<RefreshQuery>,
) -> Response {
    if q.is_refresh()
        && let Err(err) = sync_all_active_subproducts(&state).await
    {
        return html_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Workspace Sync Failed",
            &err.to_string(),
        );
    }

    match scan_workspace_overview(state.config()) {
        Ok(overview) => Html(render_workspace_landing_html(&overview)).into_response(),
        Err(err) => html_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Workspace Scan Error",
            &err.to_string(),
        ),
    }
}

async fn handle_product_page(
    State(state): State<Arc<ServerState>>,
    AxumPath(product): AxumPath<String>,
    Query(q): Query<RefreshQuery>,
) -> Response {
    if let Err(FileAccessError::Forbidden(msg)) = validate_product_slug(&product) {
        return html_error_response(StatusCode::FORBIDDEN, "Forbidden Product Slug", &msg);
    }

    if q.is_refresh()
        && let Err(err) = sync_product_subproducts(&state, &product).await
    {
        return html_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Product Sync Failed",
            &err.to_string(),
        );
    }

    match scan_product_overview(state.config(), &product) {
        Ok(Some(prod_overview)) => {
            let has_h_logo = state.config().hydrogen_logo_path().is_file();
            Html(render_product_explorer_html(&prod_overview, has_h_logo)).into_response()
        }
        Ok(None) => html_error_response(
            StatusCode::NOT_FOUND,
            "Product Not Active",
            &format!("Product '{product}' does not exist or has 0 active sub-products."),
        ),
        Err(err) => html_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Product Scan Error",
            &err.to_string(),
        ),
    }
}

async fn handle_subject_page(
    State(state): State<Arc<ServerState>>,
    AxumPath((product, subject)): AxumPath<(String, String)>,
    Query(q): Query<RefreshQuery>,
) -> Response {
    if let Err(FileAccessError::Forbidden(msg)) =
        validate_product_slug(&product).and_then(|()| validate_subject_slug(&subject))
    {
        return html_error_response(StatusCode::FORBIDDEN, "Forbidden Sub-Product Path", &msg);
    }

    match is_active_subproduct(&state, &product, &subject) {
        Ok(true) => {}
        Ok(false) => {
            return html_error_response(
                StatusCode::NOT_FOUND,
                "Sub-Product Not Active",
                &format!(
                    "Sub-product '{product}/{subject}' has no active vault, ledger, or staged files."
                ),
            );
        }
        Err(err) => {
            return html_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Discovery Error",
                &err.to_string(),
            );
        }
    }

    let report =
        match load_server_subject_report(&state, &product, &subject, q.is_refresh(), true).await {
            Ok(r) => r,
            Err(err) => {
                return html_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Status Report Error",
                    &err.to_string(),
                );
            }
        };

    let has_h_logo = state.config().hydrogen_logo_path().is_file();
    let mode = StatusRenderMode::Server {
        home_url: "/".to_string(),
        product_url: format!("/products/{product}"),
        file_url_prefix: format!("/products/{product}/{subject}/files"),
        hydrogen_logo_url: if has_h_logo {
            Some("/assets/hydrogen-logo.png".to_string())
        } else {
            None
        },
        sync_url: Some(format!("/api/products/{product}/{subject}/sync?redirect=1")),
    };

    Html(render_status_html_with_mode(&report, &mode)).into_response()
}

async fn handle_empty_files_root() -> Response {
    json_file_error_response(FileAccessError::Forbidden(
        "directory listing is forbidden".into(),
    ))
}

async fn handle_subject_file(
    State(state): State<Arc<ServerState>>,
    AxumPath((product, subject, rel_path)): AxumPath<(String, String, String)>,
) -> Response {
    let lower_rel = rel_path.to_ascii_lowercase();
    if lower_rel.contains("%2e") || lower_rel.contains("%2f") || lower_rel.contains("%5c") {
        return json_file_error_response(FileAccessError::Forbidden(
            "encoded traversal sequences are forbidden".into(),
        ));
    }

    let (canon_path, content_type) =
        match resolve_sandboxed_subject_file(state.config(), &product, &subject, &rel_path) {
            Ok(pair) => pair,
            Err(err) => return json_file_error_response(err),
        };

    match tokio::fs::read(&canon_path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            json_file_error_response(FileAccessError::NotFound)
        }
        Err(_) => json_file_error_response(FileAccessError::Forbidden(
            "failed to read sandboxed file".into(),
        )),
    }
}

async fn handle_api_overview(
    State(state): State<Arc<ServerState>>,
    Query(q): Query<RefreshQuery>,
) -> Response {
    if q.is_refresh()
        && let Err(err) = sync_all_active_subproducts(&state).await
    {
        return json_internal_error(&err);
    }

    match scan_workspace_overview(state.config()) {
        Ok(overview) => (StatusCode::OK, Json(overview)).into_response(),
        Err(err) => json_internal_error(&err),
    }
}

async fn handle_api_product(
    State(state): State<Arc<ServerState>>,
    AxumPath(product): AxumPath<String>,
    Query(q): Query<RefreshQuery>,
) -> Response {
    if let Err(err) = validate_product_slug(&product) {
        return json_file_error_response(err);
    }
    if q.is_refresh()
        && let Err(err) = sync_product_subproducts(&state, &product).await
    {
        return json_internal_error(&err);
    }

    match scan_product_overview(state.config(), &product) {
        Ok(Some(prod)) => (StatusCode::OK, Json(prod)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("product '{product}' is not active") })),
        )
            .into_response(),
        Err(err) => json_internal_error(&err),
    }
}

async fn handle_api_subject(
    State(state): State<Arc<ServerState>>,
    AxumPath((product, subject)): AxumPath<(String, String)>,
    Query(q): Query<RefreshQuery>,
) -> Response {
    if let Err(err) = validate_product_slug(&product).and_then(|()| validate_subject_slug(&subject))
    {
        return json_file_error_response(err);
    }
    match is_active_subproduct(&state, &product, &subject) {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "error": format!("sub-product '{product}/{subject}' is not active") }),
                ),
            )
                .into_response();
        }
        Err(err) => return json_internal_error(&err),
    }

    match load_server_subject_report(&state, &product, &subject, q.is_refresh(), false).await {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(err) => json_internal_error(&err),
    }
}

async fn handle_api_subject_sync(
    State(state): State<Arc<ServerState>>,
    AxumPath((product, subject)): AxumPath<(String, String)>,
    Query(q): Query<SyncQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(err) = validate_product_slug(&product).and_then(|()| validate_subject_slug(&subject))
    {
        return json_file_error_response(err);
    }
    match is_active_subproduct(&state, &product, &subject) {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "error": format!("sub-product '{product}/{subject}' is not active") }),
                ),
            )
                .into_response();
        }
        Err(err) => return json_internal_error(&err),
    }

    let cfg = state.config();
    let sqlite_path = cfg.vault_sqlite_path(&product, &subject);
    let lock = state.vault_lock(&sqlite_path).await;
    let t0 = Instant::now();

    let sync_res: Result<(ProductStatusReport, std::path::PathBuf)> = {
        let _guard = lock.lock().await;
        match build_subject_status_report(cfg, &product, &subject, true).await {
            Ok(report) => match write_status_html(cfg, &report) {
                Ok(path) => Ok((report, path)),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    };

    let (report, html_path) = match sync_res {
        Ok(pair) => pair,
        Err(err) => return json_internal_error(&err),
    };

    let force_json = q
        .format
        .as_deref()
        .is_some_and(|f| f.eq_ignore_ascii_case("json"));
    let accept_html = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.contains("text/html"));

    if !force_json {
        if let Some(ref redir) = q.redirect {
            let target = match redir.as_str() {
                "product" => format!("/products/{product}"),
                "home" => "/".to_string(),
                _ => format!("/products/{product}/{subject}"),
            };
            return Redirect::to(&target).into_response();
        }
        if accept_html {
            return Redirect::to(&format!("/products/{product}/{subject}")).into_response();
        }
    }

    let payload = SyncVaultResponse {
        status: "ok".to_string(),
        product: report.product,
        subject: report.subject,
        vault_stem: report.vault_stem,
        status_html_path: html_path.to_string_lossy().into_owned(),
        elapsed_ms: t0.elapsed().as_millis(),
        vault_summary: report.ledger.vault_summary.clone(),
        paper_count: report.ledger.papers.len(),
        item_count: report.ledger.items.len(),
        open_comment_count: report.open_comments.len(),
        pending_task_count: report.ledger.pending_task_count(),
    };
    (StatusCode::OK, Json(payload)).into_response()
}

fn is_active_subproduct(state: &ServerState, product: &str, subject: &str) -> Result<bool> {
    let active = discover_product_subjects(state.config(), product)?;
    Ok(active.iter().any(|s| s == subject))
}

async fn sync_product_subproducts(state: &ServerState, product: &str) -> Result<()> {
    let subjects = discover_product_subjects(state.config(), product)?;
    for subj in &subjects {
        let sqlite_path = state.config().vault_sqlite_path(product, subj);
        let lock = state.vault_lock(&sqlite_path).await;
        let _guard = lock.lock().await;
        let report = build_subject_status_report(state.config(), product, subj, false).await?;
        if state.config().status_html_path(product, subj).is_file() {
            let full_rep = build_subject_status_report(state.config(), product, subj, true).await?;
            let _ = write_status_html(state.config(), &full_rep)?;
        } else {
            drop(report);
        }
    }
    Ok(())
}

async fn sync_all_active_subproducts(state: &ServerState) -> Result<()> {
    let products = crate::ingest::discover_active_products(state.config())?;
    for prod in &products {
        sync_product_subproducts(state, prod).await?;
    }
    Ok(())
}

/// Build `ProductStatusReport` for `Server` mode while strictly honoring Rule A6:
/// - Acquires `ServerState::vault_lock` whenever `<stem>.sqlite` is opened so at most one `Memory`
///   connection per `.sqlite` file is ever open in this process.
/// - When `refresh == false` and `ledger.json` already exists on disk, skips SHA-256 PDF re-hashing
///   and disk writes; uses HTTP URLs (`/products/{product}/brand-logo` and `/products/{product}/{subject}/files/...`)
///   instead of multi-MB inline base64 strings.
async fn load_server_subject_report(
    state: &ServerState,
    product: &str,
    subject: &str,
    refresh: bool,
    include_comment_thumbs: bool,
) -> Result<ProductStatusReport> {
    let cfg = state.config();
    let t0 = Instant::now();
    let sqlite_path = cfg.vault_sqlite_path(product, subject);
    let bundle_path = cfg.vault_bundle_path(product, subject);
    let ledger_path = cfg.ledger_path(product, subject);
    let brand_logo_url = resolve_product_brand_logo_path(cfg, product)
        .map(|_| format!("/products/{product}/brand-logo"));

    let lock = state.vault_lock(&sqlite_path).await;
    let _guard = lock.lock().await;

    if refresh || !ledger_path.is_file() {
        let mut rep = build_subject_status_report(cfg, product, subject, false).await?;
        if include_comment_thumbs
            && !rep.open_comments.is_empty()
            && (sqlite_path.is_file() || bundle_path.is_file())
        {
            populate_comment_thumbnails(cfg, &sqlite_path, &bundle_path, &mut rep.open_comments)
                .await;
        }
        rep.brand_logo_data_uri = brand_logo_url;
        return Ok(rep);
    }

    let mut ledger = ProductLedger::load_or_new(&ledger_path, product, subject)?;
    let has_vault_file = sqlite_path.is_file() || bundle_path.is_file();

    let mut vault_snapshot: Option<CourseCatalogSnapshot> = None;
    let mut compiled_questions: BTreeMap<String, CourseQuestion> = BTreeMap::new();
    let mut open_comments: Vec<OpenCommentEntry> = Vec::new();

    if has_vault_file
        && let Ok(vault) = VaultManager::open_or_create(
            &sqlite_path,
            &bundle_path,
            &cfg.lesson_key,
            &cfg.device_id,
        )
        && let Ok(snap) = vault.catalog_snapshot().await
    {
        for q_sum in &snap.questions {
            if let Ok(Some(cq)) = vault.get_question(&q_sum.question_id).await {
                compiled_questions.insert(cq.question_id.clone(), cq);
            }
        }
        if let Ok(subj_comments) = vault.list_comments(Some("open"), None).await {
            for c in subj_comments {
                let mut att_uris = BTreeMap::new();
                if include_comment_thumbs {
                    for att in &c.attachments {
                        if att.mime_type.starts_with("image/")
                            && let Ok(bytes) =
                                vault.read_comment_attachment(&att.attachment_id).await
                        {
                            att_uris.insert(
                                att.attachment_id.clone(),
                                to_data_uri(&att.mime_type, &bytes),
                            );
                        }
                    }
                }
                open_comments.push(OpenCommentEntry {
                    subject: subject.to_string(),
                    comment: c,
                    attachment_data_uris: att_uris,
                });
            }
        }
        if ledger.vault_summary.is_none() {
            ledger.sync_with_workspace(cfg, Some(&snap))?;
            ledger.save_atomic(&ledger_path)?;
        }
        vault_snapshot = Some(snap);
    }

    let prod_prefix = UniversalId::product_prefix_for_slug(product);
    let subj_prefix = UniversalId::subject_prefix_for_slug(subject);
    let fallback_course_id = format!("{prod_prefix}-{subj_prefix}");
    let mineral_color = mineral_color_for_subject(subj_prefix, 0).to_string();
    let inbox_queued_count = ledger
        .sources
        .values()
        .filter(|s| s.stage == SourceStage::Queued)
        .count();

    let mut topic_rows = Vec::new();
    let subject_card = if let Some(ref snap) = vault_snapshot {
        let course_id = snap
            .meta
            .as_ref()
            .map(|m| m.course_id.clone())
            .unwrap_or(fallback_course_id);
        let title = snap
            .meta
            .as_ref()
            .map(|m| m.title.clone())
            .unwrap_or_else(|| human_subject_title(subject));
        let verified_q = snap
            .questions
            .iter()
            .filter(|q| !q.is_replica && (q.qa_status == "verified" || q.qa_status == "approved"))
            .count();
        let replica_q = snap.questions.iter().filter(|q| q.is_replica).count();

        for t in &snap.topics {
            let q_count = snap
                .questions
                .iter()
                .filter(|q| q.topic_ids.iter().any(|tid| tid == &t.topic_id))
                .count();
            let c_count = snap
                .topic_cards
                .iter()
                .filter(|c| c.topic_id.as_deref() == Some(t.topic_id.as_str()))
                .count();
            let open_c = snap
                .open_comment_counts
                .get(&format!("topic:{}", t.topic_id))
                .copied()
                .unwrap_or(0);
            topic_rows.push(TopicCoverageRow {
                subject: subject.to_string(),
                topic_id: t.topic_id.clone(),
                code: t.code.clone(),
                title: t.title.clone(),
                question_count: q_count,
                card_count: c_count,
                open_comment_count: open_c,
            });
        }

        SubjectStatusCard {
            subject: subject.to_string(),
            course_id,
            title,
            mineral_color,
            vault_initialized: true,
            sqlite_path,
            paper_count: snap.papers.len(),
            verified_question_count: verified_q,
            replica_count: replica_q,
            topic_count: snap.topics.len(),
            topic_card_count: snap.topic_cards.len(),
            inbox_queued_count,
            open_comment_count: usize::try_from(snap.total_open_comments.max(0)).unwrap_or(0),
        }
    } else if let Some(ref vs) = ledger.vault_summary {
        SubjectStatusCard {
            subject: subject.to_string(),
            course_id: vs.course_id.clone(),
            title: vs.title.clone(),
            mineral_color,
            vault_initialized: vs.vault_initialized,
            sqlite_path,
            paper_count: vs.paper_count,
            verified_question_count: vs.verified_question_count,
            replica_count: vs.replica_count,
            topic_count: vs.topic_count,
            topic_card_count: vs.topic_card_count,
            inbox_queued_count,
            open_comment_count: vs.open_comment_count,
        }
    } else {
        SubjectStatusCard {
            subject: subject.to_string(),
            course_id: fallback_course_id,
            title: human_subject_title(subject),
            mineral_color,
            vault_initialized: false,
            sqlite_path,
            paper_count: ledger.papers.len(),
            verified_question_count: 0,
            replica_count: 0,
            topic_count: 0,
            topic_card_count: 0,
            inbox_queued_count,
            open_comment_count: 0,
        }
    };

    let subject_dir = cfg.subject_dir(product, subject);
    let mut side_by_side_items = Vec::with_capacity(ledger.items.len());
    for (uid, l_item) in &ledger.items {
        let compiled_entry = compiled_questions.get(uid);
        let fallback_pkg: Option<QuestionPackage> = if compiled_entry.is_none() {
            l_item
                .solution_json_path
                .as_ref()
                .and_then(|rel| std::fs::read_to_string(subject_dir.join(rel)).ok())
                .and_then(|raw| serde_json::from_str::<QuestionPackage>(&raw).ok())
        } else {
            None
        };

        let compiled_label = compiled_entry
            .map(|cq| cq.label.clone())
            .or_else(|| fallback_pkg.as_ref().map(|p| p.label.clone()));
        let compiled_title = compiled_entry
            .map(|cq| cq.title.clone())
            .or_else(|| fallback_pkg.as_ref().map(|p| p.title.clone()));
        let compiled_stem = compiled_entry.map(|cq| cq.stem_text.clone()).or_else(|| {
            fallback_pkg
                .as_ref()
                .map(|p| p.question.stem_markdown_katex.clone())
        });
        let marks = compiled_entry.map(|cq| cq.marks).or_else(|| {
            fallback_pkg
                .as_ref()
                .map(|p| p.question.parts.iter().map(|pt| pt.marks).sum())
        });
        let sublevel_span = compiled_entry.map(|cq| (cq.min_sublevel, cq.max_sublevel));

        side_by_side_items.push(SideBySideQuestionRow {
            universal_id: uid.clone(),
            subject: l_item.subject.clone(),
            paper_id: l_item.paper_id.clone(),
            source_file: l_item.source_file.clone(),
            source_sha256: l_item.source_sha256.clone(),
            archived_source_path: l_item.archived_source_path.clone(),
            crop_path: l_item.crop_path.clone(),
            crop_page: l_item.page,
            crop_dimensions: l_item.crop_dimensions,
            solution_json_path: l_item.solution_json_path.clone(),
            crop_data_uri: None,
            compiled_label,
            compiled_title,
            compiled_stem,
            marks,
            sublevel_span,
            qa_status: compiled_entry
                .map(|cq| cq.qa_status.clone())
                .or_else(|| l_item.qa_status.clone()),
            is_replica: compiled_entry
                .map(|cq| cq.is_replica)
                .unwrap_or(l_item.is_replica),
            replica_scope: l_item.replica_scope,
            parent_question_id: l_item.parent_question_id.clone(),
            parent_paper_id: l_item.parent_paper_id.clone(),
            replica_ids: l_item.replica_ids.clone(),
            open_comment_count: compiled_entry.map(|cq| cq.open_comment_count).unwrap_or(0),
        });
    }

    Ok(ProductStatusReport {
        product: product.to_string(),
        subject: subject.to_string(),
        vault_stem: crate::config::HydrogenConfig::vault_stem(product, subject),
        generated_at_ms: ledger.updated_at_ms,
        elapsed_ms: t0.elapsed().as_millis(),
        ledger_path,
        status_html_path: cfg.status_html_path(product, subject),
        hydrogen_logo_data_uri: None,
        brand_logo_data_uri: brand_logo_url,
        subjects: vec![subject_card],
        ledger,
        topics: topic_rows,
        side_by_side_items,
        open_comments,
    })
}

async fn populate_comment_thumbnails(
    cfg: &crate::config::HydrogenConfig,
    sqlite_path: &std::path::Path,
    bundle_path: &std::path::Path,
    open_comments: &mut [OpenCommentEntry],
) {
    let Ok(vault) =
        VaultManager::open_or_create(sqlite_path, bundle_path, &cfg.lesson_key, &cfg.device_id)
    else {
        return;
    };
    for oc in open_comments {
        for att in &oc.comment.attachments {
            if att.mime_type.starts_with("image/")
                && !oc.attachment_data_uris.contains_key(&att.attachment_id)
                && let Ok(bytes) = vault.read_comment_attachment(&att.attachment_id).await
            {
                oc.attachment_data_uris.insert(
                    att.attachment_id.clone(),
                    to_data_uri(&att.mime_type, &bytes),
                );
            }
        }
    }
}

fn html_error_response(status: StatusCode, title: &str, detail: &str) -> Response {
    (
        status,
        Html(render_error_html(status.as_u16(), title, detail)),
    )
        .into_response()
}

fn json_file_error_response(err: FileAccessError) -> Response {
    match err {
        FileAccessError::Forbidden(reason) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "forbidden", "detail": reason })),
        )
            .into_response(),
        FileAccessError::NotFound => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" }))).into_response()
        }
    }
}

fn json_internal_error(err: &HydrogenError) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal_error", "detail": err.to_string() })),
    )
        .into_response()
}
