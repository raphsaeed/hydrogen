//! Sub-product status aggregator, terminal briefing formatter, and self-contained
//! Warm-Paper static `products/<product>/<subject>/status.html` dashboard generator (`Phase 3.1`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use memory_core_rs::{CourseCatalogSnapshot, CourseComment, CourseQuestion};
use serde::Serialize;

use crate::base64_util::{read_png_dimensions, to_data_uri};
use crate::carbon::catalog::mineral_color_for_subject;
use crate::config::HydrogenConfig;
use crate::domain::UniversalId;
use crate::error::Result;
use crate::ingest::ledger::{
    AuthoringTaskStatus, ProductLedger, ReplicaScope, SourceStage, format_question_number_ranges,
    now_ms,
};
use crate::vault::VaultManager;
use crate::vault::seed::GYR_LOGO_BLUE_ID;

/// Per-subject summary card combining `.sqlite` vault snapshot + `inbox/` queue.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectStatusCard {
    pub subject: String,
    pub course_id: String,
    pub title: String,
    pub mineral_color: String,
    pub vault_initialized: bool,
    pub sqlite_path: PathBuf,
    pub paper_count: usize,
    pub verified_question_count: usize,
    pub replica_count: usize,
    pub topic_count: usize,
    pub topic_card_count: usize,
    pub inbox_queued_count: usize,
    pub open_comment_count: usize,
}

/// Topic coverage row across initialized subject vaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicCoverageRow {
    pub subject: String,
    pub topic_id: String,
    pub code: String,
    pub title: String,
    pub question_count: usize,
    pub card_count: usize,
    pub open_comment_count: i64,
}

/// Side-by-side comparison entry pairing a `ledger.json` original source/crop reference with its compiled `L7` question.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SideBySideQuestionRow {
    pub universal_id: String,
    pub subject: String,
    pub paper_id: Option<String>,
    pub source_file: Option<String>,
    pub source_sha256: Option<String>,
    pub archived_source_path: Option<String>,
    pub crop_path: Option<String>,
    pub crop_page: Option<u32>,
    pub crop_dimensions: Option<(u32, u32)>,
    pub solution_json_path: Option<String>,
    #[serde(skip_serializing)]
    pub crop_data_uri: Option<String>,
    pub compiled_label: Option<String>,
    pub compiled_title: Option<String>,
    pub compiled_stem: Option<String>,
    pub marks: Option<f64>,
    pub sublevel_span: Option<(i64, i64)>,
    pub qa_status: Option<String>,
    pub is_replica: bool,
    pub replica_scope: Option<ReplicaScope>,
    pub parent_question_id: Option<String>,
    pub parent_paper_id: Option<String>,
    pub replica_ids: Vec<String>,
    pub open_comment_count: i64,
}

/// Open `Snap Board` / authoring comment enriched with optional inline thumbnail data URIs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCommentEntry {
    pub subject: String,
    pub comment: CourseComment,
    #[serde(skip_serializing)]
    pub attachment_data_uris: BTreeMap<String, String>,
}

/// Complete merged status report for a sub-product (`products/<product>/<subject>/`:
/// `inbox/` + unified `ledger.json` + `<stem>.sqlite` vault).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductStatusReport {
    pub product: String,
    pub subject: String,
    pub vault_stem: String,
    pub generated_at_ms: u64,
    pub elapsed_ms: u128,
    pub ledger_path: PathBuf,
    pub status_html_path: PathBuf,
    #[serde(skip_serializing)]
    pub hydrogen_logo_data_uri: Option<String>,
    #[serde(skip_serializing)]
    pub brand_logo_data_uri: Option<String>,
    pub subjects: Vec<SubjectStatusCard>,
    pub ledger: ProductLedger,
    pub topics: Vec<TopicCoverageRow>,
    pub side_by_side_items: Vec<SideBySideQuestionRow>,
    pub open_comments: Vec<OpenCommentEntry>,
}

/// Controls how `render_status_html_with_mode` resolves breadcrumbs, file links, and image URLs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StatusRenderMode {
    /// Self-contained static file (`products/<product>/<subject>/status.html`):
    /// uses inline `data:image/png;base64,...` thumbnails and relative `inbox/` / `archive/` links.
    #[default]
    StandaloneFile,
    /// Live HTTP server (`hydrogen serve` on `:7430`):
    /// renders clickable `Products ▸ <product> ▸ <subject>` breadcrumbs, optional live Sync button,
    /// and serves crops, 180 DPI pages, PDFs, and `<QID>.json` via `file_url_prefix`.
    Server {
        home_url: String,
        product_url: String,
        file_url_prefix: String,
        hydrogen_logo_url: Option<String>,
        sync_url: Option<String>,
    },
}

/// Discover ONLY products under `products/` that have at least 1 active sub-product
/// (products with `0` active sub-products, such as an unstarted `products/gcse/`, are hidden).
pub fn discover_active_products(cfg: &HydrogenConfig) -> Result<Vec<String>> {
    if !cfg.products_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut products = Vec::new();
    for res in std::fs::read_dir(&cfg.products_dir)? {
        let entry = res?;
        if !entry.path().is_dir() {
            continue;
        }
        let Some(slug) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if slug.is_empty() || slug.starts_with('.') {
            continue;
        }
        if !discover_product_subjects(cfg, &slug)?.is_empty() {
            products.push(slug);
        }
    }
    products.sort();
    Ok(products)
}

/// Discover ONLY active sub-product subject slugs under `products/<product>/<subject>/`
/// (i.e. folders that have an initialized `.sqlite`/`.raphs` vault, `ledger.json`,
/// or non-empty `inbox/` / `archive/`). Untouched subjects never appear.
pub fn discover_product_subjects(cfg: &HydrogenConfig, product: &str) -> Result<Vec<String>> {
    let product_dir = cfg.product_dir(product);
    if !product_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut active = Vec::new();
    for res in std::fs::read_dir(&product_dir)? {
        let entry = res?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(subject) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if subject.is_empty() || subject.starts_with('.') || subject == "assets" {
            continue;
        }
        if is_active_subproduct(cfg, product, &subject)? {
            active.push(subject);
        }
    }

    active.sort_by(|a, b| {
        canonical_subject_rank(a)
            .cmp(&canonical_subject_rank(b))
            .then_with(|| a.cmp(b))
    });
    Ok(active)
}

fn is_active_subproduct(cfg: &HydrogenConfig, product: &str, subject: &str) -> Result<bool> {
    if cfg.vault_sqlite_path(product, subject).is_file()
        || cfg.vault_bundle_path(product, subject).is_file()
        || cfg.ledger_path(product, subject).is_file()
    {
        return Ok(true);
    }
    if dir_has_non_hidden_entry(&cfg.subject_inbox_dir(product, subject))? {
        return Ok(true);
    }
    if dir_has_non_hidden_entry(&cfg.subject_archive_dir(product, subject))? {
        return Ok(true);
    }
    Ok(false)
}

