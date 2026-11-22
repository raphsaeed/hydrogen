//! Deterministic Universal Hierarchical ID parser, builder & validator (`docs/requirements.md` FR-2).
//!
//! Canonical grammar (`1..=128` ASCII chars):
//! `<PRODUCT>-<SUBJECT>[-<CONTAINER>[-<ITEM>[-R<NN>]]][:<PART>[:S<NN>[:L<NN>]]]`
//!
//! Supported canonical examples:
//! - `GYR-MTH` (Course ID)
//! - `GYR-MTH-P001` / `GYR-MTH-P001_R01` (Paper / Whole-Paper Replica)
//! - `GYR-MTH-P001-SEC01` / `GYR-MTH-P001-STIM01` (Paper Section / Shared Stimulus)
//! - `GYR-MTH-P001-Q014` / `GYR-MTH-P001-Q014-R01` / `GYR-MTH-P001_R01-Q014` (Paper Question & Replicas)
//! - `GYR-MTH-B01` / `GYR-MTH-B01-CH03` / `GYR-MTH-B01-CH03-Q008` (Book, Chapter & Chapter Question)
//! - `GYR-MTH-T_RAT` / `GYR-MTH-T_RAT01-C01` / `GYR-MTH-T_RAT01-Q003` (Topic, Concept Card & Topic Question)
//! - `GYR-MTH-QT-ALG-WORD` (Question Type)
//! - `GYR-MTH-P001-Q014:a:S01:L09` / `GYR-MTH-P001-Q014:main:S02:L03` (Part / Step / Sub-Level address)

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{HydrogenError, Result};

/// Maximum length of a `UniversalId` in SQLite `L7` (`1..=128` chars).
pub const MAX_UNIVERSAL_ID_LEN: usize = 128;

/// Container segment following `<PRODUCT>-<SUBJECT>-`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerSegment {
    /// Practice or mock paper (`P001`, or whole-paper replica `P001_R01`).
    Paper { number: u32, replica: Option<u32> },
    /// Book (`B01`) with optional chapter (`CH03`).
    Book {
        book_no: u32,
        chapter_no: Option<u32>,
    },
    /// Curriculum topic or sub-topic (`T_NUM`, `T_RAT01`).
    Topic { code: String },
    /// Problem archetype / question type (`QT-ALG-WORD`).
    QuestionType { code: String },
}

impl fmt::Display for ContainerSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Paper {
                number,
                replica: Some(r),
            } => write!(f, "P{number:03}_R{r:02}"),
            Self::Paper {
                number,
                replica: None,
            } => write!(f, "P{number:03}"),
            Self::Book {
                book_no,
                chapter_no: Some(ch),
            } => write!(f, "B{book_no:02}-CH{ch:02}"),
            Self::Book {
                book_no,
                chapter_no: None,
            } => write!(f, "B{book_no:02}"),
            Self::Topic { code } => write!(f, "T_{code}"),
            Self::QuestionType { code } => write!(f, "QT-{code}"),
        }
    }
}

/// Leaf item segment inside a paper, chapter, or topic container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemSegment {
    /// Question (`Q014`) with optional item-level replica (`-R01`).
    Question { number: u32, replica: Option<u32> },
    /// Topic / concept / worked-example / prerequisite card (`C01`).
    ConceptCard { number: u32 },
    /// Timed paper section (`SEC01`).
    Section { number: u32 },
    /// Shared reading/data/diagram stimulus (`STIM01`).
    Stimulus { number: u32 },
}

impl fmt::Display for ItemSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Question {
                number,
                replica: Some(r),
            } => write!(f, "Q{number:03}-R{r:02}"),
            Self::Question {
                number,
                replica: None,
            } => write!(f, "Q{number:03}"),
            Self::ConceptCard { number } => write!(f, "C{number:02}"),
            Self::Section { number } => write!(f, "SEC{number:02}"),
            Self::Stimulus { number } => write!(f, "STIM{number:02}"),
        }
    }
}

