//! Two-way Carbon `--watch` event persistence (`comment.saved`, `comment.deleted`,
//! and `catalog.actionRequested("create-replica")`) paired with `chrome.setFeedback`
//! and `chrome.patchCatalogItem` synchronization (`Phase 3.2`).

use std::collections::BTreeMap;

use carbon_protocol::{
    AttachmentSource, CarbonCommand, CarbonEvent, CatalogItemPatch, CatalogPaperPatch,
    CatalogSelectionKind, CommentKind, FeedbackAttachment, FeedbackEntry, FeedbackStatus, PaneId,
    PatchCatalogItemParams, SetFeedbackPayload,
};
use memory_core_rs::{CommentAttachmentBytes, CourseComment, CourseCommentInput};
use serde_json::json;

use crate::base64_util::{decode_base64, read_png_dimensions, to_data_uri};
use crate::carbon::CarbonClient;
use crate::config::HydrogenConfig;
use crate::domain::{ContainerSegment, ItemSegment, UniversalId};
use crate::error::Result;
use crate::ingest::ProductLedger;
use crate::vault::VaultManager;

/// Outcome recorded when `--watch` processes a live authoring mutation event from Carbon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchPersistenceOutcome {
    CommentSaved {
        comment_id: String,
        target_id: String,
        attachment_count: usize,
        open_count_for_item: u32,
    },
    CommentDeleted {
        comment_id: String,
        item_id: String,
        deleted: bool,
        open_count_for_item: u32,
    },
    ReplicaQueued {
        task_id: String,
        target_kind: String,
        target_id: String,
    },
}

/// Infer `course_comments.target_kind` (`question`, `topic_card`, `paper`, `chapter`, `book`, `topic`)
/// from a `UniversalId` string (defaulting to `"question"`).
pub fn infer_comment_target_kind(item_id: &str) -> &'static str {
    let Ok(uid) = UniversalId::parse(item_id) else {
        return "question";
    };
    match (&uid.container, &uid.item) {
        (_, Some(ItemSegment::Question { .. })) => "question",
        (_, Some(ItemSegment::ConceptCard { .. })) => "topic_card",
        (Some(ContainerSegment::Paper { .. }), None) => "paper",
        (
            Some(ContainerSegment::Book {
                chapter_no: Some(_),
                ..
            }),
            None,
        ) => "chapter",
        (
            Some(ContainerSegment::Book {
                chapter_no: None, ..
            }),
            None,
        ) => "book",
        (Some(ContainerSegment::Topic { .. }), None) => "topic",
        _ => "question",
    }
}

/// Map `carbon_protocol::CommentKind` to `memory_core_rs`'s `course_comments.comment_kind`.
pub const fn map_comment_kind_to_vault(kind: CommentKind) -> &'static str {
    match kind {
        CommentKind::Fix => "fix",
        CommentKind::Replica => "replica",
        CommentKind::Topic => "topic",
        CommentKind::Note => "note",
        CommentKind::Engine => "engine",
    }
}

/// Map `course_comments.comment_kind` back to `carbon_protocol::CommentKind`.
pub fn map_vault_kind_to_carbon(raw: &str) -> CommentKind {
    match raw {
        "fix" => CommentKind::Fix,
        "replica" => CommentKind::Replica,
        "topic" => CommentKind::Topic,
        "engine" => CommentKind::Engine,
        _ => CommentKind::Note,
    }
}

/// Map `course_comment_attachments.source` back to `carbon_protocol::AttachmentSource`.
pub fn map_vault_source_to_carbon(raw: &str) -> AttachmentSource {
    match raw {
        "board-capture" => AttachmentSource::BoardCapture,
        "clipboard" => AttachmentSource::Clipboard,
        _ => AttachmentSource::Upload,
    }
}

/// Map `AttachmentSource` to `course_comment_attachments.source` (`board-capture`, `upload`, `clipboard`).
pub const fn map_attachment_source_to_vault(src: Option<AttachmentSource>) -> &'static str {
    match src {
        Some(AttachmentSource::BoardCapture) | None => "board-capture",
        Some(AttachmentSource::Upload) => "upload",
        Some(AttachmentSource::Clipboard) => "clipboard",
    }
}

const fn pane_to_str(pane: PaneId) -> &'static str {
    match pane {
        PaneId::Left => "left",
        PaneId::Right => "right",
        PaneId::Board => "board",
        PaneId::Overlay => "overlay",
        PaneId::Detour => "detour",
    }
}

fn str_to_pane(raw: &str) -> Option<PaneId> {
    match raw {
        "left" => Some(PaneId::Left),
        "right" => Some(PaneId::Right),
        "board" => Some(PaneId::Board),
        "overlay" => Some(PaneId::Overlay),
        "detour" => Some(PaneId::Detour),
        _ => None,
    }
}