fn dir_has_non_hidden_entry(dir: &Path) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }
    for res in std::fs::read_dir(dir)? {
        let entry = res?;
        if let Some(name) = entry.file_name().to_str()
            && !name.starts_with('.')
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn canonical_subject_rank(subject: &str) -> usize {
    match subject {
        "maths" => 0,
        "english" => 1,
        "verbal-reasoning" => 2,
        "non-verbal-reasoning" => 3,
        "physics" => 4,
        "chemistry" => 5,
        "biology" => 6,
        "computer-science" => 7,
        "history" => 8,
        "geography" => 9,
        "economics" => 10,
        "further-maths" => 11,
        _ => 100,
    }
}

/// Build the complete merged `ProductStatusReport` for a single sub-product `(product, subject)`
/// across `products/<product>/<subject>/{inbox,archive,ledger.json,<stem>.sqlite}`
/// (updating `ledger.json` atomically).
pub async fn build_subject_status_report(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    include_html_media: bool,
) -> Result<ProductStatusReport> {
    let t0 = Instant::now();
    let subject_dir = cfg.subject_dir(product, subject);

    let sqlite_path = cfg.vault_sqlite_path(product, subject);
    let bundle_path = cfg.vault_bundle_path(product, subject);
    let mut vault_snapshot: Option<CourseCatalogSnapshot> = None;
    let mut compiled_questions: BTreeMap<String, CourseQuestion> = BTreeMap::new();
    let mut open_comments: Vec<OpenCommentEntry> = Vec::new();
    let mut hydrogen_logo_data_uri: Option<String> = None;
    let mut brand_logo_data_uri: Option<String> = None;

    if include_html_media {
        let h_logo = cfg.hydrogen_logo_path();
        if h_logo.is_file()
            && let Ok(bytes) = std::fs::read(&h_logo)
            && read_png_dimensions(&bytes).is_some()
        {
            hydrogen_logo_data_uri = Some(to_data_uri("image/png", &bytes));
        }
    }

    if sqlite_path.is_file() || bundle_path.is_file() {
        let vault = VaultManager::open_or_create(
            &sqlite_path,
            &bundle_path,
            &cfg.lesson_key,
            &cfg.device_id,
        )?;
        let snap = vault.catalog_snapshot().await?;
        if let Ok(uri) = vault.read_asset_data_uri(GYR_LOGO_BLUE_ID).await {
            brand_logo_data_uri = Some(uri);
        }

        for q_sum in &snap.questions {
            if let Some(cq) = vault.get_question(&q_sum.question_id).await? {
                compiled_questions.insert(cq.question_id.clone(), cq);
            }
        }

        let subj_comments = vault.list_comments(Some("open"), None).await?;
        for c in subj_comments {
            let mut att_uris = BTreeMap::new();
            if include_html_media {
                for att in &c.attachments {
                    if att.mime_type.starts_with("image/")
                        && let Ok(bytes) = vault.read_comment_attachment(&att.attachment_id).await
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

        vault_snapshot = Some(snap);
    }

    // Fallback to disk `products/<product>/assets/gyara-blue.png` if vault has not embedded `GYR-LOGO-BLUE` yet
    if include_html_media && brand_logo_data_uri.is_none() {
        let logo_disk = cfg.product_assets_dir(product).join("gyara-blue.png");
        if logo_disk.is_file()
            && let Ok(bytes) = std::fs::read(&logo_disk)
            && read_png_dimensions(&bytes).is_some()
        {
            brand_logo_data_uri = Some(to_data_uri("image/png", &bytes));
        }
    }

    let ledger_path = cfg.ledger_path(product, subject);
    let mut ledger = ProductLedger::load_or_new(&ledger_path, product, subject)?;
    ledger.sync_with_workspace(cfg, vault_snapshot.as_ref())?;
    ledger.save_atomic(&ledger_path)?;

    // Build Sub-Product Status Card & Topic Matrix
    let mut topic_rows = Vec::new();
    let prod_prefix = UniversalId::product_prefix_for_slug(product);
    let subj_prefix = UniversalId::subject_prefix_for_slug(subject);
    let fallback_course_id = format!("{prod_prefix}-{subj_prefix}");
    let mineral_color =
        mineral_color_for_subject(subj_prefix, canonical_subject_rank(subject)).to_string();

    let inbox_queued_count = ledger
        .sources
        .values()
        .filter(|s| s.stage == SourceStage::Queued)
        .count();

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
    } else {
        SubjectStatusCard {
            subject: subject.to_string(),
            course_id: fallback_course_id,
            title: human_subject_title(subject),
            mineral_color,
            vault_initialized: false,
            sqlite_path,
            paper_count: 0,
            verified_question_count: 0,
            replica_count: 0,
            topic_count: 0,
            topic_card_count: 0,
            inbox_queued_count,
            open_comment_count: 0,
        }
    };

    // Build Side-by-Side (`ledger.items` original file/crop <-> compiled `CourseQuestion`) table
    let mut side_by_side_items = Vec::with_capacity(ledger.items.len());
    for (uid, l_item) in &ledger.items {
        let compiled_entry = compiled_questions.get(uid);

        let crop_data_uri = if include_html_media {
            l_item.crop_path.as_ref().and_then(|rel| {
                let abs = subject_dir.join(rel);
                std::fs::read(abs)
                    .ok()
                    .map(|bytes| to_data_uri("image/png", &bytes))
            })
        } else {
            None
        };

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
            crop_data_uri,
            compiled_label: compiled_entry.map(|cq| cq.label.clone()),
            compiled_title: compiled_entry.map(|cq| cq.title.clone()),
            compiled_stem: compiled_entry.map(|cq| cq.stem_text.clone()),
            marks: compiled_entry.map(|cq| cq.marks),
            sublevel_span: compiled_entry.map(|cq| (cq.min_sublevel, cq.max_sublevel)),
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

    let elapsed_ms = t0.elapsed().as_millis();
    Ok(ProductStatusReport {
        product: product.to_string(),
        subject: subject.to_string(),
        vault_stem: HydrogenConfig::vault_stem(product, subject),
        generated_at_ms: now_ms(),
        elapsed_ms,
        ledger_path,
        status_html_path: cfg.status_html_path(product, subject),
        hydrogen_logo_data_uri,
        brand_logo_data_uri,
        subjects: vec![subject_card],
        ledger,
        topics: topic_rows,
        side_by_side_items,
        open_comments,
    })
}

/// Format a concise, high-signal terminal briefing for Antigravity & CLI sessions.
pub fn format_terminal_status(report: &ProductStatusReport) -> String {
    let mut out = String::with_capacity(2048);
    let _ = writeln!(
        out,
        "=== Hydrogen Sub-Product Status: `{}/{}` (`{}.raphs` · {} ms) ===",
        report.product, report.subject, report.vault_stem, report.elapsed_ms
    );
    let _ = writeln!(out, "Ledger : {}", report.ledger_path.display());

    // 1. Subjects
    let _ = writeln!(out, "\n1. Subject Vaults:");
    for s in &report.subjects {
        let v_tag = if s.vault_initialized {
            "READY"
        } else {
            "UNSEEDED"
        };
        let _ = writeln!(
            out,
            "   [{v_tag:8}] {:22} ({}) · {} papers · {} verified Qs (+{} replicas) · {} topics / {} cards · {} inbox queued · {} open comments",
            s.subject,
            s.course_id,
            s.paper_count,
            s.verified_question_count,
            s.replica_count,
            s.topic_count,
            s.topic_card_count,
            s.inbox_queued_count,
            s.open_comment_count
        );
    }

    // 2. SHA-256 Source Inventory
    let _ = writeln!(
        out,
        "\n2. Raw Source Inventory (`inbox/` & `archive/` — {} unique SHA-256 files):",
        report.ledger.sources.len()
    );
    if report.ledger.sources.is_empty() {
        let _ = writeln!(out, "   (no raw PDF/image sources found)");
    } else {
        for src in report.ledger.sources.values() {
            let short_sha = &src.sha256[..src.sha256.len().min(12)];
            let pid = src.paper_id.as_deref().unwrap_or("-");
            let dup_warn = if src.has_renamed_duplicates() {
                format!(" [RENAMED DUPLICATE: {}]", src.duplicate_paths.join(", "))
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "   - [{:11}] sha256:{short_sha} · {}/{} ({} B) -> paper={pid}{dup_warn}",
                src.stage.as_str(),
                src.subject,
                src.primary_filename,
                src.size_bytes
            );
        }
    }

    // 3. Paper Progress
    let _ = writeln!(
        out,
        "\n3. Paper Progress & Checklists ({} papers):",
        report.ledger.papers.len()
    );
    if report.ledger.papers.is_empty() {
        let _ = writeln!(out, "   (no papers sliced or compiled yet)");
    } else {
        for p in report.ledger.papers.values() {
            let exp_str = p
                .expected_questions
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".to_string());
            let missing_str = format_question_number_ranges(&p.missing_question_numbers);
            let rep_tag = if p.is_replica {
                format!(
                    " [whole-paper replica <- {}]",
                    p.parent_paper_id.as_deref().unwrap_or("?")
                )
            } else if !p.replica_paper_ids.is_empty() {
                format!(" [replica papers: {}]", p.replica_paper_ids.join(", "))
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "   - {} ({}) [{}]{rep_tag} : verified {}/{exp_str} · sliced {} · replicas {} · missing: {missing_str}",
                p.paper_id, p.subject, p.stage, p.verified_count, p.sliced_count, p.replica_count
            );
            for note in &p.notes {
                let _ = writeln!(out, "       note: {note}");
            }
        }
    }

    // 4. Replicas Lineage Summary
    let replica_papers: Vec<_> = report
        .ledger
        .papers
        .values()
        .filter(|p| p.is_replica)
        .collect();
    let replica_items: Vec<_> = report
        .ledger
        .items
        .values()
        .filter(|i| i.is_replica)
        .collect();
    let _ = writeln!(
        out,
        "\n4. Replicas Created & Origin Lineage ({} replica papers · {} replica questions):",
        replica_papers.len(),
        replica_items.len()
    );
    if replica_papers.is_empty() && replica_items.is_empty() {
        let _ = writeln!(
            out,
            "   (no whole-paper or single-question replicas created yet)"
        );
    } else {
        for rp in replica_papers {
            let _ = writeln!(
                out,
                "   - [whole_paper]     {} <- original paper {} (source: {})",
                rp.paper_id,
                rp.parent_paper_id.as_deref().unwrap_or("-"),
                rp.source_file.as_deref().unwrap_or("-")
            );
        }
        for ri in replica_items {
            let scope_lbl = ri
                .replica_scope
                .map(|s| s.as_str())
                .unwrap_or("single_question");
            let origin_ref = match (
                ri.replica_scope,
                &ri.parent_question_id,
                &ri.parent_paper_id,
            ) {
                (Some(ReplicaScope::SingleQuestion), Some(pq), _) => {
                    format!("original question {pq} (single-question replica)")
                }
                (Some(ReplicaScope::WholePaper), Some(pq), Some(pp)) => {
                    format!("original paper {pp} / question {pq}")
                }
                (_, Some(pq), _) => format!("original question {pq}"),
                (_, None, Some(pp)) => format!("original paper {pp}"),
                _ => "standalone".to_string(),
            };
            let _ = writeln!(
                out,
                "   - [{scope_lbl:15}] {} <- {origin_ref} (source: {})",
                ri.universal_id,
                ri.source_file.as_deref().unwrap_or("-")
            );
        }
    }

    // 5. Open Comments & Tasks
    let _ = writeln!(
        out,
        "\n5. Open `Snap Board` Comments ({}) & Pending Tasks ({}):",
        report.open_comments.len(),
        report.ledger.pending_task_count()
    );
    for oc in &report.open_comments {
        let c = &oc.comment;
        let loc = format_comment_subtarget(c);
        let _ = writeln!(
            out,
            "   - [comment:{}] {} on {}{} ({} attachment(s)): {}",
            c.comment_kind,
            c.comment_id,
            c.target_id,
            loc,
            c.attachments.len(),
            c.comment_text
        );
    }
    for task in report
        .ledger
        .tasks
        .iter()
        .filter(|t| t.status != AuthoringTaskStatus::Completed)
    {
        let diff = task
            .difficulty_hint
            .as_deref()
            .map(|d| format!(" ({d})"))
            .unwrap_or_default();
        let note = task
            .note
            .as_deref()
            .map(|n| format!(" — {n}"))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "   - [task:{}:{}] {} -> {}:{}{}{}",
            task.status, task.kind, task.task_id, task.target_kind, task.target_id, diff, note
        );
    }

    out
}

fn resolve_rel_href(mode: &StatusRenderMode, rel_path: &str) -> String {
    match mode {
        StatusRenderMode::StandaloneFile => rel_path.to_string(),
        StatusRenderMode::Server {
            file_url_prefix, ..
        } => format!(
            "{}/{}",
            file_url_prefix.trim_end_matches('/'),
            rel_path.trim_start_matches('/')
        ),
    }
}

/// Render a self-contained Warm-Paper HTML dashboard (`products/<product>/<subject>/status.html`).
pub fn render_status_html(report: &ProductStatusReport) -> String {
    render_status_html_with_mode(report, &StatusRenderMode::StandaloneFile)
}

/// Render the Carbon Warm-Paper HTML status dashboard in either `StandaloneFile` or live `Server` mode.
pub fn render_status_html_with_mode(
    report: &ProductStatusReport,
    mode: &StatusRenderMode,
) -> String {
    let total_verified: usize = report
        .subjects
        .iter()
        .map(|s| s.verified_question_count)
        .sum();
    let total_replicas: usize = report.subjects.iter().map(|s| s.replica_count).sum();
    let total_queued: usize = report
        .ledger
        .sources
        .values()
        .filter(|s| s.stage == SourceStage::Queued)
        .count();
    let total_duplicates: usize = report
        .ledger
        .sources
        .values()
        .map(|s| s.duplicate_paths.len())
        .sum();
    let total_cards: usize = report.subjects.iter().map(|s| s.topic_card_count).sum();
    let open_comments_len = report.open_comments.len();
    let pending_tasks_len = report.ledger.pending_task_count();
    let questions_len = report.side_by_side_items.len();
    let topics_len = report.topics.len();
    let sources_len = report.ledger.sources.len();
    let replicas_total = report
        .ledger
        .papers
        .values()
        .filter(|p| p.is_replica)
        .count()
        + report
            .ledger
            .items
            .values()
            .filter(|i| i.is_replica)
            .count();

    let mut html = String::with_capacity(45_056);
    html.push_str(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Hydrogen Authoring Status — "##,
    );
    html.push_str(&escape_html(&format!(
        "{}/{} ({})",
        report.product, report.subject, report.vault_stem
    )));
    html.push_str(
        r##"</title>
  <style>
    :root {
      --paper-bg: #FDFBF7;
      --board-bg: #FDFCF9;
      --chrome-bg: #F2ECE1;
      --chrome-border: #DBD2C2;
      --pill-bg: #FCF9F2;
      --card-bg: #FFFFFF;
      --card-border: #DEE4EC;
      --ink-primary: #1F2421;
      --ink-muted: #68645B;
      --blue: #2563EB;
      --blue-bg: #EFF6FF;
      --blue-border: #BAD4F7;
      --emerald: #15803D;
      --emerald-bg: #F0FDF4;
      --emerald-border: #B2E8C6;
      --rose: #DC2626;
      --rose-bg: #FFF1F2;
      --rose-border: #FBABB8;
      --amber: #B45309;
      --amber-bg: #FEF9C3;
      --amber-border: #CA8A04;
      --violet: #6D5CAE;
    }
    * { box-sizing: border-box; }
    html { scrollbar-gutter: stable; }
    body {
      margin: 0;
      background: var(--paper-bg);
      color: var(--ink-primary);
      font-family: "Lato", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      line-height: 1.5;
    }
    a { color: var(--blue); text-decoration: none; }
    a:hover { text-decoration: underline; }
    header.topbar {
      width: 100%;
      height: 56px;
      background: var(--chrome-bg);
      border-bottom: 1px solid var(--chrome-border);
      padding: 0 28px;
      display: grid;
      grid-template-columns: 1fr auto 1fr;
      align-items: center;
      gap: 16px;
    }
    .topbar-left {
      justify-self: start;
      display: flex;
      align-items: center;
    }
    .topbar-center {
      justify-self: center;
      display: flex;
      align-items: center;
      justify-content: center;
      text-align: center;
    }
    .topbar-center h1 {
      margin: 0;
      font-size: 1.16rem;
      font-weight: 700;
      letter-spacing: -0.01em;
      color: var(--ink-primary);
      white-space: nowrap;
    }
    .topbar-right {
      justify-self: end;
      display: flex;
      align-items: center;
    }
    .hydrogen-mark {
      height: 36px;
      width: auto;
      background: transparent;
      object-fit: contain;
      display: block;
    }
    .page-hero {
      width: fit-content;
      max-width: 100%;
      margin: 0 auto 10px;
      padding: 5px 12px;
      background: var(--board-bg);
      border: 1px solid var(--chrome-border);
      border-radius: 999px;
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: center;
      gap: 6px;
    }
    .hero-sep {
      color: var(--chrome-border);
      font-size: 0.8rem;
      user-select: none;
    }
    img.brand-logo {
      height: 20px;
      width: auto;
      background: transparent;
      display: block;
      object-fit: contain;
    }
    .breadcrumbs {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex-wrap: wrap;
      gap: 4px;
      font-size: 0.75rem;
      color: var(--ink-muted);
    }
    .crumb-pill {
      background: var(--pill-bg);
      border: 1px solid var(--chrome-border);
      border-radius: 5px;
      padding: 1px 6px;
      font-size: 0.74rem;
      font-weight: 600;
      color: var(--ink-primary);
    }
    a.crumb-pill:hover {
      border-color: var(--blue);
      color: var(--blue);
      text-decoration: none;
    }
    .crumb-active {
      font-weight: 700;
      font-size: 0.75rem;
      color: var(--ink-primary);
    }
    .crumb-sep { color: var(--ink-muted); font-size: 0.7rem; }
    .kpi-strip {
      display: inline-flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: center;
      gap: 5px;
    }
    .kpi-pill {
      background: var(--pill-bg);
      border: 1px solid var(--chrome-border);
      border-radius: 999px;
      padding: 2px 8px;
      font-size: 0.73rem;
      font-weight: 600;
      line-height: 1.35;
    }
    .kpi-pill.alert {
      background: var(--rose-bg);
      border-color: var(--rose-border);
      color: #881337;
    }
    .kpi-pill.ok {
      background: var(--emerald-bg);
      border-color: var(--emerald-border);
      color: var(--emerald);
    }
    .sync-btn {
      background: var(--blue);
      color: #FFFFFF;
      border: 1px solid #1D4ED8;
      border-radius: 999px;
      padding: 3px 10px;
      font-size: 0.73rem;
      font-weight: 700;
      cursor: pointer;
    }
    .sync-btn:hover { background: #1D4ED8; }
    main {
      max-width: 1380px;
      margin: 0 auto;
      padding: 14px 28px 52px;
    }
    nav.section-tabs {
      width: fit-content;
      max-width: 100%;
      margin: 0 auto 18px;
      display: flex;
      flex-wrap: wrap;
      justify-content: center;
      gap: 5px;
    }
    button.tab-chip, button.filter-chip {
      background: var(--pill-bg);
      color: var(--ink-primary);
      border: 1px solid var(--chrome-border);
      border-radius: 999px;
      padding: 3px 10px;
      font-size: 0.75rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.12s ease;
    }
    button.tab-chip:hover, button.filter-chip:hover {
      border-color: var(--blue-border);
      background: var(--blue-bg);
    }
    button.tab-chip.active, button.filter-chip.active {
      background: var(--blue);
      color: #FFFFFF;
      border-color: var(--blue);
    }
    .filter-bar {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      margin-bottom: 12px;
    }
    .filter-pills {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }
    input.q-search {
      background: #FFFFFF;
      border: 1px solid var(--card-border);
      border-radius: 999px;
      padding: 6px 14px;
      font-size: 0.84rem;
      min-width: 260px;
      color: var(--ink-primary);
    }
    input.q-search:focus {
      outline: none;
      border-color: var(--blue);
      box-shadow: 0 0 0 2px var(--blue-bg);
    }
    section {
      margin-bottom: 30px;
    }
    section.hidden-sec, tr.hidden-row {
      display: none !important;
    }
    h2 {
      font-size: 1.06rem;
      margin: 0 0 12px;
      color: var(--ink-primary);
      border-bottom: 1px solid var(--chrome-border);
      padding-bottom: 6px;
      text-align: center;
    }
    .subject-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 14px;
    }
    .subject-card {
      cursor: pointer;
      background: var(--card-bg);
      border: 1px solid var(--card-border);
      border-left-width: 6px;
      border-radius: 10px;
      padding: 14px 16px;
      box-shadow: 0 1px 2px rgba(31, 36, 33, 0.04);
      transition: border-color 0.14s ease, box-shadow 0.14s ease, transform 0.14s ease;
    }
    .subject-card:hover {
      border-color: var(--blue-border);
      box-shadow: 0 6px 16px rgba(31, 36, 33, 0.08);
      transform: translateY(-1px);
    }
    .subject-card .subj-head {
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      margin-bottom: 6px;
    }
    .subject-card .subj-title {
      font-weight: 700;
      font-size: 1.02rem;
    }
    .badge {
      display: inline-block;
      padding: 2px 8px;
      border-radius: 999px;
      font-size: 0.75rem;
      font-weight: 600;
      background: #F1F5F9;
      color: #475569;
      border: 1px solid #CBD5E1;
    }
    .badge.ok { background: var(--emerald-bg); color: var(--emerald); border-color: var(--emerald-border); }
    .badge.progress { background: var(--blue-bg); color: var(--blue); border-color: var(--blue-border); }
    .badge.warn { background: var(--amber-bg); color: var(--amber); border-color: var(--amber-border); }
    .badge.danger { background: var(--rose-bg); color: var(--rose); border-color: var(--rose-border); }
    .badge.ai { background: #F5F3FF; color: var(--violet); border-color: #DDD6FE; }
    .subj-metrics {
      font-size: 0.84rem;
      color: var(--ink-muted);
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 4px 10px;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      background: var(--card-bg);
      border: 1px solid var(--card-border);
      border-radius: 10px;
      overflow: hidden;
      font-size: 0.88rem;
    }
    th, td {
      text-align: left;
      padding: 10px 12px;
      border-bottom: 1px solid #EEF2F6;
      vertical-align: top;
    }
    th {
      background: var(--board-bg);
      color: var(--ink-muted);
      font-weight: 700;
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.03em;
    }
    tr:last-child td { border-bottom: none; }
    code {
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      font-size: 0.82rem;
      background: #F8FAFC;
      padding: 1px 5px;
      border-radius: 4px;
      border: 1px solid #E2E8F0;
    }
    .progress-bar-track {
      width: 100%;
      height: 10px;
      background: #EDF2F7;
      border-radius: 999px;
      overflow: hidden;
      margin-top: 4px;
    }
    .progress-bar-fill {
      height: 100%;
      background: var(--emerald);
      border-radius: 999px;
    }
    .crop-preview {
      max-width: 380px;
      max-height: 220px;
      border: 1px solid var(--card-border);
      border-radius: 6px;
      background: #FFFFFF;
      display: block;
      cursor: zoom-in;
    }
    .stem-box {
      white-space: pre-wrap;
      background: var(--board-bg);
      border: 1px solid #E2E8F0;
      border-radius: 6px;
      padding: 8px 10px;
      font-size: 0.87rem;
    }
    .comment-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
      gap: 14px;
    }
    .comment-card {
      background: var(--rose-bg);
      border: 1px solid var(--rose-border);
      border-radius: 10px;
      padding: 12px 14px;
    }
    .comment-card img.snap-thumb {
      max-width: 100%;
      max-height: 200px;
      border-radius: 6px;
      border: 1px solid var(--chrome-border);
      margin-top: 8px;
      background: #FFFFFF;
      display: block;
      cursor: zoom-in;
    }
    .muted { color: var(--ink-muted); font-size: 0.82rem; }
    #img-lightbox {
      display: none;
      position: fixed;
      inset: 0;
      background: rgba(31, 36, 33, 0.82);
      z-index: 9999;
      align-items: center;
      justify-content: center;
      padding: 28px;
    }
    #img-lightbox.open { display: flex; }
    .lightbox-card {
      background: var(--paper-bg);
      border: 1px solid var(--chrome-border);
      border-radius: 12px;
      padding: 14px;
      max-width: 94vw;
      max-height: 92vh;
      display: flex;
      flex-direction: column;
      gap: 8px;
      box-shadow: 0 20px 50px rgba(0, 0, 0, 0.35);
    }
    .lightbox-card img {
      max-width: 90vw;
      max-height: 80vh;
      object-fit: contain;
      background: #FFFFFF;
      border-radius: 6px;
      border: 1px solid var(--card-border);
    }
    .lightbox-bar {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 12px;
      font-size: 0.84rem;
      color: var(--ink-muted);
    }
  </style>
</head>
<body>
  <header class="topbar">
    <div class="topbar-left">"##,
    );

    let h_logo_src = match mode {
        StatusRenderMode::Server {
            hydrogen_logo_url: Some(url),
            ..
        } => Some(url.as_str()),
        _ => report.hydrogen_logo_data_uri.as_deref(),
    };
    if let Some(h_uri) = h_logo_src {
        match mode {
            StatusRenderMode::Server { home_url, .. } => {
                let _ = write!(
                    html,
                    r##"<a href="{}"><img class="hydrogen-mark" src="{}" alt="Hydrogen"></a>"##,
                    escape_html(home_url),
                    escape_html(h_uri)
                );
            }
            StatusRenderMode::StandaloneFile => {
                let _ = write!(
                    html,
                    r##"<img class="hydrogen-mark" src="{}" alt="Hydrogen">"##,
                    escape_html(h_uri)
                );
            }
        }
    }
    html.push_str(
        r##"</div>
    <div class="topbar-center">
      <h1>Hydrogen Dashboard</h1>
    </div>
    <div class="topbar-right"></div>
  </header>
  <main>
    <div class="page-hero">"##,
    );

    if let Some(ref logo_uri) = report.brand_logo_data_uri {
        let _ = write!(
            html,
            r##"<img class="brand-logo" src="{}" alt="Product Logo">"##,
            escape_html(logo_uri)
        );
    }

    let breadcrumbs_html = match mode {
        StatusRenderMode::Server {
            home_url,
            product_url,
            ..
        } => format!(
            r##"<nav class="breadcrumbs"><a class="crumb-pill" href="{}">Products</a><span class="crumb-sep">&#9656;</span><a class="crumb-pill" href="{}">{}</a><span class="crumb-sep">&#9656;</span><span class="crumb-active">{}</span></nav>"##,
            escape_html(home_url),
            escape_html(product_url),
            escape_html(&report.product),
            escape_html(&report.subject)
        ),
        StatusRenderMode::StandaloneFile => format!(
            r##"<nav class="breadcrumbs"><span class="crumb-pill">Products</span><span class="crumb-sep">&#9656;</span><span class="crumb-pill">{}</span><span class="crumb-sep">&#9656;</span><span class="crumb-active">{}</span></nav>"##,
            escape_html(&report.product),
            escape_html(&report.subject)
        ),
    };

    let sync_btn_html = match mode {
        StatusRenderMode::Server {
            sync_url: Some(s_url),
            ..
        } => format!(
            r##"<form method="post" action="{}" style="margin:0;"><button type="submit" class="sync-btn">Sync Vault</button></form>"##,
            escape_html(s_url)
        ),
        _ => String::new(),
    };

    let rep_suffix = if total_replicas > 0 {
        format!(" (+{total_replicas} AI)")
    } else {
        String::new()
    };
    let cards_pill = if total_cards > 0 {
        format!(r##"<span class="kpi-pill">Cards: {total_cards}</span>"##)
    } else {
        String::new()
    };

    let _ = write!(
        html,
        r##"
      {breadcrumbs_html}
      <code style="font-size:0.72rem;">{}.raphs</code>
      <span class="hero-sep">&middot;</span>
      <div class="kpi-strip">
        <span class="kpi-pill ok">Verified: {total_verified}{}</span>
        {cards_pill}
        <span class="kpi-pill{}">Inbox: {total_queued}{}</span>
        <span class="kpi-pill{}">Comments: {open_comments_len}</span>
        <span class="kpi-pill{}">Tasks: {pending_tasks_len}</span>
        {sync_btn_html}
      </div>
    </div>
    <nav class="section-tabs" aria-label="Dashboard Sections">
      <button type="button" class="tab-chip active" data-section-target="all">All</button>
      <button type="button" class="tab-chip" data-section-target="overview">Overview</button>
      <button type="button" class="tab-chip" data-section-target="questions">Questions ({questions_len})</button>
      <button type="button" class="tab-chip" data-section-target="topics">Topics ({topics_len})</button>
      <button type="button" class="tab-chip" data-section-target="replicas">Replicas ({replicas_total})</button>
      <button type="button" class="tab-chip" data-section-target="comments">Comments ({})</button>
      <button type="button" class="tab-chip" data-section-target="sources">Sources ({sources_len})</button>
    </nav>
"##,
        escape_html(&report.vault_stem),
        escape_html(&rep_suffix),
        if total_duplicates > 0 { " alert" } else { "" },
        if total_duplicates > 0 {
            format!(" ({total_duplicates} dup)")
        } else {
            String::new()
        },
        if open_comments_len > 0 { " alert" } else { "" },
        if pending_tasks_len > 0 { " alert" } else { "" },
        open_comments_len + pending_tasks_len,
    );

    // 1. Subject Cards
    html.push_str("    <section data-section=\"overview\">\n      <h2>1. Subject Vault Overview</h2>\n      <div class=\"subject-grid\">\n");
    for s in &report.subjects {
        let vault_badge = if s.vault_initialized {
            r##"<span class="badge ok">L7 Vault Ready</span>"##
        } else {
            r##"<span class="badge">Unseeded</span>"##
        };
        let _ = write!(
            html,
            r##"        <div class="subject-card" data-jump-section="questions" role="button" tabindex="0" style="border-left-color: {};">
          <div class="subj-head">
            <span class="subj-title">{}</span>
            {}
          </div>
          <div class="muted" style="margin-bottom: 8px;"><code>{}</code> · <code>{}</code></div>
          <div class="subj-metrics">
            <div>Papers: <strong>{}</strong></div>
            <div>Verified Qs: <strong>{}</strong></div>
            <div>Topics: <strong>{}</strong></div>
            <div>Topic Cards: <strong>{}</strong></div>
            <div>Inbox Queued: <strong>{}</strong></div>
            <div>Open Comments: <strong>{}</strong></div>
          </div>
        </div>
"##,
            escape_html(&s.mineral_color),
            escape_html(&s.title),
            vault_badge,
            escape_html(&s.course_id),
            escape_html(&s.subject),
            s.paper_count,
            s.verified_question_count,
            s.topic_count,
            s.topic_card_count,
            s.inbox_queued_count,
            s.open_comment_count
        );
    }
    html.push_str("      </div>\n    </section>\n");

    // 2. Paper Progress Bars & Missing Checklist
    html.push_str("    <section data-section=\"overview\">\n      <h2>2. Paper Progress &amp; Item Checklist</h2>\n");
    if report.ledger.papers.is_empty() {
        html.push_str("      <p class=\"muted\">No papers sliced or registered yet.</p>\n");
    } else {
        html.push_str(
            r##"      <table>
        <thead>
          <tr>
            <th>Paper ID</th>
            <th>Subject</th>
            <th>Original Source File (PDF / PNG)</th>
            <th>Stage</th>
            <th style="min-width: 200px;">Progress (Verified / Expected)</th>
            <th>Sliced Crops</th>
            <th>Missing / Next Questions</th>
            <th>Notes</th>
          </tr>
        </thead>
        <tbody>
"##,
        );
        for p in report.ledger.papers.values() {
            let denom = p
                .expected_questions
                .unwrap_or_else(|| p.sliced_count.max(p.verified_count))
                .max(1);
            let pct = ((f64::from(p.verified_count) / f64::from(denom)) * 100.0).clamp(0.0, 100.0);
            let exp_label = p
                .expected_questions
                .map(|e| e.to_string())
                .unwrap_or_else(|| "?".to_string());
            let missing_str = format_question_number_ranges(&p.missing_question_numbers);
            let notes_str = if p.notes.is_empty() {
                "-".to_string()
            } else {
                escape_html(&p.notes.join("; "))
            };
            let source_cell = match (&p.source_file, &p.archived_source_path) {
                (Some(fname), Some(rel)) => {
                    let href = resolve_rel_href(mode, rel);
                    format!(
                        r##"<a href="{}"><code>{}</code></a><div class="muted"><code>{}</code></div>"##,
                        escape_html(&href),
                        escape_html(fname),
                        escape_html(rel)
                    )
                }
                (Some(fname), None) => format!(r##"<code>{}</code>"##, escape_html(fname)),
                _ => r##"<span class="muted">-</span>"##.to_string(),
            };
            let paper_id_cell = if p.is_replica {
                format!(
                    r##"<code>{}</code><div><span class="badge ai">Whole-Paper Replica &larr; {}</span></div>"##,
                    escape_html(&p.paper_id),
                    escape_html(p.parent_paper_id.as_deref().unwrap_or("-"))
                )
            } else if !p.replica_paper_ids.is_empty() {
                format!(
                    r##"<code>{}</code><div class="muted">Replicas: <code>{}</code></div>"##,
                    escape_html(&p.paper_id),
                    escape_html(&p.replica_paper_ids.join(", "))
                )
            } else {
                format!(r##"<code>{}</code>"##, escape_html(&p.paper_id))
            };
            let _ = write!(
                html,
                r##"          <tr>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>
              <div><strong>{} / {}</strong> ({pct:.0}%)</div>
              <div class="progress-bar-track"><div class="progress-bar-fill" style="width: {pct:.1}%;"></div></div>
            </td>
            <td>{}</td>
            <td><code>{}</code></td>
            <td>{}</td>
          </tr>
"##,
                paper_id_cell,
                escape_html(&p.subject),
                source_cell,
                stage_badge_html(p.stage),
                p.verified_count,
                exp_label,
                p.sliced_count,
                escape_html(&missing_str),
                notes_str
            );
        }
        html.push_str("        </tbody>\n      </table>\n");
    }
    html.push_str("    </section>\n");

    // 3. Raw Inbox & Archive SHA-256 Inventory
    html.push_str("    <section data-section=\"sources\">\n      <h2>3. SHA-256 Source File Inventory (<code>inbox/</code> &amp; <code>archive/</code>)</h2>\n");
    if report.ledger.sources.is_empty() {
        html.push_str("      <p class=\"muted\">No raw PDF or image files discovered.</p>\n");
    } else {
        html.push_str(
            r##"      <table>
        <thead>
          <tr>
            <th>SHA-256</th>
            <th>Subject</th>
            <th>Primary File</th>
            <th>Size</th>
            <th>Assigned Paper</th>
            <th>Stage</th>
            <th>Duplicate Check</th>
          </tr>
        </thead>
        <tbody>
"##,
        );
        for src in report.ledger.sources.values() {
            let short_sha = &src.sha256[..src.sha256.len().min(12)];
            let mut path_links = Vec::new();
            for p in &src.inbox_paths {
                let href = resolve_rel_href(mode, p);
                path_links.push(format!(
                    r##"<a href="{}"><code>{}</code></a>"##,
                    escape_html(&href),
                    escape_html(p)
                ));
            }
            for p in &src.archive_paths {
                let href = resolve_rel_href(mode, p);
                path_links.push(format!(
                    r##"<a href="{}"><code>{}</code></a>"##,
                    escape_html(&href),
                    escape_html(p)
                ));
            }
            let file_cell = if path_links.is_empty() {
                format!(r##"<code>{}</code>"##, escape_html(&src.primary_filename))
            } else {
                format!(
                    r##"<div><strong><code>{}</code></strong></div><div class="muted" style="margin-top:3px;">{}</div>"##,
                    escape_html(&src.primary_filename),
                    path_links.join("<br>")
                )
            };
            let dup_cell = if src.has_renamed_duplicates() {
                format!(
                    r##"<span class="badge danger">RENAMED DUPLICATE</span> <code>{}</code>"##,
                    escape_html(&src.duplicate_paths.join(", "))
                )
            } else {
                r##"<span class="badge ok">Unique</span>"##.to_string()
            };
            let _ = write!(
                html,
                r##"          <tr>
            <td><code>{}</code></td>
            <td>{}</td>
            <td>{}</td>
            <td>{} B</td>
            <td><code>{}</code></td>
            <td>{}</td>
            <td>{}</td>
          </tr>
"##,
                escape_html(short_sha),
                escape_html(&src.subject),
                file_cell,
                src.size_bytes,
                escape_html(src.paper_id.as_deref().unwrap_or("-")),
                stage_badge_html(src.stage),
                dup_cell
            );
        }
        html.push_str("        </tbody>\n      </table>\n");
    }
    html.push_str("    </section>\n");

    // 4. Topic Coverage Matrix
    html.push_str("    <section data-section=\"topics\">\n      <h2>4. Curriculum Topic &amp; Concept Card Matrix</h2>\n");
    if report.topics.is_empty() {
        html.push_str("      <p class=\"muted\">No curriculum topics seeded yet.</p>\n");
    } else {
        html.push_str(
            r##"      <table>
        <thead>
          <tr>
            <th>Subject</th>
            <th>Topic ID</th>
            <th>Code</th>
            <th>Strand Title</th>
            <th>Linked Questions</th>
            <th>Topic Cards</th>
            <th>Open Comments</th>
          </tr>
        </thead>
        <tbody>
"##,
        );
        for t in &report.topics {
            let _ = write!(
                html,
                r##"          <tr>
            <td>{}</td>
            <td><code>{}</code></td>
            <td><span class="badge">{}</span></td>
            <td>{}</td>
            <td><strong>{}</strong></td>
            <td>{}</td>
            <td>{}</td>
          </tr>
"##,
                escape_html(&t.subject),
                escape_html(&t.topic_id),
                escape_html(&t.code),
                escape_html(&t.title),
                t.question_count,
                t.card_count,
                t.open_comment_count
            );
        }
        html.push_str("        </tbody>\n      </table>\n");
    }
    html.push_str("    </section>\n");

    // 5. Replicas Created & Origin Lineage
    html.push_str("    <section data-section=\"replicas\">\n      <h2>5. Replicas Created &amp; Origin Lineage</h2>\n");
    let html_rep_papers: Vec<_> = report
        .ledger
        .papers
        .values()
        .filter(|p| p.is_replica)
        .collect();
    let html_rep_items: Vec<_> = report
        .ledger
        .items
        .values()
        .filter(|i| i.is_replica)
        .collect();
    if html_rep_papers.is_empty() && html_rep_items.is_empty() {
        html.push_str("      <p class=\"muted\">No whole-paper or single-question replicas created yet.</p>\n");
    } else {
        html.push_str(
            r##"      <table>
        <thead>
          <tr>
            <th>Replica ID</th>
            <th>Replica Scope</th>
            <th>Original Paper / Question Reference</th>
            <th>Original Source File &amp; Page</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
"##,
        );
        for rp in html_rep_papers {
            let src_label = match (&rp.source_file, &rp.archived_source_path) {
                (Some(sf), Some(rel)) => {
                    let href = resolve_rel_href(mode, rel);
                    format!(
                        r##"<a href="{}"><code>{}</code></a>"##,
                        escape_html(&href),
                        escape_html(sf)
                    )
                }
                (Some(sf), None) => format!(r##"<code>{}</code>"##, escape_html(sf)),
                _ => r##"<span class="muted">-</span>"##.to_string(),
            };
            let _ = write!(
                html,
                r##"          <tr>
            <td><code>{}</code></td>
            <td><span class="badge ai">Whole-Paper Replica</span></td>
            <td>Original Paper: <code>{}</code></td>
            <td>{}</td>
            <td>{}</td>
          </tr>
"##,
                escape_html(&rp.paper_id),
                escape_html(rp.parent_paper_id.as_deref().unwrap_or("-")),
                src_label,
                stage_badge_html(rp.stage)
            );
        }
        for ri in html_rep_items {
            let (scope_badge, origin_cell) = match ri.replica_scope {
                Some(ReplicaScope::WholePaper) => (
                    r##"<span class="badge ai">Whole-Paper Member</span>"##,
                    format!(
                        r##"Original Paper: <code>{}</code> &middot; Question: <code>{}</code>"##,
                        escape_html(ri.parent_paper_id.as_deref().unwrap_or("-")),
                        escape_html(ri.parent_question_id.as_deref().unwrap_or("-"))
                    ),
                ),
                _ => (
                    r##"<span class="badge ai">Single-Question Replica</span>"##,
                    format!(
                        r##"Original Question Only: <code>{}</code>{}"##,
                        escape_html(ri.parent_question_id.as_deref().unwrap_or("-")),
                        ri.parent_paper_id
                            .as_deref()
                            .map(|pp| format!(
                                " <span class=\"muted\">(in <code>{}</code>)</span>",
                                escape_html(pp)
                            ))
                            .unwrap_or_default()
                    ),
                ),
            };
            let page_suffix = ri.page.map(|pg| format!(" · p.{pg}")).unwrap_or_default();
            let src_cell = match (&ri.source_file, &ri.archived_source_path) {
                (Some(sf), Some(rel)) => {
                    let href = resolve_rel_href(mode, rel);
                    format!(
                        r##"<a href="{}"><code>{}</code></a>{}"##,
                        escape_html(&href),
                        escape_html(sf),
                        escape_html(&page_suffix)
                    )
                }
                (Some(sf), None) => format!(
                    r##"<code>{}</code>{}"##,
                    escape_html(sf),
                    escape_html(&page_suffix)
                ),
                _ => r##"<span class="muted">-</span>"##.to_string(),
            };
            let qa_badge = ri.qa_status.as_deref().unwrap_or("draft");
            let _ = write!(
                html,
                r##"          <tr>
            <td><code>{}</code></td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td><span class="badge ok">{}</span></td>
          </tr>
"##,
                escape_html(&ri.universal_id),
                scope_badge,
                origin_cell,
                src_cell,
                escape_html(qa_badge)
            );
        }
        html.push_str("        </tbody>\n      </table>\n");
    }
    html.push_str("    </section>\n");

    // 6. Side-by-Side `archive/` Crop vs. Compiled Stem Table
    html.push_str("    <section data-section=\"questions\">\n      <h2>6. Side-by-Side <code>archive/</code> Crop vs. Compiled Question Stem</h2>\n");
    if report.side_by_side_items.is_empty() {
        html.push_str("      <p class=\"muted\">No sliced crops or compiled questions yet.</p>\n");
    } else {
        html.push_str(
            r##"      <div class="filter-bar">
        <div class="filter-pills" role="group" aria-label="Question Filter">
          <button type="button" class="filter-chip active" data-qfilter="all">All</button>
          <button type="button" class="filter-chip" data-qfilter="verified">Verified</button>
          <button type="button" class="filter-chip" data-qfilter="pending">Sliced / Pending</button>
          <button type="button" class="filter-chip" data-qfilter="replica">Replicas</button>
          <button type="button" class="filter-chip" data-qfilter="comments">Open Comments</button>
        </div>
        <input type="search" id="q-search-input" class="q-search" placeholder="Filter by ID, paper, or stem text...">
      </div>
      <table>
        <thead>
          <tr>
            <th>Universal ID</th>
            <th>Archived Source Crop (Local Private)</th>
            <th>Compiled Clean-Room Stem &amp; L9&rarr;L1 Metadata</th>
          </tr>
        </thead>
        <tbody>
"##,
        );
        for row in &report.side_by_side_items {
            let orig_source_line = match (&row.source_file, &row.archived_source_path) {
                (Some(sf), Some(arch_rel)) => {
                    let arch_href = resolve_rel_href(mode, arch_rel);
                    let sha_hint = row
                        .source_sha256
                        .as_deref()
                        .map(|s| format!(" · sha256:{}", &s[..s.len().min(12)]))
                        .unwrap_or_default();
                    format!(
                        r##"<div class="muted" style="margin-bottom:4px;">Original source: <a href="{}"><code>{}</code></a>{}</div>"##,
                        escape_html(&arch_href),
                        escape_html(sf),
                        escape_html(&sha_hint)
                    )
                }
                (Some(sf), None) => format!(
                    r##"<div class="muted" style="margin-bottom:4px;">Original source: <code>{}</code></div>"##,
                    escape_html(sf)
                ),
                _ => String::new(),
            };

            let crop_cell = match (&row.crop_data_uri, &row.crop_path) {
                (Some(uri), Some(rel)) => {
                    let href = resolve_rel_href(mode, rel);
                    let meta = match (row.crop_page, row.crop_dimensions) {
                        (Some(pg), Some((w, h))) => format!("p.{pg} · {w}×{h} px · "),
                        _ => String::new(),
                    };
                    format!(
                        r##"{}<img class="crop-preview" data-lightbox-caption="{}" src="{}" alt="Crop"><div class="muted" style="margin-top:4px;">{}<a href="{}"><code>{}</code></a></div>"##,
                        orig_source_line,
                        escape_html(&format!("{} ({rel})", row.universal_id)),
                        escape_html(uri),
                        escape_html(&meta),
                        escape_html(&href),
                        escape_html(rel)
                    )
                }
                (None, Some(rel)) => {
                    let href = resolve_rel_href(mode, rel);
                    match mode {
                        StatusRenderMode::Server { .. } => {
                            let meta = match (row.crop_page, row.crop_dimensions) {
                                (Some(pg), Some((w, h))) => format!("p.{pg} · {w}×{h} px · "),
                                _ => String::new(),
                            };
                            format!(
                                r##"{}<img class="crop-preview" loading="lazy" data-lightbox-caption="{}" src="{}" alt="Crop"><div class="muted" style="margin-top:4px;">{}<a href="{}"><code>{}</code></a></div>"##,
                                orig_source_line,
                                escape_html(&format!("{} ({rel})", row.universal_id)),
                                escape_html(&href),
                                escape_html(&meta),
                                escape_html(&href),
                                escape_html(rel)
                            )
                        }
                        StatusRenderMode::StandaloneFile => format!(
                            r##"{}<a href="{}"><code>{}</code></a>"##,
                            orig_source_line,
                            escape_html(&href),
                            escape_html(rel)
                        ),
                    }
                }
                _ if !orig_source_line.is_empty() => orig_source_line,
                _ => r##"<span class="muted">No crop recorded (standalone / AI replica)</span>"##
                    .to_string(),
            };

            let sol_json_line = row
                .solution_json_path
                .as_deref()
                .map(|p| {
                    let href = resolve_rel_href(mode, p);
                    format!(
                        r##"<div class="muted" style="margin-top:6px;">Solution JSON: <a href="{}"><code>{}</code></a></div>"##,
                        escape_html(&href),
                        escape_html(p)
                    )
                })
                .unwrap_or_default();

            let compiled_cell = if let Some(ref title) = row.compiled_title {
                let lbl = row.compiled_label.as_deref().unwrap_or("Q");
                let stem = row.compiled_stem.as_deref().unwrap_or("");
                let (min_l, max_l) = row.sublevel_span.unwrap_or((9, 9));
                let marks = row.marks.unwrap_or(1.0);
                let qa = row.qa_status.as_deref().unwrap_or("draft");
                let rep_badge = if row.is_replica {
                    match row.replica_scope {
                        Some(ReplicaScope::WholePaper) => format!(
                            r##" <span class="badge ai">Whole-Paper Replica &larr; {}</span>"##,
                            escape_html(
                                row.parent_question_id
                                    .as_deref()
                                    .or(row.parent_paper_id.as_deref())
                                    .unwrap_or("original")
                            )
                        ),
                        _ => format!(
                            r##" <span class="badge ai">Single-Q Replica &larr; {}</span>"##,
                            escape_html(row.parent_question_id.as_deref().unwrap_or("original"))
                        ),
                    }
                } else {
                    String::new()
                };
                let children_line = if !row.replica_ids.is_empty() {
                    format!(
                        r##"<div class="muted" style="margin-top:4px;">Replicas created: <code>{}</code></div>"##,
                        escape_html(&row.replica_ids.join(", "))
                    )
                } else {
                    String::new()
                };
                format!(
                    r##"<div><strong>{} · {}</strong> <span class="badge ok">{}</span> <span class="badge">L{}&rarr;L{} · {} marks</span>{}</div><div class="stem-box" style="margin-top:6px;">{}</div>{}{}"##,
                    escape_html(lbl),
                    escape_html(title),
                    escape_html(qa),
                    max_l,
                    min_l,
                    marks,
                    rep_badge,
                    escape_html(stem),
                    children_line,
                    sol_json_line
                )
            } else {
                format!(
                    r##"<span class="badge warn">Sliced — Pending Authoring</span>{}"##,
                    sol_json_line
                )
            };

            let qstate = if row
                .qa_status
                .as_deref()
                .is_some_and(|s| s == "verified" || s == "approved")
            {
                "verified"
            } else {
                "pending"
            };
            let search_haystack = format!(
                "{} {} {} {} {}",
                row.universal_id,
                row.paper_id.as_deref().unwrap_or(""),
                row.source_file.as_deref().unwrap_or(""),
                row.compiled_title.as_deref().unwrap_or(""),
                row.compiled_stem.as_deref().unwrap_or("")
            )
            .to_ascii_lowercase();

            let _ = write!(
                html,
                r##"          <tr class="q-row" data-qstate="{qstate}" data-replica="{}" data-comments="{}" data-search="{}">
            <td><code>{}</code><div class="muted">{}</div></td>
            <td>{}</td>
            <td>{}</td>
          </tr>
"##,
                row.is_replica,
                row.open_comment_count > 0,
                escape_html(&search_haystack),
                escape_html(&row.universal_id),
                escape_html(row.paper_id.as_deref().unwrap_or(&row.subject)),
                crop_cell,
                compiled_cell
            );
        }
        html.push_str("        </tbody>\n      </table>\n");
    }
    html.push_str("    </section>\n");

    // 7. Open Snap Board Comments & Authoring Tasks
    html.push_str("    <section data-section=\"comments\">\n      <h2>7. Open <code>Snap Board</code> Comments &amp; Authoring Work Queue</h2>\n");
    if report.open_comments.is_empty() && report.ledger.tasks.is_empty() {
        html.push_str(
            "      <p class=\"muted\">Zero open comments or queued authoring tasks.</p>\n",
        );
    } else {
        if !report.open_comments.is_empty() {
            html.push_str("      <div class=\"comment-grid\" style=\"margin-bottom: 16px;\">\n");
            for oc in &report.open_comments {
                let c = &oc.comment;
                let sub_loc = format_comment_subtarget(c);
                let _ = write!(
                    html,
                    r##"        <div class="comment-card">
          <div><span class="badge danger">{}</span> <code>{}{}</code> <span class="muted">({})</span></div>
          <div style="margin-top:6px; font-weight:600;">{}</div>
"##,
                    escape_html(&c.comment_kind),
                    escape_html(&c.target_id),
                    escape_html(&sub_loc),
                    escape_html(&c.comment_id),
                    escape_html(&c.comment_text)
                );
                for att in &c.attachments {
                    if let Some(uri) = oc.attachment_data_uris.get(&att.attachment_id) {
                        let _ = writeln!(
                            html,
                            r##"          <img class="snap-thumb" data-lightbox-caption="{}" src="{}" alt="Snap Board Attachment">"##,
                            escape_html(&format!("{} ({})", c.comment_id, c.target_id)),
                            escape_html(uri)
                        );
                    }
                }
                html.push_str("        </div>\n");
            }
            html.push_str("      </div>\n");
        }

        if !report.ledger.tasks.is_empty() {
            html.push_str(
                r##"      <table>
        <thead>
          <tr>
            <th>Task ID</th>
            <th>Status</th>
            <th>Kind</th>
            <th>Subject</th>
            <th>Target</th>
            <th>Difficulty / Note</th>
          </tr>
        </thead>
        <tbody>
"##,
            );
            for t in &report.ledger.tasks {
                let detail = match (&t.difficulty_hint, &t.note) {
                    (Some(d), Some(n)) => format!("({d}) {n}"),
                    (Some(d), None) => d.clone(),
                    (None, Some(n)) => n.clone(),
                    (None, None) => "-".to_string(),
                };
                let _ = write!(
                    html,
                    r##"          <tr>
            <td><code>{}</code></td>
            <td><span class="badge">{}</span></td>
            <td><code>{}</code></td>
            <td>{}</td>
            <td><code>{}:{}</code></td>
            <td>{}</td>
          </tr>
"##,
                    escape_html(&t.task_id),
                    escape_html(t.status.as_str()),
                    escape_html(t.kind.as_str()),
                    escape_html(&t.subject),
                    escape_html(&t.target_kind),
                    escape_html(&t.target_id),
                    escape_html(&detail)
                );
            }
            html.push_str("        </tbody>\n      </table>\n");
        }
    }
    html.push_str(
        r##"    </section>
  </main>
  <div id="img-lightbox" role="dialog" aria-modal="true" aria-label="Image Preview">
    <div class="lightbox-card">
      <img id="lightbox-img" src="" alt="Zoomed Preview">
      <div class="lightbox-bar">
        <span id="lightbox-caption"></span>
        <button type="button" id="lightbox-close" class="filter-chip">Close (Esc)</button>
      </div>
    </div>
  </div>
  <script>
    (function () {
      const tabBtns = document.querySelectorAll("button.tab-chip[data-section-target]");
      const sections = document.querySelectorAll("main > section[data-section]");
      tabBtns.forEach((btn) => {
        btn.addEventListener("click", () => {
          const target = btn.getAttribute("data-section-target");
          tabBtns.forEach((b) => b.classList.toggle("active", b === btn));
          sections.forEach((sec) => {
            const match = target === "all" || sec.getAttribute("data-section") === target;
            sec.classList.toggle("hidden-sec", !match);
          });
        });
      });

      document.querySelectorAll(".subject-card[data-jump-section]").forEach((card) => {
        function jumpFromCard() {
          const dest = card.getAttribute("data-jump-section") || "questions";
          const targetTab = document.querySelector(`button.tab-chip[data-section-target="${dest}"]`);
          if (targetTab) targetTab.click();
        }
        card.addEventListener("click", jumpFromCard);
        card.addEventListener("keydown", (e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            jumpFromCard();
          }
        });
      });

      let activeQFilter = "all";
      const filterBtns = document.querySelectorAll("button.filter-chip[data-qfilter]");
      const searchInput = document.getElementById("q-search-input");
      const qRows = document.querySelectorAll("tr.q-row");

      function applyQuestionFilter() {
        const q = searchInput ? searchInput.value.trim().toLowerCase() : "";
        qRows.forEach((row) => {
          let ok = true;
          if (activeQFilter === "verified") ok = row.getAttribute("data-qstate") === "verified";
          else if (activeQFilter === "pending") ok = row.getAttribute("data-qstate") === "pending";
          else if (activeQFilter === "replica") ok = row.getAttribute("data-replica") === "true";
          else if (activeQFilter === "comments") ok = row.getAttribute("data-comments") === "true";
          if (ok && q) {
            const hay = row.getAttribute("data-search") || "";
            ok = hay.indexOf(q) !== -1;
          }
          row.classList.toggle("hidden-row", !ok);
        });
      }

      filterBtns.forEach((btn) => {
        btn.addEventListener("click", () => {
          activeQFilter = btn.getAttribute("data-qfilter") || "all";
          filterBtns.forEach((b) => b.classList.toggle("active", b === btn));
          applyQuestionFilter();
        });
      });
      if (searchInput) {
        searchInput.addEventListener("input", applyQuestionFilter);
      }

      const lb = document.getElementById("img-lightbox");
      const lbImg = document.getElementById("lightbox-img");
      const lbCap = document.getElementById("lightbox-caption");
      const lbClose = document.getElementById("lightbox-close");
      function closeLightbox() {
        if (lb) lb.classList.remove("open");
      }
      document.querySelectorAll("img.crop-preview, img.snap-thumb").forEach((img) => {
        img.addEventListener("click", () => {
          if (!lb || !lbImg) return;
          lbImg.src = img.src;
          if (lbCap) lbCap.textContent = img.getAttribute("data-lightbox-caption") || img.alt || "";
          lb.classList.add("open");
        });
      });
      if (lb) {
        lb.addEventListener("click", (e) => { if (e.target === lb) closeLightbox(); });
      }
      if (lbClose) lbClose.addEventListener("click", closeLightbox);
      document.addEventListener("keydown", (e) => { if (e.key === "Escape") closeLightbox(); });
    })();
  </script>
</body>
</html>
"##,
    );
    html
}

/// Atomically write `products/<product>/<subject>/status.html` and return its path.
pub fn write_status_html(cfg: &HydrogenConfig, report: &ProductStatusReport) -> Result<PathBuf> {
    let out_path = cfg.status_html_path(&report.product, &report.subject);
    if let Some(parent) = out_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = out_path.with_extension("html.tmp");
    let html = render_status_html(report);
    std::fs::write(&tmp_path, html)?;
    std::fs::rename(&tmp_path, &out_path)?;
    Ok(out_path)
}

/// Refresh `products/<product>/<subject>/ledger.json` (and regenerate `status.html` if `force_html`
/// is true or `status.html` already exists on disk).
pub async fn refresh_subject_ledger_and_optional_html(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    force_html: bool,
) -> Result<Option<PathBuf>> {
    let should_emit_html = force_html || cfg.status_html_path(product, subject).is_file();
    let report = build_subject_status_report(cfg, product, subject, should_emit_html).await?;
    if should_emit_html {
        let path = write_status_html(cfg, &report)?;
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn format_comment_subtarget(c: &CourseComment) -> String {
    let mut s = String::new();
    if let Some(ref p) = c.part_id {
        let _ = write!(s, ":{p}");
    }
    if let Some(ref st) = c.step_id {
        let _ = write!(s, ":{st}");
    }
    if let Some(lv) = c.sublevel {
        let _ = write!(s, ":L{lv:02}");
    }
    s
}

fn human_subject_title(slug: &str) -> String {
    match slug {
        "maths" => "Maths".to_string(),
        "english" => "English".to_string(),
        "verbal-reasoning" => "Verbal Reasoning".to_string(),
        "non-verbal-reasoning" => "Non-Verbal Reasoning".to_string(),
        "physics" => "Physics".to_string(),
        other => other.replace('-', " "),
    }
}

fn stage_badge_html(stage: SourceStage) -> &'static str {
    match stage {
        SourceStage::Queued => r##"<span class="badge warn">queued</span>"##,
        SourceStage::Sliced => r##"<span class="badge progress">sliced</span>"##,
        SourceStage::InProgress => r##"<span class="badge progress">in_progress</span>"##,
        SourceStage::Completed => r##"<span class="badge ok">completed</span>"##,
    }
}

/// Minimal HTML entity escaper preventing broken markup or script injection from math/stems.
pub fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