/// Optional fine-grained `:part[:S01[:L09]]` sub-address on a question or card ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAddress {
    pub part_id: String,
    pub step_no: Option<u16>,
    pub level: Option<u8>,
}

impl fmt::Display for SubAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":{}", self.part_id)?;
        if let Some(s) = self.step_no {
            write!(f, ":S{s:02}")?;
            if let Some(l) = self.level {
                write!(f, ":L{l:02}")?;
            }
        }
        Ok(())
    }
}

/// Strongly-typed Universal Hierarchical ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniversalId {
    pub product_code: String,
    pub subject_code: String,
    pub container: Option<ContainerSegment>,
    pub item: Option<ItemSegment>,
    pub sub_address: Option<SubAddress>,
}

impl UniversalId {
    /// Parse and validate a `UniversalId` string (`1..=128` chars).
    pub fn parse(raw: &str) -> Result<Self> {
        raw.parse()
    }

    /// Construct a paper ID (`<PROD>-<SUBJ>-P<NNN>` or `<PROD>-<SUBJ>-P<NNN>_R<RR>`).
    pub fn paper(
        product_code: &str,
        subject_code: &str,
        paper_no: u32,
        replica: Option<u32>,
    ) -> Result<Self> {
        validate_upper_token("product_code", product_code)?;
        validate_upper_token("subject_code", subject_code)?;
        if paper_no == 0 {
            return Err(HydrogenError::Validation("paper_no must be >= 1".into()));
        }
        if replica == Some(0) {
            return Err(HydrogenError::Validation("replica number must be >= 1".into()));
        }
        Ok(Self {
            product_code: product_code.to_string(),
            subject_code: subject_code.to_string(),
            container: Some(ContainerSegment::Paper {
                number: paper_no,
                replica,
            }),
            item: None,
            sub_address: None,
        })
    }

    /// Construct a paper question ID (`<PROD>-<SUBJ>-P<NNN>-Q<QQQ>`).
    pub fn paper_question(
        product_code: &str,
        subject_code: &str,
        paper_no: u32,
        question_no: u32,
    ) -> Result<Self> {
        let mut base = Self::paper(product_code, subject_code, paper_no, None)?;
        if question_no == 0 {
            return Err(HydrogenError::Validation("question_no must be >= 1".into()));
        }
        base.item = Some(ItemSegment::Question {
            number: question_no,
            replica: None,
        });
        Ok(base)
    }

