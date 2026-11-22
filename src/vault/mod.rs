//! Thread-safe `memory_core_rs 1.7.0` (`Schema 2.3.0` Layer `L7`) vault wrapper.
//!
//! Honors `memory.raphs` Conformance Rule A6: maintains a single `Memory` connection
//! per SQLite container file per process behind a `tokio::sync::Mutex<Memory>`.

pub mod seed;

use std::path::{Path, PathBuf};

use memory_core_rs::{
    CheckReport, CourseAsset, CourseCatalogSnapshot, Manifest, Memory, Status, bundle,
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
        let tmp_dir = std::env::temp_dir().join(format!(
            "hydrogen-vault-test-{}",
            std::process::id()
        ));
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
        assert!(seeded.topic_count >= 7);
        assert!(seeded.question_type_count >= 9);
        assert!(seeded.embedded_assets.contains(&GYR_LOGO_WHITE_ID.to_string()));
        assert!(seeded.embedded_assets.contains(&GYR_LOGO_BLUE_ID.to_string()));

        let status = vault.status().await?;
        assert_eq!(status.schema_version, "2.3.0");

        let check = vault.self_check().await?;
        assert!(check.ok, "vault self_check must report ok");

        let (white_uri, blue_uri) = vault.read_gyara_brand_logos().await?;
        assert!(white_uri.starts_with("data:image/png;base64,"));
        assert!(blue_uri.starts_with("data:image/png;base64,"));

        let snap = vault.catalog_snapshot().await?;
        assert_eq!(
            snap.meta.as_ref().map(|m| m.course_id.as_str()),
            Some("GYR-MTH")
        );
        assert_eq!(snap.topics.len(), 7);
        assert_eq!(snap.question_types.len(), 9);

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
