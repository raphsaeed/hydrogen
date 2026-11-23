//! Unified Git-ignored sub-product authoring, crop geometry & replica lineage ledger
//! (`products/<product>/<subject>/ledger.json`).
//!
//! **Zero-Drift & Single-Local-JSON Law (`docs/system-architecture.md` §5.5 & `docs/sop.md` §1.2):**
//! - Subject `.sqlite` / `.raphs` vaults remain the single source of truth for compiled papers,
//!   questions, `L9 -> L1` solutions, topics, concept cards, media blobs, revisions, and comments.
//! - `products/<product>/<subject>/ledger.json` is the **single local JSON store** per sub-product
//!   (superseding `private-xref.json`), tracking everything Clean-Room `.raphs` vaults intentionally exclude:
//!   1. SHA-256 deduplicated `inbox/` & `archive/` raw file inventory (flagging renamed duplicates
//!      automatically) and lifecycle stage (`queued` -> `sliced` -> `in_progress` -> `completed`),
//!   2. Per-paper raster page geometry (`dpi`, `pageCount`, `pages[]`), expected question counts,
//!      sliced vs. verified item checklists, missing `Q` numbers, QA notes, and whole-paper replica lineage,
//!   3. Per-question original source/page/`bbox`/`bboxPx`/crop provenance, solution JSON path, and
//!      two-way replica lineage (`single_question` vs. `whole_paper` replicas + original parent references),
//!   4. Pending authoring task queue (e.g. `create-replica` requests from Carbon `--watch`, planned topic cards, notes).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use memory_core_rs::CourseCatalogSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::HydrogenConfig;
use crate::domain::{ContainerSegment, ItemSegment, UniversalId};
use crate::error::{HydrogenError, Result};

/// Canonical unified `ledger.json` schema version.
pub const LEDGER_SCHEMA_VERSION: &str = "2.0";

/// Integer pixel bounding box on a rendered page image (`0,0` = top-left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelBBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Page-normalized `[0.0..=1.0]` bounding box (`0,0` = top-left).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedBBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl NormalizedBBox {
    /// Convert normalized `[0.0..=1.0]` coordinates into clamped integer pixel coordinates on `(page_w, page_h)`.
    pub fn to_pixel_bbox(self, page_w: u32, page_h: u32) -> PixelBBox {
        let pw = page_w.max(1) as f32;
        let ph = page_h.max(1) as f32;
        let x0 = (self.x.clamp(0.0, 1.0) * pw).floor() as u32;
        let y0 = (self.y.clamp(0.0, 1.0) * ph).floor() as u32;
        let x1 = ((self.x + self.w).clamp(0.0, 1.0) * pw).ceil() as u32;
        let y1 = ((self.y + self.h).clamp(0.0, 1.0) * ph).ceil() as u32;

        let x = x0.min(page_w.saturating_sub(1));
        let y = y0.min(page_h.saturating_sub(1));
        let w = x1.saturating_sub(x).max(1).min(page_w.saturating_sub(x));
        let h = y1.saturating_sub(y).max(1).min(page_h.saturating_sub(y));
        PixelBBox { x, y, w, h }
    }

    /// Compute normalized `[0.0..=1.0]` coordinates from a pixel bounding box on `(page_w, page_h)`.
    pub fn from_pixel_bbox(px: PixelBBox, page_w: u32, page_h: u32) -> Self {
        let pw = page_w.max(1) as f32;
        let ph = page_h.max(1) as f32;
        Self {
            x: (px.x as f32 / pw).clamp(0.0, 1.0),
            y: (px.y as f32 / ph).clamp(0.0, 1.0),
            w: (px.w as f32 / pw).clamp(0.0, 1.0),
            h: (px.h as f32 / ph).clamp(0.0, 1.0),
        }
    }
}

/// Metadata for a single high-DPI rendered page PNG inside `archive/<paper_id>/pages/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPageRecord {
    /// 1-based page index.
    pub page: u32,
    /// Path relative to `products/<product>/<subject>/` (`archive/<paper_id>/pages/page-001.png`).
    pub image_path: String,
    pub width_px: u32,
    pub height_px: u32,
}

/// Cropped question/diagram summary produced by `slice_paper` and persisted into `ledger.items`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlicedCropRecord {
    pub universal_id: String,
    pub paper_id: String,
    pub subject: String,
    pub source_file: String,
    /// 1-based page index in `source_file`.
    pub page: u32,
    /// Normalized `[0.0..=1.0]` bounding box on `page`.
    pub bbox: NormalizedBBox,
    /// Exact pixel bounding box on the rendered high-DPI page PNG.
    pub bbox_px: PixelBBox,
    /// Path relative to `products/<product>/<subject>/` (`archive/<paper_id>/crops/<id>.png`).
    pub crop_path: String,
    pub width_px: u32,
    pub height_px: u32,
}

/// Paper-level raster metadata recorded by `slice_paper` into `ledger.papers`.
#[derive(Debug, Clone, PartialEq)]
pub struct SlicedPaperRecord {
    pub paper_id: String,
    pub subject: String,
    pub source_file: String,
    pub archived_source_path: String,
    pub page_count: u32,
    pub dpi: f32,
    pub pages: Vec<RenderedPageRecord>,
}

/// Lifecycle stage of a raw source file or paper in the authoring pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStage {
    #[default]
    Queued,
    Sliced,
    InProgress,
    Completed,
}

impl SourceStage {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Sliced => "sliced",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

impl fmt::Display for SourceStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Distinguishes a standalone single-question replica (`Q015-R01`) from a member question of a whole-paper replica (`P001_R01-Q015`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicaScope {
    /// Replica of one question only (e.g. `GYR-MTH-P001-Q015-R01` -> `GYR-MTH-P001-Q015`).
    SingleQuestion,
    /// Question belonging to a whole-paper replica (e.g. `GYR-MTH-P001_R01-Q015` -> paper `GYR-MTH-P001_R01` / question `GYR-MTH-P001-Q015`).
    WholePaper,
}

impl ReplicaScope {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SingleQuestion => "single_question",
            Self::WholePaper => "whole_paper",
        }
    }
}

impl fmt::Display for ReplicaScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// SHA-256 deduplicated raw input file tracked across `products/<product>/<subject>/{inbox,archive}/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFileEntry {
    pub sha256: String,
    pub subject: String,
    pub primary_filename: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub inbox_paths: Vec<String>,
    #[serde(default)]
    pub archive_paths: Vec<String>,
    /// Extra or renamed files in `inbox/` sharing this exact SHA-256 digest.
    #[serde(default)]
    pub duplicate_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper_id: Option<String>,
    #[serde(default)]
    pub stage: SourceStage,
    pub first_seen_at_ms: u64,
    pub updated_at_ms: u64,
}

impl SourceFileEntry {
    /// True when one or more renamed/extra copies exist in `inbox/` with identical SHA-256 bytes.
    pub fn has_renamed_duplicates(&self) -> bool {
        !self.duplicate_paths.is_empty()
    }
}

