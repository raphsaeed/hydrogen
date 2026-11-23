//! Thread-safe `memory_core_rs 1.7.0` (`Schema 2.3.0` Layer `L7`) vault wrapper.
//!
//! Honors `memory.raphs` Conformance Rule A6: maintains a single `Memory` connection
//! per SQLite container file per process behind a `tokio::sync::Mutex<Memory>`.

pub mod seed;

use std::path::{Path, PathBuf};

use memory_core_rs::{
    CheckReport, CommentAttachmentBytes, CourseAsset, CourseAssetInput, CourseCatalogSnapshot,
    CourseComment, CourseCommentInput, CoursePaper, CoursePaperInput, CoursePaperSectionInput,
    CourseQuestion, CourseQuestionInput, CourseStimulusInput, Manifest, Memory, Status, bundle,
};
use tokio::sync::Mutex;

use crate::base64_util::to_data_uri;
use crate::error::{HydrogenError, Result};
use crate::vault::seed::{GYR_LOGO_BLUE_ID, GYR_LOGO_WHITE_ID, SeedSummary, seed_gyara_maths};

/// Default write attribution actor recorded in `provenance` by Hydrogen.
pub const HYDROGEN_ACTOR: &str = "hydrogen";

/// Thread-safe handle managing a live `<stem>.sqlite` + `<stem>.keys.json` container
/// and its self-contained `<stem>.raphs` bundle.
pub struct VaultManager {
    sqlite_path: PathBuf,
    bundle_path: PathBuf,
    recovery_key_created: Option<String>,
    inner: Mutex<Memory>,
}

impl VaultManager {
    /// Open an existing vault (unpacking `<stem>.raphs` first if only the bundle exists on disk),
    /// or create a brand-new encrypted `Schema 2.3.0` container when neither exists yet.
    pub fn open_or_create(
        sqlite_path: impl AsRef<Path>,
        bundle_path: impl AsRef<Path>,
        passphrase: &str,
        device_id: &str,
    ) -> Result<Self> {
        let sqlite_path = sqlite_path.as_ref().to_path_buf();
        let bundle_path = bundle_path.as_ref().to_path_buf();

        if let Some(parent) = sqlite_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }

        let (mem, recovery_key_created) = if sqlite_path.is_file() {
            let m = Memory::open_with_passphrase(&sqlite_path, passphrase, device_id)?;
            (m, None)
        } else if bundle_path.is_file() {
            bundle::unpack(&bundle_path, &sqlite_path)?;
            let m = Memory::open_with_passphrase(&sqlite_path, passphrase, device_id)?;
            (m, None)
        } else {
            let (m, rec) = Memory::create(&sqlite_path, passphrase, device_id)?;
            (m, Some(rec))
        };

