//! Pure-Rust PDF & PNG page renderer and clean-room question/diagram cropper (`hayro` + `png`).
//!
//! Renders source pages into `products/<product>/<subject>/archive/<paper_id>/pages/`,
//! slices individual crops into `products/<product>/<subject>/archive/<paper_id>/crops/`,
//! and records `{ universal_id, source_file, page, bbox, bboxPx, cropPath }` directly in
//! `products/<product>/<subject>/ledger.json` without ever touching `<stem>.raphs` or Git-tracked directories.

use std::collections::BTreeSet;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
use serde::{Deserialize, Serialize};

use crate::config::HydrogenConfig;
use crate::domain::{ContainerSegment, UniversalId};
use crate::error::{HydrogenError, Result};
use crate::ingest::ledger::{
    NormalizedBBox, PixelBBox, ProductLedger, RenderedPageRecord, SlicedCropRecord,
    SlicedPaperRecord,
};

/// Default rasterization resolution in dots per inch (`72 pt == 1 inch`, so `180 DPI == 2.5x`).
pub const DEFAULT_RENDER_DPI: f32 = 180.0;

/// Crop coordinate specification (either `[0.0..=1.0]` normalized or exact pixel coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CropBox {
    Normalized(NormalizedBBox),
    Pixels(PixelBBox),
}

/// Explicit crop request entry (used by `--id/--page/--bbox` or `--crops-json <manifest.json>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropSpec {
    pub universal_id: String,
    /// 1-based page number.
    pub page: u32,
    pub bbox: CropBox,
}

/// Slicing strategy for `slice_paper`.
#[derive(Debug, Clone, PartialEq)]
pub enum SliceMode {
    /// Render high-DPI full-page PNGs into `archive/` only (for visual inspection before exact `--bbox` crops).
    PagesOnly,
    /// Automatically detect horizontal ink bands separated by whitespace gaps and slice `<paper_id>-Q001..QNNN`.
    AutoWhitespaceBands {
        min_gap_px: u32,
        min_band_height_px: u32,
        pad_px: u32,
        start_question_no: u32,
    },
    /// Slice explicit normalized or pixel bounding boxes into `archive/` + `ledger.json`.
    ExplicitCrops(Vec<CropSpec>),
}

impl Default for SliceMode {
    fn default() -> Self {
        Self::AutoWhitespaceBands {
            min_gap_px: 32,
            min_band_height_px: 48,
            pad_px: 14,
            start_question_no: 1,
        }
    }
}

/// Parameters for slicing a raw PDF paper or PNG worksheet into `archive/` + `ledger.json`.
#[derive(Debug, Clone)]
pub struct SlicePaperRequest {
    pub source_path: PathBuf,
    pub product: String,
    pub subject: String,
    /// Clean paper ID (e.g. `"GYR-MTH-P001"`). If `None`, assigns the next sequential paper ID in `ledger.json`.
    pub paper_id: Option<String>,
    pub dpi: f32,
    /// Optional 1-based inclusive `(start_page, end_page)` filter.
    pub page_range: Option<(u32, u32)>,
    pub mode: SliceMode,
    /// If `true`, removes the original file from `inbox/` after copying into `archive/.../source/`.
    pub move_source_to_archive: bool,
}

/// Summary returned after executing `slice_paper`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlicePaperSummary {
    pub paper_id: String,
    pub archived_source_path: PathBuf,
    pub ledger_path: PathBuf,
    pub total_pdf_pages: u32,
    pub rendered_pages: Vec<RenderedPageRecord>,
    pub sliced_items: Vec<SlicedCropRecord>,
}

