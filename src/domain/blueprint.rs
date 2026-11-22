//! Strongly-typed `QuestionBlueprint` (`question_json`), `SolutionBlueprint` (`solution_json`),
//! and `L7` vault package envelopes (`QuestionPackage`, `PaperPackage`).
//!
//! Aligned with [`docs/solution-levels.md`](../../docs/solution-levels.md) §6,
//! [`docs/carbon-visual-theme.md`](../../docs/carbon-visual-theme.md),
//! `carbon-protocol v0.1`, and `memory_core_rs 1.7.0` (`Schema 2.3.0`).

use std::collections::BTreeMap;
use std::fmt;

use carbon_protocol::{CarbonCommand, PinnedReferenceItem, VocabularyEntry};
use memory_core_rs::{
    CourseConceptLinkInput, CourseItemAssetInput, CoursePaperInput, CoursePaperSectionInput,
    CourseQuestionInput, CourseQuestionTopicLink, CourseStimulusInput,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::error::{HydrogenError, Result};

/// Canonical blueprint schema version.
pub const BLUEPRINT_SCHEMA_VERSION: &str = "1.0";

fn default_schema_version() -> String {
    BLUEPRINT_SCHEMA_VERSION.to_string()
}

fn default_theta_preset() -> String {
    "uk_gcse".to_string()
}

fn default_container_kind() -> String {
    "paper".to_string()
}

const fn default_seq_no() -> i64 {
    1
}

fn default_label() -> String {
    "Q1".to_string()
}

const fn default_difficulty() -> i64 {
    3
}

fn default_calculator_mode() -> String {
    "non-calculator".to_string()
}

fn default_qa_status() -> String {
    "verified".to_string()
}

/// Supported question answer formats in `memory.raphs` `L7` (`docs/requirements.md` FR-3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerFormat {
    #[default]
    Mcq,
    Written,
    MultiSelect,
    Interactive,
    Spoken,
    Drawing,
}

impl AnswerFormat {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Mcq => "mcq",
            Self::Written => "written",
            Self::MultiSelect => "multi_select",
            Self::Interactive => "interactive",
            Self::Spoken => "spoken",
            Self::Drawing => "drawing",
        }
    }
}

impl fmt::Display for AnswerFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Deterministic answer key and MCQ distractor diagnostics for a question part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerKeyBlueprint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correct_choice_id: Option<String>,
    pub exact_value: String,
    pub katex_value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Optional explicit mathematical expression per choice ID (`"A".."E"`) when choice labels
    /// contain currency/prose formatting beyond plain numeric/LaTeX values.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub choice_values: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub distractor_diagnostics: BTreeMap<String, String>,
}

/// Single part (`"main"`, `"a"`, `"b"`, ...) inside `QuestionBlueprint`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPartBlueprint {
    pub part_id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub marks: f64,
    pub answer_key: AnswerKeyBlueprint,
}

/// Strongly-typed `course_questions.question_json` blueprint (Left-Pane stimulus & answer spec).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionBlueprint {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub question_id: String,
    #[serde(default)]
    pub answer_format: AnswerFormat,
    pub stem_markdown_katex: String,
    #[serde(default)]
    pub dictionary: Vec<VocabularyEntry>,
    #[serde(default)]
    pub pinned_references: Vec<PinnedReferenceItem>,
    pub left_pane_carbon_commands: Vec<CarbonCommand>,
    pub parts: Vec<QuestionPartBlueprint>,
}

/// Final answer summary card & check specification for a solution part (`solution_json.parts[].finalAnswer`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalAnswerBlueprint {
    pub katex: String,
    pub carbon_commands: Vec<CarbonCommand>,
}

/// Finer pedagogical explanation card revealed on Carbon `"step.navigated"` (`action: "breakdown"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubstepBlueprint {
    pub substep_index: usize,
    pub narration: String,
    pub carbon_commands: Vec<CarbonCommand>,
}

/// Deterministic `theta_calc` grounding check embedded inside a solution step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThetaVerificationSpec {
    pub tool: String,
    pub args: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_exact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_solutions: Option<BTreeMap<String, String>>,
}

