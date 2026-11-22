//! Strictly product-isolated 4-tier Pedagogical & Visual `RuleLoader` (`rules/`).
//!
//! Resolves and concatenates the deterministic 4-tier cascade without cross-product bleed:
//! 1. `rules/common.md` — Global Common Rules
//! 2. `rules/<product>/common.md` — Product Common Rules
//! 3. `rules/<product>/<subject>/common.md` — Subject Rules + machine-read YAML frontmatter
//! 4. `rules/<product>/<subject>/levels.md` — Subject `Level 9 -> Level 1` Rules
//! 5. Optional tier/class overlay (`rules/<product>/<subject>/<tier>.md`, e.g. `foundation.md`, `higher.md`)

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{HydrogenError, Result};
use crate::theta::StagePresetConfig;

/// `theta:` block parsed from `<subject>/common.md` YAML frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleThetaConfig {
    pub enabled: bool,
    pub preset: String,
    pub angle_mode: String,
    pub allow_mixed_fractions: bool,
    pub anchor_tokens: bool,
}

impl Default for RuleThetaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            preset: "uk_gcse".to_string(),
            angle_mode: "deg".to_string(),
            allow_mixed_fractions: false,
            anchor_tokens: true,
        }
    }
}

impl RuleThetaConfig {
    /// Convert rule frontmatter settings into a `ThetaService` `StagePresetConfig`.
    pub fn to_stage_preset(&self) -> StagePresetConfig {
        StagePresetConfig {
            preset: self.preset.clone(),
            angle_mode: self.angle_mode.clone(),
            allow_mixed_fractions: self.allow_mixed_fractions,
            anchor_tokens: self.anchor_tokens,
        }
    }
}

/// `defaults:` block parsed from `<subject>/common.md` YAML frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleDefaults {
    pub answer_format: String,
    pub mcq_option_count: Option<usize>,
    pub calculator_mode: String,
    pub max_sublevel: u8,
    pub target_min_sublevel: u8,
}

impl Default for RuleDefaults {
    fn default() -> Self {
        Self {
            answer_format: "written".to_string(),
            mcq_option_count: None,
            calculator_mode: "allowed".to_string(),
            max_sublevel: 9,
            target_min_sublevel: 1,
        }
    }
}

/// Optional `level_9_policy:` block constraining `Level 9` exam-standard methods.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Level9Policy {
    pub primary_philosophy: Option<String>,
    pub allowed_algebra: Vec<String>,
    pub forbidden_at_level_9: Vec<String>,
}

/// Structured YAML frontmatter parsed from `rules/<product>/<subject>/common.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectRuleFrontmatter {
    pub product: String,
    pub course_id: String,
    pub subject: String,
    pub stage_label: String,
    pub inherits: Vec<String>,
    pub levels_file: Option<String>,
    pub theta: RuleThetaConfig,
    pub defaults: RuleDefaults,
    pub level_9_policy: Option<Level9Policy>,
}

/// Named tier slice in the assembled rulebook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleTierSection {
    /// Tier index (`1..=5`).
    pub tier: u8,
    /// Relative path inside `rules/` (e.g. `"gyaraplus/maths/common.md"`).
    pub relative_path: String,
    /// Markdown body (with YAML frontmatter stripped on Tier 3).
    pub markdown: String,
}

/// Complete product-isolated rulebook assembled dynamically from disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledRulebook {
    pub product: String,
    pub subject: String,
    pub tier_overlay: Option<String>,
    pub frontmatter: SubjectRuleFrontmatter,
    pub sections: Vec<RuleTierSection>,
    pub concatenated_markdown: String,
}

/// Dynamic, zero-bleed 4-tier rule loader rooted at `<workspace>/rules`.
#[derive(Debug, Clone)]
pub struct RuleLoader {
    rules_dir: PathBuf,
}

impl RuleLoader {
    /// Create a `RuleLoader` rooted at `rules_dir`.
    pub fn new(rules_dir: impl AsRef<Path>) -> Self {
        Self {
            rules_dir: rules_dir.as_ref().to_path_buf(),
        }
    }