    /// Map canonical product directory slug (`gyaraplus`, `gcse`, `alevel`, `primary`, `infant`) to its ID prefix.
    pub fn product_prefix_for_slug(product_slug: &str) -> &'static str {
        match product_slug {
            "gyaraplus" | "gyara" => "GYR",
            "gcse" => "GCSE",
            "alevel" => "ALV",
            "primary" => "PRI",
            "infant" => "INF",
            _ => "GEN",
        }
    }

    /// Map canonical subject directory slug (`maths`, `english`, `verbal-reasoning`, `non-verbal-reasoning`, `physics`) to its ID prefix.
    pub fn subject_prefix_for_slug(subject_slug: &str) -> &'static str {
        match subject_slug {
            "maths" => "MTH",
            "english" => "ENG",
            "verbal-reasoning" => "VR",
            "non-verbal-reasoning" => "NVR",
            "physics" => "PHY",
            _ => "GEN",
        }
    }

    /// `<PRODUCT>-<SUBJECT>` course identifier (e.g. `"GYR-MTH"`).
    pub fn course_id(&self) -> String {
        format!("{}-{}", self.product_code, self.subject_code)
    }

    /// Container ID string if present (e.g. `"GYR-MTH-P001"` or `"GYR-MTH-B01-CH03"`).
    pub fn container_id(&self) -> Option<String> {
        self.container
            .as_ref()
            .map(|c| format!("{}-{}-{c}", self.product_code, self.subject_code))
    }

    /// Base SQLite `L7` primary key without `:part:S01:L09` sub-address.
    pub fn base_id(&self) -> String {
        let mut without_sub = self.clone();
        without_sub.sub_address = None;
        without_sub.to_string()
    }

    /// Whether either the container (`P001_R01`) or the question (`-R01`) is an AI replica.
    pub fn is_replica(&self) -> bool {
        let paper_rep = matches!(
            self.container,
            Some(ContainerSegment::Paper {
                replica: Some(_),
                ..
            })
        );
        let item_rep = matches!(
            self.item,
            Some(ItemSegment::Question {
                replica: Some(_),
                ..
            })
        );
        paper_rep || item_rep
    }

    /// Return a copy with an item-level question replica suffix (`-R<nn>`).
    pub fn with_question_replica(&self, replica_no: u32) -> Result<Self> {
        if replica_no == 0 {
            return Err(HydrogenError::Validation(
                "replica_no must be >= 1".into(),
            ));
        }
        let mut next = self.clone();
        match &mut next.item {
            Some(ItemSegment::Question { replica, .. }) => {
                *replica = Some(replica_no);
                Ok(next)
            }
            _ => Err(HydrogenError::Validation(format!(
                "cannot attach question replica to non-question ID '{self}'"
            ))),
        }
    }

    /// Return a copy with a `:part[:S01[:L09]]` sub-address attached.
    pub fn with_sub_address(
        &self,
        part_id: &str,
        step_no: Option<u16>,
        level: Option<u8>,
    ) -> Result<Self> {
        let sub = build_sub_address(part_id, step_no, level)?;
        let mut next = self.clone();
        next.sub_address = Some(sub);
        let rendered = next.to_string();
        if rendered.len() > MAX_UNIVERSAL_ID_LEN {
            return Err(HydrogenError::Validation(format!(
                "UniversalId exceeds {MAX_UNIVERSAL_ID_LEN} chars: '{rendered}'"
            )));
        }
        Ok(next)
    }
}

impl fmt::Display for UniversalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.product_code, self.subject_code)?;
        if let Some(c) = &self.container {
            write!(f, "-{c}")?;
        }
        if let Some(i) = &self.item {
            write!(f, "-{i}")?;
        }
        if let Some(sub) = &self.sub_address {
            write!(f, "{sub}")?;
        }
        Ok(())
    }
}

impl FromStr for UniversalId {
    type Err = HydrogenError;

    fn from_str(raw: &str) -> Result<Self> {
        if raw.is_empty() || raw.len() > MAX_UNIVERSAL_ID_LEN {
            return Err(HydrogenError::Validation(format!(
                "UniversalId length must be 1..={MAX_UNIVERSAL_ID_LEN}, got {}",
                raw.len()
            )));
        }
        if !raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        {
            return Err(HydrogenError::Validation(format!(
                "UniversalId '{raw}' contains forbidden characters"
            )));
        }

        let (base_str, sub_opt) = match raw.split_once(':') {
            Some((b, s)) => (b, Some(parse_sub_address(s, raw)?)),
            None => (raw, None),
        };

        let parts: Vec<&str> = base_str.split('-').collect();
        if parts.len() < 2 {
            return Err(HydrogenError::Validation(format!(
                "UniversalId '{raw}' must start with `<PRODUCT>-<SUBJECT>`"
            )));
        }

        let product_code = parts[0];
        let subject_code = parts[1];
        validate_upper_token("product_code", product_code)?;
        validate_upper_token("subject_code", subject_code)?;

        let tail = &parts[2..];
        if tail.is_empty() {
            if sub_opt.is_some() {
                return Err(HydrogenError::Validation(format!(
                    "course-only UniversalId '{raw}' cannot carry a sub-address"
                )));
            }
            return Ok(Self {
                product_code: product_code.to_string(),
                subject_code: subject_code.to_string(),
                container: None,
                item: None,
                sub_address: None,
            });
        }

        // Question-Type IDs: `<PROD>-<SUBJ>-QT-<CODE...>`
        if tail[0] == "QT" {
            if tail.len() < 2 || sub_opt.is_some() {
                return Err(HydrogenError::Validation(format!(
                    "invalid question-type UniversalId '{raw}'"
                )));
            }
            let code = tail[1..].join("-");
            for tok in &tail[1..] {
                validate_upper_token("question_type_code", tok)?;
            }
            return Ok(Self {
                product_code: product_code.to_string(),
                subject_code: subject_code.to_string(),
                container: Some(ContainerSegment::QuestionType { code }),
                item: None,
                sub_address: None,
            });
        }

        let (container, consumed) = parse_container(tail, raw)?;
        let item_tokens = &tail[consumed..];
        let item = if item_tokens.is_empty() {
            None
        } else {
            Some(parse_item(item_tokens, raw)?)
        };

        if sub_opt.is_some() && item.is_none() {
            return Err(HydrogenError::Validation(format!(
                "sub-address requires an item segment in '{raw}'"
            )));
        }

        Ok(Self {
            product_code: product_code.to_string(),
            subject_code: subject_code.to_string(),
            container: Some(container),
            item,
            sub_address: sub_opt,
        })
    }
}

