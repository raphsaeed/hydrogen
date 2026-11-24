//! Fast `< 2 ms` multi-product & sub-product workspace roll-up scanner (`Phase 3A.2`).
//!
//! Reads cached `ledger.json` (`vaultSummary` + `sources` + `papers` + `tasks`) without opening
//! SQLCipher `.sqlite` vaults, honoring `memory.raphs` Conformance Rule A6 and keeping
//! Landing (`/`) and Product (`/products/{product}`) card grids instant.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::base64_util::read_png_dimensions;
use crate::carbon::catalog::mineral_color_for_subject;
use crate::config::HydrogenConfig;
use crate::domain::UniversalId;
use crate::error::Result;
use crate::ingest::{
    ProductLedger, SourceStage, discover_active_products, discover_product_subjects,
};

/// Fast summary of a single worked sub-product (`products/<product>/<subject>/`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubProductOverview {
    pub product: String,
    pub subject: String,
    pub title: String,
    pub course_id: String,
    pub vault_stem: String,
    pub bundle_filename: String,
    pub mineral_color: String,
    pub vault_initialized: bool,
    pub bundle_exists: bool,
    pub ledger_exists: bool,
    pub vault_summary_cached: bool,
    pub paper_count: usize,
    pub verified_question_count: usize,
    pub expected_question_count: usize,
    pub sliced_question_count: usize,
    pub missing_question_count: usize,
    pub replica_count: usize,
    pub replica_paper_count: usize,
    pub topic_count: usize,
    pub topic_card_count: usize,
    pub inbox_queued_count: usize,
    pub duplicate_source_count: usize,
    pub open_comment_count: usize,
    pub pending_task_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<u64>,
    pub status_url: String,
    pub api_url: String,
    pub sync_url: String,
}

/// Fast roll-up of an active product (`products/<product>/`) across its worked sub-products.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductOverview {
    pub product: String,
    pub title: String,
    pub product_code: String,
    pub has_brand_logo: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand_logo_url: Option<String>,
    pub product_url: String,
    pub api_url: String,
    pub elapsed_us: u128,
    pub active_subject_count: usize,
    pub paper_count: usize,
    pub verified_question_count: usize,
    pub expected_question_count: usize,
    pub sliced_question_count: usize,
    pub missing_question_count: usize,
    pub replica_count: usize,
    pub topic_card_count: usize,
    pub inbox_queued_count: usize,
    pub duplicate_source_count: usize,
    pub open_comment_count: usize,
    pub pending_task_count: usize,
    pub subjects: Vec<SubProductOverview>,
}

/// Top-level workspace roll-up across all active products (`>= 1` active sub-product).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOverview {
    pub service: String,
    pub version: String,
    pub elapsed_us: u128,
    pub has_hydrogen_logo: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hydrogen_logo_url: Option<String>,
    pub active_product_count: usize,
    pub active_subject_count: usize,
    pub paper_count: usize,
    pub verified_question_count: usize,
    pub expected_question_count: usize,
    pub sliced_question_count: usize,
    pub replica_count: usize,
    pub inbox_queued_count: usize,
    pub duplicate_source_count: usize,
    pub open_comment_count: usize,
    pub pending_task_count: usize,
    pub products: Vec<ProductOverview>,
}

/// Locate a product's primary PNG brand logo under `products/<product>/assets/` (strictly PNG, never SVG).
pub fn resolve_product_brand_logo_path(cfg: &HydrogenConfig, product: &str) -> Option<PathBuf> {
    let assets_dir = cfg.product_assets_dir(product);
    if !assets_dir.is_dir() {
        return None;
    }

    let preferred = [
        "gyara-blue.png".to_string(),
        format!("{product}-blue.png"),
        format!("{product}-logo.png"),
        "brand-logo.png".to_string(),
        "logo.png".to_string(),
    ];
    for candidate_name in &preferred {
        let p = assets_dir.join(candidate_name);
        if is_valid_png_file(&p) {
            return Some(p);
        }
    }

    let Ok(rd) = std::fs::read_dir(&assets_dir) else {
        return None;
    };
    let mut pngs: Vec<PathBuf> = rd
        .filter_map(|r| r.ok().map(|e| e.path()))
        .filter(|p| {
            let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
                return false;
            };
            !name.starts_with('.')
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("png"))
                && is_valid_png_file(p)
        })
        .collect();

    pngs.sort_by(|a, b| {
        let a_white = a
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|n| n.to_ascii_lowercase().contains("white"));
        let b_white = b
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|n| n.to_ascii_lowercase().contains("white"));
        a_white.cmp(&b_white).then_with(|| a.cmp(b))
    });
    pngs.into_iter().next()
}

