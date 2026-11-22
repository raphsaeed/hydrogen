//! Workspace, `.env`, and `hydrogen.toml` configuration loader (`0` external config crates).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{HydrogenError, Result};

/// Raw file-backed TOML fields in `hydrogen.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TomlConfig {
    #[serde(default)]
    pub products_dir: Option<PathBuf>,
    #[serde(default)]
    pub rules_dir: Option<PathBuf>,
    #[serde(default)]
    pub default_product: Option<String>,
    #[serde(default)]
    pub default_subject: Option<String>,
    #[serde(default)]
    pub bind_addr: Option<String>,
    #[serde(default)]
    pub carbon_ws_url: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
}

/// Resolved runtime configuration for Hydrogen CLI and `axum` server.
#[derive(Debug, Clone)]
pub struct HydrogenConfig {
    pub workspace_root: PathBuf,
    pub products_dir: PathBuf,
    pub rules_dir: PathBuf,
    pub default_product: String,
    pub default_subject: String,
    pub bind_addr: String,
    pub carbon_ws_url: String,
    pub device_id: String,
    pub lesson_key: String,
    pub gemini_api_key: Option<String>,
}

impl HydrogenConfig {
    /// Load configuration from `<workspace>/hydrogen.toml`, `<workspace>/.env`, and OS environment variables.
    /// Precedence (highest wins): OS Environment -> `.env` file -> `hydrogen.toml` -> safe defaults.
    pub fn load(workspace_root: impl AsRef<Path>) -> Result<Self> {
        let root = workspace_root.as_ref().to_path_buf();
        let dotenv_path = root.join(".env");
        let dotenv_map = if dotenv_path.is_file() {
            parse_dotenv_file(&dotenv_path)?
        } else {
            BTreeMap::new()
        };

        let toml_path = root.join("hydrogen.toml");
        let toml_cfg = if toml_path.is_file() {
            let raw = std::fs::read_to_string(&toml_path)?;
            toml::from_str::<TomlConfig>(&raw)?
        } else {
            TomlConfig::default()
        };

        let get_var = |key: &str| -> Option<String> {
            std::env::var(key)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .or_else(|| dotenv_map.get(key).cloned().filter(|v| !v.trim().is_empty()))
        };

        let products_rel = toml_cfg
            .products_dir
            .unwrap_or_else(|| PathBuf::from("products"));
        let products_dir = if products_rel.is_absolute() {
            products_rel
        } else {
            root.join(products_rel)
        };

        let rules_rel = toml_cfg.rules_dir.unwrap_or_else(|| PathBuf::from("rules"));
        let rules_dir = if rules_rel.is_absolute() {
            rules_rel
        } else {
            root.join(rules_rel)
        };

        let default_product = get_var("HYDROGEN_DEFAULT_PRODUCT")
            .or(toml_cfg.default_product)
            .unwrap_or_else(|| "gyaraplus".to_string());
        let default_subject = get_var("HYDROGEN_DEFAULT_SUBJECT")
            .or(toml_cfg.default_subject)
            .unwrap_or_else(|| "maths".to_string());
        let bind_addr = get_var("HYDROGEN_BIND_ADDR")
            .or(toml_cfg.bind_addr)
            .unwrap_or_else(|| "127.0.0.1:7430".to_string());
        let carbon_ws_url = get_var("CARBON_WS_URL")
            .or(toml_cfg.carbon_ws_url)
            .unwrap_or_else(|| "ws://127.0.0.1:7420/carbon".to_string());
        let device_id = get_var("HYDROGEN_DEVICE_ID")
            .or(toml_cfg.device_id)
            .unwrap_or_else(|| "hydrogen-local".to_string());

        let lesson_key = get_var("HYDROGEN_LESSON_KEY").ok_or_else(|| {
            HydrogenError::Config(
                "missing HYDROGEN_LESSON_KEY (set in .env or environment)".into(),
            )
        })?;

        let gemini_api_key = get_var("GEMINI_API_KEY");

        Ok(Self {
            workspace_root: root,
            products_dir,
            rules_dir,
            default_product,
            default_subject,
            bind_addr,
            carbon_ws_url,
            device_id,
            lesson_key,
            gemini_api_key,
        })
    }

