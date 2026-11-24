//! Carbon Warm-Paper Multi-Product & Sub-Product Status Server (`hydrogen serve` via `axum` on `:7430`).
//!
//! **Governing Engineering Laws (`docs/build-plan.md` Phase 3A & `docs/sop.md`):**
//! 1. **Ports & LAN Binding:** `:7420` is Carbon; `:7430` is Hydrogen (`127.0.0.1:7430` default, `--lan` -> `0.0.0.0:7430`).
//! 2. **Active-Only Discovery:** Products with `0` active sub-products (e.g. unstarted `gcse`) and unworked subjects are hidden.
//! 3. **Vault Conformance Rule A6:** At most one `Memory` handle per `.sqlite` file per process (enforced via `ServerState::vault_lock`),
//!    and Landing (`/`) + Product (`/products/{product}`) pages read cached `ledger.json` (`vaultSummary`) in `< 2 ms` without opening SQLCipher.
//! 4. **Clean-Room & Secret-File Sandbox:** `/products/{product}/{subject}/files/{*rel_path}` serves ONLY read-only files inside
//!    `inbox/` and `archive/` with an explicit extension whitelist, rejecting `..` traversal, symlinks outside the allowed subtree,
//!    hidden dotfiles, `ledger.json`, `status.html`, and `.sqlite` / `.keys.json` / `.raphs` files (`403` / `404`).

pub mod routes;
pub mod scanner;
pub mod views;

use std::collections::BTreeMap;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::config::HydrogenConfig;
use crate::error::{HydrogenError, Result};

pub use routes::{SyncVaultResponse, build_router};
pub use scanner::{
    ProductOverview, SubProductOverview, WorkspaceOverview, resolve_product_brand_logo_path,
    scan_product_overview, scan_subproduct_overview, scan_workspace_overview,
};
pub use views::{render_error_html, render_product_explorer_html, render_workspace_landing_html};

/// Default Hydrogen Status Server port (`:7430`, paired alongside Carbon on `:7420`).
pub const DEFAULT_SERVE_PORT: u16 = 7430;

/// Shared state across all `axum` handlers in `hydrogen serve`.
pub struct ServerState {
    cfg: HydrogenConfig,
    /// Per-vault SQLite path lock table enforcing `memory.raphs` Conformance Rule A6:
    /// guarantees at most a single `Memory` connection per `.sqlite` file in this process.
    vault_locks: Mutex<BTreeMap<PathBuf, Arc<Mutex<()>>>>,
}

impl ServerState {
    /// Construct shared server state from a resolved `HydrogenConfig`.
    pub fn new(cfg: HydrogenConfig) -> Self {
        Self {
            cfg,
            vault_locks: Mutex::new(BTreeMap::new()),
        }
    }

    /// Borrow the underlying `HydrogenConfig`.
    pub fn config(&self) -> &HydrogenConfig {
        &self.cfg
    }

    /// Return the process-wide `Arc<Mutex<()>>` serializing all SQLCipher `Memory` opens
    /// for a specific `<stem>.sqlite` path (Conformance Rule A6).
    pub async fn vault_lock(&self, sqlite_path: &Path) -> Arc<Mutex<()>> {
        let key = std::fs::canonicalize(sqlite_path).unwrap_or_else(|_| sqlite_path.to_path_buf());
        let mut map = self.vault_locks.lock().await;
        Arc::clone(map.entry(key).or_insert_with(|| Arc::new(Mutex::new(()))))
    }
}

/// Classification of sandboxed file-access rejections for `GET /products/{product}/{subject}/files/{*rel_path}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAccessError {
    /// Traversal, secret file (`.sqlite`, `.keys.json`, `.raphs`), hidden dotfile, disallowed subtree/extension, or escaping symlink (`403`).
    Forbidden(String),
    /// Valid sandboxed path inside `inbox/` or `archive/` with an allowed extension, but the file does not exist (`404`).
    NotFound,
}

/// Validate a URL product slug (`[a-zA-Z0-9_-]`, `1..=64` chars, no `.`, `/`, `\`, or hidden prefix).
pub fn validate_product_slug(slug: &str) -> std::result::Result<(), FileAccessError> {
    if slug.is_empty() || slug.len() > 64 {
        return Err(FileAccessError::Forbidden(
            "invalid product slug length".into(),
        ));
    }
    if slug.starts_with('.') || slug.contains("..") {
        return Err(FileAccessError::Forbidden(
            "hidden or traversal product slug is forbidden".into(),
        ));
    }
    if !slug
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(FileAccessError::Forbidden(
            "product slug contains forbidden characters".into(),
        ));
    }
    Ok(())
}