/// Unpremultiplied RGBA8 raster buffer in row-major order (`4 * width * height` bytes).
#[derive(Debug, Clone)]
pub struct RgbaBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl RgbaBuffer {
    /// Extract a clamped rectangular sub-region as a standalone `RgbaBuffer`.
    pub fn crop(&self, rect: PixelBBox) -> Result<Self> {
        if self.width == 0 || self.height == 0 {
            return Err(HydrogenError::Validation(
                "cannot crop empty image buffer".into(),
            ));
        }
        let x = rect.x.min(self.width.saturating_sub(1));
        let y = rect.y.min(self.height.saturating_sub(1));
        let w = rect.w.max(1).min(self.width.saturating_sub(x));
        let h = rect.h.max(1).min(self.height.saturating_sub(y));

        let mut out = Vec::with_capacity((w as usize) * (h as usize) * 4);
        let src_stride = (self.width as usize) * 4;
        let row_bytes = (w as usize) * 4;
        for row in 0..h {
            let src_y = (y + row) as usize;
            let start = src_y * src_stride + (x as usize) * 4;
            let end = start + row_bytes;
            let slice = self.data.get(start..end).ok_or_else(|| {
                HydrogenError::Validation("crop rectangle out of RGBA buffer bounds".into())
            })?;
            out.extend_from_slice(slice);
        }

        Ok(Self {
            width: w,
            height: h,
            data: out,
        })
    }

    /// Encode this RGBA8 buffer as lossless PNG bytes in pure Rust.
    pub fn encode_png(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|e| HydrogenError::Validation(format!("PNG header encode error: {e}")))?;
            writer
                .write_image_data(&self.data)
                .map_err(|e| HydrogenError::Validation(format!("PNG pixel encode error: {e}")))?;
        }
        Ok(out)
    }

    /// Decode PNG bytes into an unpremultiplied RGBA8 buffer in pure Rust.
    pub fn decode_png(bytes: &[u8]) -> Result<Self> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(
            png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
        );
        let mut reader = decoder
            .read_info()
            .map_err(|e| HydrogenError::Validation(format!("PNG decode error: {e}")))?;
        let buf_size = reader.output_buffer_size().ok_or_else(|| {
            HydrogenError::Validation("PNG output buffer size exceeded limits".into())
        })?;
        let mut raw = vec![0u8; buf_size];
        let info = reader
            .next_frame(&mut raw)
            .map_err(|e| HydrogenError::Validation(format!("PNG frame decode error: {e}")))?;
        raw.truncate(info.buffer_size());

        let width = info.width;
        let height = info.height;
        let expected_rgba = (width as usize) * (height as usize) * 4;

        let rgba = match info.color_type {
            png::ColorType::Rgba => raw,
            png::ColorType::GrayscaleAlpha => {
                let mut conv = Vec::with_capacity(expected_rgba);
                for &[g, a] in raw.as_chunks::<2>().0 {
                    conv.extend_from_slice(&[g, g, g, a]);
                }
                conv
            }
            png::ColorType::Rgb => {
                let mut conv = Vec::with_capacity(expected_rgba);
                for &[r, g, b] in raw.as_chunks::<3>().0 {
                    conv.extend_from_slice(&[r, g, b, 255]);
                }
                conv
            }
            png::ColorType::Grayscale => {
                let mut conv = Vec::with_capacity(expected_rgba);
                for &g in &raw {
                    conv.extend_from_slice(&[g, g, g, 255]);
                }
                conv
            }
            png::ColorType::Indexed => {
                return Err(HydrogenError::Validation(
                    "unexpected unexpanded indexed PNG frame".into(),
                ));
            }
        };

        if rgba.len() != expected_rgba {
            return Err(HydrogenError::Validation(format!(
                "decoded PNG byte length {} did not match {}x{}x4",
                rgba.len(),
                width,
                height
            )));
        }

        Ok(Self {
            width,
            height,
            data: rgba,
        })
    }
}