    /// Root `rules/` directory path.
    pub fn rules_dir(&self) -> &Path {
        &self.rules_dir
    }

    /// Load and validate the 4-tier rule cascade for `(product, subject)` plus optional `tier_overlay`
    /// (e.g., `Some("foundation")` or `Some("higher")` for GCSE Maths).
    pub fn load(
        &self,
        product: &str,
        subject: &str,
        tier_overlay: Option<&str>,
    ) -> Result<CompiledRulebook> {
        validate_slug("product", product)?;
        validate_slug("subject", subject)?;
        if let Some(t) = tier_overlay {
            validate_slug("tier", t)?;
        }

        let global_rel = "common.md".to_string();
        let product_rel = format!("{product}/common.md");
        let subject_rel = format!("{product}/{subject}/common.md");

        let global_path = self.require_file(&global_rel)?;
        let product_path = self.require_file(&product_rel)?;
        let subject_path = self.require_file(&subject_rel)?;

        let global_md = std::fs::read_to_string(&global_path)?;
        let product_md = std::fs::read_to_string(&product_path)?;
        let subject_raw = std::fs::read_to_string(&subject_path)?;

        let (frontmatter, subject_md) = parse_subject_markdown(&subject_raw, &subject_rel)?;

        // Enforce strict identity & cross-product isolation!
        if frontmatter.product != product {
            return Err(HydrogenError::Validation(format!(
                "rule frontmatter product mismatch in {subject_rel}: expected '{product}', found '{}'",
                frontmatter.product
            )));
        }
        if frontmatter.subject != subject {
            return Err(HydrogenError::Validation(format!(
                "rule frontmatter subject mismatch in {subject_rel}: expected '{subject}', found '{}'",
                frontmatter.subject
            )));
        }

        let subject_dir_rel = Path::new(product).join(subject);
        for inherited in &frontmatter.inherits {
            let resolved_rel = normalize_relative(&subject_dir_rel.join(inherited))?;
            if resolved_rel != Path::new(&global_rel) && resolved_rel != Path::new(&product_rel) {
                return Err(HydrogenError::Validation(format!(
                    "cross-product or non-canonical rule inheritance '{inherited}' in {subject_rel} resolved to '{}'",
                    resolved_rel.display()
                )));
            }
        }

        let levels_rel = match frontmatter.levels_file.as_deref() {
            Some(custom) => {
                let norm = normalize_relative(&subject_dir_rel.join(custom))?;
                if !norm.starts_with(&subject_dir_rel) {
                    return Err(HydrogenError::Validation(format!(
                        "levels_file '{custom}' in {subject_rel} escapes subject directory '{}'",
                        subject_dir_rel.display()
                    )));
                }
                norm.to_string_lossy().into_owned()
            }
            None => format!("{product}/{subject}/levels.md"),
        };
        let levels_path = self.require_file(&levels_rel)?;
        let levels_md = std::fs::read_to_string(&levels_path)?;

        let mut sections = vec![
            RuleTierSection {
                tier: 1,
                relative_path: global_rel,
                markdown: global_md.trim().to_string(),
            },
            RuleTierSection {
                tier: 2,
                relative_path: product_rel,
                markdown: product_md.trim().to_string(),
            },
            RuleTierSection {
                tier: 3,
                relative_path: subject_rel,
                markdown: subject_md.trim().to_string(),
            },
            RuleTierSection {
                tier: 4,
                relative_path: levels_rel,
                markdown: levels_md.trim().to_string(),
            },
        ];

        if let Some(tier_name) = tier_overlay {
            let tier_rel = format!("{product}/{subject}/{tier_name}.md");
            let tier_path = self.require_file(&tier_rel)?;
            let tier_md = std::fs::read_to_string(&tier_path)?;
            sections.push(RuleTierSection {
                tier: 5,
                relative_path: tier_rel,
                markdown: tier_md.trim().to_string(),
            });
        }

        let mut concatenated = String::new();
        for (idx, sec) in sections.iter().enumerate() {
            if idx > 0 {
                concatenated.push_str("\n\n---\n\n");
            }
            concatenated.push_str(&format!(
                "<!-- TIER {}: rules/{} -->\n",
                sec.tier, sec.relative_path
            ));
            concatenated.push_str(&sec.markdown);
        }

        Ok(CompiledRulebook {
            product: product.to_string(),
            subject: subject.to_string(),
            tier_overlay: tier_overlay.map(str::to_string),
            frontmatter,
            sections,
            concatenated_markdown: concatenated,
        })
    }