/// Validate a URL sub-product subject slug (rejecting `"assets"`, `.`, `..`, `/`, `\`, and secret extensions).
pub fn validate_subject_slug(slug: &str) -> std::result::Result<(), FileAccessError> {
    validate_product_slug(slug)?;
    if slug.eq_ignore_ascii_case("assets") {
        return Err(FileAccessError::Forbidden(
            "'assets' is not a sub-product directory".into(),
        ));
    }
    Ok(())
}

/// Map allowed read-only `inbox/` and `archive/` file extensions to HTTP `Content-Type` headers.
pub fn content_type_for_allowed_extension(ext_lower: &str) -> Option<&'static str> {
    match ext_lower {
        "pdf" => Some("application/pdf"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "json" => Some("application/json; charset=utf-8"),
        "mp4" => Some("video/mp4"),
        "mp3" => Some("audio/mpeg"),
        _ => None,
    }
}

fn is_forbidden_secret_filename(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    lower.starts_with('.')
        || lower == "ledger.json"
        || lower == "status.html"
        || lower == "private-xref.json"
        || lower.ends_with(".keys.json")
        || lower.ends_with(".sqlite")
        || lower.ends_with(".sqlite-wal")
        || lower.ends_with(".sqlite-shm")
        || lower.ends_with(".sqlite-journal")
        || lower.ends_with(".raphs")
        || lower.ends_with(".env")
        || lower.ends_with(".tmp")
}

/// Strictly validate and resolve a read-only file inside `products/<product>/<subject>/{inbox,archive}/**`.
///
/// Enforces:
/// 1. Valid `product` & `subject` slugs,
/// 2. Zero `\`, NUL, `.` / `..` traversal segments, or hidden `.` segments anywhere in `raw_rel_path`,
/// 3. Immediate `403` rejection of `.keys.json`, `.sqlite*`, `.raphs`, `ledger.json`, and `status.html`,
/// 4. Top-level directory MUST be `"inbox"` or `"archive"`,
/// 5. Final filename extension MUST match the explicit media/blueprint whitelist,
/// 6. Canonicalized path on disk MUST reside strictly inside the canonicalized `inbox/` or `archive/` subtree
///    of `products/<product>/<subject>/` (defeating symlinks pointing outside the sub-product sandbox).
pub fn resolve_sandboxed_subject_file(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    raw_rel_path: &str,
) -> std::result::Result<(PathBuf, &'static str), FileAccessError> {
    validate_product_slug(product)?;
    validate_subject_slug(subject)?;

    let trimmed = raw_rel_path.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(FileAccessError::Forbidden("empty file path".into()));
    }
    if trimmed.contains('\\') || trimmed.contains('\0') {
        return Err(FileAccessError::Forbidden(
            "backslash and NUL bytes are forbidden in file paths".into(),
        ));
    }

    let segments: Vec<&str> = trimmed.split('/').collect();
    for seg in &segments {
        if seg.is_empty() || *seg == "." || *seg == ".." || seg.starts_with('.') {
            return Err(FileAccessError::Forbidden(
                "path traversal and hidden dotfiles are forbidden".into(),
            ));
        }
        if is_forbidden_secret_filename(seg) {
            return Err(FileAccessError::Forbidden(
                "access to vault, key, or ledger files is forbidden".into(),
            ));
        }
    }

    let root_seg = segments[0];
    if root_seg != "inbox" && root_seg != "archive" {
        return Err(FileAccessError::Forbidden(
            "only read-only files under inbox/ and archive/ may be served".into(),
        ));
    }
    if segments.len() < 2 {
        return Err(FileAccessError::Forbidden(
            "directory listing is forbidden".into(),
        ));
    }

    let leaf_name = segments[segments.len() - 1];
    if is_forbidden_secret_filename(leaf_name) {
        return Err(FileAccessError::Forbidden(
            "secret or internal file is forbidden".into(),
        ));
    }

    let ext_lower = Path::new(leaf_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let content_type = content_type_for_allowed_extension(&ext_lower).ok_or_else(|| {
        FileAccessError::Forbidden(format!(
            "disallowed file extension '.{ext_lower}' (only pdf, png, jpg, jpeg, webp, svg, json, mp4, mp3 allowed)"
        ))
    })?;

    let subject_dir = cfg.subject_dir(product, subject);
    if !subject_dir.is_dir() {
        return Err(FileAccessError::NotFound);
    }
    let allowed_root = subject_dir.join(root_seg);
    if !allowed_root.is_dir() {
        return Err(FileAccessError::NotFound);
    }

    let mut candidate = subject_dir.clone();
    for seg in &segments {
        candidate.push(seg);
    }

    let sym_meta = match std::fs::symlink_metadata(&candidate) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FileAccessError::NotFound);
        }
        Err(_) => {
            return Err(FileAccessError::Forbidden(
                "unable to inspect file metadata".into(),
            ));
        }
    };

    let canon_allowed_root = std::fs::canonicalize(&allowed_root)
        .map_err(|_| FileAccessError::Forbidden("failed to canonicalize allowed subtree".into()))?;
    let canon_subject_dir = std::fs::canonicalize(&subject_dir).map_err(|_| {
        FileAccessError::Forbidden("failed to canonicalize sub-product root".into())
    })?;

    let canon_target = match std::fs::canonicalize(&candidate) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if sym_meta.file_type().is_symlink() {
                return Err(FileAccessError::Forbidden(
                    "broken symlink is forbidden".into(),
                ));
            }
            return Err(FileAccessError::NotFound);
        }
        Err(_) => {
            return Err(FileAccessError::Forbidden(
                "failed to canonicalize target path".into(),
            ));
        }
    };

    if !canon_target.starts_with(&canon_allowed_root)
        || !canon_target.starts_with(&canon_subject_dir)
    {
        return Err(FileAccessError::Forbidden(
            "symlink or path escapes sub-product sandbox".into(),
        ));
    }

    if !canon_target.is_file() {
        return Err(FileAccessError::Forbidden(
            "target is not a regular file".into(),
        ));
    }

    if let Some(canon_name) = canon_target.file_name().and_then(|s| s.to_str()) {
        if is_forbidden_secret_filename(canon_name) {
            return Err(FileAccessError::Forbidden(
                "canonical target resolves to a forbidden file".into(),
            ));
        }
        let canon_ext = canon_target
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if content_type_for_allowed_extension(&canon_ext).is_none() {
            return Err(FileAccessError::Forbidden(
                "canonical target resolves to a disallowed file extension".into(),
            ));
        }
    } else {
        return Err(FileAccessError::Forbidden(
            "invalid canonical filename".into(),
        ));
    }

    Ok((canon_target, content_type))
}