/// Render and slice a source PDF or PNG into `products/<product>/<subject>/archive/<paper_id>/`
/// and update `products/<product>/<subject>/ledger.json` atomically.
pub fn slice_paper(cfg: &HydrogenConfig, req: &SlicePaperRequest) -> Result<SlicePaperSummary> {
    if !req.source_path.is_file() {
        return Err(HydrogenError::Validation(format!(
            "source file does not exist: {}",
            req.source_path.display()
        )));
    }
    let dpi = if req.dpi >= 72.0 {
        req.dpi
    } else {
        DEFAULT_RENDER_DPI
    };

    let ledger_path = cfg.ledger_path(&req.product, &req.subject);
    let mut ledger = ProductLedger::load_or_new(&ledger_path, &req.product, &req.subject)?;

    let paper_id = match req.paper_id.as_deref() {
        Some(explicit) => {
            let parsed = UniversalId::parse(explicit)?;
            if !matches!(parsed.container, Some(ContainerSegment::Paper { .. }))
                || parsed.item.is_some()
            {
                return Err(HydrogenError::Validation(format!(
                    "expected a Paper UniversalId (e.g. 'GYR-MTH-P001'), got '{explicit}'"
                )));
            }
            explicit.to_string()
        }
        None => next_sequential_paper_id(&ledger, &req.product, &req.subject)?,
    };

    let subject_dir = cfg.subject_dir(&req.product, &req.subject);
    let paper_archive_dir = cfg
        .subject_archive_dir(&req.product, &req.subject)
        .join(&paper_id);
    let source_archive_dir = paper_archive_dir.join("source");
    let pages_dir = paper_archive_dir.join("pages");
    let crops_dir = paper_archive_dir.join("crops");

    std::fs::create_dir_all(&source_archive_dir)?;
    std::fs::create_dir_all(&pages_dir)?;
    std::fs::create_dir_all(&crops_dir)?;

    let source_filename = req
        .source_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| HydrogenError::Validation("invalid UTF-8 source filename".into()))?
        .to_string();

    let archived_source_abs = source_archive_dir.join(&source_filename);
    if req.source_path != archived_source_abs {
        std::fs::copy(&req.source_path, &archived_source_abs)?;
        if req.move_source_to_archive {
            std::fs::remove_file(&req.source_path)?;
        }
    }
    let archived_source_rel = relative_to(&subject_dir, &archived_source_abs);

    // Determine which 1-based pages need to be available
    let explicit_pages: Option<BTreeSet<u32>> = match &req.mode {
        SliceMode::ExplicitCrops(specs) => {
            let mut set = BTreeSet::new();
            for s in specs {
                if s.page == 0 {
                    return Err(HydrogenError::Validation(format!(
                        "crop '{}' specified 0-based page 0 (pages are 1-based)",
                        s.universal_id
                    )));
                }
                set.insert(s.page);
            }
            Some(set)
        }
        _ => None,
    };

    let ext = archived_source_abs
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let (total_pages, rendered_buffers) = if ext == "png" {
        let bytes = std::fs::read(&archived_source_abs)?;
        let buf = RgbaBuffer::decode_png(&bytes)?;
        let page_path = pages_dir.join("page-001.png");
        if !page_path.is_file() {
            std::fs::write(&page_path, &bytes)?;
        }
        (1u32, vec![(1u32, buf)])
    } else if ext == "pdf" {
        render_pdf_pages(
            &archived_source_abs,
            &pages_dir,
            dpi,
            req.page_range,
            explicit_pages.as_ref(),
        )?
    } else {
        return Err(HydrogenError::Validation(format!(
            "unsupported input file extension '.{ext}' (expected `.pdf` or `.png`)"
        )));
    };

    // Merge rendered page metadata with any previously rendered pages in `ledger.json`
    let mut page_records_by_no = std::collections::BTreeMap::<u32, RenderedPageRecord>::new();
    if let Some(existing_paper) = ledger.papers.get(&paper_id) {
        for p in &existing_paper.pages {
            page_records_by_no.insert(p.page, p.clone());
        }
    }
    for (page_no, buf) in &rendered_buffers {
        let page_abs = pages_dir.join(format!("page-{page_no:03}.png"));
        page_records_by_no.insert(
            *page_no,
            RenderedPageRecord {
                page: *page_no,
                image_path: relative_to(&subject_dir, &page_abs),
                width_px: buf.width,
                height_px: buf.height,
            },
        );
    }
    let rendered_pages: Vec<RenderedPageRecord> = page_records_by_no.into_values().collect();

    ledger.upsert_sliced_paper(SlicedPaperRecord {
        paper_id: paper_id.clone(),
        subject: req.subject.clone(),
        source_file: source_filename.clone(),
        archived_source_path: archived_source_rel,
        page_count: total_pages,
        dpi,
        pages: rendered_pages.clone(),
    });

    // Execute slicing mode
    let mut sliced_items = Vec::new();
    match &req.mode {
        SliceMode::PagesOnly => {}
        SliceMode::ExplicitCrops(specs) => {
            let page_map: std::collections::BTreeMap<u32, &RgbaBuffer> =
                rendered_buffers.iter().map(|(p, b)| (*p, b)).collect();

            for spec in specs {
                // Validate clean UniversalId (no source leaks allowed in IDs!)
                let _ = UniversalId::parse(&spec.universal_id)?;
                let page_buf = page_map.get(&spec.page).copied().ok_or_else(|| {
                    HydrogenError::Validation(format!(
                        "page {} out of range (1..={total_pages}) for crop '{}'",
                        spec.page, spec.universal_id
                    ))
                })?;

                let (bbox_px, bbox_norm) = match spec.bbox {
                    CropBox::Normalized(nb) => {
                        let px = nb.to_pixel_bbox(page_buf.width, page_buf.height);
                        let exact_nb =
                            NormalizedBBox::from_pixel_bbox(px, page_buf.width, page_buf.height);
                        (px, exact_nb)
                    }
                    CropBox::Pixels(px) => {
                        let clamped = PixelBBox {
                            x: px.x.min(page_buf.width.saturating_sub(1)),
                            y: px.y.min(page_buf.height.saturating_sub(1)),
                            w: px.w.max(1).min(
                                page_buf
                                    .width
                                    .saturating_sub(px.x.min(page_buf.width.saturating_sub(1))),
                            ),
                            h: px.h.max(1).min(
                                page_buf
                                    .height
                                    .saturating_sub(px.y.min(page_buf.height.saturating_sub(1))),
                            ),
                        };
                        let nb = NormalizedBBox::from_pixel_bbox(
                            clamped,
                            page_buf.width,
                            page_buf.height,
                        );
                        (clamped, nb)
                    }
                };

                let cropped = page_buf.crop(bbox_px)?;
                let png_bytes = cropped.encode_png()?;
                let safe_stem = spec.universal_id.replace(':', "_");
                let crop_abs = crops_dir.join(format!("{safe_stem}.png"));
                std::fs::write(&crop_abs, png_bytes)?;

                let item = SlicedCropRecord {
                    universal_id: spec.universal_id.clone(),
                    paper_id: paper_id.clone(),
                    subject: req.subject.clone(),
                    source_file: source_filename.clone(),
                    page: spec.page,
                    bbox: bbox_norm,
                    bbox_px,
                    crop_path: relative_to(&subject_dir, &crop_abs),
                    width_px: cropped.width,
                    height_px: cropped.height,
                };
                ledger.upsert_sliced_item(&item);
                sliced_items.push(item);
            }
        }
        SliceMode::AutoWhitespaceBands {
            min_gap_px,
            min_band_height_px,
            pad_px,
            start_question_no,
        } => {
            let mut next_q = (*start_question_no).max(1);
            for (page_no, page_buf) in &rendered_buffers {
                let bands = detect_horizontal_ink_bands(
                    page_buf,
                    *min_gap_px,
                    *min_band_height_px,
                    *pad_px,
                );
                for bbox_px in bands {
                    let uid = format!("{paper_id}-Q{next_q:03}");
                    let _ = UniversalId::parse(&uid)?;
                    next_q += 1;

                    let bbox_norm =
                        NormalizedBBox::from_pixel_bbox(bbox_px, page_buf.width, page_buf.height);
                    let cropped = page_buf.crop(bbox_px)?;
                    let png_bytes = cropped.encode_png()?;
                    let crop_abs = crops_dir.join(format!("{uid}.png"));
                    std::fs::write(&crop_abs, png_bytes)?;

                    let item = SlicedCropRecord {
                        universal_id: uid,
                        paper_id: paper_id.clone(),
                        subject: req.subject.clone(),
                        source_file: source_filename.clone(),
                        page: *page_no,
                        bbox: bbox_norm,
                        bbox_px,
                        crop_path: relative_to(&subject_dir, &crop_abs),
                        width_px: cropped.width,
                        height_px: cropped.height,
                    };
                    ledger.upsert_sliced_item(&item);
                    sliced_items.push(item);
                }
            }
        }
    }

    ledger.sync_with_workspace(cfg, None)?;
    ledger.save_atomic(&ledger_path)?;

    Ok(SlicePaperSummary {
        paper_id,
        archived_source_path: archived_source_abs,
        ledger_path,
        total_pdf_pages: total_pages,
        rendered_pages,
        sliced_items,
    })
}