fn is_valid_png_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; 32];
    use std::io::Read as _;
    let Ok(n) = file.read(&mut header) else {
        return false;
    };
    read_png_dimensions(&header[..n]).is_some()
}

/// Human-friendly display title for a product slug.
pub fn human_product_title(product: &str) -> String {
    match product {
        "gyaraplus" | "gyara" => "Gyara+ (11+)".to_string(),
        "gcse" => "GCSE".to_string(),
        "alevel" => "A-Level".to_string(),
        "primary" => "Primary (KS2)".to_string(),
        "infant" => "Infant (EYFS / KS1)".to_string(),
        other => title_case_slug(other),
    }
}

/// Human-friendly display title for a subject slug.
pub fn human_subject_title(subject: &str) -> String {
    match subject {
        "maths" => "Maths".to_string(),
        "english" => "English".to_string(),
        "verbal-reasoning" => "Verbal Reasoning".to_string(),
        "non-verbal-reasoning" => "Non-Verbal Reasoning".to_string(),
        "physics" => "Physics".to_string(),
        "chemistry" => "Chemistry".to_string(),
        "biology" => "Biology".to_string(),
        "computer-science" => "Computer Science".to_string(),
        "further-maths" => "Further Maths".to_string(),
        other => title_case_slug(other),
    }
}

