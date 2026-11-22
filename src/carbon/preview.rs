//! Atomic Carbon Split-Whiteboard preview batch builder for a compiled `CourseQuestion`
//! at any selected `(part_id, step_id, sublevel, substep, reveal_answer)`.

use carbon_protocol::{
    CalculatorMode, CarbonCommand, ChromePatch, ChromeTimerParams, ChromeVisibility, ClearParams,
    GridPitch, ImportantIconStyle, LayoutMode, PartTab, SetCatalogOpenParams, SetDictionaryParams,
    SetLayoutParams, SetSurfaceParams, SetThemeParams, SurfaceKind, SurfaceSpec, ThemeId,
    TimelineParams, TimerControlOp, TimerMode, TimerTarget,
};
use memory_core_rs::CourseQuestion;
use serde_json::json;

use crate::domain::{QuestionBlueprint, SolutionBlueprint};
use crate::error::{HydrogenError, Result};

/// Selection coordinates for previewing a compiled question on Carbon.
#[derive(Debug, Clone)]
pub struct PreviewSelection {
    pub part_id: Option<String>,
    pub step_id: Option<String>,
    pub level: u8,
    pub substep: Option<u32>,
    pub reveal_answer: bool,
}

impl Default for PreviewSelection {
    fn default() -> Self {
        Self {
            part_id: None,
            step_id: None,
            level: 9,
            substep: None,
            reveal_answer: false,
        }
    }
}

/// Summary of the resolved `(part, step, sublevel)` frame built for Carbon.
#[derive(Debug, Clone)]
pub struct BuiltPreviewBatch {
    pub question_id: String,
    pub part_id: String,
    pub step_id: String,
    pub step_index: usize,
    pub total_steps: usize,
    pub level: u8,
    pub archetype: String,
    pub narration: String,
    pub commands: Vec<CarbonCommand>,
}