fn render_pdf_pages(
    pdf_path: &Path,
    pages_dir: &Path,
    dpi: f32,
    page_range: Option<(u32, u32)>,
    explicit_pages: Option<&BTreeSet<u32>>,
) -> Result<(u32, Vec<(u32, RgbaBuffer)>)> {
    let raw_bytes = std::fs::read(pdf_path)?;
    let pdf = Pdf::new(raw_bytes).map_err(|e| {
        HydrogenError::Validation(format!("failed to parse PDF {}: {e:?}", pdf_path.display()))
    })?;

    let pages = pdf.pages();
    let total_pages = u32::try_from(pages.len()).unwrap_or(0);
    if total_pages == 0 {
        return Err(HydrogenError::Validation("PDF contains 0 pages".into()));
    }

    let scale = (dpi / 72.0).clamp(1.0, 5.0);
    let cache = RenderCache::new();
    let interp_settings = InterpreterSettings::default();
    let render_settings = RenderSettings::default();
    let pixmap_settings = PixmapSettings {
        x_scale: scale,
        y_scale: scale,
        bg_color: WHITE,
    };

    let mut out = Vec::new();
    for (idx, page) in pages.iter().enumerate() {
        let page_no = u32::try_from(idx + 1).unwrap_or(1);
        if let Some((start_p, end_p)) = page_range
            && (page_no < start_p || page_no > end_p)
        {
            continue;
        }
        if let Some(needed) = explicit_pages
            && !needed.contains(&page_no)
        {
            continue;
        }

        let page_png_path = pages_dir.join(format!("page-{page_no:03}.png"));

        // Reuse cached page PNG if already rendered on disk during explicit cropping
        if explicit_pages.is_some() && page_png_path.is_file() {
            let cached_bytes = std::fs::read(&page_png_path)?;
            if let Ok(decoded) = RgbaBuffer::decode_png(&cached_bytes) {
                out.push((page_no, decoded));
                continue;
            }
        }

        let pixmap = render(
            page,
            &cache,
            &interp_settings,
            &render_settings,
            &pixmap_settings,
        );
        let w = u32::from(pixmap.width());
        let h = u32::from(pixmap.height());
        let mut rgba = Vec::with_capacity((w as usize) * (h as usize) * 4);
        for px in pixmap.data() {
            if px.a == 255 || px.a == 0 {
                rgba.extend_from_slice(&[px.r, px.g, px.b, px.a]);
            } else {
                let unpremul = |c: u8| -> u8 {
                    ((u16::from(c) * 255 + u16::from(px.a / 2)) / u16::from(px.a)).min(255) as u8
                };
                rgba.extend_from_slice(&[unpremul(px.r), unpremul(px.g), unpremul(px.b), px.a]);
            }
        }

        let png_bytes = pixmap.into_png().map_err(|e| {
            HydrogenError::Validation(format!("PNG encode error on page {page_no}: {e}"))
        })?;
        std::fs::write(&page_png_path, png_bytes)?;

        out.push((
            page_no,
            RgbaBuffer {
                width: w,
                height: h,
                data: rgba,
            },
        ));
    }

    Ok((total_pages, out))
}