fn title_case_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => {
                    let mut s = first.to_uppercase().collect::<String>();
                    s.push_str(chars.as_str());
                    s
                }
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn count_raw_inbox_files_fast(dir: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0usize;
    for entry in rd.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            count = count.saturating_add(count_raw_inbox_files_fast(&p));
        } else if p.is_file() {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if matches!(
                ext.as_str(),
                "pdf" | "png" | "jpg" | "jpeg" | "webp" | "svg"
            ) {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

/// Scan a single worked sub-product strictly from `ledger.json` and lightweight file checks (`< 0.2 ms`).
pub fn scan_subproduct_overview(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    fallback_color_idx: usize,
) -> Result<SubProductOverview> {
    let prod_prefix = UniversalId::product_prefix_for_slug(product);
    let subj_prefix = UniversalId::subject_prefix_for_slug(subject);
    let fallback_course_id = format!("{prod_prefix}-{subj_prefix}");
    let mineral_color = mineral_color_for_subject(subj_prefix, fallback_color_idx).to_string();

    let vault_stem = HydrogenConfig::vault_stem(product, subject);
    let bundle_filename = format!("{vault_stem}.raphs");
    let sqlite_exists = cfg.vault_sqlite_path(product, subject).is_file();
    let bundle_exists = cfg.vault_bundle_path(product, subject).is_file();
    let ledger_path = cfg.ledger_path(product, subject);
    let ledger_exists = ledger_path.is_file();

    let status_url = format!("/products/{product}/{subject}");
    let api_url = format!("/api/products/{product}/{subject}");
    let sync_url = format!("/api/products/{product}/{subject}/sync");

    if !ledger_exists {
        let inbox_queued_count =
            count_raw_inbox_files_fast(&cfg.subject_inbox_dir(product, subject));
        return Ok(SubProductOverview {
            product: product.to_string(),
            subject: subject.to_string(),
            title: human_subject_title(subject),
            course_id: fallback_course_id,
            vault_stem,
            bundle_filename,
            mineral_color,
            vault_initialized: sqlite_exists || bundle_exists,
            bundle_exists,
            ledger_exists: false,
            vault_summary_cached: false,
            paper_count: 0,
            verified_question_count: 0,
            expected_question_count: 0,
            sliced_question_count: 0,
            missing_question_count: 0,
            replica_count: 0,
            replica_paper_count: 0,
            topic_count: 0,
            topic_card_count: 0,
            inbox_queued_count,
            duplicate_source_count: 0,
            open_comment_count: 0,
            pending_task_count: 0,
            updated_at_ms: None,
            status_url,
            api_url,
            sync_url,
        });
    }

    let ledger = ProductLedger::load_or_new(&ledger_path, product, subject)?;
    let vs = ledger.vault_summary.as_ref();

    let base_verified_from_papers: usize = ledger
        .papers
        .values()
        .filter(|p| !p.is_replica)
        .map(|p| usize::try_from(p.verified_count).unwrap_or(0))
        .sum();
    let verified_question_count = vs
        .map(|v| v.verified_question_count)
        .unwrap_or(base_verified_from_papers);

    let base_expected_from_papers: usize = ledger
        .papers
        .values()
        .filter(|p| !p.is_replica)
        .map(|p| {
            let denom = p
                .expected_questions
                .unwrap_or_else(|| p.sliced_count.max(p.verified_count));
            usize::try_from(denom).unwrap_or(0)
        })
        .sum();
    let expected_question_count = if base_expected_from_papers > 0 {
        base_expected_from_papers
    } else {
        verified_question_count
    };

    let sliced_question_count: usize = ledger
        .papers
        .values()
        .map(|p| usize::try_from(p.sliced_count).unwrap_or(0))
        .sum();
    let missing_question_count: usize = ledger
        .papers
        .values()
        .filter(|p| !p.is_replica)
        .map(|p| p.missing_question_numbers.len())
        .sum();

    let replica_items_in_ledger = ledger.items.values().filter(|i| i.is_replica).count();
    let replica_count = vs
        .map(|v| v.replica_count)
        .unwrap_or(replica_items_in_ledger);
    let replica_paper_count = ledger.papers.values().filter(|p| p.is_replica).count();

    let paper_count = vs
        .map(|v| v.paper_count)
        .unwrap_or(0)
        .max(ledger.papers.len());
    let topic_count = vs.map(|v| v.topic_count).unwrap_or(0);
    let topic_card_count = vs.map(|v| v.topic_card_count).unwrap_or(0);
    let open_comment_count = vs.map(|v| v.open_comment_count).unwrap_or(0);

    let inbox_queued_count = ledger
        .sources
        .values()
        .filter(|s| s.stage == SourceStage::Queued)
        .count();
    let duplicate_source_count: usize = ledger
        .sources
        .values()
        .map(|s| s.duplicate_paths.len())
        .sum();
    let pending_task_count = ledger.pending_task_count();

    let course_id = vs
        .map(|v| v.course_id.clone())
        .unwrap_or(fallback_course_id);
    let title = vs
        .map(|v| v.title.clone())
        .unwrap_or_else(|| human_subject_title(subject));
    let vault_initialized = vs
        .map(|v| v.vault_initialized)
        .unwrap_or(sqlite_exists || bundle_exists);

    Ok(SubProductOverview {
        product: product.to_string(),
        subject: subject.to_string(),
        title,
        course_id,
        vault_stem,
        bundle_filename,
        mineral_color,
        vault_initialized,
        bundle_exists,
        ledger_exists: true,
        vault_summary_cached: vs.is_some(),
        paper_count,
        verified_question_count,
        expected_question_count,
        sliced_question_count,
        missing_question_count,
        replica_count,
        replica_paper_count,
        topic_count,
        topic_card_count,
        inbox_queued_count,
        duplicate_source_count,
        open_comment_count,
        pending_task_count,
        updated_at_ms: Some(ledger.updated_at_ms),
        status_url,
        api_url,
        sync_url,
    })
}

/// Scan an active product (`products/<product>/`) across its worked sub-products (`None` if 0 active sub-products).
pub fn scan_product_overview(
    cfg: &HydrogenConfig,
    product: &str,
) -> Result<Option<ProductOverview>> {
    let t0 = Instant::now();
    let active_subjects = discover_product_subjects(cfg, product)?;
    if active_subjects.is_empty() {
        return Ok(None);
    }

    let mut subjects = Vec::with_capacity(active_subjects.len());
    for (idx, subj) in active_subjects.iter().enumerate() {
        subjects.push(scan_subproduct_overview(cfg, product, subj, idx)?);
    }

    let has_brand_logo = resolve_product_brand_logo_path(cfg, product).is_some();
    let brand_logo_url = if has_brand_logo {
        Some(format!("/products/{product}/brand-logo"))
    } else {
        None
    };

    let paper_count = subjects.iter().map(|s| s.paper_count).sum();
    let verified_question_count = subjects.iter().map(|s| s.verified_question_count).sum();
    let expected_question_count = subjects.iter().map(|s| s.expected_question_count).sum();
    let sliced_question_count = subjects.iter().map(|s| s.sliced_question_count).sum();
    let missing_question_count = subjects.iter().map(|s| s.missing_question_count).sum();
    let replica_count = subjects.iter().map(|s| s.replica_count).sum();
    let topic_card_count = subjects.iter().map(|s| s.topic_card_count).sum();
    let inbox_queued_count = subjects.iter().map(|s| s.inbox_queued_count).sum();
    let duplicate_source_count = subjects.iter().map(|s| s.duplicate_source_count).sum();
    let open_comment_count = subjects.iter().map(|s| s.open_comment_count).sum();
    let pending_task_count = subjects.iter().map(|s| s.pending_task_count).sum();

    Ok(Some(ProductOverview {
        product: product.to_string(),
        title: human_product_title(product),
        product_code: UniversalId::product_prefix_for_slug(product).to_string(),
        has_brand_logo,
        brand_logo_url,
        product_url: format!("/products/{product}"),
        api_url: format!("/api/products/{product}"),
        elapsed_us: t0.elapsed().as_micros(),
        active_subject_count: subjects.len(),
        paper_count,
        verified_question_count,
        expected_question_count,
        sliced_question_count,
        missing_question_count,
        replica_count,
        topic_card_count,
        inbox_queued_count,
        duplicate_source_count,
        open_comment_count,
        pending_task_count,
        subjects,
    }))
}

/// Scan all active products across the workspace (`< 2 ms`, hiding products with `0` active sub-products).
pub fn scan_workspace_overview(cfg: &HydrogenConfig) -> Result<WorkspaceOverview> {
    let t0 = Instant::now();
    let active_product_slugs = discover_active_products(cfg)?;
    let mut products = Vec::with_capacity(active_product_slugs.len());
    for prod_slug in &active_product_slugs {
        if let Some(prod_overview) = scan_product_overview(cfg, prod_slug)? {
            products.push(prod_overview);
        }
    }

    let h_logo_path = cfg.hydrogen_logo_path();
    let has_hydrogen_logo = is_valid_png_file(&h_logo_path);
    let hydrogen_logo_url = if has_hydrogen_logo {
        Some("/assets/hydrogen-logo.png".to_string())
    } else {
        None
    };

    let active_product_count = products.len();
    let active_subject_count = products.iter().map(|p| p.active_subject_count).sum();
    let paper_count = products.iter().map(|p| p.paper_count).sum();
    let verified_question_count = products.iter().map(|p| p.verified_question_count).sum();
    let expected_question_count = products.iter().map(|p| p.expected_question_count).sum();
    let sliced_question_count = products.iter().map(|p| p.sliced_question_count).sum();
    let replica_count = products.iter().map(|p| p.replica_count).sum();
    let inbox_queued_count = products.iter().map(|p| p.inbox_queued_count).sum();
    let duplicate_source_count = products.iter().map(|p| p.duplicate_source_count).sum();
    let open_comment_count = products.iter().map(|p| p.open_comment_count).sum();
    let pending_task_count = products.iter().map(|p| p.pending_task_count).sum();

    Ok(WorkspaceOverview {
        service: "hydrogen".to_string(),
        version: crate::VERSION.to_string(),
        elapsed_us: t0.elapsed().as_micros(),
        has_hydrogen_logo,
        hydrogen_logo_url,
        active_product_count,
        active_subject_count,
        paper_count,
        verified_question_count,
        expected_question_count,
        sliced_question_count,
        replica_count,
        inbox_queued_count,
        duplicate_source_count,
        open_comment_count,
        pending_task_count,
        products,
    })
}
