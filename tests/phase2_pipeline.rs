//! Phase 2 End-to-End Integration Test:
//! 4-tier `RuleLoader` -> `slice_paper` -> `verify_question_package` (full `L9 -> L1`) ->
//! `VaultManager` (`upsert_paper` + `upsert_question`) -> `build_question_preview_batch` +
//! `CarbonClient` WebSocket batch -> `pack` & `export_clean_bundle`.

use std::path::PathBuf;
use std::time::Duration;

use carbon_protocol::{
    CarbonEvent, CarbonEventEnvelope, ClientMessage, CommandAck, PROTOCOL_VERSION, PaneId,
    ServerMessage, StepNavigationKind, decode_client_message, encode_server_message,
};
use futures_util::{SinkExt, StreamExt};
use hydrogen::carbon::CarbonClient;
use hydrogen::carbon::preview::{
    PreviewSelection, apply_interactive_event, build_question_preview_batch,
};
use hydrogen::compiler::verify_question_package;
use hydrogen::config::HydrogenConfig;
use hydrogen::domain::{PaperPackage, QuestionPackage};
use hydrogen::error::{HydrogenError, Result};
use hydrogen::ingest::{RgbaBuffer, SliceMode, SlicePaperRequest, slice_paper};
use hydrogen::rules::RuleLoader;
use hydrogen::vault::VaultManager;
use memory_core_rs::CoursePaperInput;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