/// Unified per-paper entry in `ledger.json`: source PDF/PNG reference, rasterized pages,
/// expected/sliced/verified counts, missing slots, whole-paper replica lineage, and QA notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperLedgerEntry {
    pub paper_id: String,
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpi: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<RenderedPageRecord>,
    #[serde(default)]
    pub is_replica: bool,
    /// Original paper ID when `is_replica == true` (e.g. `"GYR-MTH-P001"` for `"GYR-MTH-P001_R01"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_paper_id: Option<String>,
    /// Whole-paper replicas created from this original paper (e.g. `["GYR-MTH-P001_R01"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replica_paper_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_questions: Option<u32>,
    #[serde(default)]
    pub sliced_count: u32,
    #[serde(default)]
    pub verified_count: u32,
    /// Single-question replica count attached to this paper (or total replica items on a replica paper).
    #[serde(default)]
    pub replica_count: u32,
    #[serde(default)]
    pub sliced_question_numbers: Vec<u32>,
    #[serde(default)]
    pub verified_question_numbers: Vec<u32>,
    #[serde(default)]
    pub missing_question_numbers: Vec<u32>,
    #[serde(default)]
    pub stage: SourceStage,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub updated_at_ms: u64,
}

impl PaperLedgerEntry {
    fn blank(paper_id: String, subject: String, ts: u64) -> Self {
        let parent_pid = derive_parent_paper_id(&paper_id);
        Self {
            is_replica: parent_pid.is_some(),
            parent_paper_id: parent_pid,
            paper_id,
            subject,
            source_file: None,
            archived_source_path: None,
            source_sha256: None,
            page_count: None,
            dpi: None,
            pages: Vec::new(),
            replica_paper_ids: Vec::new(),
            expected_questions: None,
            sliced_count: 0,
            verified_count: 0,
            replica_count: 0,
            sliced_question_numbers: Vec::new(),
            verified_question_numbers: Vec::new(),
            missing_question_numbers: Vec::new(),
            stage: SourceStage::Queued,
            notes: Vec::new(),
            updated_at_ms: ts,
        }
    }
}

/// Unified per-question / per-solution provenance & replica lineage record in `ledger.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemLedgerEntry {
    pub universal_id: String,
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper_id: Option<String>,
    /// Original PDF, paper, or PNG filename processed (or inherited from parent for replicas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    /// Path relative to `products/<product>/<subject>/` to the archived original PDF/PNG.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_source_path: Option<String>,
    /// 1-based page number in `source_file`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Normalized `[0.0..=1.0]` crop bounding box on `page` (present on directly sliced items).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<NormalizedBBox>,
    /// Exact pixel crop bounding box on the rendered high-DPI page PNG (present on directly sliced items).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox_px: Option<PixelBBox>,
    /// Path relative to `products/<product>/<subject>/` to the sliced question crop PNG (or inherited parent crop for replicas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop_dimensions: Option<(u32, u32)>,
    /// Path relative to `products/<product>/<subject>/` to the authored `<QID>.json` package if present in `archive/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solution_json_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qa_status: Option<String>,
    #[serde(default)]
    pub is_replica: bool,
    /// Whether this replica is a single-question replica (`single_question`) or part of a replica paper (`whole_paper`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replica_scope: Option<ReplicaScope>,
    /// Original question ID this replica was created from (e.g. `"GYR-MTH-P001-Q015"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_question_id: Option<String>,
    /// Original paper ID when modeled on a paper or paper question (e.g. `"GYR-MTH-P001"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_paper_id: Option<String>,
    /// Direct question replicas created from this original question (e.g. `["GYR-MTH-P001-Q015-R01"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replica_ids: Vec<String>,
    pub updated_at_ms: u64,
}

/// Category of queued authoring work in `ledger.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringTaskKind {
    CreateReplica,
    TopicCard,
    Note,
}

impl AuthoringTaskKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CreateReplica => "create_replica",
            Self::TopicCard => "topic_card",
            Self::Note => "note",
        }
    }
}

impl fmt::Display for AuthoringTaskKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Execution state of a queued authoring task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringTaskStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
}

impl AuthoringTaskStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

impl fmt::Display for AuthoringTaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Queued authoring backlog item (e.g. Carbon `create-replica` request or planned prerequisite card).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringTask {
    pub task_id: String,
    pub kind: AuthoringTaskKind,
    #[serde(default)]
    pub status: AuthoringTaskStatus,
    pub subject: String,
    pub target_kind: String,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// Unified sub-product `ledger.json` store persisted atomically at `products/<product>/<subject>/ledger.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductLedger {
    pub schema_version: String,
    pub product: String,
    #[serde(default)]
    pub subject: String,
    pub updated_at_ms: u64,
    /// Raw source files keyed by lowercase 64-char SHA-256 hex digest.
    #[serde(default)]
    pub sources: BTreeMap<String, SourceFileEntry>,
    /// Paper raster geometry, progress & replica lineage keyed by `paper_id` (`GYR-MTH-P001`, `GYR-MTH-P001_R01`).
    #[serde(default)]
    pub papers: BTreeMap<String, PaperLedgerEntry>,
    /// Per-question crop geometry, solution provenance & replica lineage keyed by `universal_id`.
    #[serde(default)]
    pub items: BTreeMap<String, ItemLedgerEntry>,
    /// Ordered authoring work queue.
    #[serde(default)]
    pub tasks: Vec<AuthoringTask>,
}

