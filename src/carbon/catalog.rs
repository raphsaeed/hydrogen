//! Zero-overhead projection from `memory_core_rs::CourseCatalogSnapshot` into
//! `carbon_protocol::SetCatalogParams` (`displayMode: "full-stage"`, soft light `warm-paper` palette + `GYR-LOGO-BLUE`).

use std::collections::BTreeMap;

use carbon_protocol::{
    CalculatorMode, CatalogBreakdownBar, CatalogConceptCard, CatalogConceptKind,
    CatalogDisplayMode, CatalogFilterTag, CatalogPaper, CatalogQuestionCard, CatalogStatMetric,
    CatalogStatsView, CatalogSubjectItem, CatalogTab, CatalogTagBadge, CatalogTopicNode,
    CatalogViewLevel, ImportantIconStyle, ProgressStatus, ReminderInterval, SetCatalogParams,
};
use memory_core_rs::CourseCatalogSnapshot;

/// Canonical 12-subject muted mineral palette (`docs/carbon-visual-theme.md` §2.2).
/// Explicitly assigning from this 12-colour palette ensures multi-subject grids (up to 12 subjects,
/// rendered `4 x 3` at `268 x 104 pt`) never repeat Carbon's 6-colour built-in fallback.
pub const MINERAL_SUBJECT_PALETTE_12: [&str; 12] = [
    "#5B7DB1", // 1. Slate Blue (Maths / Quantitative)
    "#4D8B7A", // 2. Eucalyptus Sage (English / Literature)
    "#7A6BA8", // 3. Wisteria Lavender (Verbal Reasoning / Languages)
    "#A8647A", // 4. Dusty Rose (Non-Verbal Reasoning / Biology)
    "#A37E58", // 5. Warm Ochre (History / Humanities)
    "#568A9E", // 6. Mist Cyan (Physics / Computing)
    "#6B8E67", // 7. Moss Olive (Geography)
    "#8F6B9E", // 8. Muted Plum (Chemistry / Further Maths)
    "#B07462", // 9. Soft Terracotta (Design & Economics)
    "#5C768D", // 10. Steel Indigo (Statistics / Mechanics)
    "#8C7D53", // 11. Antique Brass (Classics / RS)
    "#4F8A8B", // 12. Deep Seafoam (Combined Science / Music)
];

/// Return the canonical muted mineral hex colour for a subject code or fallback index (`0..12`).
pub fn mineral_color_for_subject(subject_code: &str, fallback_idx: usize) -> &'static str {
    match subject_code.to_ascii_uppercase().as_str() {
        "MTH" | "MATHS" => MINERAL_SUBJECT_PALETTE_12[0],
        "ENG" | "ENGLISH" => MINERAL_SUBJECT_PALETTE_12[1],
        "VR" | "VERBAL-REASONING" => MINERAL_SUBJECT_PALETTE_12[2],
        "NVR" | "NON-VERBAL-REASONING" | "BIO" | "BIOLOGY" => MINERAL_SUBJECT_PALETTE_12[3],
        "HIS" | "HISTORY" => MINERAL_SUBJECT_PALETTE_12[4],
        "PHY" | "PHYSICS" | "CS" => MINERAL_SUBJECT_PALETTE_12[5],
        "GEO" | "GEOGRAPHY" => MINERAL_SUBJECT_PALETTE_12[6],
        "CHM" | "CHEM" | "CHEMISTRY" | "FM" => MINERAL_SUBJECT_PALETTE_12[7],
        "ECO" | "DT" => MINERAL_SUBJECT_PALETTE_12[8],
        "STA" | "MECH" => MINERAL_SUBJECT_PALETTE_12[9],
        "RS" | "LAT" => MINERAL_SUBJECT_PALETTE_12[10],
        "SCI" | "MUS" => MINERAL_SUBJECT_PALETTE_12[11],
        _ => MINERAL_SUBJECT_PALETTE_12[fallback_idx % MINERAL_SUBJECT_PALETTE_12.len()],
    }
}