/// Convert a stored `CourseComment` into Carbon's `FeedbackEntry` wire representation.
pub fn course_comment_to_feedback_entry(
    comment: &CourseComment,
    attachment_data_uris: &BTreeMap<String, String>,
) -> FeedbackEntry {
    let status = if comment.status == "resolved" {
        FeedbackStatus::Done
    } else {
        FeedbackStatus::Pending
    };
    let author_meta = comment.sublevel.map(|lv| json!({ "sublevel": lv }));
    let attachments = comment
        .attachments
        .iter()
        .map(|att| FeedbackAttachment {
            id: att.attachment_id.clone(),
            source: Some(map_vault_source_to_carbon(&att.source)),
            pane: att.pane.as_deref().and_then(str_to_pane),
            mime_type: att.mime_type.clone(),
            filename: att.filename.clone(),
            width_px: att.width_px.and_then(|w| u32::try_from(w.max(0)).ok()),
            height_px: att.height_px.and_then(|h| u32::try_from(h.max(0)).ok()),
            data_ref: attachment_data_uris
                .get(&att.attachment_id)
                .cloned()
                .unwrap_or_default(),
            data_base64: None,
            caption: None,
        })
        .collect();

    FeedbackEntry {
        comment_id: Some(comment.comment_id.clone()),
        item_id: comment.target_id.clone(),
        part_id: comment.part_id.clone(),
        step_id: comment.step_id.clone(),
        kind: map_vault_kind_to_carbon(&comment.comment_kind),
        text: comment.comment_text.clone(),
        status,
        attachments,
        author_meta,
        resolution_note: comment.resolution_note.clone(),
        created_at_ms: Some(comment.created_at),
        updated_at_ms: Some(comment.updated_at),
    }
}

/// Load all stored `CourseComment`s for `item_id` as Carbon `FeedbackEntry` items.
pub async fn load_feedback_entries_for_item(
    vault: &VaultManager,
    item_id: &str,
    include_attachment_data: bool,
) -> Result<Vec<FeedbackEntry>> {
    let comments = vault.list_comments(None, Some(item_id)).await?;
    let mut entries = Vec::with_capacity(comments.len());
    for c in &comments {
        let mut uris = BTreeMap::new();
        if include_attachment_data {
            for att in &c.attachments {
                if att.mime_type.starts_with("image/")
                    && let Ok(bytes) = vault.read_comment_attachment(&att.attachment_id).await
                {
                    uris.insert(
                        att.attachment_id.clone(),
                        to_data_uri(&att.mime_type, &bytes),
                    );
                }
            }
        }
        entries.push(course_comment_to_feedback_entry(c, &uris));
    }
    Ok(entries)
}

/// Decode `FeedbackEntry` attachments and persist a `comment.saved` payload into `VaultManager`.
pub async fn persist_feedback_entry_to_vault(
    vault: &VaultManager,
    entry: &FeedbackEntry,
) -> Result<(String, usize)> {
    let mut decoded_attachments = Vec::new();
    for (idx, att) in entry.attachments.iter().enumerate() {
        let raw_b64_opt = att.data_base64.as_deref().or_else(|| {
            if att.data_ref.starts_with("data:") {
                Some(att.data_ref.as_str())
            } else {
                None
            }
        });
        let Some(raw_b64) = raw_b64_opt else {
            continue;
        };
        let bytes = decode_base64(raw_b64)?;
        if bytes.is_empty() {
            continue;
        }
        let png_dims = if att.mime_type == "image/png" {
            read_png_dimensions(&bytes)
        } else {
            None
        };
        let width_px = att
            .width_px
            .map(i64::from)
            .or_else(|| png_dims.map(|(w, _)| w));
        let height_px = att
            .height_px
            .map(i64::from)
            .or_else(|| png_dims.map(|(_, h)| h));

        decoded_attachments.push(CommentAttachmentBytes {
            attachment_id: if att.id.trim().is_empty() {
                None
            } else {
                Some(att.id.trim().to_string())
            },
            source: map_attachment_source_to_vault(att.source).to_string(),
            pane: att.pane.map(|p| pane_to_str(p).to_string()),
            mime_type: att.mime_type.clone(),
            filename: att.filename.clone(),
            width_px,
            height_px,
            sort_order: i64::try_from(idx).unwrap_or(0),
            bytes,
        });
    }

    let sublevel = entry
        .author_meta
        .as_ref()
        .and_then(|m| m.get("sublevel"))
        .and_then(|v| v.as_i64())
        .filter(|&lv| (1..=9).contains(&lv));

    let explicit_cid = entry
        .comment_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let input = CourseCommentInput {
        comment_id: explicit_cid,
        target_kind: infer_comment_target_kind(&entry.item_id).to_string(),
        target_id: entry.item_id.clone(),
        part_id: entry.part_id.clone(),
        step_id: entry.step_id.clone(),
        sublevel,
        comment_kind: map_comment_kind_to_vault(entry.kind).to_string(),
        comment_text: entry.text.clone(),
    };

    let att_count = decoded_attachments.len();
    let saved_id = vault.upsert_comment(&input, &decoded_attachments).await?;

    if entry.status == FeedbackStatus::Done {
        let _ = vault
            .resolve_comment(&saved_id, entry.resolution_note.as_deref())
            .await?;
    }

    Ok((saved_id, att_count))
}