impl Serialize for UniversalId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UniversalId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

fn validate_upper_token(label: &str, tok: &str) -> Result<()> {
    if tok.is_empty()
        || !tok
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return Err(HydrogenError::Validation(format!(
            "invalid {label} token '{tok}' (expected uppercase ASCII `[A-Z0-9]+`)"
        )));
    }
    Ok(())
}

fn parse_positive_u32(digits: &str, label: &str, full_raw: &str) -> Result<u32> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(HydrogenError::Validation(format!(
            "invalid numeric {label} '{digits}' in UniversalId '{full_raw}'"
        )));
    }
    let val = digits.parse::<u32>().map_err(|_| {
        HydrogenError::Validation(format!(
            "numeric overflow in {label} '{digits}' of '{full_raw}'"
        ))
    })?;
    if val == 0 {
        return Err(HydrogenError::Validation(format!(
            "{label} must be >= 1 in UniversalId '{full_raw}'"
        )));
    }
    Ok(val)
}

fn parse_container(tail: &[&str], full_raw: &str) -> Result<(ContainerSegment, usize)> {
    let head = tail[0];

    // 1. Paper: `P001` or `P001_R01`
    if let Some(rest) = head.strip_prefix('P') {
        let (p_digits, rep_opt) = match rest.split_once("_R") {
            Some((p, r)) => (
                p,
                Some(parse_positive_u32(r, "paper_replica", full_raw)?),
            ),
            None => (rest, None),
        };
        let number = parse_positive_u32(p_digits, "paper_number", full_raw)?;
        return Ok((
            ContainerSegment::Paper {
                number,
                replica: rep_opt,
            },
            1,
        ));
    }

    // 2. Book (+ optional Chapter): `B01` or `B01-CH03`
    if let Some(b_digits) = head.strip_prefix('B') {
        let book_no = parse_positive_u32(b_digits, "book_number", full_raw)?;
        if let Some(ch_tok) = tail.get(1)
            && let Some(ch_digits) = ch_tok.strip_prefix("CH")
        {
            let chapter_no = parse_positive_u32(ch_digits, "chapter_number", full_raw)?;
            return Ok((
                ContainerSegment::Book {
                    book_no,
                    chapter_no: Some(chapter_no),
                },
                2,
            ));
        }
        return Ok((
            ContainerSegment::Book {
                book_no,
                chapter_no: None,
            },
            1,
        ));
    }

    // 3. Topic: `T_RAT`, `T_RAT01`
    if let Some(code) = head.strip_prefix("T_") {
        if code.is_empty()
            || !code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(HydrogenError::Validation(format!(
                "invalid topic code '{head}' in UniversalId '{full_raw}'"
            )));
        }
        return Ok((
            ContainerSegment::Topic {
                code: code.to_string(),
            },
            1,
        ));
    }

    Err(HydrogenError::Validation(format!(
        "unrecognized container segment '{head}' in UniversalId '{full_raw}'"
    )))
}