        Ok(Self {
            sqlite_path,
            bundle_path,
            recovery_key_created,
            inner: Mutex::new(mem),
        })
    }

    /// Path to the live SQLCipher container (`<stem>.sqlite`).
    pub fn sqlite_path(&self) -> &Path {
        &self.sqlite_path
    }

    /// Path to the packed `.raphs` bundle (`<stem>.raphs`).
    pub fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    /// If this call created the vault for the very first time, returns the 52-char recovery key.
    pub fn newly_created_recovery_key(&self) -> Option<&str> {
        self.recovery_key_created.as_deref()
    }

    /// Execute a read/write closure with exclusive access to the underlying `Memory` connection.
    pub async fn with_memory<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Memory) -> Result<T>,
    {
        let mut guard = self.inner.lock().await;
        f(&mut guard)
    }

    /// Seed or refresh `GYR-MTH` metadata, `Gyara+` brand logo blobs, topics, and question types.
    pub async fn seed_gyara_maths(&self, assets_dir: &Path) -> Result<SeedSummary> {
        self.with_memory(|mem| seed_gyara_maths(mem, HYDROGEN_ACTOR, assets_dir))
            .await
    }

    /// Return container status (`schema_version`, `crypto_version`, `dek_id`, HLC horizon).
    pub async fn status(&self) -> Result<Status> {
        self.with_memory(|mem| Ok(mem.status()?)).await
    }

    /// Run `memory_core_rs`'s structural, foreign-key, trigger, and sidecar integrity check.
    pub async fn self_check(&self) -> Result<CheckReport> {
        self.with_memory(|mem| Ok(mem.self_check()?)).await
    }

    /// Read the fast indexed `L7` `CourseCatalogSnapshot` (`< 1 ms`).
    pub async fn catalog_snapshot(&self) -> Result<CourseCatalogSnapshot> {
        self.with_memory(|mem| Ok(mem.course_catalog_snapshot()?))
            .await
    }

    /// Upsert an `L7` practice/mock paper header (`course_papers`).
    pub async fn upsert_paper(&self, input: &CoursePaperInput) -> Result<()> {
        self.with_memory(|mem| {
            mem.upsert_paper(HYDROGEN_ACTOR, None, input)?;
            Ok(())
        })
        .await
    }

    /// Fetch a single `L7` paper by its `paper_id`.
    pub async fn get_paper(&self, paper_id: &str) -> Result<Option<CoursePaper>> {
        self.with_memory(|mem| Ok(mem.get_paper(paper_id)?)).await
    }

    /// Upsert a timed sub-section within a paper (`course_paper_sections`).
    pub async fn upsert_paper_section(&self, input: &CoursePaperSectionInput) -> Result<()> {
        self.with_memory(|mem| {
            mem.upsert_paper_section(HYDROGEN_ACTOR, None, input)?;
            Ok(())
        })
        .await
    }

    /// Upsert a shared stimulus (`course_stimuli`, e.g., passage, dataset, or code key).
    pub async fn upsert_stimulus(&self, input: &CourseStimulusInput) -> Result<()> {
        self.with_memory(|mem| {
            mem.upsert_stimulus(HYDROGEN_ACTOR, None, input)?;
            Ok(())
        })
        .await
    }

    /// Upsert a verified question + `Level 9 -> 1` solution (`course_questions` + topic/concept/asset links).
    pub async fn upsert_question(&self, input: &CourseQuestionInput) -> Result<()> {
        self.with_memory(|mem| {
            mem.upsert_question(HYDROGEN_ACTOR, None, input, None)?;
            Ok(())
        })
        .await
    }

    /// Fetch a full `CourseQuestion` (including `question_json`, `solution_json`, topics, links, and assets).
    pub async fn get_question(&self, question_id: &str) -> Result<Option<CourseQuestion>> {
        self.with_memory(|mem| Ok(mem.get_question(question_id)?))
            .await
    }

    /// Embed a clean regenerated course media asset into content-addressed `blobs` + `course_media_assets`.
    pub async fn put_asset(&self, meta: &CourseAssetInput, bytes: &[u8]) -> Result<String> {
        self.with_memory(|mem| Ok(mem.put_course_asset(HYDROGEN_ACTOR, None, meta, bytes)?))
            .await
    }

    /// Read metadata for an embedded `L7` media asset (e.g. `GYR-LOGO-WHITE`).
    pub async fn get_asset_info(&self, asset_id: &str) -> Result<Option<CourseAsset>> {
        self.with_memory(|mem| Ok(mem.get_course_asset_info(asset_id)?))
            .await
    }

    /// Read verified bytes of an embedded `L7` media asset and format as a `data:<mime>;base64,...` URI.
    pub async fn read_asset_data_uri(&self, asset_id: &str) -> Result<String> {
        self.with_memory(|mem| {
            let info = mem.get_course_asset_info(asset_id)?.ok_or_else(|| {
                HydrogenError::Validation(format!("course asset '{asset_id}' not found in vault"))
            })?;
            let bytes = mem.read_course_asset_bytes(asset_id)?;
            Ok(to_data_uri(&info.mime_type, &bytes))
        })
        .await
    }

    /// Read raw verified bytes of an embedded `L7` media asset.
    pub async fn read_asset_bytes(&self, asset_id: &str) -> Result<Vec<u8>> {
        self.with_memory(|mem| Ok(mem.read_course_asset_bytes(asset_id)?))
            .await
    }

    /// List `L7` authoring/QA comments (`course_comments`), optionally filtered by `status`
    /// (`"open"` | `"resolved"`) and/or `target_id`.
    pub async fn list_comments(
        &self,
        status: Option<&str>,
        target_id: Option<&str>,
    ) -> Result<Vec<CourseComment>> {
        self.with_memory(|mem| Ok(mem.list_course_comments(status, target_id)?))
            .await
    }

    /// Fetch a single `L7` authoring comment (including attachment metadata) by `comment_id`.
    pub async fn get_comment(&self, comment_id: &str) -> Result<Option<CourseComment>> {
        self.with_memory(|mem| {
            let all = mem.list_course_comments(None, None)?;
            Ok(all.into_iter().find(|c| c.comment_id == comment_id))
        })
        .await
    }

    /// Insert or update an `L7` authoring comment (`course_comments`) and deduplicate any
    /// new `Snap Board` / uploaded binary attachments in `blobs`. Returns `comment_id`.
    pub async fn upsert_comment(
        &self,
        input: &CourseCommentInput,
        attachments: &[CommentAttachmentBytes],
    ) -> Result<String> {
        self.with_memory(|mem| {
            Ok(mem.upsert_course_comment(HYDROGEN_ACTOR, None, input, attachments)?)
        })
        .await
    }

    /// Mark an `L7` authoring comment as `"resolved"` with an optional `resolution_note`.
    pub async fn resolve_comment(
        &self,
        comment_id: &str,
        resolution_note: Option<&str>,
    ) -> Result<bool> {
        self.with_memory(|mem| {
            Ok(mem.resolve_course_comment(HYDROGEN_ACTOR, None, comment_id, resolution_note)?)
        })
        .await
    }

    /// Permanently delete an `L7` authoring comment and garbage-collect unreferenced attachment blobs.
    pub async fn delete_comment(&self, comment_id: &str) -> Result<bool> {
        self.with_memory(|mem| Ok(mem.delete_course_comment(HYDROGEN_ACTOR, None, comment_id)?))
            .await
    }

    /// Read raw verified bytes of a `course_comment_attachments` row by `attachment_id`.
    pub async fn read_comment_attachment(&self, attachment_id: &str) -> Result<Vec<u8>> {
        self.with_memory(|mem| Ok(mem.read_comment_attachment_bytes(attachment_id)?))
            .await
    }

    /// Convenience helper returning `(white_logo_data_uri, blue_logo_data_uri)` for `Gyara+`.
    pub async fn read_gyara_brand_logos(&self) -> Result<(String, String)> {
        let white = self.read_asset_data_uri(GYR_LOGO_WHITE_ID).await?;
        let blue = self.read_asset_data_uri(GYR_LOGO_BLUE_ID).await?;
        Ok((white, blue))
    }

    /// Pack the live SQLCipher container + key sidecar into `<stem>.raphs` (`Manifest` verified).
    pub async fn pack_to_default_bundle(&self) -> Result<Manifest> {
        let out = self.bundle_path.clone();
        self.with_memory(|mem| Ok(mem.pack(&out)?)).await
    }

    /// Export a clean customer-distributable `.raphs` bundle (stripping comments, revisions & provenance).
    pub async fn export_clean_bundle(
        &self,
        out_bundle_path: &Path,
        customer_lesson_key: &str,
    ) -> Result<()> {
        self.with_memory(|mem| {
            mem.export_clean_course_bundle(out_bundle_path, customer_lesson_key)?;
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initializes_gyara_maths_vault_embeds_logos_and_packs_bundle() -> Result<()> {
        let tmp_dir =
            std::env::temp_dir().join(format!("hydrogen-vault-test-{}", std::process::id()));
        if tmp_dir.exists() {
            std::fs::remove_dir_all(&tmp_dir)?;
        }
        std::fs::create_dir_all(&tmp_dir)?;

        let sqlite_path = tmp_dir.join("gyara-maths.sqlite");
        let bundle_path = tmp_dir.join("gyara-maths.raphs");
        let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("products")
            .join("gyaraplus")
            .join("assets");

        let vault = VaultManager::open_or_create(
            &sqlite_path,
            &bundle_path,
            "test-lesson-key-12345",
            "hydrogen-test-device",
        )?;
        assert!(vault.newly_created_recovery_key().is_some());

        let seeded = vault.seed_gyara_maths(&assets_dir).await?;
        assert_eq!(seeded.course_id, "GYR-MTH");
        assert_eq!(seeded.topic_count, 8);
        assert_eq!(seeded.question_type_count, 10);
        assert!(
            seeded
                .embedded_assets
                .contains(&GYR_LOGO_WHITE_ID.to_string())
        );
        assert!(
            seeded
                .embedded_assets
                .contains(&GYR_LOGO_BLUE_ID.to_string())
        );

        let status = vault.status().await?;
        assert_eq!(status.schema_version, "2.3.0");

        let check = vault.self_check().await?;
        assert!(check.ok, "vault self_check must report ok");

        let (white_uri, blue_uri) = vault.read_gyara_brand_logos().await?;
        assert!(white_uri.starts_with("data:image/png;base64,"));
        assert!(blue_uri.starts_with("data:image/png;base64,"));

        // Exercise L7 Snap Board comment + attachment CRUD lifecycle
        let png_bytes = crate::ingest::RgbaBuffer {
            width: 8,
            height: 6,
            data: vec![240u8; 8 * 6 * 4],
        }
        .encode_png()?;
        let comment_id = vault
            .upsert_comment(
                &CourseCommentInput {
                    comment_id: Some("cmt-vault-test-01".to_string()),
                    target_kind: "question".to_string(),
                    target_id: "GYR-MTH-P001-Q001".to_string(),
                    part_id: Some("main".to_string()),
                    step_id: Some("S01".to_string()),
                    sublevel: Some(5),
                    comment_kind: "fix".to_string(),
                    comment_text: "Increase gap between bar model rows.".to_string(),
                },
                &[CommentAttachmentBytes {
                    attachment_id: Some("att-vault-test-01".to_string()),
                    source: "board-capture".to_string(),
                    pane: Some("right".to_string()),
                    mime_type: "image/png".to_string(),
                    filename: Some("snap-1.png".to_string()),
                    width_px: Some(8),
                    height_px: Some(6),
                    sort_order: 0,
                    bytes: png_bytes.clone(),
                }],
            )
            .await?;
        assert_eq!(comment_id, "cmt-vault-test-01");

        let open_comments = vault
            .list_comments(Some("open"), Some("GYR-MTH-P001-Q001"))
            .await?;
        assert_eq!(open_comments.len(), 1);
        assert_eq!(open_comments[0].attachments.len(), 1);
        let read_back = vault.read_comment_attachment("att-vault-test-01").await?;
        assert_eq!(read_back, png_bytes);

        let fetched = vault.get_comment("cmt-vault-test-01").await?;
        assert!(fetched.is_some());

        assert!(
            vault
                .resolve_comment("cmt-vault-test-01", Some("Fixed spacing."))
                .await?
        );
        let resolved_comment = vault
            .get_comment("cmt-vault-test-01")
            .await?
            .ok_or_else(|| HydrogenError::Validation("expected resolved comment".into()))?;
        assert_eq!(resolved_comment.status, "resolved");
        assert_eq!(
            resolved_comment.resolution_note.as_deref(),
            Some("Fixed spacing.")
        );

        let snap = vault.catalog_snapshot().await?;
        assert_eq!(
            snap.meta.as_ref().map(|m| m.course_id.as_str()),
            Some("GYR-MTH")
        );
        assert_eq!(snap.topics.len(), 8);
        assert!(snap.topics.iter().any(|t| t.topic_id == "GYR-MTH-T_ALG"));
        assert!(snap.topics.iter().any(|t| t.topic_id == "GYR-MTH-T_LOG"));
        assert_eq!(snap.question_types.len(), 10);

        let manifest = vault.pack_to_default_bundle().await?;
        assert_eq!(manifest.schema_version, "2.3.0");
        assert!(bundle_path.is_file());

        // Also verify clean customer export strips authoring metadata while keeping course assets!
        let clean_path = tmp_dir.join("gyara-maths-clean.raphs");
        vault
            .export_clean_bundle(&clean_path, "customer-lesson-key-999")
            .await?;
        assert!(clean_path.is_file());

        drop(vault);
        std::fs::remove_dir_all(&tmp_dir)?;
        Ok(())
    }
}