/// Resolve the final `SocketAddr` for `hydrogen serve [--lan] [--port <N>] [--bind <ip:port>]`.
pub fn resolve_serve_bind_addr(
    default_bind_addr: &str,
    lan: bool,
    port_override: Option<u16>,
    bind_override: Option<&str>,
) -> Result<SocketAddr> {
    let base_str = bind_override.unwrap_or(default_bind_addr).trim();
    let mut addr: SocketAddr = base_str.parse().map_err(|_| {
        HydrogenError::Config(format!(
            "invalid bind address '{base_str}' (expected `<ip>:<port>`, e.g. `127.0.0.1:7430`)"
        ))
    })?;

    if lan && bind_override.is_none() {
        addr.set_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    }
    if let Some(p) = port_override {
        addr.set_port(p);
    }
    Ok(addr)
}

/// Serve `router` on an already-bound `TcpListener` until `shutdown` resolves.
pub async fn serve_with_shutdown<F>(
    listener: TcpListener,
    router: Router,
    shutdown: F,
) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(HydrogenError::from)
}

/// Bind `bind_addr`, print startup Local + DGX Spark LAN endpoints, and serve until `Ctrl+C`.
pub async fn run_server(cfg: HydrogenConfig, bind_addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let port = local_addr.port();

    let state = Arc::new(ServerState::new(cfg));
    let overview = scan_workspace_overview(state.config())?;
    let router = build_router(state);

    println!(
        "=== Hydrogen Status & Product Explorer Server (v{}) ===",
        crate::VERSION
    );
    println!("  Bind       : {local_addr}");
    if local_addr.ip().is_unspecified() {
        println!("  Local      : http://127.0.0.1:{port}/");
        println!("  LAN (Eth)  : http://192.168.68.52:{port}/");
        println!("  LAN (WiFi) : http://192.168.68.85:{port}/");
    } else {
        println!("  URL        : http://{local_addr}/");
    }
    println!(
        "  Active     : {} product(s) · {} sub-product(s) · {} verified Q(s) (initial scan {} us)",
        overview.active_product_count,
        overview.active_subject_count,
        overview.verified_question_count,
        overview.elapsed_us
    );
    println!("  Press Ctrl+C to stop cleanly.");
    {
        use std::io::Write as _;
        let _ = std::io::stdout().flush();
    }

    serve_with_shutdown(listener, router, ctrl_c_shutdown()).await?;
    println!("\nHydrogen Status Server stopped cleanly.");
    Ok(())
}