/// Detect horizontal ink bands separated by `>= min_gap_px` of whitespace.
pub fn detect_horizontal_ink_bands(
    buf: &RgbaBuffer,
    min_gap_px: u32,
    min_band_height_px: u32,
    pad_px: u32,
) -> Vec<PixelBBox> {
    if buf.width < 16 || buf.height < 16 {
        return Vec::new();
    }

    let x_margin = (buf.width / 25).max(4);
    let x_start = x_margin;
    let x_end = buf.width.saturating_sub(x_margin).max(x_start + 1);

    let y_margin = (buf.height / 35).max(4);
    let y_start = y_margin;
    let y_end = buf.height.saturating_sub(y_margin).max(y_start + 1);

    // Minimum dark pixels on a row to count as ink (filters single-pixel dust/scan noise)
    let min_ink_pixels = ((x_end - x_start) / 160).max(2);
    let stride = (buf.width as usize) * 4;

    let mut row_has_ink = vec![false; buf.height as usize];
    for y in y_start..y_end {
        let row_off = (y as usize) * stride;
        let mut dark_count = 0u32;
        for x in x_start..x_end {
            let p = row_off + (x as usize) * 4;
            if let Some(px) = buf.data.get(p..p + 4) {
                // Dark pixel on white paper (`alpha > 32` and luminance `< 236`)
                let lum =
                    (u32::from(px[0]) * 299 + u32::from(px[1]) * 587 + u32::from(px[2]) * 114)
                        / 1000;
                if px[3] > 32 && lum < 236 {
                    dark_count += 1;
                    if dark_count >= min_ink_pixels {
                        row_has_ink[y as usize] = true;
                        break;
                    }
                }
            }
        }
    }

    // Collect raw `[y0, y1)` ink spans and merge spans separated by `< min_gap_px`
    let mut merged: Vec<(u32, u32)> = Vec::new();
    let mut y = y_start;
    while y < y_end {
        if row_has_ink[y as usize] {
            let band_start = y;
            while y < y_end && row_has_ink[y as usize] {
                y += 1;
            }
            let band_end = y;
            if let Some(last) = merged.last_mut()
                && band_start.saturating_sub(last.1) < min_gap_px
            {
                last.1 = band_end;
            } else {
                merged.push((band_start, band_end));
            }
        } else {
            y += 1;
        }
    }

    let crop_x = x_margin.saturating_sub(pad_px);
    let crop_w = buf
        .width
        .saturating_sub(crop_x * 2)
        .max(1)
        .min(buf.width.saturating_sub(crop_x));

    merged
        .into_iter()
        .filter(|(y0, y1)| y1.saturating_sub(*y0) >= min_band_height_px)
        .map(|(y0, y1)| {
            let py0 = y0.saturating_sub(pad_px);
            let py1 = (y1 + pad_px).min(buf.height);
            PixelBBox {
                x: crop_x,
                y: py0,
                w: crop_w,
                h: py1.saturating_sub(py0).max(1),
            }
        })
        .collect()
}

