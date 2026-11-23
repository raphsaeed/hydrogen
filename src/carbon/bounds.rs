//! Deterministic `BoardStateSnapshot` (`getState(includeTokens: true)`) AABB collision
//! and out-of-pane clipping inspector (`Phase 3.3`).

use std::fmt;

use carbon_protocol::{
    BoardStateSnapshot, LayoutMode, NormalizedRect, PaneId, SceneObjectKind, SceneObjectSummary,
};
use serde::{Deserialize, Serialize};

use crate::error::{HydrogenError, Result};

/// Edge tolerance (`1.5%` of pane span) accommodating glyph ascenders, drop-shadows, and hairlines.
const PANE_CLIP_EPS: f32 = 0.015;

/// Margin tolerance around a parent `SceneObjectSummary` for `\htmlId` sub-token boxes.
const TOKEN_PARENT_EPS: f32 = 0.025;

/// Minimum axis intersection (`0.8%` of pane span) before two solid objects are treated as colliding.
const SOLID_OVERLAP_MIN_AXIS: f32 = 0.008;

/// Minimum fraction (`4%`) of the smaller solid object's area covered before flagging an overlap.
const SOLID_OVERLAP_MIN_AREA_RATIO: f32 = 0.04;

/// Minimum axis intersection (`0.6%`) for sibling `\htmlId` sub-token collisions.
const TOKEN_OVERLAP_MIN_AXIS: f32 = 0.006;

/// Minimum area ratio (`15%`) for non-nested sibling `\htmlId` sub-token collisions.
const TOKEN_OVERLAP_MIN_AREA_RATIO: f32 = 0.15;

/// Category of spatial layout issue detected on Carbon's live stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundsIssueKind {
    OutOfPaneClipping,
    ObjectOverlap,
    TokenOutOfObject,
    TokenOverlap,
}

impl fmt::Display for BoundsIssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::OutOfPaneClipping => "out_of_pane_clipping",
            Self::ObjectOverlap => "object_overlap",
            Self::TokenOutOfObject => "token_out_of_object",
            Self::TokenOverlap => "token_overlap",
        };
        f.write_str(s)
    }
}

/// Pinpoint spatial collision or clipping diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundsIssue {
    pub kind: BoundsIssueKind,
    pub pane: PaneId,
    pub primary_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_id: Option<String>,
    pub bounds_a: NormalizedRect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds_b: Option<NormalizedRect>,
    pub message: String,
}

impl fmt::Display for BoundsIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{} @ {:?}] {}: {}",
            self.kind, self.pane, self.primary_id, self.message
        )
    }
}

/// Deterministic layout bounds inspection summary for a rendered Carbon board state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundsCheckReport {
    pub ok: bool,
    pub objects_checked: usize,
    pub tokens_checked: usize,
    pub issues: Vec<BoundsIssue>,
}

impl BoundsCheckReport {
    fn push(&mut self, issue: BoundsIssue) {
        self.ok = false;
        self.issues.push(issue);
    }

    /// Convert into `Err(HydrogenError::Validation(...))` when any clipping or collision issue occurred.
    pub fn into_result(self) -> Result<Self> {
        if self.ok {
            return Ok(self);
        }
        let mut msg = format!(
            "Carbon layout bounds check failed with {} issue(s) across {} objects ({} tokens):",
            self.issues.len(),
            self.objects_checked,
            self.tokens_checked
        );
        for issue in &self.issues {
            msg.push_str("\n  - ");
            msg.push_str(&issue.to_string());
        }
        Err(HydrogenError::Validation(msg))
    }
}

