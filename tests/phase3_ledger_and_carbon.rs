//! Phase 3 (`3.1 -> 3.3`) End-to-End Integration Suite:
//! 1. `ledger.json` SHA-256 deduplication, renamed duplicate detection, expected counts & auto-sync,
//! 2. Self-contained Warm-Paper `status.html` generation (`GYR-LOGO-BLUE`, crops, stems, comments, tasks),
//! 3. Two-way live Carbon `--watch` persistence (`comment.saved` `Snap Board` PNG + `comment.deleted` + `create-replica`),
//! 4. `put-asset` & `comments [list | show --export-dir | resolve]` CLI lifecycle + Clean-Room enforcement,
//! 5. Headless Carbon (`carbon-shell --headless`) `capture-carbon --check-bounds` PNG capture & AABB collision detection.

use std::net::TcpListener as StdTcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use carbon_protocol::{
    ActorRole, AnchorSpec, AttachmentSource, CarbonCommand, CatalogSelectionKind, ClientActor,
    ClientInteraction, CommentKind, FeedbackAttachment, FeedbackEntry, FeedbackStatus, PaneId,
    SpatialZoneId, SpeedPreset, SpeedSpec, WriteParams,
};
use hydrogen::base64_util::{encode_base64, read_png_dimensions};
use hydrogen::carbon::{
    BoundsIssueKind, CarbonClient, WatchPersistenceOutcome, handle_watch_persistence_event,
    inspect_board_bounds,
};
use hydrogen::config::HydrogenConfig;
use hydrogen::domain::PaperPackage;
use hydrogen::error::{HydrogenError, Result};
use hydrogen::ingest::{AuthoringTaskKind, ProductLedger, ReplicaScope, RgbaBuffer, SourceStage};
use hydrogen::vault::VaultManager;
use serde_json::json;
use tokio::time::{sleep, timeout};

const PREBUILT_CARBON_BIN: &str = "/home/raphspark/projects/carbon/target/release/carbon-shell";