impl ProductLedger {
    /// Create a fresh empty ledger for `(product, subject)`.
    pub fn new(product: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            schema_version: LEDGER_SCHEMA_VERSION.to_string(),
            product: product.into(),
            subject: subject.into(),
            updated_at_ms: now_ms(),
            sources: BTreeMap::new(),
            papers: BTreeMap::new(),
            items: BTreeMap::new(),
            tasks: Vec::new(),
        }
    }

    /// Load an existing `ledger.json` from `path`, or create a fresh store if absent.
    pub fn load_or_new(path: &Path, product: &str, subject: &str) -> Result<Self> {
        if path.is_file() {
            let raw = std::fs::read_to_string(path)?;
            let mut parsed: Self = serde_json::from_str(&raw)?;
            parsed.schema_version = LEDGER_SCHEMA_VERSION.to_string();
            if parsed.subject.is_empty() {
                parsed.subject = subject.to_string();
            }
            Ok(parsed)
        } else {
            Ok(Self::new(product, subject))
        }
    }

    /// Atomically persist `ledger.json` via a sibling `.json.tmp` file rename.
    pub fn save_atomic(&mut self, path: &Path) -> Result<()> {
        self.updated_at_ms = now_ms();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let tmp_path = path.with_extension("json.tmp");
        let pretty = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp_path, pretty)?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Record or merge rendered PDF/PNG page geometry for `paper_id` (called by `slice_paper`).
    pub fn upsert_sliced_paper(&mut self, rec: SlicedPaperRecord) {
        let ts = now_ms();
        let entry = self.papers.entry(rec.paper_id.clone()).or_insert_with(|| {
            PaperLedgerEntry::blank(rec.paper_id.clone(), rec.subject.clone(), ts)
        });
        entry.subject = rec.subject;
        entry.source_file = Some(rec.source_file);
        entry.archived_source_path = Some(rec.archived_source_path);
        entry.page_count = Some(rec.page_count);
        entry.dpi = Some(rec.dpi);
        entry.pages = rec.pages;
        entry.updated_at_ms = ts;
        self.updated_at_ms = ts;
    }

    /// Record or update a sliced question/diagram crop (`bbox`, `bboxPx`, `cropPath`) in `self.items`.
    pub fn upsert_sliced_item(&mut self, crop: &SlicedCropRecord) {
        let ts = now_ms();
        let p_meta = self.papers.get(&crop.paper_id);
        let src_sha = p_meta.and_then(|p| p.source_sha256.clone());
        let arch_src = p_meta.and_then(|p| p.archived_source_path.clone());

        let item = self
            .items
            .entry(crop.universal_id.clone())
            .or_insert_with(|| ItemLedgerEntry {
                universal_id: crop.universal_id.clone(),
                subject: crop.subject.clone(),
                paper_id: Some(crop.paper_id.clone()),
                source_file: Some(crop.source_file.clone()),
                source_sha256: src_sha.clone(),
                archived_source_path: arch_src.clone(),
                page: Some(crop.page),
                bbox: Some(crop.bbox),
                bbox_px: Some(crop.bbox_px),
                crop_path: Some(crop.crop_path.clone()),
                crop_dimensions: Some((crop.width_px, crop.height_px)),
                solution_json_path: None,
                qa_status: None,
                is_replica: false,
                replica_scope: None,
                parent_question_id: None,
                parent_paper_id: None,
                replica_ids: Vec::new(),
                updated_at_ms: ts,
            });
        item.subject = crop.subject.clone();
        item.paper_id = Some(crop.paper_id.clone());
        item.source_file = Some(crop.source_file.clone());
        if src_sha.is_some() {
            item.source_sha256 = src_sha;
        }
        if arch_src.is_some() {
            item.archived_source_path = arch_src;
        }
        item.page = Some(crop.page);
        item.bbox = Some(crop.bbox);
        item.bbox_px = Some(crop.bbox_px);
        item.crop_path = Some(crop.crop_path.clone());
        item.crop_dimensions = Some((crop.width_px, crop.height_px));
        item.updated_at_ms = ts;
        self.updated_at_ms = ts;
    }

    /// Configure the expected total question count for `paper_id` (e.g. `GYR-MTH-P001` -> `50`).
    pub fn set_expected_questions(&mut self, paper_id: &str, expected: u32) -> Result<()> {
        let parsed = UniversalId::parse(paper_id)?;
        if !matches!(parsed.container, Some(ContainerSegment::Paper { .. }))
            || parsed.item.is_some()
        {
            return Err(HydrogenError::Validation(format!(
                "expected a Paper UniversalId (e.g. 'GYR-MTH-P001'), got '{paper_id}'"
            )));
        }
        let subject = subject_slug_for_prefix(&parsed.subject_code).to_string();
        if !self.subject.is_empty() && self.subject != subject {
            return Err(HydrogenError::Validation(format!(
                "paper '{paper_id}' belongs to subject '{subject}', not ledger subject '{}'",
                self.subject
            )));
        }
        let ts = now_ms();
        let entry = self
            .papers
            .entry(paper_id.to_string())
            .or_insert_with(|| PaperLedgerEntry::blank(paper_id.to_string(), subject, ts));
        entry.expected_questions = Some(expected);
        entry.updated_at_ms = ts;
        let has_pages = entry.page_count.is_some() || !entry.pages.is_empty();
        recompute_paper_metrics(entry, has_pages);
        self.propagate_source_stages();
        Ok(())
    }

    /// Append a QA / authoring note to `paper_id`.
    pub fn add_paper_note(&mut self, paper_id: &str, note: impl Into<String>) -> Result<()> {
        let note_str = note.into();
        let trimmed = note_str.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let parsed = UniversalId::parse(paper_id)?;
        if !matches!(parsed.container, Some(ContainerSegment::Paper { .. }))
            || parsed.item.is_some()
        {
            return Err(HydrogenError::Validation(format!(
                "expected a Paper UniversalId, got '{paper_id}'"
            )));
        }
        let subject = subject_slug_for_prefix(&parsed.subject_code).to_string();
        if !self.subject.is_empty() && self.subject != subject {
            return Err(HydrogenError::Validation(format!(
                "paper '{paper_id}' belongs to subject '{subject}', not ledger subject '{}'",
                self.subject
            )));
        }
        let ts = now_ms();
        let entry = self
            .papers
            .entry(paper_id.to_string())
            .or_insert_with(|| PaperLedgerEntry::blank(paper_id.to_string(), subject, ts));
        if !entry.notes.iter().any(|n| n == trimmed) {
            entry.notes.push(trimmed.to_string());
            entry.updated_at_ms = ts;
        }
        Ok(())
    }

    /// Enqueue a `create_replica` task (deduplicating if an identical pending request already exists).
    pub fn enqueue_replica_task(
        &mut self,
        subject: &str,
        target_kind: &str,
        target_id: &str,
        difficulty_hint: Option<String>,
        note: Option<String>,
    ) -> AuthoringTask {
        if let Some(existing) = self.tasks.iter().find(|t| {
            t.kind == AuthoringTaskKind::CreateReplica
                && t.status == AuthoringTaskStatus::Pending
                && t.target_kind == target_kind
                && t.target_id == target_id
                && t.difficulty_hint == difficulty_hint
        }) {
            return existing.clone();
        }
        let next_no = self.tasks.len() + 1;
        let ts = now_ms();
        let task = AuthoringTask {
            task_id: format!("task-{next_no:04}"),
            kind: AuthoringTaskKind::CreateReplica,
            status: AuthoringTaskStatus::Pending,
            subject: subject.to_string(),
            target_kind: target_kind.to_string(),
            target_id: target_id.to_string(),
            difficulty_hint,
            note,
            created_at_ms: ts,
            updated_at_ms: ts,
        };
        self.tasks.push(task.clone());
        self.updated_at_ms = ts;
        task
    }

    /// Enqueue a general authoring backlog task (`topic_card` or `note`).
    pub fn enqueue_task(
        &mut self,
        kind: AuthoringTaskKind,
        subject: &str,
        target_kind: &str,
        target_id: &str,
        note: Option<String>,
    ) -> AuthoringTask {
        let next_no = self.tasks.len() + 1;
        let ts = now_ms();
        let task = AuthoringTask {
            task_id: format!("task-{next_no:04}"),
            kind,
            status: AuthoringTaskStatus::Pending,
            subject: subject.to_string(),
            target_kind: target_kind.to_string(),
            target_id: target_id.to_string(),
            difficulty_hint: None,
            note,
            created_at_ms: ts,
            updated_at_ms: ts,
        };
        self.tasks.push(task.clone());
        self.updated_at_ms = ts;
        task
    }

    /// Count of open (`pending` or `in_progress`) authoring tasks.
    pub fn pending_task_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status != AuthoringTaskStatus::Completed)
            .count()
    }

    /// Synchronize this unified sub-product ledger with live `inbox/` + `archive/` SHA-256 scans
    /// and an optional open `<stem>.sqlite` `CourseCatalogSnapshot`.
    ///
    /// When `vault_snapshot` is `None` (e.g. during synchronous `slice-paper`), previously
    /// recorded `verified_count` / `verified_question_numbers` are preserved without opening SQLCipher.
    pub fn sync_with_workspace(
        &mut self,
        cfg: &HydrogenConfig,
        vault_snapshot: Option<&CourseCatalogSnapshot>,
    ) -> Result<()> {
        let subject = self.subject.clone();
        let subject_dir = cfg.subject_dir(&self.product, &subject);
        let ts = now_ms();

        // 1. Scan `products/<product>/<subject>/inbox/**` and `archive/*/source/*` by SHA-256
        #[derive(Default)]
        struct ScannedGroup {
            size_bytes: u64,
            inbox_rel_paths: BTreeSet<String>,
            archive_rel_paths: BTreeSet<String>,
        }

        let mut scanned_by_hash: BTreeMap<String, ScannedGroup> = BTreeMap::new();

        let inbox_dir = cfg.subject_inbox_dir(&self.product, &subject);
        if inbox_dir.is_dir() {
            let mut raw_files = Vec::new();
            collect_raw_source_files(&inbox_dir, &mut raw_files)?;
            for file_path in raw_files {
                let (hash, size) = sha256_file(&file_path)?;
                let rel = relative_path_str(&subject_dir, &file_path);
                let grp = scanned_by_hash.entry(hash).or_default();
                grp.size_bytes = size;
                grp.inbox_rel_paths.insert(rel);
            }
        }

        let archive_dir = cfg.subject_archive_dir(&self.product, &subject);
        if archive_dir.is_dir() {
            for paper_entry in read_sorted_dir(&archive_dir)? {
                let source_dir = paper_entry.path().join("source");
                if !source_dir.is_dir() {
                    continue;
                }
                let mut raw_files = Vec::new();
                collect_raw_source_files(&source_dir, &mut raw_files)?;
                for file_path in raw_files {
                    let (hash, size) = sha256_file(&file_path)?;
                    let rel = relative_path_str(&subject_dir, &file_path);
                    let grp = scanned_by_hash.entry(hash).or_default();
                    grp.size_bytes = size;
                    grp.archive_rel_paths.insert(rel);
                }
            }
        }

        self.sources.retain(|hash, existing| {
            scanned_by_hash.contains_key(hash) || existing.paper_id.is_some()
        });
        for existing in self.sources.values_mut() {
            existing.inbox_paths.clear();
            existing.archive_paths.clear();
            existing.duplicate_paths.clear();
        }

        for (hash, grp) in scanned_by_hash {
            let mut inbox_vec: Vec<String> = grp.inbox_rel_paths.into_iter().collect();
            inbox_vec.sort_by(|a, b| {
                basename_of(a)
                    .len()
                    .cmp(&basename_of(b).len())
                    .then_with(|| a.cmp(b))
            });
            let archive_vec: Vec<String> = grp.archive_rel_paths.into_iter().collect();

            let entry = self
                .sources
                .entry(hash.clone())
                .or_insert_with(|| SourceFileEntry {
                    sha256: hash.clone(),
                    subject: subject.clone(),
                    primary_filename: archive_vec
                        .first()
                        .or_else(|| inbox_vec.first())
                        .map(|p| basename_of(p).to_string())
                        .unwrap_or_default(),
                    size_bytes: grp.size_bytes,
                    inbox_paths: Vec::new(),
                    archive_paths: Vec::new(),
                    duplicate_paths: Vec::new(),
                    paper_id: None,
                    stage: SourceStage::Queued,
                    first_seen_at_ms: ts,
                    updated_at_ms: ts,
                });

            entry.subject = subject.clone();
            entry.size_bytes = grp.size_bytes;
            if let Some(first_arch) = archive_vec.first() {
                entry.primary_filename = basename_of(first_arch).to_string();
            } else if entry.primary_filename.is_empty()
                && let Some(first_inb) = inbox_vec.first()
            {
                entry.primary_filename = basename_of(first_inb).to_string();
            }

            let mut duplicates = Vec::new();
            if !archive_vec.is_empty() {
                let mut seen_matching_primary = false;
                for inb in &inbox_vec {
                    if basename_of(inb) == entry.primary_filename && !seen_matching_primary {
                        seen_matching_primary = true;
                    } else {
                        duplicates.push(inb.clone());
                    }
                }
            } else if inbox_vec.len() > 1 {
                let canonical_idx = inbox_vec
                    .iter()
                    .position(|p| basename_of(p) == entry.primary_filename)
                    .unwrap_or(0);
                if entry.primary_filename != basename_of(&inbox_vec[canonical_idx]) {
                    entry.primary_filename = basename_of(&inbox_vec[canonical_idx]).to_string();
                }
                for (idx, inb) in inbox_vec.iter().enumerate() {
                    if idx != canonical_idx {
                        duplicates.push(inb.clone());
                    }
                }
            }

            entry.inbox_paths = inbox_vec;
            entry.archive_paths = archive_vec;
            entry.duplicate_paths = duplicates;
            entry.updated_at_ms = ts;
        }

        // 2. Link SHA-256 `sources` <-> `papers` via `archived_source_path`
        let paper_arch_pairs: Vec<(String, String)> = self
            .papers
            .values()
            .filter_map(|p| {
                p.archived_source_path
                    .as_ref()
                    .map(|arch| (p.paper_id.clone(), arch.clone()))
            })
            .collect();

        for (pid, arch_rel) in paper_arch_pairs {
            let matched_hash = self
                .sources
                .iter()
                .find(|(_, s)| s.archive_paths.iter().any(|ap| ap == &arch_rel))
                .map(|(h, _)| h.clone());
            if let Some(ref h) = matched_hash {
                if let Some(src) = self.sources.get_mut(h)
                    && src.paper_id.is_none()
                {
                    src.paper_id = Some(pid.clone());
                }
                if let Some(p_entry) = self.papers.get_mut(&pid) {
                    p_entry.source_sha256 = Some(h.clone());
                }
            }
        }

        // 3. Merge live `CourseCatalogSnapshot` from `<stem>.sqlite` vault if provided
        if let Some(snap) = vault_snapshot {
            let mut verified_by_paper: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
            let mut verified_total_by_paper: BTreeMap<String, u32> = BTreeMap::new();
            let mut replicas_by_paper: BTreeMap<String, u32> = BTreeMap::new();

            for p in &snap.papers {
                let p_entry = self.papers.entry(p.paper_id.clone()).or_insert_with(|| {
                    PaperLedgerEntry::blank(p.paper_id.clone(), subject.clone(), ts)
                });
                p_entry.subject = subject.clone();
                p_entry.is_replica = p.is_replica || derive_parent_paper_id(&p.paper_id).is_some();
                if let Some(ref explicit_parent) = p.parent_paper_id {
                    p_entry.parent_paper_id = Some(explicit_parent.clone());
                } else if p_entry.parent_paper_id.is_none() {
                    p_entry.parent_paper_id = derive_parent_paper_id(&p.paper_id);
                }
                if p_entry.expected_questions.is_none()
                    && let Some(exp) = p
                        .meta_json
                        .get("expectedQuestions")
                        .and_then(|v| v.as_u64())
                        .and_then(|n| u32::try_from(n).ok())
                {
                    p_entry.expected_questions = Some(exp);
                }
            }

            for q in &snap.questions {
                let lineage = derive_question_replica_lineage(
                    &q.question_id,
                    q.parent_question_id.as_deref(),
                    q.paper_id.as_deref(),
                );
                let is_rep = q.is_replica || lineage.is_some();
                let scope = lineage.as_ref().map(|(sc, _, _)| *sc);

                if let Some(pid) = q.paper_id.as_deref() {
                    if is_rep {
                        *replicas_by_paper.entry(pid.to_string()).or_insert(0) += 1;
                    }
                    // Single-question replicas (`Q015-R01` on original `P001`) do NOT inflate `P001`'s base verified count,
                    // whereas member questions of a whole-paper replica (`P001_R01-Q015` on `P001_R01`) DO count toward `P001_R01`.
                    let counts_toward_paper_completion =
                        !is_rep || scope == Some(ReplicaScope::WholePaper);
                    if counts_toward_paper_completion
                        && (q.qa_status == "verified" || q.qa_status == "approved")
                    {
                        *verified_total_by_paper.entry(pid.to_string()).or_insert(0) += 1;
                        if let Ok(uid) = UniversalId::parse(&q.question_id)
                            && let Some(ItemSegment::Question { number, .. }) = uid.item
                        {
                            verified_by_paper
                                .entry(pid.to_string())
                                .or_default()
                                .insert(number);
                        }
                    }
                }

                // Upsert / enrich `self.items[q.question_id]`
                let p_meta = q.paper_id.as_deref().and_then(|pid| self.papers.get(pid));
                let p_src_file = p_meta.and_then(|p| p.source_file.clone());
                let p_src_sha = p_meta.and_then(|p| p.source_sha256.clone());
                let p_arch_src = p_meta.and_then(|p| p.archived_source_path.clone());
                let sol_rel_opt = q
                    .paper_id
                    .as_deref()
                    .map(|pid| format!("archive/{pid}/{}.json", q.question_id));
                let sol_json_path = sol_rel_opt.filter(|rel| subject_dir.join(rel).is_file());

                let item_entry =
                    self.items
                        .entry(q.question_id.clone())
                        .or_insert_with(|| ItemLedgerEntry {
                            universal_id: q.question_id.clone(),
                            subject: subject.clone(),
                            paper_id: q.paper_id.clone(),
                            source_file: p_src_file.clone(),
                            source_sha256: p_src_sha.clone(),
                            archived_source_path: p_arch_src.clone(),
                            page: None,
                            bbox: None,
                            bbox_px: None,
                            crop_path: None,
                            crop_dimensions: None,
                            solution_json_path: sol_json_path.clone(),
                            qa_status: Some(q.qa_status.clone()),
                            is_replica: is_rep,
                            replica_scope: scope,
                            parent_question_id: lineage.as_ref().and_then(|(_, pq, _)| pq.clone()),
                            parent_paper_id: lineage.as_ref().and_then(|(_, _, pp)| pp.clone()),
                            replica_ids: Vec::new(),
                            updated_at_ms: ts,
                        });
                item_entry.subject = subject.clone();
                if item_entry.paper_id.is_none() {
                    item_entry.paper_id = q.paper_id.clone();
                }
                if item_entry.source_file.is_none() {
                    item_entry.source_file = p_src_file;
                }
                if item_entry.source_sha256.is_none() {
                    item_entry.source_sha256 = p_src_sha;
                }
                if item_entry.archived_source_path.is_none() {
                    item_entry.archived_source_path = p_arch_src;
                }
                if sol_json_path.is_some() {
                    item_entry.solution_json_path = sol_json_path;
                }
                item_entry.qa_status = Some(q.qa_status.clone());
                item_entry.is_replica = is_rep;
                if scope.is_some() {
                    item_entry.replica_scope = scope;
                }
                if let Some((_, pq, pp)) = lineage {
                    if pq.is_some() {
                        item_entry.parent_question_id = pq;
                    }
                    if pp.is_some() {
                        item_entry.parent_paper_id = pp;
                    }
                }
                item_entry.updated_at_ms = ts;
            }

            for p_entry in self.papers.values_mut() {
                let v_nums: Vec<u32> = verified_by_paper
                    .remove(&p_entry.paper_id)
                    .map(|s| s.into_iter().collect())
                    .unwrap_or_default();
                let v_total = verified_total_by_paper
                    .remove(&p_entry.paper_id)
                    .unwrap_or(0)
                    .max(u32::try_from(v_nums.len()).unwrap_or(0));
                let r_total = replicas_by_paper.remove(&p_entry.paper_id).unwrap_or(0);

                p_entry.verified_question_numbers = v_nums;
                p_entry.verified_count = v_total;
                p_entry.replica_count = r_total;
                p_entry.updated_at_ms = ts;
            }
        }

        // 4. Refresh `solution_json_path` & `source_sha256` on existing sliced items + compute `sliced_count`
        let mut sliced_q_by_paper: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
        let mut raw_crop_count_by_paper: BTreeMap<String, u32> = BTreeMap::new();

        struct PaperSourceRef {
            source_file: Option<String>,
            source_sha256: Option<String>,
            archived_source_path: Option<String>,
        }

        let paper_source_lookup: BTreeMap<String, PaperSourceRef> = self
            .papers
            .iter()
            .map(|(pid, p)| {
                (
                    pid.clone(),
                    PaperSourceRef {
                        source_file: p.source_file.clone(),
                        source_sha256: p.source_sha256.clone(),
                        archived_source_path: p.archived_source_path.clone(),
                    },
                )
            })
            .collect();

        for item in self.items.values_mut() {
            if let Some(ref pid) = item.paper_id {
                if let Some(psrc) = paper_source_lookup.get(pid) {
                    if item.source_file.is_none() {
                        item.source_file = psrc.source_file.clone();
                    }
                    if item.source_sha256.is_none() && psrc.source_sha256.is_some() {
                        item.source_sha256 = psrc.source_sha256.clone();
                    }
                    if item.archived_source_path.is_none() && psrc.archived_source_path.is_some() {
                        item.archived_source_path = psrc.archived_source_path.clone();
                    }
                }
                let sol_rel = format!("archive/{pid}/{}.json", item.universal_id);
                if subject_dir.join(&sol_rel).is_file() {
                    item.solution_json_path = Some(sol_rel);
                }
            }

            // Count directly sliced crops (`bbox.is_some()`) toward `sliced_count`
            if item.bbox.is_some()
                && !item.is_replica
                && let Some(ref pid) = item.paper_id
            {
                *raw_crop_count_by_paper.entry(pid.clone()).or_insert(0) += 1;
                if let Ok(uid) = UniversalId::parse(&item.universal_id)
                    && let Some(ItemSegment::Question {
                        number,
                        replica: None,
                    }) = uid.item
                {
                    sliced_q_by_paper
                        .entry(pid.clone())
                        .or_default()
                        .insert(number);
                }
            }
        }

        // 5. Resolve Whole-Paper & Single-Question Replica Lineage + Inherited Origin References
        self.reconcile_replica_lineage();

        // 6. Finalize `sliced_count`, `missing_question_numbers`, and `stage` across all papers
        for p_entry in self.papers.values_mut() {
            let s_nums: Vec<u32> = sliced_q_by_paper
                .remove(&p_entry.paper_id)
                .map(|s| s.into_iter().collect())
                .unwrap_or_default();
            let raw_crops = raw_crop_count_by_paper
                .remove(&p_entry.paper_id)
                .unwrap_or(0);
            p_entry.sliced_count = raw_crops.max(u32::try_from(s_nums.len()).unwrap_or(0));
            p_entry.sliced_question_numbers = s_nums;
            let has_pages = p_entry.page_count.is_some() || !p_entry.pages.is_empty();
            recompute_paper_metrics(p_entry, has_pages);
        }

        self.propagate_source_stages();
        self.updated_at_ms = ts;
        Ok(())
    }

    fn reconcile_replica_lineage(&mut self) {
        // A. Whole-paper replica lineage (`replica_paper_ids` + inherited source file from original paper)
        for p in self.papers.values_mut() {
            p.replica_paper_ids.clear();
            if p.parent_paper_id.is_none() {
                p.parent_paper_id = derive_parent_paper_id(&p.paper_id);
            }
            if p.parent_paper_id.is_some() {
                p.is_replica = true;
            }
        }

        let replica_papers: Vec<(String, String)> = self
            .papers
            .values()
            .filter_map(|p| {
                p.parent_paper_id
                    .as_ref()
                    .map(|parent_id| (p.paper_id.clone(), parent_id.clone()))
            })
            .collect();

        for (rep_pid, parent_pid) in replica_papers {
            let parent_snap = self.papers.get(&parent_pid).map(|orig| {
                (
                    orig.source_file.clone(),
                    orig.archived_source_path.clone(),
                    orig.source_sha256.clone(),
                    orig.expected_questions,
                )
            });
            if let Some(orig) = self.papers.get_mut(&parent_pid)
                && !orig.replica_paper_ids.contains(&rep_pid)
            {
                orig.replica_paper_ids.push(rep_pid.clone());
            }
            if let Some((sf, arch, sha, exp)) = parent_snap
                && let Some(rep) = self.papers.get_mut(&rep_pid)
            {
                if rep.source_file.is_none() {
                    rep.source_file = sf;
                }
                if rep.archived_source_path.is_none() {
                    rep.archived_source_path = arch;
                }
                if rep.source_sha256.is_none() {
                    rep.source_sha256 = sha;
                }
                if rep.expected_questions.is_none() {
                    rep.expected_questions = exp;
                }
            }
        }

        // B. Question-level replica lineage (`replica_ids` + inherited original source/page/crop)
        for item in self.items.values_mut() {
            item.replica_ids.clear();
            if let Some((scope, pq, pp)) = derive_question_replica_lineage(
                &item.universal_id,
                item.parent_question_id.as_deref(),
                item.paper_id.as_deref(),
            ) {
                item.is_replica = true;
                if item.replica_scope.is_none() {
                    item.replica_scope = Some(scope);
                }
                if item.parent_question_id.is_none() {
                    item.parent_question_id = pq;
                }
                if item.parent_paper_id.is_none() {
                    item.parent_paper_id = pp;
                }
            }
        }

        let replica_items: Vec<(String, Option<String>, Option<String>)> = self
            .items
            .values()
            .filter(|i| i.is_replica)
            .map(|i| {
                (
                    i.universal_id.clone(),
                    i.parent_question_id.clone(),
                    i.parent_paper_id.clone(),
                )
            })
            .collect();

        for (rep_qid, parent_qid_opt, parent_pid_opt) in replica_items {
            if let Some(ref parent_qid) = parent_qid_opt {
                let parent_q_snap = self.items.get(parent_qid).map(|orig| {
                    (
                        orig.source_file.clone(),
                        orig.source_sha256.clone(),
                        orig.archived_source_path.clone(),
                        orig.page,
                        orig.crop_path.clone(),
                        orig.crop_dimensions,
                        orig.paper_id.clone(),
                    )
                });
                if let Some(orig) = self.items.get_mut(parent_qid)
                    && !orig.replica_ids.contains(&rep_qid)
                {
                    orig.replica_ids.push(rep_qid.clone());
                }
                if let Some((sf, sha, arch, pg, crop, dims, orig_pid)) = parent_q_snap
                    && let Some(rep) = self.items.get_mut(&rep_qid)
                {
                    if rep.source_file.is_none() {
                        rep.source_file = sf;
                    }
                    if rep.source_sha256.is_none() {
                        rep.source_sha256 = sha;
                    }
                    if rep.archived_source_path.is_none() {
                        rep.archived_source_path = arch;
                    }
                    if rep.page.is_none() {
                        rep.page = pg;
                    }
                    if rep.crop_path.is_none() {
                        rep.crop_path = crop;
                    }
                    if rep.crop_dimensions.is_none() {
                        rep.crop_dimensions = dims;
                    }
                    if rep.parent_paper_id.is_none() {
                        rep.parent_paper_id = orig_pid;
                    }
                }
            } else if let Some(ref parent_pid) = parent_pid_opt {
                let parent_p_snap = self.papers.get(parent_pid).map(|orig_p| {
                    (
                        orig_p.source_file.clone(),
                        orig_p.source_sha256.clone(),
                        orig_p.archived_source_path.clone(),
                    )
                });
                if let Some((sf, sha, arch)) = parent_p_snap
                    && let Some(rep) = self.items.get_mut(&rep_qid)
                {
                    if rep.source_file.is_none() {
                        rep.source_file = sf;
                    }
                    if rep.source_sha256.is_none() {
                        rep.source_sha256 = sha;
                    }
                    if rep.archived_source_path.is_none() {
                        rep.archived_source_path = arch;
                    }
                }
            }
        }
    }

    fn propagate_source_stages(&mut self) {
        for src in self.sources.values_mut() {
            if let Some(ref pid) = src.paper_id
                && let Some(p_entry) = self.papers.get(pid)
            {
                src.stage = p_entry.stage;
                continue;
            }
            if !src.archive_paths.is_empty() {
                src.stage = SourceStage::Sliced;
            } else {
                src.stage = SourceStage::Queued;
            }
        }
    }
}