    fn require_file(&self, rel: &str) -> Result<PathBuf> {
        let path = self.rules_dir.join(rel);
        if !path.is_file() {
            return Err(HydrogenError::Validation(format!(
                "required rule file missing: {}",
                path.display()
            )));
        }
        Ok(path)
    }
}

fn validate_slug(label: &str, slug: &str) -> Result<()> {
    if slug.is_empty()
        || slug.len() > 64
        || !slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || slug.starts_with('-')
        || slug.ends_with('-')
    {
        return Err(HydrogenError::Validation(format!(
            "invalid {label} slug '{slug}' (expected lowercase ASCII `[a-z0-9-]+`)"
        )));
    }
    Ok(())
}

/// Pure-Rust lexical relative path normalizer rejecting root or escaping `..` components.
fn normalize_relative(path: &Path) -> Result<PathBuf> {
    let mut stack: Vec<String> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::Normal(os) => {
                let s = os.to_str().ok_or_else(|| {
                    HydrogenError::Validation("non-UTF8 rule path component".into())
                })?;
                stack.push(s.to_string());
            }
            Component::ParentDir => {
                if stack.pop().is_none() {
                    return Err(HydrogenError::Validation(format!(
                        "rule path '{}' escapes rules root directory",
                        path.display()
                    )));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(HydrogenError::Validation(format!(
                    "absolute rule path '{}' is forbidden",
                    path.display()
                )));
            }
        }
    }
    let mut out = PathBuf::new();
    for part in stack {
        out.push(part);
    }
    Ok(out)
}