/// Single populated sub-level (`9` down to `1`) for a solving step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SublevelBlueprint {
    pub archetype: String,
    pub narration: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speech_tokens: Vec<String>,
    pub carbon_commands: Vec<CarbonCommand>,
}

/// Solving step (`S01`, `S02`, ...) containing `substeps[]`, Theta checks, and `Levels 9 -> 1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolutionStepBlueprint {
    pub step_id: String,
    pub step_index: usize,
    pub step_number: u32,
    pub step_goal: String,
    #[serde(default)]
    pub breakdown_available: bool,
    #[serde(default)]
    pub substeps: Vec<SubstepBlueprint>,
    #[serde(default)]
    pub prerequisite_card_ids: Vec<String>,
    #[serde(default)]
    pub theta_verification: Vec<ThetaVerificationSpec>,
    #[serde(with = "sublevel_map_serde")]
    pub sublevels: BTreeMap<u8, SublevelBlueprint>,
}

/// Per-part solution tree (`solution_json.parts[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolutionPartBlueprint {
    pub part_id: String,
    #[serde(default)]
    pub uses_prior_parts: Vec<String>,
    #[serde(default)]
    pub exported_exact: BTreeMap<String, String>,
    pub final_answer: FinalAnswerBlueprint,
    pub steps: Vec<SolutionStepBlueprint>,
}

/// Strongly-typed `course_questions.solution_json` blueprint (`Steps x Sub-Levels 9 -> 1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolutionBlueprint {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub question_id: String,
    #[serde(default = "default_theta_preset")]
    pub theta_preset: String,
    pub parts: Vec<SolutionPartBlueprint>,
}

/// Question-to-topic link accepting both `camelCase` (`topicId`) and `snake_case` (`topic_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionTopicSpec {
    #[serde(alias = "topic_id")]
    pub topic_id: String,
    #[serde(default, alias = "is_primary")]
    pub is_primary: bool,
}

impl From<&QuestionTopicSpec> for CourseQuestionTopicLink {
    fn from(s: &QuestionTopicSpec) -> Self {
        Self {
            topic_id: s.topic_id.clone(),
            is_primary: s.is_primary,
        }
    }
}

/// Directed concept/prerequisite link accepting both `camelCase` and `snake_case`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptLinkSpec {
    #[serde(alias = "target_kind")]
    pub target_kind: String,
    #[serde(alias = "target_id")]
    pub target_id: String,
    #[serde(alias = "link_kind")]
    pub link_kind: String,
    #[serde(default, alias = "step_hint")]
    pub step_hint: Option<String>,
}

impl From<&ConceptLinkSpec> for CourseConceptLinkInput {
    fn from(s: &ConceptLinkSpec) -> Self {
        Self {
            target_kind: s.target_kind.clone(),
            target_id: s.target_id.clone(),
            link_kind: s.link_kind.clone(),
            step_hint: s.step_hint.clone(),
        }
    }
}

/// Media asset binding accepting both `camelCase` and `snake_case`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemAssetSpec {
    #[serde(alias = "asset_id")]
    pub asset_id: String,
    #[serde(default = "default_asset_role")]
    pub role: String,
    #[serde(default, alias = "part_id")]
    pub part_id: String,
    #[serde(default, alias = "step_id")]
    pub step_id: String,
    #[serde(default)]
    pub sublevel: i64,
}

fn default_asset_role() -> String {
    "diagram".to_string()
}

impl From<&ItemAssetSpec> for CourseItemAssetInput {
    fn from(s: &ItemAssetSpec) -> Self {
        Self {
            asset_id: s.asset_id.clone(),
            role: s.role.clone(),
            part_id: s.part_id.clone(),
            step_id: s.step_id.clone(),
            sublevel: s.sublevel,
        }
    }
}

