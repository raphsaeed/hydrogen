//! Clean-room raw PDF/image ingestion, high-DPI page rendering, cropping,
//! unified sub-product `ledger.json` (sources, crops, progress & replica lineage),
//! and Warm-Paper static `status.html` reporting.

pub mod ledger;
pub mod report;
pub mod slicer;

pub use ledger::{
    AuthoringTask, AuthoringTaskKind, AuthoringTaskStatus, ItemLedgerEntry, LEDGER_SCHEMA_VERSION,
    NormalizedBBox, PaperLedgerEntry, PixelBBox, ProductLedger, RenderedPageRecord, ReplicaScope,
    SlicedCropRecord, SlicedPaperRecord, SourceFileEntry, SourceStage, VaultSummary,
    derive_parent_paper_id, derive_question_replica_lineage, format_question_number_ranges,
    sha256_file, subject_slug_for_prefix, sync_and_save_subject_ledger,
};
pub use report::{
    OpenCommentEntry, ProductStatusReport, SideBySideQuestionRow, StatusRenderMode,
    SubjectStatusCard, TopicCoverageRow, build_subject_status_report, discover_active_products,
    discover_product_subjects, escape_html, format_terminal_status,
    refresh_subject_ledger_and_optional_html, render_status_html, render_status_html_with_mode,
    write_status_html,
};
pub use slicer::{
    CropBox, CropSpec, DEFAULT_RENDER_DPI, RgbaBuffer, SliceMode, SlicePaperRequest,
    SlicePaperSummary, detect_horizontal_ink_bands, slice_paper,
};