/// Build Carbon's Full-Stage Course Gallery (`chrome.setCatalog`) payload
/// from an `L7` `CourseCatalogSnapshot` and embedded `brandLogo` data URI (`GYR-LOGO-BLUE`).
///
/// Note: Learner statistics (`status`, `important`, `reminder`, `due_reminder`) are intentionally
/// left at neutral defaults here because `<product>-<subject>.raphs` stores zero learner state
/// (SIGMA merges `<student>.raphs` at student runtime; Hydrogen populates Author QA metrics in `stats`).
pub fn project_full_stage_catalog(
    snap: &CourseCatalogSnapshot,
    brand_logo_data_uri: Option<String>,
) -> SetCatalogParams {
    let course_id = snap
        .meta
        .as_ref()
        .map(|m| m.course_id.clone())
        .unwrap_or_else(|| "GYR-MTH".to_string());
    let vault_title = snap.meta.as_ref().map(|m| m.title.clone());
    let vault_subtitle = snap.meta.as_ref().and_then(|m| m.subtitle.clone());

    let subject_color = snap
        .meta
        .as_ref()
        .and_then(|m| m.config_json.get("branding"))
        .and_then(|b| b.get("subjectColor"))
        .and_then(|v| v.as_str())
        .unwrap_or("#5B7DB1")
        .to_string();

    // Map topic ID -> title for card labels
    let topic_titles: BTreeMap<&str, &str> = snap
        .topics
        .iter()
        .map(|t| (t.topic_id.as_str(), t.title.as_str()))
        .collect();

    // Group paper questions by `paper_id`
    let mut questions_by_paper: BTreeMap<&str, Vec<CatalogQuestionCard>> = BTreeMap::new();
    let mut question_ids_by_topic: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut standalone_topic_questions: BTreeMap<&str, Vec<CatalogQuestionCard>> = BTreeMap::new();

    for q in &snap.questions {
        let mut tags = Vec::new();
        if q.is_replica {
            tags.push("ai".to_string());
        } else {
            tags.push("real".to_string());
        }

        let topic_labels: Vec<String> = q
            .topic_ids
            .iter()
            .filter_map(|tid| topic_titles.get(tid.as_str()).map(|s| (*s).to_string()))
            .collect();

        let marks_u32 = if q.marks > 0.0 {
            Some(q.marks.round() as u32)
        } else {
            None
        };
        let difficulty_u8 = u8::try_from(q.difficulty.clamp(1, 5)).ok();
        let open_comments_u32 = u32::try_from(q.open_comment_count.max(0)).unwrap_or(0);

        let card = CatalogQuestionCard {
            id: q.question_id.clone(),
            label: q.label.clone(),
            title: q.title.clone(),
            sublabel: Some(format!("p{}", q.seq_no.max(1))),
            badge: if q.is_replica {
                Some("AI".to_string())
            } else {
                None
            },
            marks: marks_u32,
            difficulty: difficulty_u8,
            color: None,
            tags,
            topic_ids: q.topic_ids.clone(),
            topic_labels,
            status: ProgressStatus::Default,
            important: false,
            reminder: ReminderInterval::Off,
            due_reminder: false,
            open_comment_count: open_comments_u32,
        };

        for tid in &q.topic_ids {
            question_ids_by_topic
                .entry(tid.as_str())
                .or_default()
                .push(q.question_id.clone());
        }

        if let Some(pid) = q.paper_id.as_deref() {
            questions_by_paper.entry(pid).or_default().push(card);
        } else if let Some(tid) = q.primary_topic_id.as_deref() {
            standalone_topic_questions
                .entry(tid)
                .or_default()
                .push(card);
        }
    }

    let papers: Vec<CatalogPaper> = snap
        .papers
        .iter()
        .map(|p| {
            let items = questions_by_paper
                .remove(p.paper_id.as_str())
                .unwrap_or_default();
            let q_count = u32::try_from(items.len()).unwrap_or(0);
            let open_comments: u32 = items
                .iter()
                .map(|i| i.open_comment_count)
                .fold(0u32, u32::saturating_add);

            let calc_mode = match p.calculator_mode.as_str() {
                "allowed" => Some(CalculatorMode::Allowed),
                "non-calculator" => Some(CalculatorMode::NonCalculator),
                _ => None,
            };

            let mut tag_badges = Vec::new();
            let mut tags = Vec::new();
            if p.is_replica {
                tag_badges.push(CatalogTagBadge {
                    label: "AI".to_string(),
                    color: Some("#6D5CAE".to_string()),
                });
                tags.push("ai".to_string());
            } else {
                tags.push("real".to_string());
            }

            CatalogPaper {
                id: p.paper_id.clone(),
                title: p.title.clone(),
                badge: p.badge.clone(),
                tag_badges,
                tags,
                color: None,
                duration_sec: p.duration_sec.and_then(|d| u32::try_from(d.max(0)).ok()),
                total_marks: p.total_marks.map(|m| m.round() as u32),
                calculator_mode: calc_mode,
                question_count: q_count,
                attempted_count: 0,
                completed_count: 0,
                due_count: 0,
                has_important: false,
                progress_label: None,
                open_comment_count: open_comments,
                items,
            }
        })
        .collect();

    // Group concept cards by `topic_id`
    let mut cards_by_topic: BTreeMap<&str, Vec<CatalogConceptCard>> = BTreeMap::new();
    for c in &snap.topic_cards {
        if let Some(tid) = c.topic_id.as_deref() {
            let kind = match c.card_kind.as_str() {
                "worked_example" => CatalogConceptKind::Example,
                "formula" | "reference_sheet" => CatalogConceptKind::Formula,
                _ => CatalogConceptKind::Concept,
            };
            cards_by_topic
                .entry(tid)
                .or_default()
                .push(CatalogConceptCard {
                    id: c.card_id.clone(),
                    kind,
                    title: c.title.clone(),
                    badge: Some(c.card_kind.clone()),
                    color: None,
                    completed: false,
                    open_comment_count: u32::try_from(c.open_comment_count.max(0)).unwrap_or(0),
                });
        }
    }

    let topics: Vec<CatalogTopicNode> = snap
        .topics
        .iter()
        .map(|t| {
            let item_ids = question_ids_by_topic
                .remove(t.topic_id.as_str())
                .unwrap_or_default();
            let standalone = standalone_topic_questions
                .remove(t.topic_id.as_str())
                .unwrap_or_default();
            let q_count = u32::try_from(item_ids.len() + standalone.len()).unwrap_or(0);
            let cards = cards_by_topic
                .remove(t.topic_id.as_str())
                .unwrap_or_default();

            CatalogTopicNode {
                id: t.topic_id.clone(),
                parent_id: t.parent_topic_id.clone(),
                title: t.title.clone(),
                badge: Some(t.code.clone()),
                tags: Vec::new(),
                color: None,
                question_count: q_count,
                completed_count: 0,
                due_count: 0,
                has_important: false,
                mastery_pct: None,
                cards,
                item_ids,
                items: standalone,
            }
        })
        .collect();

    // Gyara+ soft light-theme Subject Hub Cards (matching Carbon's WarmPaper gallery palette;
    // `logo: None` lets Carbon inherit `SetCatalogParams.brand_logo` = `gyara-blue.png` cleanly)
    let available_subjects = if course_id.starts_with("GYR-") {
        vec![
            CatalogSubjectItem {
                id: "GYR-MTH".to_string(),
                title: "Maths".to_string(),
                badge: Some("11+".to_string()),
                color: Some(mineral_color_for_subject("MTH", 0).to_string()),
                logo: None,
                progress_text: Some(format!("{} Papers", papers.len())),
                due_count: 0,
            },
            CatalogSubjectItem {
                id: "GYR-ENG".to_string(),
                title: "English".to_string(),
                badge: Some("11+".to_string()),
                color: Some(mineral_color_for_subject("ENG", 1).to_string()),
                logo: None,
                progress_text: None,
                due_count: 0,
            },
            CatalogSubjectItem {
                id: "GYR-VR".to_string(),
                title: "Verbal Reasoning".to_string(),
                badge: Some("11+".to_string()),
                color: Some(mineral_color_for_subject("VR", 2).to_string()),
                logo: None,
                progress_text: None,
                due_count: 0,
            },
            CatalogSubjectItem {
                id: "GYR-NVR".to_string(),
                title: "Non-Verbal Reasoning".to_string(),
                badge: Some("11+".to_string()),
                color: Some(mineral_color_for_subject("NVR", 3).to_string()),
                logo: None,
                progress_text: None,
                due_count: 0,
            },
        ]
    } else {
        vec![CatalogSubjectItem {
            id: course_id.clone(),
            title: vault_title.clone().unwrap_or_else(|| course_id.clone()),
            badge: None,
            color: Some(subject_color.clone()),
            logo: None,
            progress_text: Some(format!("{} Papers", papers.len())),
            due_count: 0,
        }]
    };

    // Author QA metrics for Hydrogen Author Preview
    let total_q = snap.questions.len();
    let verified_q = snap
        .questions
        .iter()
        .filter(|q| q.qa_status == "verified" || q.qa_status == "approved")
        .count();
    let replica_q = snap.questions.iter().filter(|q| q.is_replica).count();

    let stats = CatalogStatsView {
        headline: Some("Hydrogen Authoring QA Overview".to_string()),
        metrics: vec![
            CatalogStatMetric {
                id: "total-questions".to_string(),
                label: "Total Questions".to_string(),
                value: total_q.to_string(),
                subtext: Some(format!("{} papers · {} topics", papers.len(), topics.len())),
                color: Some(subject_color.clone()),
            },
            CatalogStatMetric {
                id: "verified-questions".to_string(),
                label: "Theta & Gate Verified".to_string(),
                value: format!("{verified_q}/{total_q}"),
                subtext: Some(format!("{replica_q} AI replicas")),
                color: Some("#15803D".to_string()),
            },
            CatalogStatMetric {
                id: "open-comments".to_string(),
                label: "Open Comments".to_string(),
                value: snap.total_open_comments.to_string(),
                subtext: Some("Author feedback queue".to_string()),
                color: if snap.total_open_comments > 0 {
                    Some("#DC2626".to_string())
                } else {
                    Some("#68645B".to_string())
                },
            },
        ],
        breakdowns: topics
            .iter()
            .map(|t| {
                let ratio = if total_q > 0 {
                    (t.question_count as f32) / (total_q as f32)
                } else {
                    0.0
                };
                CatalogBreakdownBar {
                    id: t.id.clone(),
                    label: t.title.clone(),
                    value_text: format!("{} Qs", t.question_count),
                    ratio,
                    color: Some(subject_color.clone()),
                }
            })
            .collect(),
    };

    let initial_view_level = if papers.is_empty() {
        CatalogViewLevel::Subjects
    } else {
        CatalogViewLevel::SubjectRoot
    };
    let initial_tab = if papers.is_empty() && !topics.is_empty() {
        CatalogTab::Topics
    } else {
        CatalogTab::Papers
    };

    SetCatalogParams {
        open: true,
        display_mode: CatalogDisplayMode::FullStage,
        view_level: Some(initial_view_level),
        width_pt: None,
        active_tab: initial_tab,
        active_subject_id: Some(course_id),
        active_paper_id: None,
        active_topic_id: None,
        active_item_id: None,
        auto_collapse_on_select: true,
        important_icon: Some(ImportantIconStyle::Triangle),
        brand_logo: brand_logo_data_uri,
        vault_title,
        vault_subtitle,
        tab_labels: None,
        filter_tags: vec![
            CatalogFilterTag {
                id: "real".to_string(),
                label: "Real papers".to_string(),
            },
            CatalogFilterTag {
                id: "ai".to_string(),
                label: "AI practice".to_string(),
            },
        ],
        available_subjects,
        papers,
        topics,
        stats: Some(stats),
        author_actions: vec!["create-replica".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn twelve_subject_mineral_palette_has_twelve_distinct_hex_colors() {
        let unique: BTreeSet<&str> = MINERAL_SUBJECT_PALETTE_12.iter().copied().collect();
        assert_eq!(unique.len(), 12);
        for idx in 0..12 {
            let c = mineral_color_for_subject("CUSTOM", idx);
            assert!(c.starts_with('#') && c.len() == 7);
        }
    }
}