/// Derive original paper ID from a whole-paper replica ID (`"GYR-MTH-P001_R01"` -> `Some("GYR-MTH-P001")`).
pub fn derive_parent_paper_id(paper_id: &str) -> Option<String> {
    let uid = UniversalId::parse(paper_id).ok()?;
    if uid.item.is_some() {
        return None;
    }
    if let Some(ContainerSegment::Paper {
        number,
        replica: Some(_),
    }) = uid.container
    {
        UniversalId::paper(&uid.product_code, &uid.subject_code, number, None)
            .ok()
            .map(|u| u.to_string())
    } else {
        None
    }
}

/// Derive `(ReplicaScope, parent_question_id, parent_paper_id)` for a question replica.
pub fn derive_question_replica_lineage(
    question_id: &str,
    explicit_parent_question_id: Option<&str>,
    explicit_paper_id: Option<&str>,
) -> Option<(ReplicaScope, Option<String>, Option<String>)> {
    let explicit_pq = explicit_parent_question_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Ok(uid) = UniversalId::parse(question_id) {
        match (&uid.container, &uid.item) {
            // Case 1: Single-question replica (`GYR-MTH-P001-Q015-R01` or `GYR-MTH-T_ALG-Q003-R01`)
            (
                _,
                Some(ItemSegment::Question {
                    number,
                    replica: Some(_),
                }),
            ) => {
                let mut parent_uid = uid.clone();
                parent_uid.sub_address = None;
                parent_uid.item = Some(ItemSegment::Question {
                    number: *number,
                    replica: None,
                });
                if let Some(ContainerSegment::Paper {
                    number: pno,
                    replica: Some(_),
                }) = parent_uid.container
                {
                    parent_uid.container = Some(ContainerSegment::Paper {
                        number: pno,
                        replica: None,
                    });
                }
                let pq = explicit_pq.or_else(|| Some(parent_uid.to_string()));
                let pp = match uid.container {
                    Some(ContainerSegment::Paper { number: pno, .. }) => {
                        UniversalId::paper(&uid.product_code, &uid.subject_code, pno, None)
                            .ok()
                            .map(|u| u.to_string())
                    }
                    _ => explicit_paper_id.map(str::to_string),
                };
                return Some((ReplicaScope::SingleQuestion, pq, pp));
            }
            // Case 2: Member question of a whole-paper replica (`GYR-MTH-P001_R01-Q015`)
            (
                Some(ContainerSegment::Paper {
                    number: pno,
                    replica: Some(_),
                }),
                Some(ItemSegment::Question {
                    number: qno,
                    replica: None,
                }),
            ) => {
                let orig_paper =
                    UniversalId::paper(&uid.product_code, &uid.subject_code, *pno, None)
                        .ok()
                        .map(|u| u.to_string());
                let orig_question = explicit_pq.or_else(|| {
                    UniversalId::paper_question(&uid.product_code, &uid.subject_code, *pno, *qno)
                        .ok()
                        .map(|u| u.to_string())
                });
                return Some((ReplicaScope::WholePaper, orig_question, orig_paper));
            }
            _ => {}
        }
    }

    explicit_pq.map(|pq| {
        let pp = UniversalId::parse(&pq)
            .ok()
            .and_then(|u| u.container_id())
            .or_else(|| explicit_paper_id.map(str::to_string));
        (ReplicaScope::SingleQuestion, Some(pq), pp)
    })
}

