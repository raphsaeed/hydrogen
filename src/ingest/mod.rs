//! Clean-room raw PDF/image ingestion, high-DPI page rendering, cropping, and `private-xref.json`.

pub mod slicer;
pub mod xref;

pub use slicer::{
    CropBox, CropSpec, DEFAULT_RENDER_DPI, RgbaBuffer, SliceMode, SlicePaperRequest,
    SlicePaperSummary, detect_horizontal_ink_bands, slice_paper,
};
pub use xref::{
    ItemXrefEntry, NormalizedBBox, PaperXrefEntry, PixelBBox, PrivateXrefStore, RenderedPageRecord,
    XREF_SCHEMA_VERSION,
};