fn next_sequential_paper_id(
    ledger: &ProductLedger,
    product: &str,
    subject: &str,
) -> Result<String> {
    let prod_code = UniversalId::product_prefix_for_slug(product);
    let subj_code = UniversalId::subject_prefix_for_slug(subject);
    let mut max_no = 0u32;
    for existing_id in ledger.papers.keys() {
        if let Ok(uid) = UniversalId::parse(existing_id)
            && uid.product_code == prod_code
            && uid.subject_code == subj_code
            && let Some(ContainerSegment::Paper {
                number,
                replica: None,
            }) = uid.container
            && number > max_no
        {
            max_no = number;
        }
    }
    Ok(UniversalId::paper(prod_code, subj_code, max_no + 1, None)?.to_string())
}

fn relative_to(base: &Path, target: &Path) -> String {
    target
        .strip_prefix(base)
        .unwrap_or(target)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_worksheet_png(width: u32, height: u32, bands: &[(u32, u32)]) -> Result<Vec<u8>> {
        let mut rgba = vec![255u8; (width as usize) * (height as usize) * 4];
        let stride = (width as usize) * 4;
        for &(y0, y1) in bands {
            for y in y0..y1.min(height) {
                for x in 40..(width - 40) {
                    let off = (y as usize) * stride + (x as usize) * 4;
                    rgba[off] = 25;
                    rgba[off + 1] = 30;
                    rgba[off + 2] = 35;
                    rgba[off + 3] = 255;
                }
            }
        }
        RgbaBuffer {
            width,
            height,
            data: rgba,
        }
        .encode_png()
    }

    #[test]
    fn slices_synthetic_png_auto_bands_and_explicit_crops_into_ledger() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!("hydrogen-slicer-test-{}", std::process::id()));
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

        let sample_png = synthetic_worksheet_png(800, 1200, &[(100, 220), (320, 460), (580, 760)])?;
        let src_path = cfg
            .subject_inbox_dir("gyaraplus", "maths")
            .join("raw-external-sheet.png");
        std::fs::write(&src_path, sample_png)?;

        // 1. Auto whitespace-band mode -> detects 3 bands (`GYR-MTH-P001-Q001..Q003`)
        let auto_summary = slice_paper(
            &cfg,
            &SlicePaperRequest {
                source_path: src_path.clone(),
                product: "gyaraplus".into(),
                subject: "maths".into(),
                paper_id: None,
                dpi: 180.0,
                page_range: None,
                mode: SliceMode::default(),
                move_source_to_archive: false,
            },
        )?;
        assert_eq!(auto_summary.paper_id, "GYR-MTH-P001");
        assert_eq!(auto_summary.sliced_items.len(), 3);
        assert_eq!(
            auto_summary.sliced_items[0].universal_id,
            "GYR-MTH-P001-Q001"
        );
        assert_eq!(
            auto_summary.sliced_items[2].universal_id,
            "GYR-MTH-P001-Q003"
        );

        // 2. Explicit normalized bbox crop mode -> slices exact diagram box
        let exp_summary = slice_paper(
            &cfg,
            &SlicePaperRequest {
                source_path: src_path,
                product: "gyaraplus".into(),
                subject: "maths".into(),
                paper_id: Some("GYR-MTH-P001".into()),
                dpi: 180.0,
                page_range: None,
                mode: SliceMode::ExplicitCrops(vec![CropSpec {
                    universal_id: "GYR-MTH-P001-Q014".into(),
                    page: 1,
                    bbox: CropBox::Normalized(NormalizedBBox {
                        x: 0.05,
                        y: 0.25,
                        w: 0.90,
                        h: 0.20,
                    }),
                }]),
                move_source_to_archive: false,
            },
        )?;
        assert_eq!(exp_summary.sliced_items.len(), 1);
        assert_eq!(exp_summary.sliced_items[0].width_px, 720);
        assert_eq!(exp_summary.sliced_items[0].height_px, 240);

        let ledger = ProductLedger::load_or_new(
            &cfg.ledger_path("gyaraplus", "maths"),
            "gyaraplus",
            "maths",
        )?;
        assert!(ledger.items.contains_key("GYR-MTH-P001-Q001"));
        assert!(ledger.items.contains_key("GYR-MTH-P001-Q014"));
        assert_eq!(
            ledger.items["GYR-MTH-P001-Q001"].crop_path.as_deref(),
            Some("archive/GYR-MTH-P001/crops/GYR-MTH-P001-Q001.png")
        );
        assert!(ledger.items["GYR-MTH-P001-Q001"].bbox.is_some());
        assert!(ledger.items["GYR-MTH-P001-Q001"].bbox_px.is_some());
        assert_eq!(ledger.papers["GYR-MTH-P001"].page_count, Some(1));

        std::fs::remove_dir_all(&tmp)?;
        Ok(())
    }
}