#[derive(Debug, Default)]
struct RawYamlDoc {
    scalars: BTreeMap<String, String>,
    lists: BTreeMap<String, Vec<String>>,
    nested_scalars: BTreeMap<String, BTreeMap<String, String>>,
    nested_lists: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

/// Split `--- ... ---` YAML frontmatter from `<subject>/common.md` and parse into `SubjectRuleFrontmatter`.
pub fn parse_subject_markdown(
    raw: &str,
    source_label: &str,
) -> Result<(SubjectRuleFrontmatter, String)> {
    let trimmed = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let after_open = trimmed
        .strip_prefix("---\n")
        .or_else(|| trimmed.strip_prefix("---\r\n"))
        .ok_or_else(|| {
            HydrogenError::Validation(format!(
                "missing opening `---` YAML frontmatter delimiter in {source_label}"
            ))
        })?;

    let (yaml_block, body) = find_closing_frontmatter(after_open).ok_or_else(|| {
        HydrogenError::Validation(format!(
            "missing closing `---` YAML frontmatter delimiter in {source_label}"
        ))
    })?;

    let doc = parse_minimal_yaml(yaml_block, source_label)?;

    let req_scalar = |key: &str| -> Result<String> {
        doc.scalars.get(key).cloned().ok_or_else(|| {
            HydrogenError::Validation(format!(
                "missing required frontmatter key '{key}' in {source_label}"
            ))
        })
    };

    let product = req_scalar("product")?;
    let course_id = req_scalar("course_id")?;
    let subject = req_scalar("subject")?;
    let stage_label = req_scalar("stage_label")?;
    let inherits = doc.lists.get("inherits").cloned().unwrap_or_default();
    let levels_file = doc.scalars.get("levels_file").cloned();

    // `theta:` section
    let mut theta = RuleThetaConfig::default();
    if let Some(tmap) = doc.nested_scalars.get("theta") {
        if let Some(v) = tmap.get("enabled") {
            theta.enabled = parse_yaml_bool(v, "theta.enabled", source_label)?;
        }
        if let Some(v) = tmap.get("preset") {
            theta.preset = v.clone();
        }
        if let Some(v) = tmap.get("angle_mode") {
            theta.angle_mode = v.clone();
        }
        if let Some(v) = tmap.get("allow_mixed_fractions") {
            theta.allow_mixed_fractions =
                parse_yaml_bool(v, "theta.allow_mixed_fractions", source_label)?;
        }
        if let Some(v) = tmap.get("anchor_tokens") {
            theta.anchor_tokens = parse_yaml_bool(v, "theta.anchor_tokens", source_label)?;
        }
    }

    // `defaults:` section
    let mut defaults = RuleDefaults::default();
    if let Some(dmap) = doc.nested_scalars.get("defaults") {
        if let Some(v) = dmap.get("answer_format") {
            defaults.answer_format = v.clone();
        }
        if let Some(v) = dmap.get("mcq_option_count") {
            defaults.mcq_option_count = Some(v.parse::<usize>().map_err(|_| {
                HydrogenError::Validation(format!(
                    "invalid integer defaults.mcq_option_count='{v}' in {source_label}"
                ))
            })?);
        }
        if let Some(v) = dmap.get("calculator_mode") {
            defaults.calculator_mode = v.clone();
        }
        if let Some(v) = dmap.get("max_sublevel") {
            defaults.max_sublevel = v.parse::<u8>().map_err(|_| {
                HydrogenError::Validation(format!(
                    "invalid u8 defaults.max_sublevel='{v}' in {source_label}"
                ))
            })?;
        }
        if let Some(v) = dmap.get("target_min_sublevel") {
            defaults.target_min_sublevel = v.parse::<u8>().map_err(|_| {
                HydrogenError::Validation(format!(
                    "invalid u8 defaults.target_min_sublevel='{v}' in {source_label}"
                ))
            })?;
        }
    }

    // Optional `level_9_policy:` section
    let has_l9_scalars = doc.nested_scalars.contains_key("level_9_policy");
    let has_l9_lists = doc.nested_lists.contains_key("level_9_policy");
    let level_9_policy = if has_l9_scalars || has_l9_lists {
        let primary_philosophy = doc
            .nested_scalars
            .get("level_9_policy")
            .and_then(|m| m.get("primary_philosophy"))
            .cloned();
        let allowed_algebra = doc
            .nested_lists
            .get("level_9_policy")
            .and_then(|m| m.get("allowed_algebra"))
            .cloned()
            .unwrap_or_default();
        let forbidden_at_level_9 = doc
            .nested_lists
            .get("level_9_policy")
            .and_then(|m| m.get("forbidden_at_level_9"))
            .cloned()
            .unwrap_or_default();
        Some(Level9Policy {
            primary_philosophy,
            allowed_algebra,
            forbidden_at_level_9,
        })
    } else {
        None
    };

    let fm = SubjectRuleFrontmatter {
        product,
        course_id,
        subject,
        stage_label,
        inherits,
        levels_file,
        theta,
        defaults,
        level_9_policy,
    };

    Ok((fm, body.to_string()))
}

fn find_closing_frontmatter(after_open: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    for line in after_open.split_inclusive('\n') {
        let trimmed_line = line.trim_end_matches(['\r', '\n']);
        if trimmed_line == "---" {
            let yaml = &after_open[..offset];
            let rest = &after_open[offset + line.len()..];
            return Some((yaml, rest));
        }
        offset += line.len();
    }
    if after_open.ends_with("\n---") {
        let yaml = &after_open[..after_open.len() - 3];
        return Some((yaml, ""));
    }
    None
}

fn parse_yaml_bool(val: &str, field: &str, source_label: &str) -> Result<bool> {
    match val {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        _ => Err(HydrogenError::Validation(format!(
            "invalid boolean {field}='{val}' in {source_label}"
        ))),
    }
}

fn strip_yaml_scalar(raw: &str) -> String {
    let s = raw.trim();
    if ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
        && s.len() >= 2
    {
        return s[1..s.len() - 1].to_string();
    }
    if let Some((before_comment, _)) = s.split_once(" #") {
        return before_comment.trim().to_string();
    }
    s.to_string()
}

fn parse_minimal_yaml(yaml: &str, source_label: &str) -> Result<RawYamlDoc> {
    let mut doc = RawYamlDoc::default();
    let mut current_section: Option<String> = None;
    let mut current_subkey: Option<String> = None;

    for (line_idx, raw_line) in yaml.lines().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw_line.bytes().take_while(|b| *b == b' ').count();

        if indent == 0 {
            current_subkey = None;
            let (k, v) = split_yaml_kv(trimmed, line_idx + 1, source_label)?;
            if v.is_empty() {
                current_section = Some(k.to_string());
            } else {
                current_section = None;
                doc.scalars.insert(k.to_string(), strip_yaml_scalar(v));
            }
        } else if let Some(item) = trimmed.strip_prefix("- ") {
            let val = strip_yaml_scalar(item);
            let sec = current_section.as_deref().ok_or_else(|| {
                HydrogenError::Validation(format!(
                    "unexpected YAML list item at line {} in {source_label}",
                    line_idx + 1
                ))
            })?;
            if let Some(sub) = current_subkey.as_deref() {
                doc.nested_lists
                    .entry(sec.to_string())
                    .or_default()
                    .entry(sub.to_string())
                    .or_default()
                    .push(val);
            } else {
                doc.lists.entry(sec.to_string()).or_default().push(val);
            }
        } else {
            let sec = current_section.as_deref().ok_or_else(|| {
                HydrogenError::Validation(format!(
                    "unexpected indented YAML key at line {} in {source_label}",
                    line_idx + 1
                ))
            })?;
            let (k, v) = split_yaml_kv(trimmed, line_idx + 1, source_label)?;
            if v.is_empty() {
                current_subkey = Some(k.to_string());
                doc.nested_lists
                    .entry(sec.to_string())
                    .or_default()
                    .entry(k.to_string())
                    .or_default();
            } else {
                current_subkey = None;
                doc.nested_scalars
                    .entry(sec.to_string())
                    .or_default()
                    .insert(k.to_string(), strip_yaml_scalar(v));
            }
        }
    }