async fn ctrl_c_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_serve_bind_addr_combinations() -> Result<()> {
        let def = resolve_serve_bind_addr("127.0.0.1:7430", false, None, None)?;
        assert_eq!(def.to_string(), "127.0.0.1:7430");

        let lan = resolve_serve_bind_addr("127.0.0.1:7430", true, None, None)?;
        assert_eq!(lan.to_string(), "0.0.0.0:7430");

        let lan_port = resolve_serve_bind_addr("127.0.0.1:7430", true, Some(7499), None)?;
        assert_eq!(lan_port.to_string(), "0.0.0.0:7499");

        let custom_bind =
            resolve_serve_bind_addr("127.0.0.1:7430", false, None, Some("127.0.0.1:8088"))?;
        assert_eq!(custom_bind.to_string(), "127.0.0.1:8088");

        assert!(resolve_serve_bind_addr("not-an-addr", false, None, None).is_err());
        Ok(())
    }

    #[test]
    fn validates_slugs_and_blocks_secret_or_traversal_paths() -> Result<()> {
        assert!(validate_product_slug("gyaraplus").is_ok());
        assert!(validate_subject_slug("non-verbal-reasoning").is_ok());
        assert!(validate_subject_slug("assets").is_err());
        assert!(validate_product_slug("..").is_err());
        assert!(validate_product_slug(".git").is_err());
        assert!(validate_product_slug("a/b").is_err());

        let tmp =
            std::env::temp_dir().join(format!("hydrogen-sandbox-unit-{}", std::process::id()));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(tmp.join("products/gyaraplus/maths/archive/GYR-MTH-P001/crops"))?;
        std::fs::create_dir_all(tmp.join("products/gyaraplus/maths/inbox"))?;
        std::fs::write(
            tmp.join("products/gyaraplus/maths/gyara-maths.keys.json"),
            "{}",
        )?;
        std::fs::write(
            tmp.join("products/gyaraplus/maths/gyara-maths.sqlite"),
            b"secret",
        )?;
        std::fs::write(
            tmp.join("products/gyaraplus/maths/archive/GYR-MTH-P001/crops/Q001.png"),
            b"png",
        )?;

        let cfg = HydrogenConfig {
            workspace_root: tmp.clone(),
            products_dir: tmp.join("products"),
            rules_dir: tmp.join("rules"),
            default_product: "gyaraplus".into(),
            default_subject: "maths".into(),
            bind_addr: "127.0.0.1:7430".into(),
            carbon_ws_url: "ws://127.0.0.1:7420/carbon".into(),
            device_id: "test".into(),
            lesson_key: "key".into(),
            gemini_api_key: None,
        };

        let (ok_path, ctype) = resolve_sandboxed_subject_file(
            &cfg,
            "gyaraplus",
            "maths",
            "archive/GYR-MTH-P001/crops/Q001.png",
        )
        .map_err(|e| HydrogenError::Validation(format!("{e:?}")))?;
        assert!(ok_path.ends_with("Q001.png"));
        assert_eq!(ctype, "image/png");

        assert!(matches!(
            resolve_sandboxed_subject_file(
                &cfg,
                "gyaraplus",
                "maths",
                "archive/GYR-MTH-P001/crops/MISSING.png"
            ),
            Err(FileAccessError::NotFound)
        ));

        for forbidden in [
            "gyara-maths.keys.json",
            "gyara-maths.sqlite",
            "gyara-maths.raphs",
            "ledger.json",
            "status.html",
            "../gyara-maths.keys.json",
            "archive/../../gyara-maths.keys.json",
            "archive/.hidden.png",
            "archive/fake.keys.json",
            "archive/script.sh",
            "inbox",
        ] {
            assert!(
                matches!(
                    resolve_sandboxed_subject_file(&cfg, "gyaraplus", "maths", forbidden),
                    Err(FileAccessError::Forbidden(_))
                ),
                "expected Forbidden for '{forbidden}'"
            );
        }

        std::fs::remove_dir_all(&tmp)?;
        Ok(())
    }
}