/// Complete single-question authoring & `L7` upsert envelope (`verify-question` & `upsert-question`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPackage {
    #[serde(default = "default_container_kind", alias = "container_kind")]
    pub container_kind: String,
    #[serde(default, alias = "paper_id")]
    pub paper_id: Option<String>,
    #[serde(default, alias = "section_id")]
    pub section_id: Option<String>,
    #[serde(default, alias = "chapter_id")]
    pub chapter_id: Option<String>,
    #[serde(default, alias = "stimulus_id")]
    pub stimulus_id: Option<String>,
    #[serde(default = "default_seq_no", alias = "seq_no")]
    pub seq_no: i64,
    #[serde(default = "default_label")]
    pub label: String,
    pub title: String,
    #[serde(default, alias = "question_type_id")]
    pub question_type_id: Option<String>,
    #[serde(default = "default_difficulty")]
    pub difficulty: i64,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default = "default_calculator_mode", alias = "calculator_mode")]
    pub calculator_mode: String,
    #[serde(default, alias = "recommended_time_sec")]
    pub recommended_time_sec: Option<i64>,
    #[serde(default, alias = "is_replica")]
    pub is_replica: bool,
    #[serde(default, alias = "parent_question_id")]
    pub parent_question_id: Option<String>,
    #[serde(default = "default_qa_status", alias = "qa_status")]
    pub qa_status: String,
    #[serde(default)]
    pub topics: Vec<QuestionTopicSpec>,
    #[serde(default, alias = "concept_links")]
    pub concept_links: Vec<ConceptLinkSpec>,
    #[serde(default, alias = "item_assets")]
    pub item_assets: Vec<ItemAssetSpec>,
    pub question: QuestionBlueprint,
    pub solution: SolutionBlueprint,
}

impl QuestionPackage {
    /// Convert this verified `QuestionPackage` into `memory_core_rs::CourseQuestionInput`
    /// with derived `marks`, `part_count`, `step_count`, `min_sublevel`, and `max_sublevel`.
    pub fn to_course_question_input(&self) -> Result<CourseQuestionInput> {
        let total_marks: f64 = self.question.parts.iter().map(|p| p.marks).sum();
        let part_count = i64::try_from(self.question.parts.len()).unwrap_or(1);
        let total_steps: usize = self.solution.parts.iter().map(|p| p.steps.len()).sum();
        let step_count = i64::try_from(total_steps).unwrap_or(1);

        let mut min_lvl: u8 = 9;
        let mut max_lvl: u8 = 1;
        for part in &self.solution.parts {
            for step in &part.steps {
                for &lvl in step.sublevels.keys() {
                    if lvl < min_lvl {
                        min_lvl = lvl;
                    }
                    if lvl > max_lvl {
                        max_lvl = lvl;
                    }
                }
            }
        }

        let question_json = serde_json::to_value(&self.question)?;
        let solution_json = serde_json::to_value(&self.solution)?;

        if self.question.question_id != self.solution.question_id {
            return Err(HydrogenError::Validation(format!(
                "questionId mismatch: question has '{}', solution has '{}'",
                self.question.question_id, self.solution.question_id
            )));
        }

        Ok(CourseQuestionInput {
            question_id: self.question.question_id.clone(),
            container_kind: self.container_kind.clone(),
            paper_id: self.paper_id.clone(),
            section_id: self.section_id.clone(),
            chapter_id: self.chapter_id.clone(),
            stimulus_id: self.stimulus_id.clone(),
            seq_no: self.seq_no,
            label: self.label.clone(),
            title: self.title.clone(),
            answer_format: self.question.answer_format.as_str().to_string(),
            question_type_id: self.question_type_id.clone(),
            difficulty: self.difficulty,
            tier: self.tier.clone(),
            marks: total_marks,
            calculator_mode: self.calculator_mode.clone(),
            recommended_time_sec: self.recommended_time_sec,
            part_count,
            step_count,
            min_sublevel: i64::from(min_lvl),
            max_sublevel: i64::from(max_lvl),
            is_replica: self.is_replica,
            parent_question_id: self.parent_question_id.clone(),
            qa_status: self.qa_status.clone(),
            stem_text: self.question.stem_markdown_katex.clone(),
            question_json,
            solution_json,
            topics: self.topics.iter().map(CourseQuestionTopicLink::from).collect(),
            concept_links: self
                .concept_links
                .iter()
                .map(CourseConceptLinkInput::from)
                .collect(),
            item_assets: self
                .item_assets
                .iter()
                .map(CourseItemAssetInput::from)
                .collect(),
        })
    }
}