    Ok(doc)
}

fn split_yaml_kv<'a>(line: &'a str, line_no: usize, source_label: &str) -> Result<(&'a str, &'a str)> {
    let Some((k, v)) = line.split_once(':') else {
        return Err(HydrogenError::Validation(format!(
            "malformed YAML line {line_no} in {source_label}: '{line}'"
        )));
    };
    let key = k.trim();
    if key.is_empty() {
        return Err(HydrogenError::Validation(format!(
            "empty YAML key at line {line_no} in {source_label}"
        )));
    }
    Ok((key, v.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_rules_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rules")
    }

    #[test]
    fn loads_all_product_subject_cascades_and_frontmatter() -> Result<()> {
        let loader = RuleLoader::new(repo_rules_dir());

        // 1. Gyara+ 11+ Maths
        let gyr_mth = loader.load("gyaraplus", "maths", None)?;
        assert_eq!(gyr_mth.frontmatter.course_id, "GYR-MTH");
        assert_eq!(gyr_mth.frontmatter.stage_label, "11+");
        assert!(gyr_mth.frontmatter.theta.enabled);
        assert_eq!(gyr_mth.frontmatter.theta.preset, "uk_gcse");
        assert_eq!(gyr_mth.frontmatter.theta.angle_mode, "deg");
        assert!(gyr_mth.frontmatter.theta.allow_mixed_fractions);
        assert!(gyr_mth.frontmatter.theta.anchor_tokens);
        assert_eq!(gyr_mth.frontmatter.defaults.answer_format, "mcq");
        assert_eq!(gyr_mth.frontmatter.defaults.mcq_option_count, Some(5));
        assert_eq!(gyr_mth.frontmatter.defaults.calculator_mode, "non-calculator");
        assert_eq!(gyr_mth.sections.len(), 4);

        let l9 = gyr_mth
            .frontmatter
            .level_9_policy
            .as_ref()
            .ok_or_else(|| HydrogenError::Validation("expected level_9_policy".into()))?;
        assert_eq!(
            l9.primary_philosophy.as_deref(),
            Some("algebraic_method_first")
        );
        assert!(l9.allowed_algebra.iter().any(|s| s == "simultaneous_linear_elimination"));
        assert!(l9.forbidden_at_level_9.iter().any(|s| s == "quadratic_equations"));

        // Verify ZERO GCSE / A-Level rule files leaked into Gyara+ concatenation
        assert!(!gyr_mth.concatenated_markdown.contains("rules/gcse/"));
        assert!(!gyr_mth.concatenated_markdown.contains("rules/alevel/"));

        // 2. Remaining 9 subject vaults + GCSE Foundation/Higher overlays
        let matrix = [
            ("gyaraplus", "english", None, "GYR-ENG", false),
            ("gyaraplus", "verbal-reasoning", None, "GYR-VR", true),
            ("gyaraplus", "non-verbal-reasoning", None, "GYR-NVR", false),
            ("gcse", "maths", Some("foundation"), "GCSE-MTH", true),
            ("gcse", "maths", Some("higher"), "GCSE-MTH", true),
            ("gcse", "physics", None, "GCSE-PHY", true),
            ("alevel", "maths", None, "ALV-MTH", true),
            ("alevel", "physics", None, "ALV-PHY", true),
            ("primary", "maths", None, "PRI-MTH", true),
            ("infant", "maths", None, "INF-MTH", true),
        ];

        for (prod, subj, tier, expected_course, expected_theta) in matrix {
            let book = loader.load(prod, subj, tier)?;
            assert_eq!(book.frontmatter.course_id, expected_course);
            assert_eq!(book.frontmatter.theta.enabled, expected_theta);
            let expected_secs = if tier.is_some() { 5 } else { 4 };
            assert_eq!(book.sections.len(), expected_secs);
        }

        Ok(())
    }

    #[test]
    fn rejects_path_traversal_and_cross_product_inheritance() -> Result<()> {
        let loader = RuleLoader::new(repo_rules_dir());
        assert!(loader.load("../gcse", "maths", None).is_err());
        assert!(loader.load("gyaraplus", "../gcse", None).is_err());

        // Test synthetic cross-product bleed in `inherits:`
        let tmp = std::env::temp_dir().join(format!("hydrogen-rules-iso-{}", std::process::id()));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(tmp.join("gyaraplus/maths"))?;
        std::fs::create_dir_all(tmp.join("gcse"))?;
        std::fs::write(tmp.join("common.md"), "# Global")?;
        std::fs::write(tmp.join("gyaraplus/common.md"), "# Gyara")?;
        std::fs::write(tmp.join("gcse/common.md"), "# GCSE")?;
        std::fs::write(tmp.join("gyaraplus/maths/levels.md"), "# Levels")?;
        std::fs::write(
            tmp.join("gyaraplus/maths/common.md"),
            "---\nproduct: gyaraplus\ncourse_id: GYR-MTH\nsubject: maths\nstage_label: \"11+\"\ninherits:\n  - ../../common.md\n  - ../../gcse/common.md\n---\n# Body\n",
        )?;

        let tmp_loader = RuleLoader::new(&tmp);
        let err = tmp_loader.load("gyaraplus", "maths", None);
        assert!(err.is_err(), "cross-product inheritance must be rejected");
        std::fs::remove_dir_all(&tmp)?;
        Ok(())
    }
}