    /// Root directory for a product (`products/<product>`, e.g., `products/gyaraplus`).
    pub fn product_dir(&self, product: &str) -> PathBuf {
        self.products_dir.join(product)
    }

    /// Git-tracked brand & visual assets directory for a product (`products/<product>/assets`).
    pub fn product_assets_dir(&self, product: &str) -> PathBuf {
        self.product_dir(product).join("assets")
    }

    /// Git-ignored raw input inbox directory (`products/<product>/inbox/<subject>`).
    pub fn subject_inbox_dir(&self, product: &str, subject: &str) -> PathBuf {
        self.product_dir(product).join("inbox").join(subject)
    }

    /// Git-ignored processed archive & crops directory (`products/<product>/archive/<subject>`).
    pub fn subject_archive_dir(&self, product: &str, subject: &str) -> PathBuf {
        self.product_dir(product).join("archive").join(subject)
    }

    /// Canonical `.raphs` vault stem for `(product, subject)` (e.g., `gyaraplus` + `maths` -> `gyara-maths`).
    pub fn vault_stem(product: &str, subject: &str) -> String {
        let prefix = match product {
            "gyaraplus" | "gyara" => "gyara",
            other => other,
        };
        format!("{prefix}-{subject}")
    }

    /// Live SQLCipher working container path (`products/<product>/<stem>.sqlite`).
    pub fn vault_sqlite_path(&self, product: &str, subject: &str) -> PathBuf {
        let stem = Self::vault_stem(product, subject);
        self.product_dir(product).join(format!("{stem}.sqlite"))
    }

    /// Packed self-contained `.raphs` bundle path (`products/<product>/<stem>.raphs`).
    pub fn vault_bundle_path(&self, product: &str, subject: &str) -> PathBuf {
        let stem = Self::vault_stem(product, subject);
        self.product_dir(product).join(format!("{stem}.raphs"))
    }
}

/// Pure-Rust `.env` parser supporting `#` comments, `export KEY=VAL`, and single/double quotes.
pub fn parse_dotenv_file(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_dotenv_str(&content))
}

/// Parse `.env` text into a deterministic `BTreeMap`.
pub fn parse_dotenv_str(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        if key.is_empty() {
            continue;
        }
        let mut val = v.trim();
        if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
            || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
        {
            val = &val[1..val.len() - 1];
        } else if let Some((before_comment, _)) = val.split_once(" #") {
            val = before_comment.trim();
        }
        map.insert(key.to_string(), val.to_string());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv_and_vault_stems() {
        let env_text = r#"
            # Comment line
            HYDROGEN_LESSON_KEY="secret-key-123"
            export CARBON_WS_URL=ws://192.168.1.20:7420/carbon # inline note
            EMPTY_KEY=
        "#;
        let parsed = parse_dotenv_str(env_text);
        assert_eq!(
            parsed.get("HYDROGEN_LESSON_KEY").map(String::as_str),
            Some("secret-key-123")
        );
        assert_eq!(
            parsed.get("CARBON_WS_URL").map(String::as_str),
            Some("ws://192.168.1.20:7420/carbon")
        );
        assert_eq!(parsed.get("EMPTY_KEY").map(String::as_str), Some(""));

        assert_eq!(
            HydrogenConfig::vault_stem("gyaraplus", "maths"),
            "gyara-maths"
        );
        assert_eq!(
            HydrogenConfig::vault_stem("gyaraplus", "non-verbal-reasoning"),
            "gyara-non-verbal-reasoning"
        );
        assert_eq!(HydrogenConfig::vault_stem("gcse", "physics"), "gcse-physics");
    }
}