fn recompute_paper_metrics(entry: &mut PaperLedgerEntry, has_sliced_pages: bool) {
    let verified_set: BTreeSet<u32> = entry.verified_question_numbers.iter().copied().collect();
    let max_sliced = entry.sliced_question_numbers.last().copied().unwrap_or(0);
    let max_verified = entry.verified_question_numbers.last().copied().unwrap_or(0);
    let target_max = entry
        .expected_questions
        .unwrap_or_else(|| max_sliced.max(max_verified));

    entry.missing_question_numbers = if target_max > 0 {
        (1..=target_max)
            .filter(|q| !verified_set.contains(q))
            .collect()
    } else {
        Vec::new()
    };

    entry.stage = if entry
        .expected_questions
        .is_some_and(|exp| exp > 0 && entry.verified_count >= exp)
    {
        SourceStage::Completed
    } else if entry.verified_count > 0 {
        SourceStage::InProgress
    } else if entry.sliced_count > 0 || has_sliced_pages {
        SourceStage::Sliced
    } else {
        SourceStage::Queued
    };
}

/// Convenience helper loading `ledger.json` for `(product, subject)`, syncing with `vault_snapshot`,
/// and atomically saving `products/<product>/<subject>/ledger.json`.
pub fn sync_and_save_subject_ledger(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    vault_snapshot: Option<&CourseCatalogSnapshot>,
) -> Result<ProductLedger> {
    let ledger_path = cfg.ledger_path(product, subject);
    let mut ledger = ProductLedger::load_or_new(&ledger_path, product, subject)?;
    ledger.sync_with_workspace(cfg, vault_snapshot)?;
    ledger.save_atomic(&ledger_path)?;
    Ok(ledger)
}