fn full_l9_to_l1_paper_package() -> Result<PaperPackage> {
    let raw = r##"{
        "paper": {
            "paperId": "GYR-MTH-P001",
            "code": "P001",
            "title": "Practice Paper 01",
            "badge": "50 min · Non-Calculator",
            "durationSec": 3000,
            "calculatorMode": "non-calculator",
            "defaultAnswerFormat": "mcq",
            "totalMarks": 2.0,
            "isReplica": false,
            "sortOrder": 10
        },
        "sections": [],
        "stimuli": [],
        "questions": [
            {
                "containerKind": "paper",
                "paperId": "GYR-MTH-P001",
                "seqNo": 1,
                "label": "Q1",
                "title": "Simultaneous Ticket Bundles",
                "questionTypeId": "GYR-MTH-QT-ALG-WORD",
                "difficulty": 3,
                "calculatorMode": "non-calculator",
                "recommendedTimeSec": 60,
                "isReplica": false,
                "qaStatus": "verified",
                "topics": [{ "topicId": "GYR-MTH-T_ALG", "isPrimary": true }],
                "question": {
                    "schemaVersion": "1.0",
                    "questionId": "GYR-MTH-P001-Q001",
                    "answerFormat": "mcq",
                    "stemMarkdownKatex": "$2$ adult tickets and $3$ child tickets cost $\\pounds 29$.\n\n$5$ adult tickets and $3$ child tickets cost $\\pounds 53$.\n\nWhat is the cost of $1$ adult ticket?",
                    "dictionary": [
                        { "term": "simultaneous", "definition": "Two clues that are both true at the same time." }
                    ],
                    "pinnedReferences": [],
                    "leftPaneCarbonCommands": [
                        {
                            "op": "scene.write",
                            "params": {
                                "id": "q1-stem",
                                "pane": "left",
                                "zone": "spine",
                                "style": "typeset",
                                "content": "$2$ adult tickets and $3$ child tickets cost $\\pounds 29$.\n\n$5$ adult tickets and $3$ child tickets cost $\\pounds 53$.\n\nWhat is the cost of $1$ adult ticket?"
                            }
                        },
                        {
                            "op": "interact.ask",
                            "params": {
                                "promptId": "q1-mcq",
                                "pane": "left",
                                "kind": "choice-chips",
                                "label": "Select one answer:",
                                "choices": [
                                    { "id": "A", "label": "$\\pounds 5$" },
                                    { "id": "B", "label": "$\\pounds 7$" },
                                    { "id": "C", "label": "$\\pounds 8$" },
                                    { "id": "D", "label": "$\\pounds 12$" },
                                    { "id": "E", "label": "$\\pounds 24$" }
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
                                "correctChoiceId": "C",
                                "exactValue": "8",
                                "katexValue": "\\pounds 8",
                                "unit": "GBP",
                                "distractorDiagnostics": {
                                    "A": "Found the cost of 1 child ticket ((29 - 16)/3 is not 5; confused clues).",
                                    "B": "Divided 53 - 29 = 24 inaccurately.",
                                    "D": "Divided £24 by 2 instead of 3 extra adult tickets.",
                                    "E": "Stopped at 3A = £24 without dividing by 3."
                                }
                            }
                        }
                    ]
                },
                "solution": {
                    "schemaVersion": "1.0",
                    "questionId": "GYR-MTH-P001-Q001",
                    "thetaPreset": "uk_gcse",
                    "parts": [
                        {
                            "partId": "main",
                            "usesPriorParts": [],
                            "exportedExact": { "adult_ticket": "8", "child_ticket": "13/3" },
                            "finalAnswer": {
                                "katex": "$\\pounds 8\\text{ (Option C)}$",
                                "carbonCommands": [
                                    {
                                        "op": "scene.write",
                                        "params": {
                                            "id": "q1_ans",
                                            "pane": "right",
                                            "zone": "spine",
                                            "style": "typeset",
                                            "color": "#15803D",
                                            "content": "1 Adult ticket $= \\mathbf{\\pounds 8}$ **(Option C)**",
                                            "card": { "fillColor": "#F0FDF4", "borderColor": "#16A34A", "borderStyle": "solid" }
                                        }
                                    },
                                    {
                                        "op": "focus.annotate",
                                        "params": {
                                            "id": "q1_chk",
                                            "targetIds": ["q1_ans"],
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
                                    "stepGoal": "Eliminate C by subtracting the two ticket equations to find A = £8",
                                    "breakdownAvailable": true,
                                    "substeps": [
                                        {
                                            "substepIndex": 0,
                                            "narration": "Both bundles have 3 child tickets, so the £24 price jump comes purely from the 3 extra adult tickets.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.write",
                                                    "params": {
                                                        "id": "q1_s1_sub0",
                                                        "pane": "right",
                                                        "zone": "scratch",
                                                        "style": "casual",
                                                        "scale": 0.92,
                                                        "rotationDeg": -1.5,
                                                        "content": "Same $3C$ in both clues $\\implies$ subtract to cancel $3C$ cleanly!",
                                                        "card": { "headerBanner": "11+ SHORTCUT", "fillColor": "#EFF6FF", "borderColor": "#3B82F6", "borderStyle": "solid" }
                                                    }
                                                }
                                            ]
                                        }
                                    ],
                                    "prerequisiteCardIds": [],
                                    "thetaVerification": [
                                        {
                                            "tool": "calc.solve",
                                            "args": { "equations": ["2*A + 3*C = 29", "5*A + 3*C = 53"], "solve_for": ["A", "C"] },
                                            "expectedSolutions": { "A": "8" }
                                        },
                                        {
                                            "tool": "calc.eval",
                                            "args": { "expr": "(53 - 29) / (5 - 2)" },
                                            "expectedExact": "8"
                                        }
                                    ],
                                    "sublevels": {
                                        "9": {
                                            "archetype": "expected_exam_step",
                                            "narration": "In an 11+ exam, let A and C be adult and child tickets. Subtracting 2A + 3C = 29 from 5A + 3C = 53 gives 3A = 24, so A = £8.",
                                            "speechTokens": ["s1_l9:eq_diff", "s1_l9:val_a"],
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.write",
                                                    "params": {
                                                        "id": "s1_l9",
                                                        "pane": "right",
                                                        "zone": "spine",
                                                        "style": "handwritten",
                                                        "content": "$(5A + 3C) - (2A + 3C) = 53 - 29 \\implies \\htmlId{eq_diff}{3A = 24} \\implies \\htmlId{val_a}{A = \\pounds 8}$"
                                                    }
                                                }
                                            ]
                                        },
                                        "8": {
                                            "archetype": "guided_exam_step",
                                            "narration": "Stack the bigger bundle above the smaller bundle, subtract to cancel 3C, and divide 24 by 3 to get £8.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.write",
                                                    "params": {
                                                        "id": "s1_l8",
                                                        "pane": "right",
                                                        "zone": "spine",
                                                        "style": "handwritten",
                                                        "content": "$5A + 3C = 53$ minus $2A + 3C = 29 \\xrightarrow{-3C} 3A = 24 \\xrightarrow{\\div 3} A = \\pounds 8$"
                                                    }
                                                }
                                            ]
                                        },
                                        "7": {
                                            "archetype": "token_annotated",
                                            "narration": "Notice 3C matches in both equations, so subtracting eliminates C in one move.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.write",
                                                    "params": {
                                                        "id": "s1_l7",
                                                        "pane": "right",
                                                        "zone": "spine",
                                                        "style": "handwritten",
                                                        "content": "$5A + \\htmlId{c_match}{3C} = 53$ and $2A + 3C = 29 \\implies 3A = 24 \\implies A = \\pounds 8$"
                                                    }
                                                },
                                                {
                                                    "op": "focus.annotate",
                                                    "params": {
                                                        "id": "s1_l7_ann",
                                                        "targetIds": ["s1_l7:c_match"],
                                                        "kind": "box",
                                                        "color": "c1",
                                                        "label": "Matching $3C$ cancels out"
                                                    }
                                                }
                                            ]
                                        },
                                        "6": {
                                            "archetype": "unpacked_micro_steps",
                                            "narration": "Step by step: 5A minus 2A leaves 3A, 53 minus 29 leaves £24, and £24 divided by 3 gives £8.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.write",
                                                    "params": {
                                                        "id": "s1_l6",
                                                        "pane": "right",
                                                        "zone": "spine",
                                                        "style": "handwritten",
                                                        "content": "$5A - 2A = 3A$,\\quad $\\pounds 53 - \\pounds 29 = \\pounds 24$,\\quad $\\pounds 24 \\div 3 = \\pounds 8$"
                                                    }
                                                }
                                            ]
                                        },
                                        "5": {
                                            "archetype": "trap_contrast",
                                            "narration": "Don't stop at £24 like Option E! £24 is the price of 3 adult tickets, so divide by 3 to get £8.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.write",
                                                    "params": {
                                                        "id": "s1_l5",
                                                        "pane": "right",
                                                        "zone": "spine",
                                                        "style": "typeset",
                                                        "content": "**NEVER EVER** pick $\\pounds 24$ (Option E) — that is $3$ adult tickets! Divide by $3$: $\\pounds 24 \\div 3 = \\pounds 8$.",
                                                        "card": { "headerBanner": "TRAP OPTION E", "fillColor": "#FDF2F8", "borderColor": "#9F1239", "borderStyle": "solid" }
                                                    }
                                                }
                                            ]
                                        },
                                        "4": {
                                            "archetype": "chunk_strip",
                                            "narration": "Split the £24 difference into 3 equal adult chunks: £8 + £8 + £8 = £24.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.place2D",
                                                    "params": {
                                                        "id": "s1_l4",
                                                        "pane": "right",
                                                        "zone": "diagram",
                                                        "spec": {
                                                            "macro": "fractionBar",
                                                            "parts": 3,
                                                            "shaded": 3,
                                                            "label": "Extra $\\pounds 24 = 3 \\times \\pounds 8$"
                                                        }
                                                    }
                                                }
                                            ]
                                        },
                                        "3": {
                                            "archetype": "spatial_bar_model",
                                            "narration": "Compare the two bar models: both share the 3C block, and the 3 extra A boxes stick out worth £24, so 1 A box is £8.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.place2D",
                                                    "params": {
                                                        "id": "s1_l3",
                                                        "pane": "right",
                                                        "zone": "diagram",
                                                        "spec": {
                                                            "macro": "barModel",
                                                            "totalLabel": "Difference $= \\pounds 53 - \\pounds 29 = \\pounds 24$",
                                                            "rows": [
                                                                [{ "value": 2, "label": "$2A$" }, { "value": 3, "label": "$3C$" }],
                                                                [{ "value": 2, "label": "$2A$" }, { "value": 3, "label": "$3C$" }, { "value": 3, "label": "$3A = \\pounds 24$" }]
                                                            ]
                                                        }
                                                    }
                                                }
                                            ]
                                        },
                                        "2": {
                                            "archetype": "number_line_hops",
                                            "narration": "Count from £29 up to £53 in 3 equal adult jumps of +£8: 29 to 37 to 45 to 53.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.place2D",
                                                    "params": {
                                                        "id": "s1_l2",
                                                        "pane": "right",
                                                        "zone": "diagram",
                                                        "spec": {
                                                            "macro": "numberLine",
                                                            "min": 29,
                                                            "max": 53,
                                                            "label": "$+\\pounds 8$ per extra adult ticket"
                                                        }
                                                    }
                                                }
                                            ]
                                        },
                                        "1": {
                                            "archetype": "concrete_objects",
                                            "narration": "Share 24 pound coins equally onto 3 adult tickets: each adult ticket gets 8 pound coins.",
                                            "carbonCommands": [
                                                {
                                                    "op": "scene.place2D",
                                                    "params": {
                                                        "id": "s1_l1",
                                                        "pane": "right",
                                                        "zone": "diagram",
                                                        "spec": {
                                                            "macro": "counters",
                                                            "groups": 3,
                                                            "perGroup": 8,
                                                            "label": "$24$ coins shared across $3$ tickets $= 8$ each"
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
            }
        ]
    }"##;
    Ok(serde_json::from_str(raw)?)
}

#[tokio::test]
async fn phase2_end_to_end_slice_verify_upsert_preview_and_pack() -> Result<()> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("hydrogen-phase2-e2e-{}", std::process::id()));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(tmp.join("products/gyaraplus/inbox/maths"))?;
    std::fs::create_dir_all(tmp.join("products/gyaraplus/assets"))?;

    // Copy brand PNGs into temp workspace
    for logo in ["gyara-blue.png", "gyara-white.png"] {
        std::fs::copy(
            repo_root.join("products/gyaraplus/assets").join(logo),
            tmp.join("products/gyaraplus/assets").join(logo),
        )?;
    }

    let cfg = HydrogenConfig {
        workspace_root: tmp.clone(),
        products_dir: tmp.join("products"),
        rules_dir: repo_root.join("rules"),
        default_product: "gyaraplus".into(),
        default_subject: "maths".into(),
        bind_addr: "127.0.0.1:7430".into(),
        carbon_ws_url: "ws://127.0.0.1:7420/carbon".into(),
        device_id: "phase2-e2e-test".into(),
        lesson_key: "phase2-lesson-key-789".into(),
        gemini_api_key: None,
    };

    // 1. Load 4-tier rules
    let rulebook = RuleLoader::new(&cfg.rules_dir).load("gyaraplus", "maths", None)?;
    assert_eq!(rulebook.frontmatter.course_id, "GYR-MTH");

    // 2. Slice a synthetic PNG sheet into `archive/` + `private-xref.json`
    let mut sheet_rgba = vec![255u8; 600 * 400 * 4];
    for y in 80..200 {
        for x in 50..550 {
            let off = (y * 600 + x) * 4;
            sheet_rgba[off] = 20;
            sheet_rgba[off + 1] = 20;
            sheet_rgba[off + 2] = 20;
            sheet_rgba[off + 3] = 255;
        }
    }
    let png_bytes = RgbaBuffer {
        width: 600,
        height: 400,
        data: sheet_rgba,
    }
    .encode_png()?;
    let inbox_sheet = cfg
        .subject_inbox_dir("gyaraplus", "maths")
        .join("Practice-Sheet-Test.png");
    std::fs::write(&inbox_sheet, png_bytes)?;

    let sliced = slice_paper(
        &cfg,
        &SlicePaperRequest {
            source_path: inbox_sheet,
            product: "gyaraplus".into(),
            subject: "maths".into(),
            paper_id: Some("GYR-MTH-P001".into()),
            dpi: 180.0,
            page_range: None,
            mode: SliceMode::default(),
            move_source_to_archive: false,
        },
    )?;
    assert_eq!(sliced.sliced_items.len(), 1);
    assert_eq!(sliced.sliced_items[0].universal_id, "GYR-MTH-P001-Q001");

    // 3. Verify complete L9 -> L1 QuestionPackage through Theta + Carbon + Warm-Paper + Clean-Room Gate
    let paper_pkg = full_l9_to_l1_paper_package()?;
    let q_pkg: &QuestionPackage = &paper_pkg.questions[0];
    let gate_report = verify_question_package(q_pkg, Some(&rulebook)).into_result()?;
    assert!(gate_report.ok);
    assert!(gate_report.theta_checks_run >= 4);

    // 4. Upsert Paper + Question into encrypted L7 vault
    let vault = VaultManager::open_or_create(
        cfg.vault_sqlite_path("gyaraplus", "maths"),
        cfg.vault_bundle_path("gyaraplus", "maths"),
        &cfg.lesson_key,
        &cfg.device_id,
    )?;
    let _ = vault
        .seed_gyara_maths(&cfg.product_assets_dir("gyaraplus"))
        .await?;
    vault
        .upsert_paper(&CoursePaperInput::from(&paper_pkg.paper))
        .await?;
    vault
        .upsert_question(&q_pkg.to_course_question_input()?)
        .await?;

    let stored_q = vault
        .get_question("GYR-MTH-P001-Q001")
        .await?
        .ok_or_else(|| HydrogenError::Validation("stored question missing".into()))?;
    assert_eq!(stored_q.min_sublevel, 1);
    assert_eq!(stored_q.max_sublevel, 9);

    // 5. Build and dispatch Carbon preview batches across Levels 9, 5, and 3
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let ws_url = format!("ws://{addr}/carbon");
    let mock_server = tokio::spawn(async move {
        let Ok((tcp, _)) = listener.accept().await else {
            return;
        };
        let Ok(mut ws) = accept_async(tcp).await else {
            return;
        };
        let ready = ServerMessage::Event(CarbonEventEnvelope {
            seq: 1,
            timestamp_ms: 1763838000000,
            actor: None,
            event: CarbonEvent::BoardReady {
                protocol_version: PROTOCOL_VERSION.to_string(),
                gpu_backend: "Test-Stage".into(),
                viewport_px: [1920, 1080],
                scale_factor: 2.0,
            },
        });
        if let Ok(raw) = encode_server_message(&ready) {
            let _ = ws.send(Message::Text(raw.into())).await;
        }
        while let Some(Ok(Message::Text(txt))) = ws.next().await {
            if let Ok(ClientMessage::Batch { id, commands, .. }) = decode_client_message(&txt) {
                let ack = ServerMessage::Ack(CommandAck {
                    request_id: id,
                    ok: true,
                    action_ids: commands.iter().map(|c| c.op_name().to_string()).collect(),
                    error: None,
                });
                if let Ok(raw) = encode_server_message(&ack) {
                    let _ = ws.send(Message::Text(raw.into())).await;
                }
            }
        }
    });

    let client = CarbonClient::connect(&ws_url, "e2e-driver", Duration::from_secs(2)).await?;
    let mut cur_qid = "GYR-MTH-P001-Q001".to_string();
    let mut live_sel = PreviewSelection::default();
    let all_qs = vec![stored_q.clone()];

    assert!(apply_interactive_event(
        &CarbonEvent::StepActionRequested {
            action_id: "level-5".into(),
            target_token_id: None,
            pane: PaneId::Right,
        },
        &mut cur_qid,
        &mut live_sel,
        &all_qs,
    ));
    assert_eq!(live_sel.level, 5);

    assert!(apply_interactive_event(
        &CarbonEvent::StepNavigated {
            action: StepNavigationKind::Breakdown,
            item_id: None,
            part_id: None,
            step_id: None,
            active_step: 0,
            total_steps: 1,
            active_substep: Some(0),
            handled_locally: false,
        },
        &mut cur_qid,
        &mut live_sel,
        &all_qs,
    ));
    assert_eq!(live_sel.substep, Some(0));

    assert!(apply_interactive_event(
        &CarbonEvent::AnswerToggled {
            item_id: None,
            part_id: None,
            revealed: true,
        },
        &mut cur_qid,
        &mut live_sel,
        &all_qs,
    ));
    assert!(live_sel.reveal_answer);

    for lvl in [9u8, 5, 3, 1] {
        let batch = build_question_preview_batch(
            &stored_q,
            &PreviewSelection {
                part_id: None,
                step_id: Some("S01".into()),
                level: lvl,
                substep: Some(0),
                reveal_answer: true,
            },
        )?;
        let ack = client.send_batch(batch.commands).await?;
        assert!(ack.ok);
    }
    mock_server.abort();

    // 6. Pack `.raphs` and verify clean customer export
    let manifest = vault.pack_to_default_bundle().await?;
    assert_eq!(manifest.schema_version, "2.3.0");
    let clean_out = tmp.join("gyara-maths-customer.raphs");
    vault
        .export_clean_bundle(&clean_out, "customer-key-abc")
        .await?;
    assert!(clean_out.is_file());

    drop(vault);
    std::fs::remove_dir_all(&tmp)?;
    Ok(())
}