/// Inspect `snapshot.objects` (and nested `\htmlId` `tokens`) for out-of-pane clipping and
/// unexpected AABB collisions.
///
/// Note: `SceneObjectKind::Annotation` and `SceneObjectKind::Scribble` intentionally overlay target
/// equations/diagrams and are therefore checked for pane clipping rather than solid-object collision.
pub fn inspect_board_bounds(
    snapshot: &BoardStateSnapshot,
    pane_filter: Option<PaneId>,
) -> BoundsCheckReport {
    let mut report = BoundsCheckReport {
        ok: true,
        objects_checked: 0,
        tokens_checked: 0,
        issues: Vec::new(),
    };

    let target_pane = pane_filter.map(|p| canonicalize_pane(p, snapshot.layout.mode));
    let mut filtered_objects: Vec<(PaneId, &SceneObjectSummary)> = Vec::new();

    for obj in &snapshot.objects {
        let eff_pane = canonicalize_pane(obj.pane, snapshot.layout.mode);
        if let Some(tp) = target_pane
            && eff_pane != tp
        {
            continue;
        }
        filtered_objects.push((eff_pane, obj));
    }

    for &(eff_pane, obj) in &filtered_objects {
        report.objects_checked += 1;
        let scrollable = pane_is_scrollable(snapshot, eff_pane);
        let b = obj.bounds;

        // 1. Object degenerate or out-of-pane clipping check
        if b.w <= 0.0 || b.h <= 0.0 {
            report.push(BoundsIssue {
                kind: BoundsIssueKind::OutOfPaneClipping,
                pane: eff_pane,
                primary_id: obj.id.clone(),
                secondary_id: None,
                bounds_a: b,
                bounds_b: None,
                message: format!(
                    "degenerate non-positive object bounds ({:.3}x{:.3})",
                    b.w, b.h
                ),
            });
        } else {
            let clips_left = b.x < -PANE_CLIP_EPS;
            let clips_right = b.x + b.w > 1.0 + PANE_CLIP_EPS;
            let clips_top = b.y < -PANE_CLIP_EPS;
            let clips_bottom = if scrollable {
                b.y + b.h > 4.0
            } else {
                b.y + b.h > 1.0 + PANE_CLIP_EPS
            };

            if clips_left || clips_right || clips_top || clips_bottom {
                report.push(BoundsIssue {
                    kind: BoundsIssueKind::OutOfPaneClipping,
                    pane: eff_pane,
                    primary_id: obj.id.clone(),
                    secondary_id: None,
                    bounds_a: b,
                    bounds_b: None,
                    message: format!(
                        "object clips pane bounds [x={:.3}..{:.3}, y={:.3}..{:.3}] (scrollable={scrollable})",
                        b.x,
                        b.x + b.w,
                        b.y,
                        b.y + b.h
                    ),
                });
            }
        }

        // 2. Sub-token (`\htmlId`) clipping & non-nested sibling overlap checks
        for tok in &obj.tokens {
            report.tokens_checked += 1;
            let tb = tok.bounds;
            let full_tok_id = format!("{}:{}", obj.id, tok.token_id);

            if tb.w <= 0.0 || tb.h <= 0.0 {
                report.push(BoundsIssue {
                    kind: BoundsIssueKind::TokenOutOfObject,
                    pane: eff_pane,
                    primary_id: full_tok_id,
                    secondary_id: Some(obj.id.clone()),
                    bounds_a: tb,
                    bounds_b: Some(b),
                    message: format!(
                        "degenerate non-positive token bounds ({:.3}x{:.3})",
                        tb.w, tb.h
                    ),
                });
                continue;
            }

            if tb.x < -PANE_CLIP_EPS || tb.x + tb.w > 1.0 + PANE_CLIP_EPS || tb.y < -PANE_CLIP_EPS {
                report.push(BoundsIssue {
                    kind: BoundsIssueKind::OutOfPaneClipping,
                    pane: eff_pane,
                    primary_id: full_tok_id.clone(),
                    secondary_id: None,
                    bounds_a: tb,
                    bounds_b: None,
                    message: format!(
                        "token clips pane bounds [x={:.3}..{:.3}, y={:.3}..{:.3}]",
                        tb.x,
                        tb.x + tb.w,
                        tb.y,
                        tb.y + tb.h
                    ),
                });
            }

            if tb.x < b.x - TOKEN_PARENT_EPS
                || tb.x + tb.w > b.x + b.w + TOKEN_PARENT_EPS
                || tb.y < b.y - TOKEN_PARENT_EPS
                || tb.y + tb.h > b.y + b.h + TOKEN_PARENT_EPS
            {
                report.push(BoundsIssue {
                    kind: BoundsIssueKind::TokenOutOfObject,
                    pane: eff_pane,
                    primary_id: full_tok_id,
                    secondary_id: Some(obj.id.clone()),
                    bounds_a: tb,
                    bounds_b: Some(b),
                    message: format!(
                        "token box [{:.3}, {:.3}, {:.3}x{:.3}] escapes parent object '{}' [{:.3}, {:.3}, {:.3}x{:.3}]",
                        tb.x, tb.y, tb.w, tb.h, obj.id, b.x, b.y, b.w, b.h
                    ),
                });
            }
        }

        for i in 0..obj.tokens.len() {
            for j in (i + 1)..obj.tokens.len() {
                let ta = &obj.tokens[i];
                let tb = &obj.tokens[j];
                if ta.token_id == tb.token_id {
                    continue;
                }
                // Nested `\htmlId{outer}{... \htmlId{inner}{...}}` is valid when one contains the other
                if rect_contains(ta.bounds, tb.bounds, 0.005)
                    || rect_contains(tb.bounds, ta.bounds, 0.005)
                {
                    continue;
                }
                if let Some((iw, ih, ratio)) = intersection_metrics(ta.bounds, tb.bounds)
                    && iw > TOKEN_OVERLAP_MIN_AXIS
                    && ih > TOKEN_OVERLAP_MIN_AXIS
                    && ratio > TOKEN_OVERLAP_MIN_AREA_RATIO
                {
                    report.push(BoundsIssue {
                        kind: BoundsIssueKind::TokenOverlap,
                        pane: eff_pane,
                        primary_id: format!("{}:{}", obj.id, ta.token_id),
                        secondary_id: Some(format!("{}:{}", obj.id, tb.token_id)),
                        bounds_a: ta.bounds,
                        bounds_b: Some(tb.bounds),
                        message: format!(
                            "sibling tokens '{}' and '{}' overlap ({:.1}% of smaller token)",
                            ta.token_id,
                            tb.token_id,
                            ratio * 100.0
                        ),
                    });
                }
            }
        }
    }

    // 3. Pairwise solid-object AABB collision check on the same canonical pane
    for i in 0..filtered_objects.len() {
        let (pane_a, obj_a) = filtered_objects[i];
        if !is_solid_kind(obj_a.kind) {
            continue;
        }
        for &(pane_b, obj_b) in &filtered_objects[(i + 1)..] {
            if pane_a != pane_b || !is_solid_kind(obj_b.kind) {
                continue;
            }
            if let Some((iw, ih, ratio)) = intersection_metrics(obj_a.bounds, obj_b.bounds)
                && iw > SOLID_OVERLAP_MIN_AXIS
                && ih > SOLID_OVERLAP_MIN_AXIS
                && ratio > SOLID_OVERLAP_MIN_AREA_RATIO
            {
                report.push(BoundsIssue {
                    kind: BoundsIssueKind::ObjectOverlap,
                    pane: pane_a,
                    primary_id: obj_a.id.clone(),
                    secondary_id: Some(obj_b.id.clone()),
                    bounds_a: obj_a.bounds,
                    bounds_b: Some(obj_b.bounds),
                    message: format!(
                        "solid objects '{}' ({:?}) and '{}' ({:?}) collide by {:.3}x{:.3} ({:.1}% of smaller item)",
                        obj_a.id,
                        obj_a.kind,
                        obj_b.id,
                        obj_b.kind,
                        iw,
                        ih,
                        ratio * 100.0
                    ),
                });
            }
        }
    }

    report
}