/// Compute lowercase 64-character SHA-256 hex digest and byte length of `path` using streaming reads.
pub fn sha256_file(path: &Path) -> Result<(String, u64)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut total_bytes = 0u64;
    let mut buf = [0u8; 65_536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total_bytes = total_bytes.saturating_add(n as u64);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    for &b in digest.as_slice() {
        hex.push(HEX_CHARS[(b >> 4) as usize] as char);
        hex.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    Ok((hex, total_bytes))
}

/// Format sorted question numbers into compact human-readable ranges (e.g. `"Q002..Q014, Q016..Q050"`).
pub fn format_question_number_ranges(nums: &[u32]) -> String {
    if nums.is_empty() {
        return "none".to_string();
    }
    let mut sorted = nums.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut spans: Vec<(u32, u32)> = Vec::new();
    for n in sorted {
        if let Some(last) = spans.last_mut()
            && n == last.1.saturating_add(1)
        {
            last.1 = n;
        } else {
            spans.push((n, n));
        }
    }

    spans
        .into_iter()
        .map(|(a, b)| {
            if a == b {
                format!("Q{a:03}")
            } else {
                format!("Q{a:03}..Q{b:03}")
            }
        })
        .collect::<Vec<String>>()
        .join(", ")
}

/// Map `UniversalId` uppercase subject prefix (`MTH`, `ENG`, `VR`, `NVR`, `PHY`, etc.) back to canonical directory slug.
pub fn subject_slug_for_prefix(subject_code: &str) -> &'static str {
    match subject_code {
        "MTH" => "maths",
        "ENG" => "english",
        "VR" => "verbal-reasoning",
        "NVR" => "non-verbal-reasoning",
        "PHY" => "physics",
        "CHM" => "chemistry",
        "BIO" => "biology",
        "CS" => "computer-science",
        "HIS" => "history",
        "GEO" => "geography",
        "ECO" => "economics",
        "FM" => "further-maths",
        _ => "maths",
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn is_supported_raw_source(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    if name.starts_with('.') {
        return false;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "pdf" | "png" | "jpg" | "jpeg" | "webp" | "svg"
    )
}

fn collect_raw_source_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in read_sorted_dir(dir)? {
        let path = entry.path();
        if path.is_dir() {
            collect_raw_source_files(&path, out)?;
        } else if path.is_file() && is_supported_raw_source(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn read_sorted_dir(dir: &Path) -> Result<Vec<std::fs::DirEntry>> {
    let mut entries = Vec::new();
    for res in std::fs::read_dir(dir)? {
        entries.push(res?);
    }
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

fn relative_path_str(base: &Path, target: &Path) -> String {
    target
        .strip_prefix(base)
        .unwrap_or(target)
        .to_string_lossy()
        .replace('\\', "/")
}

fn basename_of(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_question_ranges_detects_sha256_dups_and_tracks_replica_lineage() -> Result<()> {
        assert_eq!(format_question_number_ranges(&[]), "none");
        assert_eq!(
            format_question_number_ranges(&[2, 3, 4, 5, 14, 16, 17, 18]),
            "Q002..Q005, Q014, Q016..Q018"
        );

        let tmp =
            std::env::temp_dir().join(format!("hydrogen-ledger-unit-test-{}", std::process::id()));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(tmp.join("products/gyaraplus/maths/inbox"))?;

        let cfg = HydrogenConfig {
            workspace_root: tmp.clone(),
            products_dir: tmp.join("products"),
            rules_dir: tmp.join("rules"),
            default_product: "gyaraplus".into(),
            default_subject: "maths".into(),
            bind_addr: "127.0.0.1:7430".into(),
            carbon_ws_url: "ws://127.0.0.1:7420/carbon".into(),
            device_id: "test-dev".into(),
            lesson_key: "test-key".into(),
            gemini_api_key: None,
        };

        let inbox = cfg.subject_inbox_dir("gyaraplus", "maths");
        let payload = b"%PDF-1.4 synthetic exam paper bytes for sha256 dedup test";
        std::fs::write(inbox.join("Paper-A.pdf"), payload)?;
        std::fs::write(inbox.join("Paper-A-Renamed-Copy.pdf"), payload)?;

        let mut ledger = sync_and_save_subject_ledger(&cfg, "gyaraplus", "maths", None)?;
        assert_eq!(
            ledger.sources.len(),
            1,
            "identical SHA-256 files must deduplicate"
        );
        let (_, src) = ledger
            .sources
            .iter()
            .next()
            .ok_or_else(|| HydrogenError::Validation("expected 1 source entry".into()))?;
        assert!(src.has_renamed_duplicates());
        assert_eq!(src.duplicate_paths, vec!["inbox/Paper-A-Renamed-Copy.pdf"]);

        // Record original sliced paper + crop, then verify single-question & whole-paper replica lineage
        ledger.upsert_sliced_paper(SlicedPaperRecord {
            paper_id: "GYR-MTH-P001".into(),
            subject: "maths".into(),
            source_file: "Paper-A.pdf".into(),
            archived_source_path: "archive/GYR-MTH-P001/source/Paper-A.pdf".into(),
            page_count: 12,
            dpi: 180.0,
            pages: Vec::new(),
        });
        ledger.upsert_sliced_item(&SlicedCropRecord {
            universal_id: "GYR-MTH-P001-Q015".into(),
            paper_id: "GYR-MTH-P001".into(),
            subject: "maths".into(),
            source_file: "Paper-A.pdf".into(),
            page: 6,
            bbox: NormalizedBBox {
                x: 0.05,
                y: 0.10,
                w: 0.90,
                h: 0.40,
            },
            bbox_px: PixelBBox {
                x: 50,
                y: 100,
                w: 900,
                h: 400,
            },
            crop_path: "archive/GYR-MTH-P001/crops/GYR-MTH-P001-Q015.png".into(),
            width_px: 900,
            height_px: 400,
        });
        ledger.set_expected_questions("GYR-MTH-P001", 10)?;

        // Register a whole-paper replica (`GYR-MTH-P001_R01`) + member item + single-question replica (`GYR-MTH-P001-Q015-R01`)
        ledger.papers.insert(
            "GYR-MTH-P001_R01".into(),
            PaperLedgerEntry::blank("GYR-MTH-P001_R01".into(), "maths".into(), 1),
        );
        ledger.items.insert(
            "GYR-MTH-P001-Q015-R01".into(),
            ItemLedgerEntry {
                universal_id: "GYR-MTH-P001-Q015-R01".into(),
                subject: "maths".into(),
                paper_id: Some("GYR-MTH-P001".into()),
                source_file: None,
                source_sha256: None,
                archived_source_path: None,
                page: None,
                bbox: None,
                bbox_px: None,
                crop_path: None,
                crop_dimensions: None,
                solution_json_path: None,
                qa_status: Some("verified".into()),
                is_replica: true,
                replica_scope: None,
                parent_question_id: None,
                parent_paper_id: None,
                replica_ids: Vec::new(),
                updated_at_ms: 1,
            },
        );
        ledger.items.insert(
            "GYR-MTH-P001_R01-Q015".into(),
            ItemLedgerEntry {
                universal_id: "GYR-MTH-P001_R01-Q015".into(),
                subject: "maths".into(),
                paper_id: Some("GYR-MTH-P001_R01".into()),
                source_file: None,
                source_sha256: None,
                archived_source_path: None,
                page: None,
                bbox: None,
                bbox_px: None,
                crop_path: None,
                crop_dimensions: None,
                solution_json_path: None,
                qa_status: Some("verified".into()),
                is_replica: true,
                replica_scope: None,
                parent_question_id: None,
                parent_paper_id: None,
                replica_ids: Vec::new(),
                updated_at_ms: 1,
            },
        );

        ledger.sync_with_workspace(&cfg, None)?;

        let orig_p = &ledger.papers["GYR-MTH-P001"];
        assert_eq!(orig_p.replica_paper_ids, vec!["GYR-MTH-P001_R01"]);
        let rep_p = &ledger.papers["GYR-MTH-P001_R01"];
        assert!(rep_p.is_replica);
        assert_eq!(rep_p.parent_paper_id.as_deref(), Some("GYR-MTH-P001"));
        assert_eq!(rep_p.source_file.as_deref(), Some("Paper-A.pdf"));

        let single_rep = &ledger.items["GYR-MTH-P001-Q015-R01"];
        assert_eq!(single_rep.replica_scope, Some(ReplicaScope::SingleQuestion));
        assert_eq!(
            single_rep.parent_question_id.as_deref(),
            Some("GYR-MTH-P001-Q015")
        );
        assert_eq!(single_rep.source_file.as_deref(), Some("Paper-A.pdf"));
        assert_eq!(single_rep.page, Some(6));
        assert_eq!(
            single_rep.crop_path.as_deref(),
            Some("archive/GYR-MTH-P001/crops/GYR-MTH-P001-Q015.png")
        );

        let paper_member_rep = &ledger.items["GYR-MTH-P001_R01-Q015"];
        assert_eq!(
            paper_member_rep.replica_scope,
            Some(ReplicaScope::WholePaper)
        );
        assert_eq!(
            paper_member_rep.parent_paper_id.as_deref(),
            Some("GYR-MTH-P001")
        );
        assert_eq!(
            paper_member_rep.parent_question_id.as_deref(),
            Some("GYR-MTH-P001-Q015")
        );

        let orig_q = &ledger.items["GYR-MTH-P001-Q015"];
        assert!(
            orig_q
                .replica_ids
                .contains(&"GYR-MTH-P001-Q015-R01".to_string())
        );

        std::fs::remove_dir_all(&tmp)?;
        Ok(())
    }
}