fn default_answer_format_str() -> String {
    "mcq".to_string()
}

fn empty_json_obj() -> Value {
    serde_json::json!({})
}

/// Paper header blueprint accepting both `camelCase` and `snake_case`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperHeaderSpec {
    #[serde(alias = "paper_id")]
    pub paper_id: String,
    pub code: String,
    pub title: String,
    #[serde(default)]
    pub badge: Option<String>,
    #[serde(default, alias = "duration_sec")]
    pub duration_sec: Option<i64>,
    #[serde(default = "default_calculator_mode", alias = "calculator_mode")]
    pub calculator_mode: String,
    #[serde(default = "default_answer_format_str", alias = "default_answer_format")]
    pub default_answer_format: String,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default, alias = "total_marks")]
    pub total_marks: Option<f64>,
    #[serde(default, alias = "is_replica")]
    pub is_replica: bool,
    #[serde(default, alias = "parent_paper_id")]
    pub parent_paper_id: Option<String>,
    #[serde(default, alias = "sort_order")]
    pub sort_order: i64,
    #[serde(default = "empty_json_obj", alias = "meta_json")]
    pub meta_json: Value,
}

impl From<&PaperHeaderSpec> for CoursePaperInput {
    fn from(p: &PaperHeaderSpec) -> Self {
        Self {
            paper_id: p.paper_id.clone(),
            code: p.code.clone(),
            title: p.title.clone(),
            badge: p.badge.clone(),
            duration_sec: p.duration_sec,
            calculator_mode: p.calculator_mode.clone(),
            default_answer_format: p.default_answer_format.clone(),
            tier: p.tier.clone(),
            total_marks: p.total_marks,
            is_replica: p.is_replica,
            parent_paper_id: p.parent_paper_id.clone(),
            sort_order: p.sort_order,
            meta_json: p.meta_json.clone(),
        }
    }
}

/// Timed paper section blueprint accepting both `camelCase` and `snake_case`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperSectionSpec {
    #[serde(alias = "section_id")]
    pub section_id: String,
    #[serde(alias = "paper_id")]
    pub paper_id: String,
    #[serde(alias = "section_no")]
    pub section_no: i64,
    pub title: String,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default, alias = "duration_sec")]
    pub duration_sec: Option<i64>,
    #[serde(default, alias = "calculator_mode")]
    pub calculator_mode: Option<String>,
}

impl From<&PaperSectionSpec> for CoursePaperSectionInput {
    fn from(s: &PaperSectionSpec) -> Self {
        Self {
            section_id: s.section_id.clone(),
            paper_id: s.paper_id.clone(),
            section_no: s.section_no,
            title: s.title.clone(),
            instructions: s.instructions.clone(),
            duration_sec: s.duration_sec,
            calculator_mode: s.calculator_mode.clone(),
        }
    }
}

/// Shared stimulus blueprint accepting both `camelCase` and `snake_case`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StimulusSpec {
    #[serde(alias = "stimulus_id")]
    pub stimulus_id: String,
    #[serde(alias = "stimulus_kind")]
    pub stimulus_kind: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, alias = "body_text")]
    pub body_text: String,
    #[serde(default = "empty_json_obj", alias = "stimulus_json")]
    pub stimulus_json: Value,
}

impl From<&StimulusSpec> for CourseStimulusInput {
    fn from(s: &StimulusSpec) -> Self {
        Self {
            stimulus_id: s.stimulus_id.clone(),
            stimulus_kind: s.stimulus_kind.clone(),
            title: s.title.clone(),
            body_text: s.body_text.clone(),
            stimulus_json: s.stimulus_json.clone(),
        }
    }
}