const fn canonicalize_pane(pane: PaneId, mode: LayoutMode) -> PaneId {
    match (mode, pane) {
        (LayoutMode::Split, PaneId::Board) => PaneId::Right,
        (_, other) => other,
    }
}

fn pane_is_scrollable(snapshot: &BoardStateSnapshot, pane: PaneId) -> bool {
    match pane {
        PaneId::Left => snapshot.surfaces.left.scrollable,
        PaneId::Right | PaneId::Board => snapshot.surfaces.right.scrollable,
        PaneId::Detour => snapshot
            .surfaces
            .detour
            .as_ref()
            .is_none_or(|s| s.scrollable),
        PaneId::Overlay => false,
    }
}

const fn is_solid_kind(kind: SceneObjectKind) -> bool {
    matches!(
        kind,
        SceneObjectKind::Writing
            | SceneObjectKind::Diagram2D
            | SceneObjectKind::Image
            | SceneObjectKind::Scene3D
            | SceneObjectKind::Prompt
    )
}

fn rect_contains(outer: NormalizedRect, inner: NormalizedRect, eps: f32) -> bool {
    inner.x >= outer.x - eps
        && inner.y >= outer.y - eps
        && (inner.x + inner.w) <= (outer.x + outer.w + eps)
        && (inner.y + inner.h) <= (outer.y + outer.h + eps)
}

