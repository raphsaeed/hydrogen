//! Deterministic Quality Gate (`Phase 2.2`):
//! 1. **Theta Math & `A–E` MCQ Uniqueness Gate** (step `thetaVerification`, multi-part `exportedExact` zero-drift chain, `answerKey`, and 5-option distractor uniqueness),
//! 2. **Carbon Command, Warm-Paper Theme & KaTeX Gate** (`c1..c8` ink/card tokens, fine strokes `<= 2.2 pt`, no duplicate `✓` in `"kind": "check"`, PNG-only logos, `$...$` math wrapping, execution-order `targetId` / `\htmlId` resolution),
//! 3. **Pedagogical & Clean-Room Lint** (contiguous `9 -> min_sublevel` levels, non-identical adjacent levels, board prose word caps, and zero source-paper/board/crop leakage).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use carbon_protocol::{
    AnchorSpec, AnnotationKind, CarbonCommand, MathRenderStyle, PaneId, PromptKind, TextForm,
    ThemeId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{
    AnswerFormat, ItemSegment, QuestionBlueprint, QuestionPackage, SolutionBlueprint, UniversalId,
};
use crate::error::{HydrogenError, Result};
use crate::rules::CompiledRulebook;
use crate::theta::{StagePresetConfig, ThetaService};

/// Maximum stroke/border width allowed on diagrams, axes, and callout cards (`docs/carbon-visual-theme.md` §3).
pub const MAX_DIAGRAM_STROKE_PT: f64 = 2.2;

/// Maximum prose words (outside `$...$`) on a single `focus.annotate` pointer label.
pub const MAX_ANNOTATION_PROSE_WORDS: usize = 18;

/// Maximum prose words (outside `$...$`) on a single Right-Pane `scene.write` command (prevents unanchored prose walls).
pub const MAX_RIGHT_WRITE_PROSE_WORDS: usize = 45;

/// Classification of a deterministic quality gate violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateCategory {
    Identity,
    ThetaMath,
    CarbonSchema,
    WarmPaperTheme,
    KatexDiscipline,
    Pedagogy,
    CleanRoom,
}

impl fmt::Display for GateCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Identity => "identity",
            Self::ThetaMath => "theta_math",
            Self::CarbonSchema => "carbon_schema",
            Self::WarmPaperTheme => "warm_paper_theme",
            Self::KatexDiscipline => "katex_discipline",
            Self::Pedagogy => "pedagogy",
            Self::CleanRoom => "clean_room",
        };
        f.write_str(s)
    }
}

/// Pinpoint diagnostic emitted when a question/solution violates a gate rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateIssue {
    pub category: GateCategory,
    pub location: String,
    pub message: String,
}

impl fmt::Display for GateIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.category, self.location, self.message)
    }
}

/// Complete deterministic verification report for a question + `9 -> 1` solution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateReport {
    pub ok: bool,
    pub question_id: String,
    pub theta_checks_run: usize,
    pub carbon_commands_checked: usize,
    pub issues: Vec<GateIssue>,
}

impl GateReport {
    fn push(
        &mut self,
        category: GateCategory,
        location: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.ok = false;
        self.issues.push(GateIssue {
            category,
            location: location.into(),
            message: message.into(),
        });
    }

    /// Convert into `Err(HydrogenError::Validation(...))` when any gate issue was recorded.
    pub fn into_result(self) -> Result<Self> {
        if self.ok {
            return Ok(self);
        }
        let mut msg = format!(
            "quality gate failed for '{}' with {} issue(s):",
            self.question_id,
            self.issues.len()
        );
        for issue in &self.issues {
            msg.push_str("\n  - ");
            msg.push_str(&issue.to_string());
        }
        Err(HydrogenError::Validation(msg))
    }
}

/// Verify a `QuestionPackage` envelope + `QuestionBlueprint` + `SolutionBlueprint` against all Phase 2.2 gates.
pub fn verify_question_package(
    pkg: &QuestionPackage,
    rulebook: Option<&CompiledRulebook>,
) -> GateReport {
    let mut report = verify_blueprints(&pkg.question, &pkg.solution, rulebook);

    if pkg.title.trim().is_empty() {
        report.push(
            GateCategory::Identity,
            "package.title",
            "question title must not be empty",
        );
    }
    if !(1..=5).contains(&pkg.difficulty) {
        report.push(
            GateCategory::Identity,
            "package.difficulty",
            format!("difficulty must be in 1..=5, got {}", pkg.difficulty),
        );
    }
    if let Ok(uid) = UniversalId::parse(&pkg.question.question_id) {
        if pkg.is_replica != uid.is_replica() {
            report.push(
                GateCategory::Identity,
                "package.isReplica",
                format!(
                    "isReplica={} disagrees with UniversalId '{}'",
                    pkg.is_replica, uid
                ),
            );
        }
        if pkg.container_kind == "paper" {
            match (&pkg.paper_id, uid.container_id()) {
                (Some(pid), Some(cid)) if pid != &cid => {
                    report.push(
                        GateCategory::Identity,
                        "package.paperId",
                        format!("paperId '{pid}' does not match question container '{cid}'"),
                    );
                }
                (None, _) => {
                    report.push(
                        GateCategory::Identity,
                        "package.paperId",
                        "containerKind == 'paper' requires paperId to be set",
                    );
                }
                _ => {}
            }
        }
    }

    // Check package-level strings for clean-room leaks as well
    check_clean_room_text(&pkg.title, "package.title", &mut report);
    for asset in &pkg.item_assets {
        check_clean_room_text(&asset.asset_id, "package.itemAssets.assetId", &mut report);
    }

    report
}

/// Verify `QuestionBlueprint` (`question_json`) and `SolutionBlueprint` (`solution_json`) directly.
pub fn verify_blueprints(
    question: &QuestionBlueprint,
    solution: &SolutionBlueprint,
    rulebook: Option<&CompiledRulebook>,
) -> GateReport {
    let mut report = GateReport {
        ok: true,
        question_id: question.question_id.clone(),
        theta_checks_run: 0,
        carbon_commands_checked: 0,
        issues: Vec::new(),
    };

    // 1. Identity & Multi-Part Structural Checks
    check_identity_and_structure(question, solution, rulebook, &mut report);

    // 2. Theta Math, Multi-Part `exportedExact` Chain & `A-E` MCQ Uniqueness Gate
    check_theta_and_mcq(question, solution, rulebook, &mut report);

    // 3. Carbon Command, Warm-Paper Theme, Stroke Width, Checkmark, KaTeX & `targetId` Gate
    check_carbon_and_katex(question, solution, &mut report);

    // 4. Pedagogical `9 -> 1` Contiguity, Anti-Repetition, Prose Caps & Clean-Room Gate
    check_pedagogy_and_clean_room(question, solution, &mut report);

    report
}

fn check_identity_and_structure(
    question: &QuestionBlueprint,
    solution: &SolutionBlueprint,
    rulebook: Option<&CompiledRulebook>,
    report: &mut GateReport,
) {
    match UniversalId::parse(&question.question_id) {
        Ok(uid) => {
            if uid.sub_address.is_some() {
                report.push(
                    GateCategory::Identity,
                    "question.questionId",
                    "questionId must not carry a `:part:S01:L09` sub-address",
                );
            }
            if !matches!(uid.item, Some(ItemSegment::Question { .. })) {
                report.push(
                    GateCategory::Identity,
                    "question.questionId",
                    format!("expected a Question (`-Q<nnn>`) UniversalId, got '{uid}'"),
                );
            }
            if let Some(rb) = rulebook
                && uid.course_id() != rb.frontmatter.course_id
            {
                report.push(
                    GateCategory::Identity,
                    "question.questionId",
                    format!(
                        "question course prefix '{}' does not match rulebook course_id '{}'",
                        uid.course_id(),
                        rb.frontmatter.course_id
                    ),
                );
            }
        }
        Err(err) => {
            report.push(
                GateCategory::Identity,
                "question.questionId",
                err.to_string(),
            );
        }
    }

    if question.question_id != solution.question_id {
        report.push(
            GateCategory::Identity,
            "solution.questionId",
            format!(
                "solution.questionId '{}' does not match question.questionId '{}'",
                solution.question_id, question.question_id
            ),
        );
    }

    if question.parts.is_empty() {
        report.push(
            GateCategory::Identity,
            "question.parts",
            "question must contain at least one part",
        );
    }
    if question.parts.len() != solution.parts.len() {
        report.push(
            GateCategory::Identity,
            "solution.parts",
            format!(
                "question has {} part(s) while solution has {} part(s)",
                question.parts.len(),
                solution.parts.len()
            ),
        );
    }

    let mut seen_parts = BTreeSet::new();
    for (idx, q_part) in question.parts.iter().enumerate() {
        if !seen_parts.insert(q_part.part_id.as_str()) {
            report.push(
                GateCategory::Identity,
                format!("question.parts[{idx}].partId"),
                format!("duplicate partId '{}'", q_part.part_id),
            );
        }
        if q_part.marks <= 0.0 {
            report.push(
                GateCategory::Identity,
                format!("question.parts[{idx}].marks"),
                format!("part marks must be > 0, got {}", q_part.marks),
            );
        }
        if let Some(sol_part) = solution.parts.get(idx)
            && sol_part.part_id != q_part.part_id
        {
            report.push(
                GateCategory::Identity,
                format!("solution.parts[{idx}].partId"),
                format!(
                    "expected partId '{}' matching question.parts[{idx}], found '{}'",
                    q_part.part_id, sol_part.part_id
                ),
            );
        }
    }
}