/// Complete paper authoring & `L7` upsert envelope (`upsert-paper <file.json>`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperPackage {
    pub paper: PaperHeaderSpec,
    #[serde(default)]
    pub sections: Vec<PaperSectionSpec>,
    #[serde(default)]
    pub stimuli: Vec<StimulusSpec>,
    #[serde(default)]
    pub questions: Vec<QuestionPackage>,
}

mod sublevel_map_serde {
    use super::*;
    use serde::ser::SerializeMap;

    pub fn serialize<S>(
        map: &BTreeMap<u8, SublevelBlueprint>,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser_map = serializer.serialize_map(Some(map.len()))?;
        // Emit keys in canonical `9 -> 1` descending order (`"9"`, `"8"`, ..., `"1"`).
        for (lvl, val) in map.iter().rev() {
            ser_map.serialize_entry(&lvl.to_string(), val)?;
        }
        ser_map.end()
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> std::result::Result<BTreeMap<u8, SublevelBlueprint>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_map = BTreeMap::<String, SublevelBlueprint>::deserialize(deserializer)?;
        let mut out = BTreeMap::new();
        for (k, v) in raw_map {
            let lvl = k.parse::<u8>().map_err(|_| {
                serde::de::Error::custom(format!(
                    "invalid sublevel key '{k}' (expected integer '1'..'9')"
                ))
            })?;
            if !(1..=9).contains(&lvl) {
                return Err(serde::de::Error::custom(format!(
                    "sublevel key '{lvl}' out of valid range 1..=9"
                )));
            }
            out.insert(lvl, v);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_question_and_solution_blueprints_with_strict_carbon_commands() -> Result<()> {
        let raw_q = serde_json::json!({
            "schemaVersion": "1.0",
            "questionId": "GYR-MTH-P001-Q014",
            "answerFormat": "mcq",
            "stemMarkdownKatex": "In a sale, all prices are reduced by $20\\%$. A coat now costs $\\pounds 48$.\n\nWhat was the original price?",
            "dictionary": [
                { "term": "reduced", "definition": "Made smaller in price or amount." }
            ],
            "pinnedReferences": [],
            "leftPaneCarbonCommands": [
                {
                    "op": "scene.write",
                    "params": {
                        "id": "q-stem",
                        "pane": "left",
                        "zone": "spine",
                        "style": "typeset",
                        "content": "In a sale, all prices are reduced by $20\\%$. A coat now costs $\\pounds 48$."
                    }
                },
                {
                    "op": "interact.ask",
                    "params": {
                        "promptId": "q-mcq",
                        "pane": "left",
                        "kind": "choice-chips",
                        "label": "Select one answer:",
                        "choices": [
                            { "id": "A", "label": "$\\pounds 38.40$" },
                            { "id": "B", "label": "$\\pounds 57.60$" },
                            { "id": "C", "label": "$\\pounds 58.00$" },
                            { "id": "D", "label": "$\\pounds 60.00$" },
                            { "id": "E", "label": "$\\pounds 62.40$" }
                        ]
                    }
                }
            ],
            "parts": [
                {
                    "partId": "main",
                    "label": null,
                    "marks": 2.0,
                    "answerKey": {
                        "correctChoiceId": "D",
                        "exactValue": "60",
                        "katexValue": "\\pounds 60.00",
                        "unit": "GBP",
                        "distractorDiagnostics": {
                            "A": "Subtracted 20% of £48.",
                            "B": "Added 20% of £48.",
                            "C": "Added £10.",
                            "E": "Added 30% of £48."
                        }
                    }
                }
            ]
        });

        let parsed_q: QuestionBlueprint = serde_json::from_value(raw_q)?;
        assert_eq!(parsed_q.question_id, "GYR-MTH-P001-Q014");
        assert_eq!(parsed_q.answer_format, AnswerFormat::Mcq);
        assert_eq!(parsed_q.left_pane_carbon_commands.len(), 2);

        let raw_sol = serde_json::json!({
            "schemaVersion": "1.0",
            "questionId": "GYR-MTH-P001-Q014",
            "thetaPreset": "uk_gcse",
            "parts": [
                {
                    "partId": "main",
                    "usesPriorParts": [],
                    "exportedExact": { "original_price": "60" },
                    "finalAnswer": {
                        "katex": "$\\pounds 60.00\\text{ (Option D)}$",
                        "carbonCommands": [
                            {
                                "op": "scene.write",
                                "params": {
                                    "id": "ans_final",
                                    "pane": "right",
                                    "zone": "spine",
                                    "style": "typeset",
                                    "color": "#15803D",
                                    "content": "Original price $= \\mathbf{\\pounds 60.00}$",
                                    "card": { "fillColor": "#F0FDF4", "borderColor": "#16A34A", "borderStyle": "solid" }
                                }
                            },
                            {
                                "op": "focus.annotate",
                                "params": {
                                    "id": "ans_chk",
                                    "targetIds": ["ans_final"],
                                    "kind": "check",
                                    "color": "#16A34A"
                                }
                            }
                        ]
                    },
                    "steps": [
                        {
                            "stepId": "S01",
                            "stepIndex": 0,
                            "stepNumber": 1,
                            "stepGoal": "Relate £48 to 80%",
                            "breakdownAvailable": false,
                            "substeps": [],
                            "prerequisiteCardIds": [],
                            "thetaVerification": [
                                { "tool": "calc.eval", "args": { "expr": "100 - 20" }, "expectedExact": "80" }
                            ],
                            "sublevels": {
                                "9": {
                                    "archetype": "expected_exam_step",
                                    "narration": "80 percent is £48.",
                                    "speechTokens": ["s1_l9_eq:pct80"],
                                    "carbonCommands": [
                                        {
                                            "op": "scene.write",
                                            "params": {
                                                "id": "s1_l9_eq",
                                                "pane": "right",
                                                "zone": "spine",
                                                "style": "typeset",
                                                "content": "$100\\% - 20\\% = \\htmlId{pct80}{80\\%}$"
                                            }
                                        }
                                    ]
                                },
                                "8": {
                                    "archetype": "guided_exam_step",
                                    "narration": "Subtract 20 percent from 100 percent.",
                                    "carbonCommands": [
                                        {
                                            "op": "scene.write",
                                            "params": {
                                                "id": "s1_l8_eq",
                                                "pane": "right",
                                                "zone": "spine",
                                                "style": "typeset",
                                                "content": "$100\\% \\xrightarrow{-20\\%} 80\\% = \\pounds 48$"
                                            }
                                        }
                                    ]
                                }
                            }
                        }
                    ]
                }
            ]
        });

        let parsed_sol: SolutionBlueprint = serde_json::from_value(raw_sol)?;
        assert_eq!(parsed_sol.parts[0].steps[0].sublevels.len(), 2);

        let pkg = QuestionPackage {
            container_kind: "paper".into(),
            paper_id: Some("GYR-MTH-P001".into()),
            section_id: None,
            chapter_id: None,
            stimulus_id: None,
            seq_no: 14,
            label: "Q14".into(),
            title: "Reverse Percentage Coat Sale".into(),
            question_type_id: Some("GYR-MTH-QT-FDP".into()),
            difficulty: 3,
            tier: None,
            calculator_mode: "non-calculator".into(),
            recommended_time_sec: Some(60),
            is_replica: false,
            parent_question_id: None,
            qa_status: "verified".into(),
            topics: vec![QuestionTopicSpec {
                topic_id: "GYR-MTH-T_FDP".into(),
                is_primary: true,
            }],
            concept_links: Vec::new(),
            item_assets: Vec::new(),
            question: parsed_q,
            solution: parsed_sol,
        };

        let l7 = pkg.to_course_question_input()?;
        assert_eq!(l7.marks, 2.0);
        assert_eq!(l7.min_sublevel, 8);
        assert_eq!(l7.max_sublevel, 9);
        assert_eq!(l7.answer_format, "mcq");
        Ok(())
    }
}