fn parse_item(tokens: &[&str], full_raw: &str) -> Result<ItemSegment> {
    let head = tokens[0];

    if let Some(s_digits) = head.strip_prefix("SEC") {
        if tokens.len() != 1 {
            return Err(HydrogenError::Validation(format!(
                "unexpected trailing tokens after section in '{full_raw}'"
            )));
        }
        let number = parse_positive_u32(s_digits, "section_number", full_raw)?;
        return Ok(ItemSegment::Section { number });
    }

    if let Some(st_digits) = head.strip_prefix("STIM") {
        if tokens.len() != 1 {
            return Err(HydrogenError::Validation(format!(
                "unexpected trailing tokens after stimulus in '{full_raw}'"
            )));
        }
        let number = parse_positive_u32(st_digits, "stimulus_number", full_raw)?;
        return Ok(ItemSegment::Stimulus { number });
    }

    if let Some(c_digits) = head.strip_prefix('C') {
        if tokens.len() != 1 {
            return Err(HydrogenError::Validation(format!(
                "unexpected trailing tokens after concept card in '{full_raw}'"
            )));
        }
        let number = parse_positive_u32(c_digits, "card_number", full_raw)?;
        return Ok(ItemSegment::ConceptCard { number });
    }

    if let Some(q_digits) = head.strip_prefix('Q') {
        let number = parse_positive_u32(q_digits, "question_number", full_raw)?;
        let replica = match tokens.len() {
            1 => None,
            2 => {
                let r_digits = tokens[1].strip_prefix('R').ok_or_else(|| {
                    HydrogenError::Validation(format!(
                        "expected `-R<nn>` replica suffix, found '{}' in '{full_raw}'",
                        tokens[1]
                    ))
                })?;
                Some(parse_positive_u32(r_digits, "question_replica", full_raw)?)
            }
            _ => {
                return Err(HydrogenError::Validation(format!(
                    "unexpected extra segments after question ID in '{full_raw}'"
                )));
            }
        };
        return Ok(ItemSegment::Question { number, replica });
    }

    Err(HydrogenError::Validation(format!(
        "unrecognized item segment '{head}' in UniversalId '{full_raw}'"
    )))
}

fn build_sub_address(part_id: &str, step_no: Option<u16>, level: Option<u8>) -> Result<SubAddress> {
    if part_id.is_empty()
        || part_id.len() > 16
        || !part_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(HydrogenError::Validation(format!(
            "invalid sub-address part_id '{part_id}'"
        )));
    }
    if level.is_some() && step_no.is_none() {
        return Err(HydrogenError::Validation(
            "sub-address level (`L09`) requires a step (`S01`)".into(),
        ));
    }
    if let Some(s) = step_no
        && s == 0
    {
        return Err(HydrogenError::Validation(
            "sub-address step number must be >= 1".into(),
        ));
    }
    if let Some(l) = level
        && !(1..=9).contains(&l)
    {
        return Err(HydrogenError::Validation(format!(
            "sub-address level must be within 1..=9, got {l}"
        )));
    }
    Ok(SubAddress {
        part_id: part_id.to_string(),
        step_no,
        level,
    })
}