fn check_theta_and_mcq(
    question: &QuestionBlueprint,
    solution: &SolutionBlueprint,
    rulebook: Option<&CompiledRulebook>,
    report: &mut GateReport,
) {
    let theta_enabled = rulebook
        .map(|r| r.frontmatter.theta.enabled)
        .unwrap_or(true);
    let stage_cfg = rulebook
        .map(|r| r.frontmatter.theta.to_stage_preset())
        .unwrap_or_else(|| StagePresetConfig {
            preset: solution.theta_preset.clone(),
            ..StagePresetConfig::gyara_11plus_maths()
        });
    let theta = ThetaService::new(stage_cfg);

    // Collect `interact.ask` ChoiceChips prompts on the Left Pane for MCQ verification
    let ask_prompts: Vec<&carbon_protocol::AskParams> = question
        .left_pane_carbon_commands
        .iter()
        .filter_map(|cmd| match cmd {
            CarbonCommand::Ask(ask) if ask.kind == PromptKind::ChoiceChips => Some(ask),
            _ => None,
        })
        .collect();

    let expected_mcq_count = rulebook
        .and_then(|r| r.frontmatter.defaults.mcq_option_count)
        .unwrap_or(5);

    let mut prior_exports: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    for (part_idx, q_part) in question.parts.iter().enumerate() {
        let part_loc = format!("parts[{}]({})", part_idx, q_part.part_id);
        let Some(sol_part) = solution.parts.get(part_idx) else {
            continue;
        };

        // Resolve inherited exact bindings from `usesPriorParts`
        let mut inherited_vars: BTreeMap<String, Value> = BTreeMap::new();
        for prior_id in &sol_part.uses_prior_parts {
            match prior_exports.get(prior_id) {
                Some(exports) => {
                    for (var_name, exact_expr) in exports {
                        inherited_vars.insert(var_name.clone(), Value::String(exact_expr.clone()));
                    }
                }
                None => {
                    report.push(
                        GateCategory::ThetaMath,
                        format!("solution.{part_loc}.usesPriorParts"),
                        format!("references unknown or non-preceding partId '{prior_id}'"),
                    );
                }
            }
        }

        if theta_enabled {
            // 1. Verify `answerKey.exactValue` parses & evaluates cleanly
            report.theta_checks_run += 1;
            if let Err(err) =
                theta.eval_with_vars(&q_part.answer_key.exact_value, inherited_vars.clone())
            {
                report.push(
                    GateCategory::ThetaMath,
                    format!("question.{part_loc}.answerKey.exactValue"),
                    format!(
                        "answerKey.exactValue '{}' failed Theta evaluation: {err}",
                        q_part.answer_key.exact_value
                    ),
                );
            }

            // 2. Verify every `exportedExact` entry parses & evaluates without drift
            for (export_var, export_expr) in &sol_part.exported_exact {
                report.theta_checks_run += 1;
                if let Err(err) = theta.eval_with_vars(export_expr, inherited_vars.clone()) {
                    report.push(
                        GateCategory::ThetaMath,
                        format!("solution.{part_loc}.exportedExact.{export_var}"),
                        format!("exportedExact '{export_expr}' failed Theta evaluation: {err}"),
                    );
                }
            }

            // 3. Execute every `thetaVerification` check in every step
            for step in &sol_part.steps {
                if step.theta_verification.is_empty() {
                    report.push(
                        GateCategory::ThetaMath,
                        format!("solution.{part_loc}.{}.thetaVerification", step.step_id),
                        "math-enabled subject requires at least one thetaVerification check per step",
                    );
                }
                for (chk_idx, chk) in step.theta_verification.iter().enumerate() {
                    let chk_loc = format!(
                        "solution.{part_loc}.{}.thetaVerification[{chk_idx}]",
                        step.step_id
                    );
                    report.theta_checks_run += 1;
                    run_single_theta_check(&theta, chk, &inherited_vars, &chk_loc, report);
                }
            }
        }

        // 4. 5-Option (`A-E`) MCQ Uniqueness & Distractor Diagnostic Gate
        if question.answer_format == AnswerFormat::Mcq {
            let prompt_opt = ask_prompts.get(part_idx).or_else(|| ask_prompts.first());
            match prompt_opt {
                None => {
                    report.push(
                        GateCategory::CarbonSchema,
                        format!("question.{part_loc}.leftPaneCarbonCommands"),
                        "MCQ question must include an `interact.ask` (`kind: \"choice-chips\"`) command on the Left Pane",
                    );
                }
                Some(ask) => {
                    if ask.choices.len() != expected_mcq_count {
                        report.push(
                            GateCategory::ThetaMath,
                            format!("question.{part_loc}.choices"),
                            format!(
                                "expected {expected_mcq_count} MCQ choices, found {}",
                                ask.choices.len()
                            ),
                        );
                    }
                    let expected_ids: Vec<String> = (0..ask.choices.len())
                        .map(|i| char::from(b'A' + u8::try_from(i).unwrap_or(0)).to_string())
                        .collect();
                    let actual_ids: Vec<String> =
                        ask.choices.iter().map(|c| c.id.clone()).collect();
                    if actual_ids != expected_ids {
                        report.push(
                            GateCategory::CarbonSchema,
                            format!("question.{part_loc}.choices"),
                            format!(
                                "MCQ choice IDs must be {expected_ids:?}, found {actual_ids:?}"
                            ),
                        );
                    }

                    let Some(correct_id) = q_part.answer_key.correct_choice_id.as_deref() else {
                        report.push(
                            GateCategory::ThetaMath,
                            format!("question.{part_loc}.answerKey.correctChoiceId"),
                            "MCQ answerKey must specify correctChoiceId (e.g. 'A'..'E')",
                        );
                        prior_exports
                            .insert(q_part.part_id.clone(), sol_part.exported_exact.clone());
                        continue;
                    };

                    let Some(expected_choice_idx) =
                        ask.choices.iter().position(|c| c.id == correct_id)
                    else {
                        report.push(
                            GateCategory::ThetaMath,
                            format!("question.{part_loc}.answerKey.correctChoiceId"),
                            format!(
                                "correctChoiceId '{correct_id}' is not among choices {actual_ids:?}"
                            ),
                        );
                        prior_exports
                            .insert(q_part.part_id.clone(), sol_part.exported_exact.clone());
                        continue;
                    };

                    // Distractor diagnostics check
                    for choice in &ask.choices {
                        if choice.id == correct_id {
                            if q_part
                                .answer_key
                                .distractor_diagnostics
                                .contains_key(&choice.id)
                            {
                                report.push(
                                    GateCategory::Pedagogy,
                                    format!("question.{part_loc}.answerKey.distractorDiagnostics"),
                                    format!(
                                        "correctChoiceId '{correct_id}' must not be listed in distractorDiagnostics"
                                    ),
                                );
                            }
                        } else {
                            let has_diag = q_part
                                .answer_key
                                .distractor_diagnostics
                                .get(&choice.id)
                                .is_some_and(|d| !d.trim().is_empty());
                            if !has_diag {
                                report.push(
                                    GateCategory::Pedagogy,
                                    format!("question.{part_loc}.answerKey.distractorDiagnostics"),
                                    format!(
                                        "missing distractorDiagnostic explaining wrong option '{}'",
                                        choice.id
                                    ),
                                );
                            }
                        }
                    }

                    // Mathematical distinctness & sole exact match via Theta
                    if theta_enabled && ask.choices.len() >= 2 {
                        let expr_strings: Vec<String> = ask
                            .choices
                            .iter()
                            .map(|c| {
                                q_part
                                    .answer_key
                                    .choice_values
                                    .get(&c.id)
                                    .cloned()
                                    .unwrap_or_else(|| extract_math_from_choice_label(&c.label))
                            })
                            .collect();
                        let expr_refs: Vec<&str> =
                            expr_strings.iter().map(String::as_str).collect();
                        report.theta_checks_run += 1;
                        match theta.verify_mcq_options(&expr_refs, &q_part.answer_key.exact_value) {
                            Ok(matched_idx) => {
                                if matched_idx != expected_choice_idx {
                                    report.push(
                                        GateCategory::ThetaMath,
                                        format!("question.{part_loc}.answerKey"),
                                        format!(
                                            "Theta matched option '{}' for exactValue '{}', but correctChoiceId is '{correct_id}'",
                                            ask.choices[matched_idx].id,
                                            q_part.answer_key.exact_value
                                        ),
                                    );
                                }
                            }
                            Err(err) => {
                                report.push(
                                    GateCategory::ThetaMath,
                                    format!("question.{part_loc}.choices"),
                                    format!("MCQ Theta uniqueness gate failed: {err}"),
                                );
                            }
                        }
                    }
                }
            }
        }

        prior_exports.insert(q_part.part_id.clone(), sol_part.exported_exact.clone());
    }
}