/// Push updated `chrome.setFeedback` (when `target_id == active_item_id`) and `chrome.patchCatalogItem`
/// open-comment badge counts back to Carbon.
pub async fn sync_item_feedback_to_carbon(
    client: &CarbonClient,
    vault: &VaultManager,
    target_id: &str,
    active_item_id: &str,
) -> Result<u32> {
    let snap = vault.catalog_snapshot().await?;
    let t_kind = infer_comment_target_kind(target_id);
    let item_open = snap
        .open_comment_counts
        .get(&format!("{t_kind}:{target_id}"))
        .copied()
        .unwrap_or(0);
    let item_open_u32 = u32::try_from(item_open.max(0)).unwrap_or(0);

    let paper_patch = if let Ok(uid) = UniversalId::parse(target_id)
        && let Some(pid) = uid.container_id()
        && matches!(uid.container, Some(ContainerSegment::Paper { .. }))
    {
        let paper_open: i64 = snap
            .questions
            .iter()
            .filter(|q| q.paper_id.as_deref() == Some(pid.as_str()))
            .map(|q| q.open_comment_count)
            .sum();
        Some(CatalogPaperPatch {
            id: pid,
            open_comment_count: Some(u32::try_from(paper_open.max(0)).unwrap_or(0)),
            ..CatalogPaperPatch::default()
        })
    } else {
        None
    };

    let mut commands = Vec::with_capacity(2);
    if target_id == active_item_id {
        let entries = load_feedback_entries_for_item(vault, target_id, true).await?;
        commands.push(CarbonCommand::SetFeedback(SetFeedbackPayload::List {
            entries,
        }));
    }
    commands.push(CarbonCommand::PatchCatalogItem(Box::new(
        PatchCatalogItemParams {
            active_item_id: None,
            item_patch: Some(CatalogItemPatch {
                id: target_id.to_string(),
                open_comment_count: Some(item_open_u32),
                ..CatalogItemPatch::default()
            }),
            paper_patch,
            topic_patch: None,
            stats: None,
        },
    )));
    let _ = client.send_batch(commands).await?;
    Ok(item_open_u32)
}

/// Process a live two-way authoring event from Carbon (`comment.saved`, `comment.deleted`,
/// or `catalog.actionRequested("create-replica")`), persisting immediately to `VaultManager`
/// and/or `products/<product>/<subject>/ledger.json` and syncing Carbon's open comment state.
pub async fn handle_watch_persistence_event(
    event: &CarbonEvent,
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    vault: &VaultManager,
    client: &CarbonClient,
    active_item_id: &str,
) -> Result<Option<WatchPersistenceOutcome>> {
    match event {
        CarbonEvent::CommentSaved { entry } => {
            let (saved_id, att_count) = persist_feedback_entry_to_vault(vault, entry).await?;
            let open_count =
                sync_item_feedback_to_carbon(client, vault, &entry.item_id, active_item_id).await?;
            Ok(Some(WatchPersistenceOutcome::CommentSaved {
                comment_id: saved_id,
                target_id: entry.item_id.clone(),
                attachment_count: att_count,
                open_count_for_item: open_count,
            }))
        }
        CarbonEvent::CommentDeleted {
            comment_id,
            item_id,
            part_id,
        } => {
            let resolved_cid = match comment_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(explicit) => explicit.to_string(),
                None => {
                    let candidates = vault.list_comments(None, Some(item_id)).await?;
                    candidates
                        .into_iter()
                        .find(|c| c.part_id.as_deref() == part_id.as_deref())
                        .map(|c| c.comment_id)
                        .unwrap_or_else(|| {
                            format!("{item_id}:{}", part_id.as_deref().unwrap_or("root"))
                        })
                }
            };
            let deleted = vault.delete_comment(&resolved_cid).await?;
            let open_count =
                sync_item_feedback_to_carbon(client, vault, item_id, active_item_id).await?;
            Ok(Some(WatchPersistenceOutcome::CommentDeleted {
                comment_id: resolved_cid,
                item_id: item_id.clone(),
                deleted,
                open_count_for_item: open_count,
            }))
        }
        CarbonEvent::CatalogActionRequested {
            action,
            target_kind,
            target_id,
        } if action == "create-replica" => {
            let kind_str = match target_kind {
                CatalogSelectionKind::Question => "question",
                CatalogSelectionKind::Paper => "paper",
                CatalogSelectionKind::Topic => "topic",
                CatalogSelectionKind::TopicCard => "topic_card",
                CatalogSelectionKind::Subject => "subject",
            };
            let ledger_path = cfg.ledger_path(product, subject);
            let mut ledger = ProductLedger::load_or_new(&ledger_path, product, subject)?;
            let task = ledger.enqueue_replica_task(
                subject,
                kind_str,
                target_id,
                None,
                Some("Queued live from Carbon catalog menu".to_string()),
            );
            ledger.save_atomic(&ledger_path)?;
            Ok(Some(WatchPersistenceOutcome::ReplicaQueued {
                task_id: task.task_id,
                target_kind: kind_str.to_string(),
                target_id: target_id.clone(),
            }))
        }
        _ => Ok(None),
    }
}