fn parse_sub_address(sub_str: &str, full_raw: &str) -> Result<SubAddress> {
    let cols: Vec<&str> = sub_str.split(':').collect();
    if cols.is_empty() || cols.len() > 3 {
        return Err(HydrogenError::Validation(format!(
            "invalid sub-address ':{sub_str}' in '{full_raw}'"
        )));
    }
    let part_id = cols[0];
    let step_no = if let Some(s_tok) = cols.get(1) {
        let s_digits = s_tok.strip_prefix('S').ok_or_else(|| {
            HydrogenError::Validation(format!(
                "expected `S<nn>` step segment, found '{s_tok}' in '{full_raw}'"
            ))
        })?;
        let val = parse_positive_u32(s_digits, "step_number", full_raw)?;
        let step_u16 = u16::try_from(val).map_err(|_| {
            HydrogenError::Validation(format!("step_number '{val}' too large in '{full_raw}'"))
        })?;
        Some(step_u16)
    } else {
        None
    };

    let level = if let Some(l_tok) = cols.get(2) {
        let l_digits = l_tok.strip_prefix('L').ok_or_else(|| {
            HydrogenError::Validation(format!(
                "expected `L<nn>` level segment, found '{l_tok}' in '{full_raw}'"
            ))
        })?;
        let val = parse_positive_u32(l_digits, "sublevel", full_raw)?;
        let lvl_u8 = u8::try_from(val).map_err(|_| {
            HydrogenError::Validation(format!("sublevel '{val}' out of range in '{full_raw}'"))
        })?;
        Some(lvl_u8)
    } else {
        None
    };

    build_sub_address(part_id, step_no, level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_roundtrips_all_canonical_universal_ids() -> Result<()> {
        let cases = [
            ("GYR-MTH", false, None),
            ("GYR-MTH-P001", false, None),
            ("GYR-MTH-P001_R01", true, None),
            ("GYR-MTH-P001-SEC01", false, Some("GYR-MTH-P001")),
            ("GYR-MTH-P001-STIM02", false, Some("GYR-MTH-P001")),
            ("GYR-MTH-P001-Q014", false, Some("GYR-MTH-P001")),
            ("GYR-MTH-P001-Q014-R01", true, Some("GYR-MTH-P001")),
            ("GYR-MTH-P001_R01-Q014", true, Some("GYR-MTH-P001_R01")),
            ("GYR-MTH-B01", false, None),
            ("GYR-MTH-B01-CH03", false, None),
            ("GYR-MTH-B01-CH03-Q008", false, Some("GYR-MTH-B01-CH03")),
            ("GYR-MTH-T_RAT", false, None),
            ("GYR-MTH-T_RAT01-C01", false, Some("GYR-MTH-T_RAT01")),
            ("GYR-MTH-QT-ALG-WORD", false, None),
            ("GYR-MTH-P001-Q014:a:S01:L09", false, Some("GYR-MTH-P001")),
            ("GCSE-PHY-P002-Q005:main:S03:L01", false, Some("GCSE-PHY-P002")),
        ];

        for (raw, expected_replica, expected_container_for_item) in cases {
            let parsed = UniversalId::parse(raw)?;
            assert_eq!(parsed.to_string(), raw);
            assert_eq!(parsed.is_replica(), expected_replica);
            if parsed.item.is_some() {
                assert_eq!(
                    parsed.container_id().as_deref(),
                    expected_container_for_item
                );
            }
        }

        let q = UniversalId::paper_question("GYR", "MTH", 1, 14)?;
        assert_eq!(q.to_string(), "GYR-MTH-P001-Q014");
        assert_eq!(q.with_question_replica(2)?.to_string(), "GYR-MTH-P001-Q014-R02");
        assert_eq!(
            q.with_sub_address("a", Some(1), Some(9))?.to_string(),
            "GYR-MTH-P001-Q014:a:S01:L09"
        );
        assert_eq!(
            q.with_sub_address("a", Some(1), Some(9))?.base_id(),
            "GYR-MTH-P001-Q014"
        );

        // Reject invalid cases
        for bad in [
            "",
            "gyr-mth-p001",
            "GYR-MTH-P000-Q001",
            "GYR-MTH-P001-Q014:a:S01:L10",
            "GYR-MTH-P001-Q014:a:S00:L05",
            "GYR-MTH:a:S01:L09",
            "Practice-Paper-01.pdf",
        ] {
            assert!(
                UniversalId::parse(bad).is_err(),
                "expected '{bad}' to be rejected"
            );
        }

        Ok(())
    }
}