fn intersection_metrics(a: NormalizedRect, b: NormalizedRect) -> Option<(f32, f32, f32)> {
    let ix0 = a.x.max(b.x);
    let iy0 = a.y.max(b.y);
    let ix1 = (a.x + a.w).min(b.x + b.w);
    let iy1 = (a.y + a.h).min(b.y + b.h);
    let iw = ix1 - ix0;
    let ih = iy1 - iy0;
    if iw <= 0.0 || ih <= 0.0 {
        return None;
    }
    let inter_area = iw * ih;
    let min_area = (a.w * a.h).min(b.w * b.h).max(1e-6);
    Some((iw, ih, inter_area / min_area))
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_protocol::{
        ChromeSnapshot, ChromeVisibility, DualTimersSnapshot, ImportantIconStyle, LayoutSnapshot,
        LoupeSnapshot, OpticsSnapshot, PROTOCOL_VERSION, ProgressStatus, ReactiveStateSnapshot,
        ReferenceViewMode, ReminderInterval, SpatialZoneId, SplitOrientation, SurfaceSpec,
        SurfacesSnapshot, ThemeId, TimerMode, TimerSnapshot, TokenBoundsSummary, UserInkSummary,
    };

    fn base_snapshot(objects: Vec<SceneObjectSummary>) -> BoardStateSnapshot {
        let timer = TimerSnapshot {
            mode: TimerMode::Countdown,
            running: false,
            configured_sec: None,
            elapsed_sec: 0,
            remaining_sec: 0,
            overtime_sec: 0,
            warning_active: false,
            expired: false,
            formatted: "00:00".into(),
        };
        let surf = SurfaceSpec {
            scrollable: true,
            ..SurfaceSpec::default()
        };
        BoardStateSnapshot {
            protocol_version: PROTOCOL_VERSION.into(),
            layout: LayoutSnapshot {
                mode: LayoutMode::Split,
                split_ratio: 0.34,
                left_collapsed: false,
                reference_view: ReferenceViewMode::Question,
                effective_orientation: SplitOrientation::Horizontal,
                left_pane_bounds: Some(NormalizedRect::new(0.0, 0.0, 0.34, 1.0)),
                right_pane_bounds: NormalizedRect::new(0.34, 0.0, 0.66, 1.0),
            },
            theme: ThemeId::WarmPaper,
            surfaces: SurfacesSnapshot {
                left: surf.clone(),
                right: surf,
                detour: None,
            },
            region_space: Vec::new(),
            objects,
            optics: OpticsSnapshot {
                camera_scale: 1.0,
                camera_center: [0.5, 0.5],
                loupe: LoupeSnapshot {
                    visible: false,
                    x: 0.5,
                    y: 0.5,
                    radius_px: 100.0,
                    magnification: 2.0,
                },
            },
            chrome: ChromeSnapshot {
                mode: ChromeVisibility::Full,
                title: "Test".into(),
                active_item_id: None,
                paper_badge: None,
                item_label: None,
                marks_chip: None,
                status: ProgressStatus::Default,
                important: false,
                important_icon: ImportantIconStyle::Triangle,
                reminder: ReminderInterval::Off,
                reminder_due: false,
                dictionary_enabled: false,
                dictionary_count: 0,
                catalog_button_visible: true,
                author_mode: true,
                part_tabs: Vec::new(),
                active_part_id: None,
                total_steps: 1,
                active_step: 0,
                active_substep: None,
                breakdown_available: false,
                answer_revealed: false,
                caption_visible: false,
                caption_text: None,
                feedback_status: None,
                feedback: Vec::new(),
                calculator_mode: carbon_protocol::CalculatorMode::NonCalculator,
                step_actions: Vec::new(),
                pinned_references: Vec::new(),
            },
            timers: DualTimersSnapshot {
                session: timer.clone(),
                item: timer,
            },
            detour_depth: 0,
            detour_stack: Vec::new(),
            user_ink: UserInkSummary::default(),
            reactive: ReactiveStateSnapshot::default(),
            media: None,
            catalog: None,
            telemetry: None,
        }
    }

    #[test]
    fn passes_clean_stack_with_annotation_and_catches_collision_and_clipping() {
        let clean = base_snapshot(vec![
            SceneObjectSummary {
                id: "eq-1".into(),
                kind: SceneObjectKind::Writing,
                pane: PaneId::Right,
                zone: SpatialZoneId::Spine,
                bounds: NormalizedRect::new(0.05, 0.08, 0.62, 0.09),
                tokens: vec![
                    TokenBoundsSummary {
                        token_id: "lhs".into(),
                        bounds: NormalizedRect::new(0.06, 0.09, 0.18, 0.07),
                    },
                    TokenBoundsSummary {
                        token_id: "rhs".into(),
                        bounds: NormalizedRect::new(0.32, 0.09, 0.22, 0.07),
                    },
                ],
            },
            // Annotation overlay on `eq-1:rhs` is intentional and must NOT fail!
            SceneObjectSummary {
                id: "ann-1".into(),
                kind: SceneObjectKind::Annotation,
                pane: PaneId::Right,
                zone: SpatialZoneId::Spine,
                bounds: NormalizedRect::new(0.31, 0.08, 0.24, 0.09),
                tokens: Vec::new(),
            },
            SceneObjectSummary {
                id: "eq-2".into(),
                kind: SceneObjectKind::Writing,
                pane: PaneId::Right,
                zone: SpatialZoneId::Spine,
                bounds: NormalizedRect::new(0.05, 0.22, 0.50, 0.09),
                tokens: Vec::new(),
            },
        ]);

        let ok_rep = inspect_board_bounds(&clean, None);
        assert!(ok_rep.ok, "expected clean layout, got {:?}", ok_rep.issues);
        assert_eq!(ok_rep.objects_checked, 3);
        assert_eq!(ok_rep.tokens_checked, 2);

        let bad = base_snapshot(vec![
            SceneObjectSummary {
                id: "eq-collide-a".into(),
                kind: SceneObjectKind::Writing,
                pane: PaneId::Right,
                zone: SpatialZoneId::Spine,
                bounds: NormalizedRect::new(0.08, 0.10, 0.55, 0.12),
                tokens: Vec::new(),
            },
            SceneObjectSummary {
                id: "diag-collide-b".into(),
                kind: SceneObjectKind::Diagram2D,
                pane: PaneId::Board, // Canonicalizes to `Right` in Split mode!
                zone: SpatialZoneId::Diagram,
                bounds: NormalizedRect::new(0.12, 0.14, 0.45, 0.20),
                tokens: Vec::new(),
            },
            SceneObjectSummary {
                id: "clipped-right".into(),
                kind: SceneObjectKind::Writing,
                pane: PaneId::Left,
                zone: SpatialZoneId::Spine,
                bounds: NormalizedRect::new(0.60, 0.10, 0.52, 0.08),
                tokens: Vec::new(),
            },
        ]);

        let bad_rep = inspect_board_bounds(&bad, None);
        assert!(!bad_rep.ok);
        assert!(
            bad_rep
                .issues
                .iter()
                .any(|i| i.kind == BoundsIssueKind::ObjectOverlap)
        );
        assert!(
            bad_rep
                .issues
                .iter()
                .any(|i| i.kind == BoundsIssueKind::OutOfPaneClipping)
        );
    }
}