fn sample_paper_package() -> Result<PaperPackage> {
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
                    "dictionary": [],
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
                                    "A": "Confused child and adult ticket clues.",
                                    "B": "Subtracted 53 - 29 inaccurately.",
                                    "D": "Divided £24 by 2 instead of 3.",
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
                            "exportedExact": { "adult_ticket": "8" },
                            "finalAnswer": {
                                "katex": "$\\pounds 8\\text{ (Option C)}$",
                                "carbonCommands": [
                                    {
                                        "op": "scene.write",
                                        "params": {
                                            "id": "q1_ans",
                                            "pane": "right",
                                            "zone": "spine",
                                            "anchor": { "mode": "below", "targetId": "s1_l9", "offsetPt": [0, 16] },
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
                                    "stepGoal": "Eliminate C by subtracting the two equations to find A = £8",
                                    "breakdownAvailable": false,
                                    "substeps": [],
                                    "prerequisiteCardIds": [],
                                    "thetaVerification": [
                                        {
                                            "tool": "calc.solve",
                                            "args": { "equations": ["2*A + 3*C = 29", "5*A + 3*C = 53"], "solve_for": ["A", "C"] },
                                            "expectedSolutions": { "A": "8" }
                                        }
                                    ],
                                    "sublevels": {
                                        "9": {
                                            "archetype": "expected_exam_step",
                                            "narration": "Subtract 2A + 3C = 29 from 5A + 3C = 53 to get 3A = 24, so A = £8.",
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
                                            "narration": "Cancel 3C and divide 24 by 3.",
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

fn setup_temp_workspace(tag: &str) -> Result<(PathBuf, HydrogenConfig)> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("hydrogen-{tag}-{}", std::process::id()));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(tmp.join("products/gyaraplus/maths/inbox"))?;
    std::fs::create_dir_all(tmp.join("products/gyaraplus/assets"))?;

    for logo in ["gyara-blue.png", "gyara-white.png"] {
        std::fs::copy(
            repo_root.join("products/gyaraplus/assets").join(logo),
            tmp.join("products/gyaraplus/assets").join(logo),
        )?;
    }

    let rules_dir = repo_root.join("rules");
    let toml_text = format!(
        "products_dir = \"products\"\nrules_dir = \"{}\"\ndefault_product = \"gyaraplus\"\ndefault_subject = \"maths\"\ncarbon_ws_url = \"ws://127.0.0.1:7420/carbon\"\ndevice_id = \"phase3-test-dev\"\n",
        rules_dir.display()
    );
    std::fs::write(tmp.join("hydrogen.toml"), toml_text)?;
    std::fs::write(
        tmp.join(".env"),
        "HYDROGEN_LESSON_KEY=\"phase3-test-lesson-key\"\n",
    )?;

    let cfg = HydrogenConfig::load(&tmp)?;
    Ok((tmp, cfg))
}

fn run_hydrogen_bin(cwd: &Path, args: &[&str]) -> Result<(bool, String, String)> {
    let bin = env!("CARGO_BIN_EXE_hydrogen");
    let out = Command::new(bin).current_dir(cwd).args(args).output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    Ok((out.status.success(), stdout, stderr))
}

fn synthetic_sheet_png() -> Result<Vec<u8>> {
    let mut rgba = vec![255u8; 640 * 420 * 4];
    for y in 90..210 {
        for x in 48..592 {
            let off = (y * 640 + x) * 4;
            rgba[off] = 24;
            rgba[off + 1] = 28;
            rgba[off + 2] = 32;
            rgba[off + 3] = 255;
        }
    }
    RgbaBuffer {
        width: 640,
        height: 420,
        data: rgba,
    }
    .encode_png()
}

struct HeadlessCarbonGuard(Child);

impl Drop for HeadlessCarbonGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn spawn_ephemeral_headless_carbon() -> Result<(String, HeadlessCarbonGuard)> {
    let port = {
        let listener = StdTcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };
    let child = Command::new(PREBUILT_CARBON_BIN)
        .args(["--headless", "--split", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let guard = HeadlessCarbonGuard(child);

    let ws_url = format!("ws://127.0.0.1:{port}/carbon");
    for _ in 0..40 {
        if CarbonClient::connect(&ws_url, "probe", Duration::from_millis(250))
            .await
            .is_ok()
        {
            return Ok((ws_url, guard));
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(HydrogenError::CarbonTransport(format!(
        "headless carbon-shell failed to listen on {ws_url}"
    )))
}

#[test]
fn phase3_ledger_sha256_dedup_status_html_and_put_asset_cli() -> Result<()> {
    let (tmp, cfg) = setup_temp_workspace("p3-ledger-cli")?;

    // 1. Drop a worksheet PNG + a byte-identical renamed copy into `maths/inbox/`
    let sheet_bytes = synthetic_sheet_png()?;
    let inbox_dir = cfg.subject_inbox_dir("gyaraplus", "maths");
    let primary_sheet = inbox_dir.join("Sheet-01.png");
    let duplicate_sheet = inbox_dir.join("Sheet-01-Renamed-Copy.png");
    std::fs::write(&primary_sheet, &sheet_bytes)?;
    std::fs::write(&duplicate_sheet, &sheet_bytes)?;

    // 2. Run `hydrogen status --set-expected GYR-MTH-P001 5`
    let (ok, stdout, stderr) = run_hydrogen_bin(
        &tmp,
        &[
            "status",
            "--product",
            "gyaraplus",
            "--set-expected",
            "GYR-MTH-P001",
            "5",
        ],
    )?;
    assert!(ok, "status CLI failed: {stderr}");
    assert!(stdout.contains("RENAMED DUPLICATE"));
    assert!(
        !stdout.contains("verbal-reasoning") && !stdout.contains("english"),
        "unworked subjects must not appear in status output"
    );

    let ledger_after_scan =
        ProductLedger::load_or_new(&cfg.ledger_path("gyaraplus", "maths"), "gyaraplus", "maths")?;
    assert_eq!(ledger_after_scan.subject, "maths");
    assert_eq!(
        ledger_after_scan.sources.len(),
        1,
        "identical SHA-256 files must collapse to 1 source record"
    );

    // 3. Slice `Sheet-01.png` into `GYR-MTH-P001` (`Q001`) -> auto-updates `ledger.json`
    let primary_str = primary_sheet.to_string_lossy().into_owned();
    let (ok_slice, _, err_slice) = run_hydrogen_bin(
        &tmp,
        &[
            "slice-paper",
            &primary_str,
            "--paper-id",
            "GYR-MTH-P001",
            "--auto",
        ],
    )?;
    assert!(ok_slice, "slice-paper failed: {err_slice}");

    let ledger_after_slice =
        ProductLedger::load_or_new(&cfg.ledger_path("gyaraplus", "maths"), "gyaraplus", "maths")?;
    let p_sliced = ledger_after_slice
        .papers
        .get("GYR-MTH-P001")
        .ok_or_else(|| HydrogenError::Validation("missing GYR-MTH-P001 in ledger".into()))?;
    assert_eq!(p_sliced.sliced_count, 1);
    assert_eq!(p_sliced.stage, SourceStage::Sliced);

    // 4. Test `put-asset` CLI (clean PNG + SVG, plus Clean-Room leak rejection)
    let clean_img_path = tmp.join("clean-blocks.png");
    let clean_png = RgbaBuffer {
        width: 24,
        height: 16,
        data: vec![220u8; 24 * 16 * 4],
    }
    .encode_png()?;
    std::fs::write(&clean_img_path, &clean_png)?;
    let clean_img_str = clean_img_path.to_string_lossy().into_owned();

    let (ok_asset, out_asset, err_asset) = run_hydrogen_bin(
        &tmp,
        &[
            "put-asset",
            &clean_img_str,
            "--asset-id",
            "GYR-MTH-P001-Q001-IMG01",
            "--alt",
            "Wooden prism blocks",
            "--caption",
            "Clean regenerated block illustration",
        ],
    )?;
    assert!(ok_asset, "put-asset PNG failed: {err_asset}");
    assert!(out_asset.contains("GYR-MTH-P001-Q001-IMG01"));

    let clean_svg_path = tmp.join("clean-shape.svg");
    std::fs::write(
        &clean_svg_path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><circle cx="20" cy="20" r="16" stroke="#2563EB" stroke-width="1.5" fill="none"/></svg>"##,
    )?;
    let clean_svg_str = clean_svg_path.to_string_lossy().into_owned();
    let (ok_svg, _, err_svg) = run_hydrogen_bin(
        &tmp,
        &[
            "put-asset",
            &clean_svg_str,
            "--asset-id",
            "GYR-MTH-P001-Q001-SVG01",
        ],
    )?;
    assert!(ok_svg, "put-asset SVG failed: {err_svg}");

    // Reject Clean-Room violation in `--alt`
    let (ok_leak, _, _) = run_hydrogen_bin(
        &tmp,
        &[
            "put-asset",
            &clean_img_str,
            "--asset-id",
            "GYR-MTH-P001-Q001-IMG02",
            "--alt",
            "Scanned from GL Assessment Practice-01.pdf",
        ],
    )?;
    assert!(!ok_leak, "put-asset must reject Clean-Room source leaks");

    // 5. Upsert `paper.json` -> auto-updates `ledger.json` (`verified 1/5`, missing `Q002..Q005`)
    let paper_pkg = sample_paper_package()?;
    let paper_json_path = tmp.join("paper.json");
    std::fs::write(&paper_json_path, serde_json::to_string_pretty(&paper_pkg)?)?;
    let paper_json_str = paper_json_path.to_string_lossy().into_owned();

    let (ok_up, _, err_up) = run_hydrogen_bin(&tmp, &["upsert-paper", &paper_json_str, "--html"])?;
    assert!(ok_up, "upsert-paper --html failed: {err_up}");

    // 6. Upsert a single-question replica (`GYR-MTH-P001-Q001-R01`) -> verify replica lineage in `ledger.json` + `status.html`
    let mut rep_q = paper_pkg.questions[0].clone();
    rep_q.seq_no = 2;
    rep_q.label = "Q1-R1".to_string();
    rep_q.title = "Simultaneous Ticket Bundles (Replica 01)".to_string();
    rep_q.is_replica = true;
    rep_q.parent_question_id = Some("GYR-MTH-P001-Q001".to_string());
    rep_q.question.question_id = "GYR-MTH-P001-Q001-R01".to_string();
    rep_q.solution.question_id = "GYR-MTH-P001-Q001-R01".to_string();
    let rep_json_path = tmp.join("q001-r01.json");
    std::fs::write(&rep_json_path, serde_json::to_string_pretty(&rep_q)?)?;
    let rep_json_str = rep_json_path.to_string_lossy().into_owned();
    let (ok_rep, _, err_rep) =
        run_hydrogen_bin(&tmp, &["upsert-question", &rep_json_str, "--html"])?;
    assert!(ok_rep, "upsert-question replica failed: {err_rep}");

    assert!(
        !tmp.join("products/gyaraplus/maths/private-xref.json")
            .exists(),
        "private-xref.json must not exist (merged into unified ledger.json)"
    );

    let ledger_final =
        ProductLedger::load_or_new(&cfg.ledger_path("gyaraplus", "maths"), "gyaraplus", "maths")?;
    let p_final = ledger_final
        .papers
        .get("GYR-MTH-P001")
        .ok_or_else(|| HydrogenError::Validation("missing GYR-MTH-P001 after upsert".into()))?;
    assert_eq!(p_final.verified_count, 1);
    assert_eq!(p_final.replica_count, 1);
    assert_eq!(p_final.page_count, Some(1));
    assert_eq!(p_final.dpi, Some(180.0));
    assert_eq!(p_final.expected_questions, Some(5));
    assert_eq!(p_final.source_file.as_deref(), Some("Sheet-01.png"));
    assert_eq!(
        p_final.archived_source_path.as_deref(),
        Some("archive/GYR-MTH-P001/source/Sheet-01.png")
    );
    assert_eq!(p_final.missing_question_numbers, vec![2, 3, 4, 5]);
    assert_eq!(p_final.stage, SourceStage::InProgress);

    let q_item = ledger_final.items.get("GYR-MTH-P001-Q001").ok_or_else(|| {
        HydrogenError::Validation("missing GYR-MTH-P001-Q001 in ledger.items".into())
    })?;
    assert_eq!(q_item.source_file.as_deref(), Some("Sheet-01.png"));
    assert!(q_item.bbox.is_some());
    assert!(q_item.bbox_px.is_some());
    assert_eq!(
        q_item.crop_path.as_deref(),
        Some("archive/GYR-MTH-P001/crops/GYR-MTH-P001-Q001.png")
    );
    assert_eq!(q_item.qa_status.as_deref(), Some("verified"));
    assert_eq!(q_item.replica_ids, vec!["GYR-MTH-P001-Q001-R01"]);

    let rep_item = ledger_final
        .items
        .get("GYR-MTH-P001-Q001-R01")
        .ok_or_else(|| HydrogenError::Validation("missing replica in ledger.items".into()))?;
    assert!(rep_item.is_replica);
    assert_eq!(rep_item.replica_scope, Some(ReplicaScope::SingleQuestion));
    assert_eq!(
        rep_item.parent_question_id.as_deref(),
        Some("GYR-MTH-P001-Q001")
    );
    assert_eq!(rep_item.source_file.as_deref(), Some("Sheet-01.png"));
    assert_eq!(
        rep_item.crop_path.as_deref(),
        Some("archive/GYR-MTH-P001/crops/GYR-MTH-P001-Q001.png")
    );

    assert!(
        tmp.join("products/gyaraplus/maths/gyara-maths.sqlite")
            .is_file()
    );
    let html_path = cfg.status_html_path("gyaraplus", "maths");
    assert!(html_path.is_file());
    let html_content = std::fs::read_to_string(&html_path)?;
    assert!(html_content.contains("#FDFBF7"));
    assert!(html_content.contains("gyaraplus/maths"));
    assert!(html_content.contains("gyara-maths.raphs"));
    assert!(!html_content.contains("Verbal Reasoning"));
    assert!(!html_content.contains("private-xref.json"));
    assert!(html_content.contains("Single-Question Replica"));
    assert!(html_content.contains("Original Question Only"));
    assert!(html_content.contains("GYR-MTH-P001-Q001-R01"));
    assert!(html_content.contains("data:image/png;base64,"));
    assert!(html_content.contains("RENAMED DUPLICATE"));
    assert!(html_content.contains("Original source:"));
    assert!(html_content.contains("Sheet-01.png"));
    assert!(html_content.contains("Q002..Q005"));
    assert!(html_content.contains("Simultaneous Ticket Bundles"));

    std::fs::remove_dir_all(&tmp)?;
    Ok(())
}

#[tokio::test]
async fn phase3_two_way_watch_persistence_comments_cli_and_headless_capture_bounds() -> Result<()> {
    let (tmp, cfg) = setup_temp_workspace("p3-watch-carbon")?;

    // Slice a crop and upsert Q001 so both `ledger.json` and `gyara-maths.sqlite` are populated
    let sheet_path = cfg
        .subject_inbox_dir("gyaraplus", "maths")
        .join("Practice-Sheet.png");
    std::fs::write(&sheet_path, synthetic_sheet_png()?)?;
    let sheet_str = sheet_path.to_string_lossy().into_owned();
    let (ok_sl, _, err_sl) = run_hydrogen_bin(
        &tmp,
        &[
            "slice-paper",
            &sheet_str,
            "--paper-id",
            "GYR-MTH-P001",
            "--auto",
        ],
    )?;
    assert!(ok_sl, "slice failed: {err_sl}");

    let paper_pkg = sample_paper_package()?;
    let paper_json_path = tmp.join("paper.json");
    std::fs::write(&paper_json_path, serde_json::to_string_pretty(&paper_pkg)?)?;
    let paper_json_str = paper_json_path.to_string_lossy().into_owned();
    let (ok_up, _, err_up) = run_hydrogen_bin(&tmp, &["upsert-paper", &paper_json_str])?;
    assert!(ok_up, "upsert-paper failed: {err_up}");

    // Spawn real prebuilt headless `carbon-shell` on an isolated ephemeral port
    let (ws_url, _carbon_guard) = spawn_ephemeral_headless_carbon().await?;

    // 1. Verify `hydrogen capture-carbon --check-bounds` CLI against real headless Carbon
    let full_snap_path = tmp.join("captures/q001-full.png");
    let full_snap_str = full_snap_path.to_string_lossy().into_owned();
    let (ok_cap, out_cap, err_cap) = run_hydrogen_bin(
        &tmp,
        &[
            "capture-carbon",
            "GYR-MTH-P001-Q001",
            "--step",
            "S01",
            "--level",
            "9",
            "--reveal-answer",
            "--check-bounds",
            "--out",
            &full_snap_str,
            "--carbon",
            &ws_url,
        ],
    )?;
    assert!(ok_cap, "capture-carbon --check-bounds failed: {err_cap}");
    assert!(out_cap.contains("BOUNDS OK"));
    let cap_bytes = std::fs::read(&full_snap_path)?;
    let (cap_w, cap_h) = read_png_dimensions(&cap_bytes)
        .ok_or_else(|| HydrogenError::Validation("captured file is not a valid PNG".into()))?;
    assert!(cap_w > 200 && cap_h > 200);

    // 2. Connect Hydrogen `--watch` driver + second UI client to exercise real two-way Carbon events
    {
        let vault = VaultManager::open_or_create(
            cfg.vault_sqlite_path("gyaraplus", "maths"),
            cfg.vault_bundle_path("gyaraplus", "maths"),
            &cfg.lesson_key,
            &cfg.device_id,
        )?;
        let snap = vault.catalog_snapshot().await?;

        let driver =
            CarbonClient::connect(&ws_url, "hydrogen-watch-driver", Duration::from_secs(3)).await?;
        let _ = driver.push_author_catalog(&snap, None).await?;
        let mut events_rx = driver
            .take_event_receiver()
            .await
            .ok_or_else(|| HydrogenError::Validation("missing driver event_rx".into()))?;

        let ui_client =
            CarbonClient::connect(&ws_url, "ipad-author-ui", Duration::from_secs(3)).await?;
        let author_actor = ClientActor {
            student_id: None,
            device_id: Some("ipad-author-ui".to_string()),
            role: Some(ActorRole::Teacher),
        };

        // 2a. Save a `Snap Board` comment (`cmt-snap-01`) with PNG attachment + temp comment (`cmt-temp-02`)
        let snap_png = RgbaBuffer {
            width: 32,
            height: 20,
            data: vec![180u8; 32 * 20 * 4],
        }
        .encode_png()?;

        let _ = ui_client
            .send_interaction(
                author_actor.clone(),
                ClientInteraction::SaveComment {
                    entry: FeedbackEntry {
                        comment_id: Some("cmt-snap-01".to_string()),
                        item_id: "GYR-MTH-P001-Q001".to_string(),
                        part_id: Some("main".to_string()),
                        step_id: Some("S01".to_string()),
                        kind: CommentKind::Fix,
                        text: "Highlight matching 3C in royal blue before subtracting.".to_string(),
                        status: FeedbackStatus::Pending,
                        attachments: vec![FeedbackAttachment {
                            id: "att-snap-01".to_string(),
                            source: Some(AttachmentSource::BoardCapture),
                            pane: Some(PaneId::Right),
                            mime_type: "image/png".to_string(),
                            filename: Some("board-snap-1.png".to_string()),
                            width_px: Some(32),
                            height_px: Some(20),
                            data_ref: String::new(),
                            data_base64: Some(encode_base64(&snap_png)),
                            caption: None,
                        }],
                        author_meta: Some(json!({ "sublevel": 8 })),
                        resolution_note: None,
                        created_at_ms: Some(1_763_838_100_000),
                        updated_at_ms: Some(1_763_838_100_000),
                    },
                },
            )
            .await?;

        let _ = ui_client
            .send_interaction(
                author_actor.clone(),
                ClientInteraction::SaveComment {
                    entry: FeedbackEntry {
                        comment_id: Some("cmt-temp-02".to_string()),
                        item_id: "GYR-MTH-P001-Q001".to_string(),
                        part_id: Some("main".to_string()),
                        step_id: None,
                        kind: CommentKind::Note,
                        text: "Temporary scratch comment to delete.".to_string(),
                        status: FeedbackStatus::Pending,
                        attachments: Vec::new(),
                        author_meta: None,
                        resolution_note: None,
                        created_at_ms: None,
                        updated_at_ms: None,
                    },
                },
            )
            .await?;

        let _ = ui_client
            .send_interaction(
                author_actor.clone(),
                ClientInteraction::DeleteComment {
                    comment_id: Some("cmt-temp-02".to_string()),
                    item_id: "GYR-MTH-P001-Q001".to_string(),
                    part_id: Some("main".to_string()),
                },
            )
            .await?;

        let _ = ui_client
            .send_interaction(
                author_actor,
                ClientInteraction::RequestCatalogAction {
                    action: "create-replica".to_string(),
                    target_kind: CatalogSelectionKind::Question,
                    target_id: "GYR-MTH-P001-Q001".to_string(),
                },
            )
            .await?;

        // Drain and persist all 4 real events emitted by headless Carbon
        let mut saw_saved_snap = false;
        let mut saw_deleted_temp = false;
        let mut saw_replica_task = false;

        for _ in 0..12 {
            let Ok(Some(env)) = timeout(Duration::from_secs(2), events_rx.recv()).await else {
                break;
            };
            if let Some(outcome) = handle_watch_persistence_event(
                &env.event,
                &cfg,
                "gyaraplus",
                "maths",
                &vault,
                &driver,
                "GYR-MTH-P001-Q001",
            )
            .await?
            {
                match outcome {
                    WatchPersistenceOutcome::CommentSaved {
                        comment_id,
                        attachment_count,
                        ..
                    } if comment_id == "cmt-snap-01" => {
                        assert_eq!(attachment_count, 1);
                        saw_saved_snap = true;
                    }
                    WatchPersistenceOutcome::CommentDeleted {
                        comment_id,
                        deleted,
                        ..
                    } if comment_id == "cmt-temp-02" => {
                        assert!(deleted);
                        saw_deleted_temp = true;
                    }
                    WatchPersistenceOutcome::ReplicaQueued { target_id, .. }
                        if target_id == "GYR-MTH-P001-Q001" =>
                    {
                        saw_replica_task = true;
                    }
                    _ => {}
                }
            }
            if saw_saved_snap && saw_deleted_temp && saw_replica_task {
                break;
            }
        }

        assert!(saw_saved_snap, "expected comment.saved for cmt-snap-01");
        assert!(saw_deleted_temp, "expected comment.deleted for cmt-temp-02");
        assert!(
            saw_replica_task,
            "expected catalog.actionRequested create-replica"
        );

        let open_in_vault = vault
            .list_comments(Some("open"), Some("GYR-MTH-P001-Q001"))
            .await?;
        assert_eq!(open_in_vault.len(), 1);
        assert_eq!(open_in_vault[0].comment_id, "cmt-snap-01");
        assert_eq!(open_in_vault[0].sublevel, Some(8));
        assert_eq!(
            vault.read_comment_attachment("att-snap-01").await?,
            snap_png
        );

        // 2b. Verify headless Carbon live bounds inspector catches horizontal out-of-pane clipping (`x > 1.0`)
        let _ = driver
            .send_batch(vec![
                CarbonCommand::SetCatalogOpen(carbon_protocol::SetCatalogOpenParams {
                    open: false,
                    ..carbon_protocol::SetCatalogOpenParams::default()
                }),
                CarbonCommand::SetSurface(carbon_protocol::SetSurfaceParams {
                    pane: PaneId::Right,
                    surface: carbon_protocol::SurfaceSpec {
                        scrollable: true,
                        ..carbon_protocol::SurfaceSpec::default()
                    },
                }),
                CarbonCommand::Write(WriteParams {
                    id: "clipped-offscreen-x".to_string(),
                    pane: PaneId::Right,
                    zone: SpatialZoneId::Spine,
                    anchor: AnchorSpec::Coords { x: 1.30, y: 0.20 },
                    content: "$3x + 5 = 20$".to_string(),
                    speed: SpeedSpec::Preset(SpeedPreset::Instant),
                    ..WriteParams::default()
                }),
            ])
            .await?;
        let clipped_state = driver.get_state(true).await?;
        let clipped_rep = inspect_board_bounds(&clipped_state, Some(PaneId::Right));
        assert!(
            !clipped_rep.ok,
            "expected out-of-pane clipping for x=1.30 on right pane"
        );
        assert!(
            clipped_rep
                .issues
                .iter()
                .any(|i| i.kind == BoundsIssueKind::OutOfPaneClipping)
        );
    } // `vault` dropped cleanly here (Rule A6) before running CLI `comments` & `status` subprocesses!

    // 3. Verify `ledger.json` queued replica task + `status --html` comment thumbnail
    let ledger =
        ProductLedger::load_or_new(&cfg.ledger_path("gyaraplus", "maths"), "gyaraplus", "maths")?;
    assert_eq!(ledger.tasks.len(), 1);
    assert_eq!(ledger.tasks[0].kind, AuthoringTaskKind::CreateReplica);
    assert_eq!(ledger.tasks[0].target_id, "GYR-MTH-P001-Q001");

    let (ok_st, _, err_st) =
        run_hydrogen_bin(&tmp, &["status", "--product", "gyaraplus", "--html"])?;
    assert!(ok_st, "status --html failed: {err_st}");
    let html_with_cmt = std::fs::read_to_string(cfg.status_html_path("gyaraplus", "maths"))?;
    assert!(html_with_cmt.contains("cmt-snap-01"));
    assert!(html_with_cmt.contains("snap-thumb"));

    // 4. Exercise `hydrogen comments [list | show --export-dir | resolve]` CLI lifecycle
    let (ok_list, out_list, err_list) = run_hydrogen_bin(&tmp, &["comments", "list"])?;
    assert!(ok_list, "comments list failed: {err_list}");
    assert!(out_list.contains("cmt-snap-01"));

    let export_dir = tmp.join("exported-comment");
    let export_dir_str = export_dir.to_string_lossy().into_owned();
    let (ok_show, out_show, err_show) = run_hydrogen_bin(
        &tmp,
        &[
            "comments",
            "show",
            "cmt-snap-01",
            "--export-dir",
            &export_dir_str,
        ],
    )?;
    assert!(ok_show, "comments show --export-dir failed: {err_show}");
    assert!(out_show.contains("cmt-snap-01"));
    assert!(export_dir.join("comment.json").is_file());
    assert!(export_dir.join("att-snap-01.png").is_file());
    assert!(
        export_dir
            .join("source-crop-GYR-MTH-P001-Q001.png")
            .is_file()
    );
    assert!(export_dir.join("GYR-MTH-P001-Q001-vault.json").is_file());

    let (ok_res, out_res, err_res) = run_hydrogen_bin(
        &tmp,
        &[
            "comments",
            "resolve",
            "cmt-snap-01",
            "--note",
            "Highlighted 3C in c1 royal blue.",
        ],
    )?;
    assert!(ok_res, "comments resolve failed: {err_res}");
    assert!(out_res.contains("Resolved comment cmt-snap-01"));

    let (ok_open_after, out_open_after, _) = run_hydrogen_bin(&tmp, &["comments", "list"])?;
    assert!(ok_open_after);
    assert!(out_open_after.contains("No open comments"));

    std::fs::remove_dir_all(&tmp)?;
    Ok(())
}