/// Build an atomic `CarbonCommand` batch rendering the Left-Pane question stem + MCQ choices,
/// synchronized 3-Group Bottom Bar `chrome.update`, prior steps (`Level 9`), and the target
/// `(step, sublevel)` (+ optional `substep` breakdown and `finalAnswer`).
pub fn build_question_preview_batch(
    cq: &CourseQuestion,
    sel: &PreviewSelection,
) -> Result<BuiltPreviewBatch> {
    if !(1..=9).contains(&sel.level) {
        return Err(HydrogenError::Validation(format!(
            "preview sublevel must be within 1..=9, got {}",
            sel.level
        )));
    }

    let q_bp: QuestionBlueprint = serde_json::from_value(cq.question_json.clone())?;
    let s_bp: SolutionBlueprint = serde_json::from_value(cq.solution_json.clone())?;

    let part_idx = match sel.part_id.as_deref() {
        Some(pid) => q_bp
            .parts
            .iter()
            .position(|p| p.part_id == pid)
            .ok_or_else(|| {
                HydrogenError::Validation(format!(
                    "part '{pid}' not found in question '{}'",
                    cq.question_id
                ))
            })?,
        None => 0,
    };

    let q_part = q_bp.parts.get(part_idx).ok_or_else(|| {
        HydrogenError::Validation(format!("question '{}' has no parts", cq.question_id))
    })?;
    let s_part = s_bp.parts.get(part_idx).ok_or_else(|| {
        HydrogenError::Validation(format!(
            "solution '{}' missing part '{}'",
            cq.question_id, q_part.part_id
        ))
    })?;

    if s_part.steps.is_empty() {
        return Err(HydrogenError::Validation(format!(
            "solution part '{}' has 0 steps",
            s_part.part_id
        )));
    }

    let step_idx = match sel.step_id.as_deref() {
        Some(sid) => s_part
            .steps
            .iter()
            .position(|st| {
                st.step_id.eq_ignore_ascii_case(sid)
                    || sid
                        .parse::<u32>()
                        .is_ok_and(|n| n == st.step_number)
            })
            .ok_or_else(|| {
                HydrogenError::Validation(format!(
                    "step '{sid}' not found in '{}' part '{}'",
                    cq.question_id, s_part.part_id
                ))
            })?,
        None => 0,
    };

    let target_step = &s_part.steps[step_idx];
    let target_sublevel = target_step.sublevels.get(&sel.level).ok_or_else(|| {
        let available: Vec<String> = target_step
            .sublevels
            .keys()
            .rev()
            .map(u8::to_string)
            .collect();
        HydrogenError::Validation(format!(
            "sublevel {} not populated on {}:{} (available: [{}])",
            sel.level,
            cq.question_id,
            target_step.step_id,
            available.join(", ")
        ))
    })?;

    let part_tabs: Vec<PartTab> = if q_bp.parts.len() > 1 {
        q_bp.parts
            .iter()
            .map(|p| PartTab {
                id: p.part_id.clone(),
                label: p
                    .label
                    .clone()
                    .unwrap_or_else(|| format!("({})", p.part_id)),
            })
            .collect()
    } else {
        Vec::new()
    };

    let calc_mode = match cq.calculator_mode.as_str() {
        "allowed" => CalculatorMode::Allowed,
        _ => CalculatorMode::NonCalculator,
    };
    let marks_str = if (q_part.marks - q_part.marks.round()).abs() < 1e-6 {
        let m = q_part.marks.round() as i64;
        if m == 1 {
            "1 mark".to_string()
        } else {
            format!("{m} marks")
        }
    } else {
        format!("{:.1} marks", q_part.marks)
    };

    let mut commands = Vec::with_capacity(16);

    // 1. Dismiss Full-Stage Catalog overlay, clear board, and configure Warm-Paper Split stage
    commands.push(CarbonCommand::SetCatalogOpen(SetCatalogOpenParams {
        open: false,
        ..SetCatalogOpenParams::default()
    }));
    commands.push(CarbonCommand::Clear(ClearParams::default()));
    commands.push(CarbonCommand::SetTheme(SetThemeParams {
        theme: ThemeId::WarmPaper,
        overrides: None,
    }));
    commands.push(CarbonCommand::SetLayout(SetLayoutParams {
        mode: Some(LayoutMode::Split),
        split_ratio: Some(0.34),
        left_collapsed: Some(false),
        ..SetLayoutParams::default()
    }));
    commands.push(CarbonCommand::SetSurface(SetSurfaceParams {
        pane: carbon_protocol::PaneId::Left,
        surface: SurfaceSpec {
            kind: SurfaceKind::Plain,
            scrollable: true,
            ..SurfaceSpec::default()
        },
    }));
    commands.push(CarbonCommand::SetSurface(SetSurfaceParams {
        pane: carbon_protocol::PaneId::Right,
        surface: SurfaceSpec {
            kind: SurfaceKind::Squared,
            grid_pitch: GridPitch::Medium,
            major_grid: true,
            scrollable: true,
            ..SurfaceSpec::default()
        },
    }));

    // 2. Synchronize Header & 3-Group Bottom Bar Chrome (`authorMode: true`)
    commands.push(CarbonCommand::ChromeUpdate(ChromePatch {
        mode: Some(ChromeVisibility::Full),
        title: Some(format!("{} · {}", cq.label, cq.title)),
        active_item_id: Some(cq.question_id.clone()),
        paper_badge: cq.paper_id.clone(),
        item_label: Some(cq.label.clone()),
        marks_chip: Some(marks_str),
        important_icon: Some(ImportantIconStyle::Triangle),
        dictionary_enabled: Some(!q_bp.dictionary.is_empty()),
        catalog_button_visible: Some(true),
        author_mode: Some(true),
        active_step_id: Some(target_step.step_id.clone()),
        author_meta: Some(json!({
            "sublevel": sel.level,
            "archetype": target_sublevel.archetype,
        })),
        part_tabs: Some(part_tabs),
        active_part_id: Some(q_part.part_id.clone()),
        total_steps: Some(u32::try_from(s_part.steps.len()).unwrap_or(1)),
        active_step: Some(u32::try_from(step_idx).unwrap_or(0)),
        active_substep: Some(sel.substep),
        breakdown_available: Some(target_step.breakdown_available),
        answer_revealed: Some(sel.reveal_answer),
        caption_visible: Some(true),
        caption_text: Some(target_sublevel.narration.clone()),
        calculator_mode: Some(calc_mode),
        pinned_references: Some(q_bp.pinned_references.clone()),
        ..ChromePatch::default()
    }));

    // 3. Optional hover vocabulary dictionary & per-item exam timer
    if !q_bp.dictionary.is_empty() {
        commands.push(CarbonCommand::SetDictionary(SetDictionaryParams {
            enabled: Some(true),
            entries: q_bp.dictionary.clone(),
        }));
    }
    if let Some(sec) = cq.recommended_time_sec.and_then(|s| u32::try_from(s.max(1)).ok()) {
        commands.push(CarbonCommand::ChromeTimer(ChromeTimerParams {
            target: TimerTarget::Item,
            op: TimerControlOp::Configure {
                mode: TimerMode::Countdown,
                duration_sec: Some(sec),
                auto_start: false,
            },
        }));
    }

    // 4. Left-Pane Question Stem, Stimulus Diagrams & `A-E` Choice Chips
    commands.extend(q_bp.left_pane_carbon_commands.iter().cloned());

    // 5. Right-Pane Prior Steps (`0..step_idx` at `Level 9`) + Target `(step_idx, sel.level)`
    for prior_step in &s_part.steps[..step_idx] {
        if let Some(l9) = prior_step.sublevels.get(&9) {
            commands.extend(l9.carbon_commands.iter().cloned());
            commands.push(CarbonCommand::Timeline(TimelineParams::Step));
        }
    }
    commands.extend(target_sublevel.carbon_commands.iter().cloned());

    // 6. Optional `substeps[]` breakdown card(s)
    if let Some(sub_up_to) = sel.substep {
        let limit = (sub_up_to as usize).saturating_add(1).min(target_step.substeps.len());
        for ss in &target_step.substeps[..limit] {
            commands.push(CarbonCommand::Timeline(TimelineParams::Substep));
            commands.extend(ss.carbon_commands.iter().cloned());
        }
    }

    // 7. Optional `finalAnswer` card & check tick
    if sel.reveal_answer {
        commands.extend(s_part.final_answer.carbon_commands.iter().cloned());
    }

    Ok(BuiltPreviewBatch {
        question_id: cq.question_id.clone(),
        part_id: q_part.part_id.clone(),
        step_id: target_step.step_id.clone(),
        step_index: step_idx,
        total_steps: s_part.steps.len(),
        level: sel.level,
        archetype: target_sublevel.archetype.clone(),
        narration: target_sublevel.narration.clone(),
        commands,
    })
}
