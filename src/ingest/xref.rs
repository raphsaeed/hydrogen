//! Local Git-ignored `private-xref.json` store mapping clean `UniversalId`s back to archived source crops.
//!
//! **Clean-Room Law (`docs/system-architecture.md` §1.4):**
//! This file lives strictly at `products/<product>/private-xref.json` (excluded from Git and `.raphs`).
//! Source filenames, page numbers, and cropped scans NEVER enter `<product>-<subject>.raphs`.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Canonical `private-xref.json` schema version.
pub const XREF_SCHEMA_VERSION: &str = "1.0";

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

/// Metadata for a single high-DPI rendered page PNG inside `archive/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPageRecord {
    /// 1-based page index.
    pub page: u32,
    /// Path relative to `products/<product>/` (inside `archive/`).
    pub image_path: String,
    pub width_px: u32,
    pub height_px: u32,
}

/// Paper-level private provenance mapping (`GYR-MTH-P001` <-> original archived PDF/image).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperXrefEntry {
    pub paper_id: String,
    pub subject: String,
    pub source_file: String,
    pub archived_source_path: String,
    pub page_count: u32,
    pub dpi: f32,
    pub pages: Vec<RenderedPageRecord>,
    pub updated_at_ms: u64,
}

/// Question / diagram crop mapping (`{ universal_id, source_file, page, bbox }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemXrefEntry {
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
    /// Path relative to `products/<product>/` (`archive/<subject>/<paper_id>/crops/<id>.png`).
    pub crop_path: String,
    pub width_px: u32,
    pub height_px: u32,
    pub updated_at_ms: u64,
}

/// Product-scoped `private-xref.json` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateXrefStore {
    pub schema_version: String,
    pub product: String,
    pub updated_at_ms: u64,
    #[serde(default)]
    pub papers: BTreeMap<String, PaperXrefEntry>,
    #[serde(default)]
    pub items: BTreeMap<String, ItemXrefEntry>,
}

impl PrivateXrefStore {
    /// Create an empty cross-reference store for `product`.
    pub fn new(product: impl Into<String>) -> Self {
        Self {
            schema_version: XREF_SCHEMA_VERSION.to_string(),
            product: product.into(),
            updated_at_ms: now_ms(),
            papers: BTreeMap::new(),
            items: BTreeMap::new(),
        }
    }

    /// Load an existing `private-xref.json` from `path`, or initialize an empty store if not present yet.
    pub fn load_or_new(path: &Path, product: &str) -> Result<Self> {
        if path.is_file() {
            let raw = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&raw)?)
        } else {
            Ok(Self::new(product))
        }
    }

    /// Atomically persist `private-xref.json` to `path` via a sibling `.tmp` file.
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

    /// Insert or update a paper provenance entry.
    pub fn upsert_paper(&mut self, entry: PaperXrefEntry) {
        self.papers.insert(entry.paper_id.clone(), entry);
        self.updated_at_ms = now_ms();
    }

    /// Insert or update a question/figure crop entry.
    pub fn upsert_item(&mut self, entry: ItemXrefEntry) {
        self.items.insert(entry.universal_id.clone(), entry);
        self.updated_at_ms = now_ms();
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