fn run_single_theta_check(
    theta: &ThetaService,
    chk: &crate::domain::ThetaVerificationSpec,
    inherited_vars: &BTreeMap<String, Value>,
    loc: &str,
    report: &mut GateReport,
) {
    if chk.tool == "calc.eval" {
        let mut req: theta_calc::EvalRequest = match serde_json::from_value(chk.args.clone()) {
            Ok(r) => r,
            Err(e) => {
                report.push(
                    GateCategory::ThetaMath,
                    loc,
                    format!("invalid calc.eval args: {e}"),
                );
                return;
            }
        };
        for (k, v) in inherited_vars {
            req.vars.entry(k.clone()).or_insert_with(|| v.clone());
        }
        match theta.eval(&req) {
            Ok(res) => {
                if let Some(expected) = chk.expected_exact.as_deref() {
                    match theta.expressions_equivalent(&res.exact, expected) {
                        Ok(true) => {}
                        Ok(false) => {
                            report.push(
                                GateCategory::ThetaMath,
                                loc,
                                format!(
                                    "calc.eval '{}' produced exact '{}', expected '{expected}'",
                                    req.expr, res.exact
                                ),
                            );
                        }
                        Err(e) => {
                            report.push(
                                GateCategory::ThetaMath,
                                loc,
                                format!("equivalence check failed: {e}"),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                report.push(
                    GateCategory::ThetaMath,
                    loc,
                    format!("calc.eval failed: {e}"),
                );
            }
        }
        return;
    }

    if chk.tool == "calc.solve" {
        let req: theta_calc::SolveRequest = match serde_json::from_value(chk.args.clone()) {
            Ok(r) => r,
            Err(e) => {
                report.push(
                    GateCategory::ThetaMath,
                    loc,
                    format!("invalid calc.solve args: {e}"),
                );
                return;
            }
        };
        match theta.solve(&req) {
            Ok(resp) => {
                if let Some(exp_sols) = &chk.expected_solutions {
                    let branches = resp.solutions.as_deref().unwrap_or(&[]);
                    let matched = branches.iter().any(|branch| {
                        exp_sols.iter().all(|(var, exp_val)| {
                            branch.get(var).is_some_and(|actual| {
                                theta
                                    .expressions_equivalent(&actual.exact, exp_val)
                                    .unwrap_or(false)
                            })
                        })
                    });
                    if !matched {
                        report.push(
                            GateCategory::ThetaMath,
                            loc,
                            format!(
                                "calc.solve solutions {:?} did not match expectedSolutions {:?}",
                                resp.solutions, exp_sols
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                report.push(
                    GateCategory::ThetaMath,
                    loc,
                    format!("calc.solve failed: {e}"),
                );
            }
        }
        return;
    }

    // Other tools (`calc.formula`, `calc.transform`, `calc.stats`, `calc.analyze_graph`)
    match theta.dispatch_tool(&chk.tool, &chk.args) {
        Ok(out) => {
            if out.get("ok").and_then(Value::as_bool) != Some(true) {
                report.push(
                    GateCategory::ThetaMath,
                    loc,
                    format!("Theta tool '{}' returned failure: {out}", chk.tool),
                );
                return;
            }
            if let Some(expected) = chk.expected_exact.as_deref() {
                let actual_opt = out
                    .get("result")
                    .and_then(|r| r.get("exact"))
                    .and_then(Value::as_str)
                    .or_else(|| out.get("exact").and_then(Value::as_str));
                match actual_opt {
                    Some(actual) => {
                        if !theta
                            .expressions_equivalent(actual, expected)
                            .unwrap_or(false)
                        {
                            report.push(
                                GateCategory::ThetaMath,
                                loc,
                                format!(
                                    "{} produced exact '{actual}', expected '{expected}'",
                                    chk.tool
                                ),
                            );
                        }
                    }
                    None => {
                        report.push(
                            GateCategory::ThetaMath,
                            loc,
                            format!("{} output lacked an `exact` field to compare", chk.tool),
                        );
                    }
                }
            }
        }
        Err(e) => {
            report.push(
                GateCategory::ThetaMath,
                loc,
                format!("dispatch_tool '{}' failed: {e}", chk.tool),
            );
        }
    }
}

/// Extract clean evaluable math from an MCQ choice label like `"$\\pounds 38.40$"` or `"$17\\text{ cm}^2$"`.
pub fn extract_math_from_choice_label(label: &str) -> String {
    let mut s = label.trim().to_string();
    if s.starts_with('$') && s.ends_with('$') && s.len() >= 2 {
        s = s[1..s.len() - 1].trim().to_string();
    } else if s.starts_with("\\(") && s.ends_with("\\)") && s.len() >= 4 {
        s = s[2..s.len() - 2].trim().to_string();
    }

    for token in ["\\pounds", "£", "$", "\\mathbf{", "\\textbf{"] {
        if s.contains("\\mathbf{") || s.contains("\\textbf{") {
            s = strip_latex_wrapper(&s, "\\mathbf");
            s = strip_latex_wrapper(&s, "\\textbf");
        } else {
            s = s.replace(token, "");
        }
    }
    // Strip trailing `\text{...}` unit labels so pure numeric/symbolic comparison works against unitless `exactValue`
    while let Some(idx) = s.find("\\text{") {
        let after = &s[idx + 6..];
        if let Some(end_rel) = after.find('}') {
            let mut next = String::with_capacity(s.len());
            next.push_str(&s[..idx]);
            next.push_str(&after[end_rel + 1..]);
            s = next;
        } else {
            break;
        }
    }
    s = s.replace("\\,", " ").replace("\\;", " ").replace("\\%", "");
    s.trim().to_string()
}

fn strip_latex_wrapper(input: &str, cmd: &str) -> String {
    let pat = format!("{cmd}{{");
    let mut current = input.to_string();
    while let Some(start) = current.find(&pat) {
        let inner_start = start + pat.len();
        let bytes = current.as_bytes();
        let mut depth = 1usize;
        let mut pos = inner_start;
        while pos < bytes.len() && depth > 0 {
            if bytes[pos] == b'{' {
                depth += 1;
            } else if bytes[pos] == b'}' {
                depth -= 1;
            }
            pos += 1;
        }
        if depth == 0 {
            let inner = &current[inner_start..pos - 1];
            let mut rebuilt = String::with_capacity(current.len());
            rebuilt.push_str(&current[..start]);
            rebuilt.push_str(inner);
            rebuilt.push_str(&current[pos..]);
            current = rebuilt;
        } else {
            break;
        }
    }
    current
}

#[derive(Debug, Clone, Default)]
struct BoardSymbolTable {
    objects: BTreeSet<String>,
    tokens: BTreeSet<String>,
}

impl BoardSymbolTable {
    fn register_object(&mut self, id: &str) {
        if !id.is_empty() {
            self.objects.insert(id.to_string());
        }
    }

    fn register_html_ids(&mut self, obj_id: &str, text: &str) {
        for tok in extract_html_id_names(text) {
            self.tokens.insert(format!("{obj_id}:{tok}"));
            self.tokens.insert(tok);
        }
    }

    fn register_2d_sub_ids(&mut self, parent_id: &str, spec: &Value) {
        if let Some(elements) = spec.get("elements").and_then(Value::as_array) {
            for el in elements {
                if let Some(eid) = el.get("id").and_then(Value::as_str) {
                    self.objects.insert(eid.to_string());
                    self.objects.insert(format!("{parent_id}:{eid}"));
                    // Register table cell addresses `tbl:r<R>c<C>`
                    if el.get("kind").and_then(Value::as_str) == Some("table")
                        && let Some(rows) = el.get("rows").and_then(Value::as_array)
                    {
                        for (r_idx, row) in rows.iter().enumerate() {
                            if let Some(cols) = row.as_array() {
                                for c_idx in 0..cols.len() {
                                    self.tokens.insert(format!("{eid}:r{r_idx}c{c_idx}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn contains_target(&self, target_id: &str) -> bool {
        self.objects.contains(target_id) || self.tokens.contains(target_id)
    }
}

fn check_carbon_and_katex(
    question: &QuestionBlueprint,
    solution: &SolutionBlueprint,
    report: &mut GateReport,
) {
    check_katex_text(
        &question.stem_markdown_katex,
        "question.stemMarkdownKatex",
        report,
    );

    if question.left_pane_carbon_commands.is_empty() {
        report.push(
            GateCategory::CarbonSchema,
            "question.leftPaneCarbonCommands",
            "leftPaneCarbonCommands must contain at least the question stem",
        );
    }

    let mut base_table = BoardSymbolTable::default();
    let mut has_left_typeset_stem = false;
    for (idx, cmd) in question.left_pane_carbon_commands.iter().enumerate() {
        let loc = format!("question.leftPaneCarbonCommands[{idx}]");
        if let CarbonCommand::Write(w) = cmd {
            if w.pane != PaneId::Left {
                report.push(
                    GateCategory::CarbonSchema,
                    &loc,
                    format!(
                        "leftPaneCarbonCommands scene.write must target `pane: \"left\"`, got {:?}",
                        w.pane
                    ),
                );
            }
            if w.style == MathRenderStyle::Typeset {
                has_left_typeset_stem = true;
            }
        }
        validate_carbon_command(cmd, &loc, true, false, &mut base_table, report);
    }

    if !has_left_typeset_stem {
        report.push(
            GateCategory::WarmPaperTheme,
            "question.leftPaneCarbonCommands",
            "Left-Pane must include a `style: \"typeset\"` (`Lato` + `PennstanderMath`) question stem `scene.write`",
        );
    }

    for (part_idx, sol_part) in solution.parts.iter().enumerate() {
        let part_loc = format!("solution.parts[{part_idx}]({})", sol_part.part_id);
        check_katex_text(
            &sol_part.final_answer.katex,
            &format!("{part_loc}.finalAnswer.katex"),
            report,
        );

        // Track cumulative `Level 9` spine across steps `0..N` so cross-step `below` anchors resolve,
        // while validating each lower sub-level `8..min` atop the preceding steps' `Level 9` board state.
        let mut prior_steps_table = base_table.clone();

        for step in &sol_part.steps {
            let step_loc = format!("{part_loc}.{}", step.step_id);

            // Validate each populated sublevel (`9` down to `min`)
            for (&lvl, sub) in step.sublevels.iter().rev() {
                let lvl_loc = format!("{step_loc}.sublevels.{lvl}");
                let mut step_table = prior_steps_table.clone();
                for (c_idx, cmd) in sub.carbon_commands.iter().enumerate() {
                    let cmd_loc = format!("{lvl_loc}.carbonCommands[{c_idx}]");
                    validate_carbon_command(cmd, &cmd_loc, false, false, &mut step_table, report);
                }
                for tok_ref in &sub.speech_tokens {
                    if !step_table.contains_target(tok_ref) {
                        report.push(
                            GateCategory::CarbonSchema,
                            format!("{lvl_loc}.speechTokens"),
                            format!("speechToken '{tok_ref}' does not match any placed `\\htmlId` or object ID"),
                        );
                    }
                }
                if lvl == 9 {
                    prior_steps_table = step_table;
                }
            }

            // Validate `substeps[]` breakdown cards atop the step's `Level 9` state
            let mut substep_table = prior_steps_table.clone();
            for (ss_idx, ss) in step.substeps.iter().enumerate() {
                let ss_loc = format!("{step_loc}.substeps[{ss_idx}]");
                for (c_idx, cmd) in ss.carbon_commands.iter().enumerate() {
                    let cmd_loc = format!("{ss_loc}.carbonCommands[{c_idx}]");
                    validate_carbon_command(cmd, &cmd_loc, false, true, &mut substep_table, report);
                }
            }
        }

        // Validate `finalAnswer.carbonCommands` (must work both standalone on immediate "Reveal Answer" and at end of steps)
        let mut final_table = prior_steps_table.clone();
        for (c_idx, cmd) in sol_part.final_answer.carbon_commands.iter().enumerate() {
            let cmd_loc = format!("{part_loc}.finalAnswer.carbonCommands[{c_idx}]");
            validate_carbon_command(cmd, &cmd_loc, false, false, &mut final_table, report);
        }
    }
}

fn validate_carbon_command(
    cmd: &CarbonCommand,
    loc: &str,
    is_left_pane: bool,
    is_substep_breakdown: bool,
    sym: &mut BoardSymbolTable,
    report: &mut GateReport,
) {
    report.carbon_commands_checked += 1;

    match cmd {
        CarbonCommand::SetTheme(t) => {
            if t.theme == ThemeId::DarkGlass {
                report.push(
                    GateCategory::WarmPaperTheme,
                    loc,
                    "dark-glass theme is forbidden; use `warm-paper` or `classroom-white`",
                );
            }
        }
        CarbonCommand::SetSurface(s) => {
            if let Some(bg) = s.surface.background_color.as_deref()
                && !is_approved_card_fill(bg)
            {
                report.push(
                    GateCategory::WarmPaperTheme,
                    loc,
                    format!(
                        "surface backgroundColor '{bg}' violates Warm-Paper light canvas rules"
                    ),
                );
            }
        }
        CarbonCommand::SetCatalog(cat) => {
            if let Some(logo) = cat.brand_logo.as_deref() {
                validate_png_logo(logo, &format!("{loc}.brandLogo"), report);
            }
            for (i, subj) in cat.available_subjects.iter().enumerate() {
                if let Some(logo) = subj.logo.as_deref()
                    && !logo.is_empty()
                {
                    validate_png_logo(logo, &format!("{loc}.availableSubjects[{i}].logo"), report);
                }
            }
        }
        CarbonCommand::Write(w) => {
            if w.id.trim().is_empty() {
                report.push(
                    GateCategory::CarbonSchema,
                    loc,
                    "scene.write `id` must not be empty",
                );
            }
            validate_anchor_target(&w.anchor, loc, sym, report);

            if w.style == MathRenderStyle::ClassicSerif {
                report.push(
                    GateCategory::WarmPaperTheme,
                    format!("{loc}.style"),
                    "`classic-serif` is forbidden by default; use one of the 5 curated natural voices (`handwritten`, `typeset`, `casual`, `marker`, `typewriter`) with `PennstanderMath`",
                );
            }
            if is_substep_breakdown
                && !matches!(
                    w.style,
                    MathRenderStyle::Casual
                        | MathRenderStyle::Handwritten
                        | MathRenderStyle::Typeset
                )
            {
                report.push(
                    GateCategory::WarmPaperTheme,
                    format!("{loc}.style"),
                    "substeps[] breakdown cards must use `casual` (`Single Day`), `handwritten` (`Pennstander`), or `typeset` (`Lato`)",
                );
            }
            if w.content.contains("NEVER EVER") && w.style != MathRenderStyle::Typeset {
                report.push(
                    GateCategory::WarmPaperTheme,
                    format!("{loc}.style"),
                    "`NEVER EVER` Golden-Rule cards must set `style: \"typeset\"` (`Lato` clean wrap)",
                );
            }

            if let Some(sc) = w.scale
                && !(0.65..=1.50).contains(&sc)
            {
                report.push(
                    GateCategory::WarmPaperTheme,
                    format!("{loc}.scale"),
                    format!("scene.write scale {sc} out of harmonious range `0.65..=1.50`"),
                );
            }
            if let Some(deg) = w.rotation_deg {
                let normal_tilt = (-12.0..=12.0).contains(&deg);
                let vertical_axis = (deg.abs() - 90.0).abs() < 0.5;
                if !normal_tilt && !vertical_axis {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        format!("{loc}.rotationDeg"),
                        format!(
                            "scene.write rotationDeg {deg} out of harmonious tilt bounds (`-12°..+12°` or `±90°`)"
                        ),
                    );
                }
            }

            let is_solid_typewriter_stamp = w.style == MathRenderStyle::Typewriter
                && w.card
                    .as_deref()
                    .and_then(|c| c.fill_color.as_deref())
                    .is_some_and(is_solid_stamp_badge_fill);

            if let Some(color) = w.color.as_deref() {
                if is_solid_typewriter_stamp && color.trim().eq_ignore_ascii_case("#FFFFFF") {
                    // Inverted `#FFFFFF` ink is approved on solid `typewriter` stamp badges (`#1565C0`, `#991B1B`).
                } else {
                    validate_ink_color(color, &format!("{loc}.color"), report);
                }
            }
            if let Some(card) = w.card.as_deref() {
                if let Some(fill) = card.fill_color.as_deref() {
                    let fill_ok = is_approved_card_fill(fill)
                        || (w.style == MathRenderStyle::Typewriter
                            && is_solid_stamp_badge_fill(fill));
                    if !fill_ok {
                        report.push(
                            GateCategory::WarmPaperTheme,
                            format!("{loc}.card.fillColor"),
                            format!(
                                "card fillColor '{fill}' must be a Warm-Paper pastel fill (or solid `#1565C0`/`#991B1B` on `typewriter` stamps)"
                            ),
                        );
                    }
                }
                if let Some(border) = card.border_color.as_deref() {
                    validate_border_or_ink_color(
                        border,
                        &format!("{loc}.card.borderColor"),
                        report,
                    );
                }
                if let Some(hc) = card.header_color.as_deref() {
                    validate_border_or_ink_color(hc, &format!("{loc}.card.headerColor"), report);
                }
                if let Some(bw) = card.border_width_pt
                    && f64::from(bw) > MAX_DIAGRAM_STROKE_PT
                {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        format!("{loc}.card.borderWidthPt"),
                        format!("card borderWidthPt {bw} exceeds max {MAX_DIAGRAM_STROKE_PT} pt"),
                    );
                }
            }

            check_katex_text(&w.content, &format!("{loc}.content"), report);
            if !is_left_pane && !is_substep_breakdown {
                let prose_words = count_prose_words_outside_math(&w.content);
                if prose_words > MAX_RIGHT_WRITE_PROSE_WORDS {
                    report.push(
                        GateCategory::Pedagogy,
                        format!("{loc}.content"),
                        format!(
                            "Right-Pane scene.write has {prose_words} prose words outside `$...$` (max {MAX_RIGHT_WRITE_PROSE_WORDS}; move conversational prose to `narration`)"
                        ),
                    );
                }
            }

            for (s_idx, span) in w.spans.iter().enumerate() {
                if span.form == Some(TextForm::ClassicSerif) {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        format!("{loc}.spans[{s_idx}].form"),
                        "`classic-serif` form is forbidden by default; use natural `PennstanderMath` voices",
                    );
                }
                if let Some(c) = span.color.as_deref() {
                    if is_solid_typewriter_stamp && c.trim().eq_ignore_ascii_case("#FFFFFF") {
                        // Approved `#FFFFFF` stamp text on solid `typewriter` badge.
                    } else {
                        validate_ink_color(c, &format!("{loc}.spans[{s_idx}].color"), report);
                    }
                }
                check_katex_text(&span.text, &format!("{loc}.spans[{s_idx}].text"), report);
                if let Some(tid) = span.token_id.as_deref() {
                    sym.tokens.insert(format!("{}:{tid}", w.id));
                    sym.tokens.insert(tid.to_string());
                }
                sym.register_html_ids(&w.id, &span.text);
            }
            for (t_idx, ts) in w.token_styles.iter().enumerate() {
                if ts.form == Some(TextForm::ClassicSerif) {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        format!("{loc}.tokenStyles[{t_idx}].form"),
                        "`classic-serif` form is forbidden by default; use natural `PennstanderMath` voices",
                    );
                }
                if let Some(c) = ts.color.as_deref() {
                    validate_ink_color(c, &format!("{loc}.tokenStyles[{t_idx}].color"), report);
                }
            }

            sym.register_object(&w.id);
            sym.register_html_ids(&w.id, &w.content);
        }
        CarbonCommand::Place2D(p) => {
            if p.id.trim().is_empty() {
                report.push(
                    GateCategory::CarbonSchema,
                    loc,
                    "scene.place2D `id` must not be empty",
                );
            }
            validate_anchor_target(&p.anchor, loc, sym, report);
            inspect_spec_json(&p.spec, &format!("{loc}.spec"), report);
            sym.register_object(&p.id);
            sym.register_2d_sub_ids(&p.id, &p.spec);
        }
        CarbonCommand::Place3D(p) => {
            if p.id.trim().is_empty() {
                report.push(
                    GateCategory::CarbonSchema,
                    loc,
                    "scene.place3D `id` must not be empty",
                );
            }
            validate_anchor_target(&p.anchor, loc, sym, report);
            inspect_spec_json(&p.spec, &format!("{loc}.spec"), report);
            sym.register_object(&p.id);
        }
        CarbonCommand::PlaceImage(img) => {
            validate_anchor_target(&img.anchor, loc, sym, report);
            check_clean_room_text(&img.src, &format!("{loc}.src"), report);
            sym.register_object(&img.id);
            for hs in &img.tokens {
                sym.tokens.insert(format!("{}:{}", img.id, hs.token_id));
                sym.tokens.insert(hs.token_id.clone());
            }
        }
        CarbonCommand::Scribble(scr) => {
            if let Some(c) = scr.color.as_deref() {
                validate_ink_color(c, &format!("{loc}.color"), report);
            }
            sym.register_object(&scr.id);
        }
        CarbonCommand::Annotate(ann) => {
            if ann.target_ids.is_empty() {
                report.push(
                    GateCategory::CarbonSchema,
                    format!("{loc}.targetIds"),
                    "focus.annotate `targetIds` must contain at least one target",
                );
            }
            if let Some(from_id) = ann.from_id.as_deref()
                && !sym.contains_target(from_id)
            {
                report.push(
                    GateCategory::CarbonSchema,
                    format!("{loc}.fromId"),
                    format!("focus.annotate fromId '{from_id}' referenced before creation"),
                );
            }
            for tid in &ann.target_ids {
                if !sym.contains_target(tid) {
                    report.push(
                        GateCategory::CarbonSchema,
                        format!("{loc}.targetIds"),
                        format!("focus.annotate targetId '{tid}' referenced before creation"),
                    );
                }
            }
            if let Some(c) = ann.color.as_deref() {
                validate_ink_color(c, &format!("{loc}.color"), report);
            }
            if let Some(lbl) = ann.label.as_deref() {
                if ann.kind == AnnotationKind::Check
                    && (lbl.contains('✓')
                        || lbl.contains('✔')
                        || lbl.contains('✅')
                        || lbl.contains("\\checkmark"))
                {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        format!("{loc}.label"),
                        "forbidden literal checkmark inside `focus.annotate` with `kind: \"check\"` (Carbon draws the corner tick automatically)",
                    );
                }
                check_katex_text(lbl, &format!("{loc}.label"), report);
                let words = count_prose_words_outside_math(lbl);
                if words > MAX_ANNOTATION_PROSE_WORDS {
                    report.push(
                        GateCategory::Pedagogy,
                        format!("{loc}.label"),
                        format!(
                            "annotation callout has {words} prose words (max {MAX_ANNOTATION_PROSE_WORDS})"
                        ),
                    );
                }
            }
            sym.register_object(&ann.id);
        }
        CarbonCommand::Rub(rub) => {
            if rub.target_ids.is_empty() {
                report.push(
                    GateCategory::CarbonSchema,
                    format!("{loc}.targetIds"),
                    "edit.rub `targetIds` must not be empty",
                );
            }
            for tid in &rub.target_ids {
                if !sym.contains_target(tid) {
                    report.push(
                        GateCategory::CarbonSchema,
                        format!("{loc}.targetIds"),
                        format!("edit.rub targetId '{tid}' referenced before creation"),
                    );
                }
            }
            if let Some(rep) = &rub.replacement {
                if rep.style == Some(MathRenderStyle::ClassicSerif) {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        format!("{loc}.replacement.style"),
                        "`classic-serif` is forbidden by default in edit.rub replacement",
                    );
                }
                if let Some(c) = rep.color.as_deref() {
                    validate_ink_color(c, &format!("{loc}.replacement.color"), report);
                }
                check_katex_text(&rep.content, &format!("{loc}.replacement.content"), report);
                if let Some(rep_id) = rep.id.as_deref() {
                    sym.register_object(rep_id);
                    sym.register_html_ids(rep_id, &rep.content);
                }
            }
        }
        CarbonCommand::Modify(m) => {
            if !sym.contains_target(&m.target_id) {
                report.push(
                    GateCategory::CarbonSchema,
                    format!("{loc}.targetId"),
                    format!(
                        "edit.modify targetId '{}' referenced before creation",
                        m.target_id
                    ),
                );
            }
            inspect_spec_json(&m.patch, &format!("{loc}.patch"), report);
        }
        CarbonCommand::Erase(er) => {
            for tid in &er.target_ids {
                if !sym.contains_target(tid) {
                    report.push(
                        GateCategory::CarbonSchema,
                        format!("{loc}.targetIds"),
                        format!("edit.erase targetId '{tid}' referenced before creation"),
                    );
                }
            }
        }
        CarbonCommand::AttentionScale(sc) => {
            if !sym.contains_target(&sc.target_id) {
                report.push(
                    GateCategory::CarbonSchema,
                    format!("{loc}.targetId"),
                    format!(
                        "focus.scale targetId '{}' referenced before creation",
                        sc.target_id
                    ),
                );
            }
        }
        CarbonCommand::Spotlight(sp) => {
            for tid in &sp.target_ids {
                if !sym.contains_target(tid) {
                    report.push(
                        GateCategory::CarbonSchema,
                        format!("{loc}.targetIds"),
                        format!("focus.spotlight targetId '{tid}' referenced before creation"),
                    );
                }
            }
        }
        CarbonCommand::SetCardCollapsed(cc) => {
            if !sym.contains_target(&cc.id) {
                report.push(
                    GateCategory::CarbonSchema,
                    format!("{loc}.id"),
                    format!(
                        "card.setCollapsed id '{}' referenced before creation",
                        cc.id
                    ),
                );
            }
        }
        CarbonCommand::Ask(ask) => {
            validate_anchor_target(&ask.anchor, loc, sym, report);
            if let Some(lbl) = ask.label.as_deref() {
                check_katex_text(lbl, &format!("{loc}.label"), report);
            }
            for (c_idx, ch) in ask.choices.iter().enumerate() {
                check_katex_text(&ch.label, &format!("{loc}.choices[{c_idx}].label"), report);
            }
            for tid in &ask.target_ids {
                if !sym.contains_target(tid) {
                    report.push(
                        GateCategory::CarbonSchema,
                        format!("{loc}.targetIds"),
                        format!("interact.ask targetId '{tid}' referenced before creation"),
                    );
                }
            }
            sym.register_object(&ask.prompt_id);
        }
        _ => {}
    }
}

fn validate_anchor_target(
    anchor: &AnchorSpec,
    loc: &str,
    sym: &BoardSymbolTable,
    report: &mut GateReport,
) {
    let target_opt = match anchor {
        AnchorSpec::Next | AnchorSpec::AtSlot { .. } | AnchorSpec::Coords { .. } => None,
        AnchorSpec::Beside { target_id, .. }
        | AnchorSpec::Right { target_id, .. }
        | AnchorSpec::Left { target_id, .. }
        | AnchorSpec::Below { target_id, .. }
        | AnchorSpec::Above { target_id, .. }
        | AnchorSpec::AboveLeft { target_id, .. }
        | AnchorSpec::AboveRight { target_id, .. }
        | AnchorSpec::BelowLeft { target_id, .. }
        | AnchorSpec::BelowRight { target_id, .. }
        | AnchorSpec::AutoMargin { target_id, .. } => Some(target_id.as_str()),
    };
    if let Some(tid) = target_opt
        && !sym.contains_target(tid)
    {
        report.push(
            GateCategory::CarbonSchema,
            format!("{loc}.anchor.targetId"),
            format!("anchor targetId '{tid}' referenced before creation in execution order"),
        );
    }
}

fn validate_png_logo(logo: &str, loc: &str, report: &mut GateReport) {
    let lower = logo.trim().to_ascii_lowercase();
    if lower.contains("svg")
        || !(lower.starts_with("data:image/png;base64,") || lower.ends_with(".png"))
    {
        report.push(
            GateCategory::WarmPaperTheme,
            loc,
            "brand and subject logos must be strictly PNG (`data:image/png;base64,...` or `.png`), never SVG",
        );
    }
}

fn is_semantic_token(color: &str) -> bool {
    matches!(color, "c1" | "c2" | "c3" | "c4" | "c5" | "c6" | "c7" | "c8")
}

/// Approved high-contrast `c1..c8` & curated voice ink hexes (`docs/carbon-visual-theme.md` §3–§4).
fn is_approved_ink_hex(hex_upper: &str) -> bool {
    matches!(
        hex_upper,
        "#1F2421"
            | "#0F172A"
            | "#68645B"
            | "#2563EB"
            | "#1D4ED8"
            | "#0284C7"
            | "#15803D"
            | "#16A34A"
            | "#DC2626"
            | "#B91C1C"
            | "#991B1B"
            | "#9F1239"
            | "#B45309"
            | "#D97706"
            | "#A16207"
            | "#6D28D9"
            | "#7C3AED"
            | "#0D9488"
            | "#475569"
    )
}

/// Solid stamp card fills permitted exclusively on `style: "typewriter"` badges (`EXAMPLE`, `NON-CALC`, `KEY TRAP`).
fn is_solid_stamp_badge_fill(color: &str) -> bool {
    matches!(
        color.trim().to_ascii_uppercase().as_str(),
        "#1565C0" | "#0D47A1" | "#991B1B" | "#7F1D1D" | "#15803D"
    )
}

/// Approved Warm-Paper `c1..c8` card/cell pastel fills (`docs/carbon-visual-theme.md` §3–§4).
fn is_approved_card_fill(color: &str) -> bool {
    if is_semantic_token(color) {
        return true;
    }
    let up = color.trim().to_ascii_uppercase();
    matches!(
        up.as_str(),
        "#FFFFFF"
            | "#FDFBF7"
            | "#FDFCF9"
            | "#FCF9F2"
            | "#EFF6FF"
            | "#F0F9FF"
            | "#F0FDF4"
            | "#FFF1F2"
            | "#FDF2F8"
            | "#FFFBEB"
            | "#FEF9C3"
            | "#F5F3FF"
            | "#F0FDFA"
            | "#F8FAFC"
            | "#DCFCE7"
            | "#DBEAFE"
            | "#FEF3C7"
    )
}

fn is_approved_border_hex(hex_upper: &str) -> bool {
    is_approved_ink_hex(hex_upper)
        || matches!(
            hex_upper,
            "#3B82F6"
                | "#38BDF8"
                | "#F87171"
                | "#9F1239"
                | "#F59E0B"
                | "#CA8A04"
                | "#A16207"
                | "#8B5CF6"
                | "#14B8A6"
                | "#CBD5E1"
                | "#DEE4EC"
                | "#DBD2C2"
                | "#0D47A1"
                | "#1565C0"
                | "#7F1D1D"
        )
}

fn parse_hex_rgb(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    let chan = |v: u8| -> f64 {
        let s = f64::from(v) / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * chan(r) + 0.7152 * chan(g) + 0.0722 * chan(b)
}

fn validate_ink_color(color: &str, loc: &str, report: &mut GateReport) {
    let trimmed = color.trim();
    if is_semantic_token(trimmed) {
        return;
    }
    let up = trimmed.to_ascii_uppercase();
    if is_approved_ink_hex(&up) {
        return;
    }
    if let Some((r, g, b)) = parse_hex_rgb(&up) {
        let lum = relative_luminance(r, g, b);
        if lum > 0.55 {
            report.push(
                GateCategory::WarmPaperTheme,
                loc,
                format!(
                    "light/pastel/yellow/white ink '{trimmed}' (luminance {lum:.2}) is invisible on Warm-Paper; use `c1..c8` tokens"
                ),
            );
            return;
        }
    }
    report.push(
        GateCategory::WarmPaperTheme,
        loc,
        format!(
            "non-canonical ink color '{trimmed}'; restrict solution ink to `c1..c8` or exact Warm-Paper hex tokens (`docs/carbon-visual-theme.md` §3)"
        ),
    );
}

fn validate_border_or_ink_color(color: &str, loc: &str, report: &mut GateReport) {
    let trimmed = color.trim();
    if is_semantic_token(trimmed) || is_approved_border_hex(&trimmed.to_ascii_uppercase()) {
        return;
    }
    report.push(
        GateCategory::WarmPaperTheme,
        loc,
        format!(
            "non-canonical card/diagram border color '{trimmed}'; use `c1..c8` border tokens (`#3B82F6`, `#16A34A`, `#F87171`, `#F59E0B`, `#8B5CF6`, `#14B8A6`, `#CBD5E1`)"
        ),
    );
}

fn inspect_spec_json(val: &Value, loc: &str, report: &mut GateReport) {
    match val {
        Value::Object(map) => {
            let is_label_el = map.get("kind").and_then(Value::as_str) == Some("label");
            let has_low_opacity = map
                .get("fillOpacity")
                .or_else(|| map.get("opacity"))
                .and_then(Value::as_f64)
                .is_some_and(|op| op <= 0.35);

            for (k, v) in map {
                let child_loc = format!("{loc}.{k}");
                if matches!(
                    k.as_str(),
                    "strokeWidth"
                        | "lineWidth"
                        | "borderWidth"
                        | "borderWidthPt"
                        | "axisWidth"
                        | "widthPt"
                ) && let Some(w) = v.as_f64()
                    && w > MAX_DIAGRAM_STROKE_PT
                {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        &child_loc,
                        format!(
                            "diagram stroke `{k}: {w}` exceeds fine-stroke ceiling ({MAX_DIAGRAM_STROKE_PT} pt)"
                        ),
                    );
                }

                if is_label_el
                    && matches!(
                        k.as_str(),
                        "background" | "backgroundColor" | "boxFill" | "fillColor" | "fill"
                    )
                    && v.as_str()
                        .is_some_and(|s| !s.eq_ignore_ascii_case("transparent") && !s.is_empty())
                {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        &child_loc,
                        "diagram labels must be transparent (opaque label background boxes clip Carbon's grid)",
                    );
                }

                if matches!(k.as_str(), "backgroundColor" | "canvasFill")
                    && let Some(c) = v.as_str()
                    && !is_approved_card_fill(c)
                {
                    report.push(
                        GateCategory::WarmPaperTheme,
                        &child_loc,
                        format!("dark or non-Warm-Paper diagram background '{c}' is forbidden"),
                    );
                }

                if matches!(k.as_str(), "fill" | "fillColor")
                    && let Some(c) = v.as_str()
                    && !c.eq_ignore_ascii_case("transparent")
                    && !c.eq_ignore_ascii_case("none")
                    && !c.starts_with('@')
                {
                    // Semi-transparent shape fills (`fill: "#2563EB", fillOpacity: 0.14`) may use `c1..c8` ink/border colors;
                    // opaque card/cell fills must be soft pastels (`luminance >= 0.85`).
                    let ok = is_approved_card_fill(c)
                        || (has_low_opacity
                            && (is_semantic_token(c)
                                || is_approved_border_hex(&c.to_ascii_uppercase())));
                    if !ok
                        && let Some((r, g, b)) = parse_hex_rgb(c)
                        && relative_luminance(r, g, b) < 0.50
                    {
                        report.push(
                            GateCategory::WarmPaperTheme,
                            &child_loc,
                            format!(
                                "opaque dark fill '{c}' is forbidden on Warm-Paper (use soft pastel fill or `fillOpacity <= 0.30`)"
                            ),
                        );
                    }
                }

                if matches!(k.as_str(), "color" | "stroke" | "textColor")
                    && let Some(c) = v.as_str()
                    && !c.starts_with('@')
                {
                    validate_border_or_ink_color(c, &child_loc, report);
                }

                if matches!(
                    k.as_str(),
                    "label" | "text" | "totalLabel" | "xLabel" | "yLabel" | "caption"
                ) && let Some(txt) = v.as_str()
                {
                    check_katex_text(txt, &child_loc, report);
                }

                if matches!(k.as_str(), "headers" | "vertexLabels")
                    && let Some(arr) = v.as_array()
                {
                    for (i, item) in arr.iter().enumerate() {
                        if let Some(txt) = item.as_str() {
                            check_katex_text(txt, &format!("{child_loc}[{i}]"), report);
                        }
                    }
                }

                inspect_spec_json(v, &child_loc, report);
            }
        }
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                inspect_spec_json(item, &format!("{loc}[{idx}]"), report);
            }
        }
        _ => {}
    }
}

/// Verify balanced `$...$` / `\(...\)` math delimiters and reject raw ASCII math leaks outside `$...$`.
pub fn check_katex_text(text: &str, loc: &str, report: &mut GateReport) {
    let Some(prose_segments) = split_prose_outside_math(text) else {
        report.push(
            GateCategory::KatexDiscipline,
            loc,
            format!("unbalanced `$...$` or `\\(...\\)` math delimiter in: {text}"),
        );
        return;
    };

    for seg in prose_segments {
        if let Some(leak) = detect_ascii_math_leak(&seg) {
            report.push(
                GateCategory::KatexDiscipline,
                loc,
                format!("raw ASCII math leak `{leak}` outside `$...$` in: {text}"),
            );
            break;
        }
    }
}

/// Split `text` into prose slices outside `$...$`, `$$...$$`, `\(...\)`, and `\[...\]`.
/// Returns `None` if delimiters are unbalanced.
fn split_prose_outside_math(text: &str) -> Option<Vec<String>> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut prose = Vec::new();
    let mut buf = String::new();

    while i < len {
        // Escaped `\$`
        if bytes[i] == b'\\' && i + 1 < len && bytes[i + 1] == b'$' {
            buf.push_str("\\$");
            i += 2;
            continue;
        }
        // `\(` ... `\)`
        if bytes[i] == b'\\' && i + 1 < len && bytes[i + 1] == b'(' {
            prose.push(std::mem::take(&mut buf));
            i += 2;
            let mut closed = false;
            while i + 1 < len {
                if bytes[i] == b'\\' && bytes[i + 1] == b')' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return None;
            }
            continue;
        }
        // `\[` ... `\]`
        if bytes[i] == b'\\' && i + 1 < len && bytes[i + 1] == b'[' {
            prose.push(std::mem::take(&mut buf));
            i += 2;
            let mut closed = false;
            while i + 1 < len {
                if bytes[i] == b'\\' && bytes[i + 1] == b']' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return None;
            }
            continue;
        }
        // `$$...$$` or `$...$`
        if bytes[i] == b'$' {
            prose.push(std::mem::take(&mut buf));
            let is_double = i + 1 < len && bytes[i + 1] == b'$';
            i += if is_double { 2 } else { 1 };
            let mut closed = false;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                    continue;
                }
                if is_double {
                    if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'$' {
                        i += 2;
                        closed = true;
                        break;
                    }
                } else if bytes[i] == b'$' {
                    i += 1;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return None;
            }
            continue;
        }

        if let Some(ch) = text[i..].chars().next() {
            buf.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    if !buf.is_empty() {
        prose.push(buf);
    }
    Some(prose)
}

fn detect_ascii_math_leak(prose: &str) -> Option<String> {
    // Strip reactive `@eval(...)` expressions before checking prose
    let cleaned = strip_reactive_eval(prose);
    let lower = cleaned.to_ascii_lowercase();

    for pat in [
        "sqrt(", "cbrt(", "!=", "<=", ">=", "\\frac", "\\times", "\\div", "\\htmlid", "\\pounds",
    ] {
        if lower.contains(pat) {
            return Some(pat.to_string());
        }
    }

    // Check caret exponent (`x^2`, `10^3`, `cm^2`)
    let bytes = cleaned.as_bytes();
    for idx in 0..bytes.len() {
        if bytes[idx] == b'^' {
            let prev = idx.checked_sub(1).map(|p| bytes[p]);
            let next = bytes.get(idx + 1).copied();
            if prev.is_some_and(|b| b.is_ascii_alphanumeric() || b == b')')
                && next.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'{')
            {
                return Some("^".to_string());
            }
        }
        // Bare numeric or single-letter fraction (`3/5`, `17/4`, `x/2`) outside `$...$`
        if bytes[idx] == b'/' && idx > 0 && idx + 1 < bytes.len() {
            let l_start = scan_alnum_left(&bytes[..idx]);
            let r_end = idx + 1 + scan_alnum_right(&bytes[idx + 1..]);
            let left_tok = &cleaned[l_start..idx];
            let right_tok = &cleaned[idx + 1..r_end];
            let left_is_num_or_var = !left_tok.is_empty()
                && (left_tok.bytes().all(|b| b.is_ascii_digit()) || left_tok.len() == 1);
            let right_is_num_or_var = !right_tok.is_empty()
                && (right_tok.bytes().all(|b| b.is_ascii_digit()) || right_tok.len() == 1);
            if left_is_num_or_var && right_is_num_or_var {
                return Some(format!("{left_tok}/{right_tok}"));
            }
        }
        // Bare `=` between digits/variables outside `$...$`
        if bytes[idx] == b'=' {
            let left = cleaned[..idx].trim_end();
            let right = cleaned[idx + 1..].trim_start();
            let l_math = left
                .bytes()
                .last()
                .is_some_and(|b| b.is_ascii_digit() || b == b')');
            let r_math = right
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_digit() || b == b'-' || b == b'(');
            if l_math && r_math {
                return Some("=".to_string());
            }
        }
    }

    None
}

fn strip_reactive_eval(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(start) = out.find("@eval(") {
        let bytes = out.as_bytes();
        let mut depth = 1usize;
        let mut pos = start + 6;
        while pos < bytes.len() && depth > 0 {
            if bytes[pos] == b'(' {
                depth += 1;
            } else if bytes[pos] == b')' {
                depth -= 1;
            }
            pos += 1;
        }
        let mut next = String::with_capacity(out.len());
        next.push_str(&out[..start]);
        next.push_str(&out[pos..]);
        out = next;
    }
    out
}

fn scan_alnum_left(slice: &[u8]) -> usize {
    let mut i = slice.len();
    while i > 0 && slice[i - 1].is_ascii_alphanumeric() {
        i -= 1;
    }
    i
}

fn scan_alnum_right(slice: &[u8]) -> usize {
    let mut i = 0;
    while i < slice.len() && slice[i].is_ascii_alphanumeric() {
        i += 1;
    }
    i
}

fn extract_html_id_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let pat = "\\htmlId{";
    let mut rest = text;
    while let Some(idx) = rest.find(pat) {
        let after = &rest[idx + pat.len()..];
        if let Some(close) = after.find('}') {
            let tok = after[..close].trim();
            if !tok.is_empty() {
                names.push(tok.to_string());
            }
            rest = &after[close + 1..];
        } else {
            break;
        }
    }
    names
}

fn count_prose_words_outside_math(text: &str) -> usize {
    let Some(segments) = split_prose_outside_math(text) else {
        return text.split_whitespace().count();
    };
    segments
        .iter()
        .map(|s| {
            s.split_whitespace()
                .filter(|w| w.bytes().any(|b| b.is_ascii_alphabetic()))
                .count()
        })
        .sum()
}

fn check_pedagogy_and_clean_room(
    question: &QuestionBlueprint,
    solution: &SolutionBlueprint,
    report: &mut GateReport,
) {
    // 1. Clean-room anonymization scan across full serialized blueprints
    if let Ok(q_raw) = serde_json::to_string(question) {
        check_clean_room_text(&q_raw, "question_json", report);
    }
    if let Ok(s_raw) = serde_json::to_string(solution) {
        check_clean_room_text(&s_raw, "solution_json", report);
    }

    // 2. Pedagogical `9 -> min_sublevel` contiguity, `substeps` consistency & anti-repetition
    for (p_idx, part) in solution.parts.iter().enumerate() {
        let part_loc = format!("solution.parts[{p_idx}]({})", part.part_id);
        if part.steps.is_empty() {
            report.push(
                GateCategory::Pedagogy,
                format!("{part_loc}.steps"),
                "each solution part must contain at least one step",
            );
        }
        if part.final_answer.carbon_commands.is_empty() {
            report.push(
                GateCategory::Pedagogy,
                format!("{part_loc}.finalAnswer.carbonCommands"),
                "finalAnswer must contain at least one Carbon command",
            );
        }

        for (s_idx, step) in part.steps.iter().enumerate() {
            let step_loc = format!("{part_loc}.{}", step.step_id);
            if step.step_index != s_idx {
                report.push(
                    GateCategory::Pedagogy,
                    format!("{step_loc}.stepIndex"),
                    format!("expected stepIndex {s_idx}, got {}", step.step_index),
                );
            }
            if step.step_number != u32::try_from(s_idx + 1).unwrap_or(1) {
                report.push(
                    GateCategory::Pedagogy,
                    format!("{step_loc}.stepNumber"),
                    format!(
                        "expected stepNumber {}, got {}",
                        s_idx + 1,
                        step.step_number
                    ),
                );
            }
            if step.step_goal.trim().is_empty() {
                report.push(
                    GateCategory::Pedagogy,
                    format!("{step_loc}.stepGoal"),
                    "stepGoal must not be empty",
                );
            }

            if step.breakdown_available && step.substeps.is_empty() {
                report.push(
                    GateCategory::Pedagogy,
                    format!("{step_loc}.substeps"),
                    "breakdownAvailable is true, but substeps[] is empty",
                );
            }
            if !step.breakdown_available && !step.substeps.is_empty() {
                report.push(
                    GateCategory::Pedagogy,
                    format!("{step_loc}.breakdownAvailable"),
                    "substeps[] is populated, so breakdownAvailable must be true",
                );
            }

            if step.sublevels.is_empty() {
                report.push(
                    GateCategory::Pedagogy,
                    format!("{step_loc}.sublevels"),
                    "step must populate sublevels starting at 9",
                );
                continue;
            }

            let max_lvl = step.sublevels.keys().copied().max().unwrap_or(0);
            let min_lvl = step.sublevels.keys().copied().min().unwrap_or(0);
            if max_lvl != 9 {
                report.push(
                    GateCategory::Pedagogy,
                    format!("{step_loc}.sublevels"),
                    format!("every step must start at Level 9 (`max_sublevel = 9`), got {max_lvl}"),
                );
            }
            for expected_lvl in min_lvl..=max_lvl {
                if !step.sublevels.contains_key(&expected_lvl) {
                    report.push(
                        GateCategory::Pedagogy,
                        format!("{step_loc}.sublevels"),
                        format!(
                            "non-contiguous sublevels: missing Level {expected_lvl} between {max_lvl} and {min_lvl}"
                        ),
                    );
                }
            }

            let mut prev_lvl_sig: Option<(u8, String, String)> = None;
            for (&lvl, sub) in step.sublevels.iter().rev() {
                let lvl_loc = format!("{step_loc}.sublevels.{lvl}");
                if sub.narration.trim().is_empty() {
                    report.push(
                        GateCategory::Pedagogy,
                        format!("{lvl_loc}.narration"),
                        "narration script for SIGMA must not be empty",
                    );
                }
                if sub.carbon_commands.is_empty() {
                    report.push(
                        GateCategory::Pedagogy,
                        format!("{lvl_loc}.carbonCommands"),
                        "every populated sub-level must render visual Carbon commands",
                    );
                }
                let cmd_sig = serde_json::to_string(&sub.carbon_commands).unwrap_or_default();
                let nar_sig = sub.narration.trim().to_ascii_lowercase();
                if let Some((prev_lvl, prev_cmds, prev_nar)) = &prev_lvl_sig {
                    if &cmd_sig == prev_cmds {
                        report.push(
                            GateCategory::Pedagogy,
                            &lvl_loc,
                            format!(
                                "Anti-Repetition violation: Level {lvl} has identical carbonCommands to Level {prev_lvl}"
                            ),
                        );
                    } else if &nar_sig == prev_nar {
                        report.push(
                            GateCategory::Pedagogy,
                            &lvl_loc,
                            format!(
                                "Anti-Repetition violation: Level {lvl} has identical narration to Level {prev_lvl}"
                            ),
                        );
                    }
                }
                prev_lvl_sig = Some((lvl, cmd_sig, nar_sig));
            }
        }
    }
}

fn check_clean_room_text(text: &str, loc: &str, report: &mut GateReport) {
    let lower = text.to_ascii_lowercase();
    let forbidden_markers = [
        ".pdf",
        "inbox/",
        "archive/",
        "ledger.json",
        "status.html",
        "private-xref",
        "practice-paper-",
        "north london",
        "consortium",
        "gl assessment",
        "cem 11+",
        "iseb",
        "haberdashers",
        "csse",
        "bond 11+",
        "edexcel",
        "aqa ",
    ];
    for marker in forbidden_markers {
        if lower.contains(marker) {
            report.push(
                GateCategory::CleanRoom,
                loc,
                format!(
                    "clean-room anonymization violation: forbidden external/source marker '{marker}' detected"
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RuleLoader;
    use std::path::PathBuf;

    fn sample_valid_package() -> Result<QuestionPackage> {
        let raw = r##"{
            "containerKind": "paper",
            "paperId": "GYR-MTH-P001",
            "seqNo": 14,
            "label": "Q14",
            "title": "Reverse Percentage Sale Coat",
            "questionTypeId": "GYR-MTH-QT-FDP",
            "difficulty": 3,
            "calculatorMode": "non-calculator",
            "recommendedTimeSec": 60,
            "isReplica": false,
            "qaStatus": "verified",
            "topics": [{ "topicId": "GYR-MTH-T_FDP", "isPrimary": true }],
            "question": {
                "schemaVersion": "1.0",
                "questionId": "GYR-MTH-P001-Q014",
                "answerFormat": "mcq",
                "stemMarkdownKatex": "In a sale, all prices are reduced by $20\\%$. A coat now costs $\\pounds 48$.\n\nWhat was the original price of the coat?",
                "dictionary": [
                    { "term": "reduced", "definition": "Lowered in price from the original amount." }
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
                            "content": "In a sale, all prices are reduced by $20\\%$. A coat now costs $\\pounds 48$.\n\nWhat was the original price of the coat?"
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
                                "A": "Subtracted 20% of £48 instead of reversing.",
                                "B": "Added 20% of £48 (classic reverse-percentage trap).",
                                "C": "Added £10 instead of scaling.",
                                "E": "Added 30% of £48."
                            }
                        }
                    }
                ]
            },
            "solution": {
                "schemaVersion": "1.0",
                "questionId": "GYR-MTH-P001-Q014",
                "thetaPreset": "uk_gcse",
                "parts": [
                    {
                        "partId": "main",
                        "usesPriorParts": [],
                        "exportedExact": { "original_price": "60" },
                        "finalAnswer": {
                            "katex": "$\\pounds 60.00$",
                            "carbonCommands": [
                                {
                                    "op": "scene.write",
                                    "params": {
                                        "id": "ans_final",
                                        "pane": "right",
                                        "zone": "spine",
                                        "style": "typeset",
                                        "color": "#15803D",
                                        "content": "Original price $= \\mathbf{\\pounds 60.00}$ **(Option D)**",
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
                                "stepGoal": "Form algebraic equation for 80% of Original P",
                                "breakdownAvailable": true,
                                "substeps": [
                                    {
                                        "substepIndex": 0,
                                        "narration": "100 percent minus 20 percent leaves 80 percent, which is four fifths.",
                                        "carbonCommands": [
                                            {
                                                "op": "scene.write",
                                                "params": {
                                                    "id": "s1_sub0",
                                                    "pane": "right",
                                                    "zone": "scratch",
                                                    "style": "typeset",
                                                    "content": "Remaining fraction: $100\\% - 20\\% = 80\\% = \\frac{4}{5}$.",
                                                    "card": { "headerBanner": "WHY 4/5?", "fillColor": "#EFF6FF", "borderColor": "#3B82F6", "borderStyle": "solid" }
                                                }
                                            }
                                        ]
                                    }
                                ],
                                "prerequisiteCardIds": [],
                                "thetaVerification": [
                                    { "tool": "calc.solve", "args": { "equations": ["(4/5) * P = 48"], "solve_for": ["P"] }, "expectedSolutions": { "P": "60" } }
                                ],
                                "sublevels": {
                                    "9": {
                                        "archetype": "expected_exam_step",
                                        "narration": "Let P be the original price. A 20 percent reduction leaves four fifths of P equal to £48, so P equals £60.",
                                        "speechTokens": ["s1_l9_eq:frac45", "s1_l9_eq:ans60"],
                                        "carbonCommands": [
                                            {
                                                "op": "scene.write",
                                                "params": {
                                                    "id": "s1_l9_eq",
                                                    "pane": "right",
                                                    "zone": "spine",
                                                    "style": "typeset",
                                                    "content": "Let Original $= P$:\\quad $\\htmlId{frac45}{\\frac{4}{5}P} = 48 \\implies P = 48 \\times \\frac{5}{4} = \\htmlId{ans60}{\\pounds 60}$"
                                                }
                                            }
                                        ]
                                    },
                                    "8": {
                                        "archetype": "guided_exam_step",
                                        "narration": "Clear the denominator 5 and divide by 4, then check that 20 percent off £60 gives £48.",
                                        "carbonCommands": [
                                            {
                                                "op": "scene.write",
                                                "params": {
                                                    "id": "s1_l8_eq",
                                                    "pane": "right",
                                                    "zone": "spine",
                                                    "style": "typeset",
                                                    "content": "$\\frac{4}{5}P = 48 \\xrightarrow{\\div 4} \\frac{1}{5}P = 12 \\xrightarrow{\\times 5} P = \\pounds 60$"
                                                }
                                            },
                                            {
                                                "op": "focus.annotate",
                                                "params": {
                                                    "id": "s1_l8_chk",
                                                    "targetIds": ["s1_l8_eq"],
                                                    "kind": "label",
                                                    "color": "c2",
                                                    "label": "Check: $\\pounds 60 - \\pounds 12 = \\pounds 48$"
                                                }
                                            }
                                        ]
                                    },
                                    "7": {
                                        "archetype": "spatial_bar_bridge",
                                        "narration": "Draw 5 equal fifths for P. Four shaded boxes equal £48, so each box is £12 and all 5 boxes make £60.",
                                        "carbonCommands": [
                                            {
                                                "op": "scene.place2D",
                                                "params": {
                                                    "id": "s1_l7_bar",
                                                    "pane": "right",
                                                    "zone": "diagram",
                                                    "spec": {
                                                        "macro": "barModel",
                                                        "totalLabel": "Original $P$ ($5 \\times \\pounds 12 = \\pounds 60$)",
                                                        "rows": [
                                                            [
                                                                { "value": 4, "label": "$\\pounds 48$ ($80\\%$)" },
                                                                { "value": 1, "label": "$20\\%$ off" }
                                                            ]
                                                        ]
                                                    }
                                                }
                                            }
                                        ]
                                    }
                                }
                            }
                        ]
                    }
                ]
            }
        }"##;
        Ok(serde_json::from_str(raw)?)
    }

    #[test]
    fn passes_valid_11plus_question_and_catches_all_gate_violations() -> Result<()> {
        let loader = RuleLoader::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rules"));
        let rules = loader.load("gyaraplus", "maths", None)?;

        let valid = sample_valid_package()?;
        let ok_report = verify_question_package(&valid, Some(&rules)).into_result()?;
        assert!(ok_report.theta_checks_run >= 3);
        assert!(ok_report.carbon_commands_checked >= 6);

        // 1. Duplicate MCQ option + wrong math answer must fail ThetaMath
        let mut bad_mcq = sample_valid_package()?;
        bad_mcq.question.parts[0].answer_key.exact_value = "57.6".into();
        let rep = verify_question_package(&bad_mcq, Some(&rules));
        assert!(!rep.ok);
        assert!(
            rep.issues
                .iter()
                .any(|i| i.category == GateCategory::ThetaMath)
        );

        // 2. Raw ASCII math leak + duplicate `✓` in `"kind": "check"` + thick stroke > 2.2 pt + dark card fill
        let mut bad_visual = sample_valid_package()?;
        bad_visual.question.stem_markdown_katex = "Solve x^2 + 3/5 = 10.".into();
        bad_visual.solution.parts[0]
            .final_answer
            .carbon_commands
            .push(serde_json::from_value(serde_json::json!({
                "op": "focus.annotate",
                "params": {
                    "id": "dup_chk",
                    "targetIds": ["ans_final"],
                    "kind": "check",
                    "color": "#FFFF00",
                    "label": "Verified ✓"
                }
            }))?);
        bad_visual.solution.parts[0].steps[0]
            .sublevels
            .get_mut(&7)
            .ok_or_else(|| HydrogenError::Validation("missing L7".into()))?
            .carbon_commands
            .push(serde_json::from_value(serde_json::json!({
                "op": "scene.place2D",
                "params": {
                    "id": "thick_diag",
                    "pane": "right",
                    "spec": { "strokeWidth": 4.0, "backgroundColor": "#111827" }
                }
            }))?);

        let vrep = verify_question_package(&bad_visual, Some(&rules));
        assert!(!vrep.ok);
        assert!(
            vrep.issues
                .iter()
                .any(|i| i.category == GateCategory::KatexDiscipline)
        );
        assert!(
            vrep.issues
                .iter()
                .filter(|i| i.category == GateCategory::WarmPaperTheme)
                .count()
                >= 3
        );

        // 3. Non-contiguous sublevels (`9, 8, 5` missing `7, 6`) + clean-room PDF leak
        let mut bad_ped = sample_valid_package()?;
        bad_ped.title = "From Practice-Paper-01.pdf GL Assessment".into();
        let l7 = bad_ped.solution.parts[0].steps[0]
            .sublevels
            .remove(&7)
            .ok_or_else(|| HydrogenError::Validation("missing L7".into()))?;
        bad_ped.solution.parts[0].steps[0].sublevels.insert(5, l7);
        let prep = verify_question_package(&bad_ped, Some(&rules));
        assert!(!prep.ok);
        assert!(
            prep.issues
                .iter()
                .any(|i| i.category == GateCategory::Pedagogy)
        );
        assert!(
            prep.issues
                .iter()
                .any(|i| i.category == GateCategory::CleanRoom)
        );

        Ok(())
    }

    #[test]
    fn validates_five_curated_font_voices_and_rejects_classic_serif() -> Result<()> {
        let loader = RuleLoader::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rules"));
        let rules = loader.load("gyaraplus", "maths", None)?;

        let mut pkg = sample_valid_package()?;

        // Reference Example 4A on Left Pane: `typewriter` stamp + `typeset` stem + `handwritten` E.g.
        pkg.question.left_pane_carbon_commands = serde_json::from_str(
            r##"[
              {
                "op": "scene.write",
                "params": {
                  "id": "q-stamp",
                  "pane": "left",
                  "zone": "spine",
                  "style": "typewriter",
                  "rotationDeg": -3.5,
                  "spans": [{ "text": "EXAMPLE", "color": "#FFFFFF", "sizePt": 15.0, "form": "typewriter" }],
                  "card": { "fillColor": "#1565C0", "borderColor": "#0D47A1", "borderStyle": "solid", "borderWidthPt": 1.4 }
                }
              },
              {
                "op": "scene.write",
                "params": {
                  "id": "q-stem",
                  "pane": "left",
                  "zone": "spine",
                  "anchor": { "mode": "below", "targetId": "q-stamp", "offsetPt": [0, 10] },
                  "style": "typeset",
                  "color": "#0F172A",
                  "content": "In a sale, all prices are reduced by $20\\%$. A coat now costs $\\pounds 48$."
                }
              },
              {
                "op": "scene.write",
                "params": {
                  "id": "q-eg",
                  "pane": "left",
                  "zone": "spine",
                  "anchor": { "mode": "below", "targetId": "q-stem", "offsetPt": [14, 10] },
                  "style": "handwritten",
                  "scale": 1.05,
                  "content": "$\\textcolor{#0284C7}{\\text{E.g. } 100\\% - 20\\% = }\\htmlId{eg-ans}{\\textcolor{#DC2626}{80\\%}}$"
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
            ]"##,
        )?;

        // Reference Example 4B on Right Pane: `handwritten` step + `casual` tilted `#FEF9C3` RE-CHECK card + `marker` headline
        pkg.solution.parts[0].steps[0]
            .sublevels
            .get_mut(&8)
            .ok_or_else(|| HydrogenError::Validation("missing L8".into()))?
            .carbon_commands = serde_json::from_str(
            r##"[
              {
                "op": "scene.write",
                "params": {
                  "id": "sol-head",
                  "pane": "right",
                  "zone": "spine",
                  "style": "marker",
                  "scale": 1.05,
                  "color": "#1D4ED8",
                  "content": "Reverse Percentage Unitary Scaling"
                }
              },
              {
                "op": "scene.write",
                "params": {
                  "id": "sol-s1",
                  "pane": "right",
                  "zone": "spine",
                  "anchor": { "mode": "below", "targetId": "sol-head", "offsetPt": [0, 8] },
                  "style": "handwritten",
                  "color": "#1F2421",
                  "content": "$\\frac{4}{5}P = 48 \\implies P = 48 \\div 4 \\times 5 = \\htmlId{sol-sixty}{\\textcolor{#15803D}{\\pounds 60}}$"
                }
              },
              {
                "op": "scene.write",
                "params": {
                  "id": "note-recheck",
                  "pane": "right",
                  "zone": "scratch",
                  "anchor": { "mode": "auto-margin", "targetId": "sol-sixty", "prefer": "right", "autoConnector": true },
                  "style": "casual",
                  "rotationDeg": 1.5,
                  "scale": 0.90,
                  "content": "Quick check: $20\\%$ of $\\pounds 60 = \\pounds 12$, and $\\pounds 60 - \\pounds 12 = \\pounds 48$!",
                  "card": {
                    "fillColor": "#FEF9C3",
                    "borderColor": "#CA8A04",
                    "borderStyle": "dashed",
                    "headerBanner": "RE-CHECK SALE",
                    "headerColor": "#A16207",
                    "collapsible": true
                  }
                }
              }
            ]"##,
        )?;

        let ok_rep = verify_question_package(&pkg, Some(&rules)).into_result()?;
        assert!(ok_rep.ok);

        // Verify `classic-serif` (or alias `classic-tex`) is deterministically rejected!
        pkg.solution.parts[0].steps[0]
            .sublevels
            .get_mut(&9)
            .ok_or_else(|| HydrogenError::Validation("missing L9".into()))?
            .carbon_commands
            .push(serde_json::from_str(
                r#"{
                    "op": "scene.write",
                    "params": {
                        "id": "bad_serif",
                        "pane": "right",
                        "style": "classic-serif",
                        "content": "$x = 60$"
                    }
                }"#,
            )?);
        let serif_rep = verify_question_package(&pkg, Some(&rules));
        assert!(!serif_rep.ok);
        assert!(
            serif_rep
                .issues
                .iter()
                .any(|i| i.message.contains("classic-serif"))
        );

        Ok(())
    }
}
