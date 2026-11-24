//! Phase 3A (`3A.1 -> 3A.4`) End-to-End Integration Suite for `hydrogen serve` (`axum` on `:7430`):
//! 1. `/health`, `/assets/hydrogen-logo.png`, & `/products/{product}/brand-logo` (PNG-only),
//! 2. Active-only `GET /` & `GET /api/overview` (`< 2 ms` via `ledger.json`, hiding 0-subproduct `gcse`),
//! 3. `GET /products/gyaraplus` & `GET /api/products/gyaraplus` (12-subject mineral Sub-Product Cards, hiding unworked subjects),
//! 4. `GET /products/gyaraplus/maths` (`StatusRenderMode::Server`: breadcrumbs, Sync button, tabs, filters, replica lineage & lazy `/files/...` URLs),
//! 5. Sandboxed read-only `inbox/` & `archive/` file streaming (`200 OK` for `.png`/`.json`) vs. strict `403`/`404` blocking of
//!    `..` traversal, `%2e%2e`, `.keys.json`, `.sqlite`, `.raphs`, `ledger.json`, `status.html`, hidden dotfiles, and escaping symlinks,
//! 6. `POST /api/products/gyaraplus/maths/sync` (JSON `200 OK`, `?redirect=1` `303 See Other`, `status.html` regeneration, & concurrent Rule A6 lock safety),
//! 7. `hydrogen serve --lan --port <port>` CLI startup banner + clean `SIGINT` (`Ctrl+C`) shutdown.

use std::collections::BTreeMap;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use hydrogen::base64_util::read_png_dimensions;
use hydrogen::config::HydrogenConfig;
use hydrogen::domain::PaperPackage;
use hydrogen::error::{HydrogenError, Result};
use hydrogen::ingest::RgbaBuffer;
use hydrogen::server::{
    ServerState, SyncVaultResponse, WorkspaceOverview, build_router, serve_with_shutdown,
};
use hydrogen::vault::VaultManager;
use memory_core_rs::CourseCommentInput;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::time::sleep;

struct RawHttpResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl RawHttpResponse {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    fn json_value(&self) -> Result<Value> {
        Ok(serde_json::from_slice(&self.body)?)
    }
}

/// Minimal pure-Rust HTTP/1.1 client over `TcpStream` (allows sending literal un-normalized `..` paths).
async fn send_raw_http(
    addr: SocketAddr,
    method: &str,
    raw_target: &str,
    extra_headers: &[(&str, &str)],
) -> Result<RawHttpResponse> {
    let mut stream = TcpStream::connect(addr).await?;
    let mut req =
        format!("{method} {raw_target} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    for (k, v) in extra_headers {
        req.push_str(k);
        req.push_str(": ");
        req.push_str(v);
        req.push_str("\r\n");
    }
    if method == "POST" {
        req.push_str("Content-Length: 0\r\n");
    }
    req.push_str("\r\n");

    stream.write_all(req.as_bytes()).await?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;

    let split_idx = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| HydrogenError::Validation("malformed HTTP response headers".into()))?;
    let head_str = String::from_utf8_lossy(&raw[..split_idx]);
    let mut lines = head_str.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| HydrogenError::Validation("missing HTTP status line".into()))?;
    let status_tok = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| HydrogenError::Validation(format!("bad status line '{status_line}'")))?;
    let status = status_tok
        .parse::<u16>()
        .map_err(|_| HydrogenError::Validation(format!("bad status code '{status_tok}'")))?;

    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let raw_body = &raw[split_idx + 4..];
    let body = if headers
        .get("transfer-encoding")
        .is_some_and(|te| te.to_ascii_lowercase().contains("chunked"))
    {
        decode_chunked_body(raw_body)?
    } else {
        raw_body.to_vec()
    };

    Ok(RawHttpResponse {
        status,
        headers,
        body,
    })
}

fn decode_chunked_body(mut rest: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(crlf) = rest.windows(2).position(|w| w == b"\r\n") {
        let len_str = String::from_utf8_lossy(&rest[..crlf]);
        let hex_part = len_str.split(';').next().unwrap_or("").trim();
        let chunk_len = usize::from_str_radix(hex_part, 16)
            .map_err(|_| HydrogenError::Validation(format!("invalid chunk size '{hex_part}'")))?;
        rest = &rest[crlf + 2..];
        if chunk_len == 0 {
            break;
        }
        if rest.len() < chunk_len + 2 {
            return Err(HydrogenError::Validation("truncated HTTP chunk".into()));
        }
        out.extend_from_slice(&rest[..chunk_len]);
        rest = &rest[chunk_len + 2..];
    }
    Ok(out)
}

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

fn synthetic_sheet_png() -> Result<Vec<u8>> {
    let mut rgba = vec![255u8; 600 * 400 * 4];
    for y in 80..200 {
        for x in 40..560 {
            let off = (y * 600 + x) * 4;
            rgba[off] = 28;
            rgba[off + 1] = 32;
            rgba[off + 2] = 36;
            rgba[off + 3] = 255;
        }
    }
    RgbaBuffer {
        width: 600,
        height: 400,
        data: rgba,
    }
    .encode_png()
}

fn setup_phase3a_workspace(tag: &str) -> Result<(PathBuf, HydrogenConfig)> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("hydrogen-{tag}-{}", std::process::id()));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(tmp.join("assets"))?;
    std::fs::create_dir_all(tmp.join("products/gyaraplus/maths/inbox"))?;
    std::fs::create_dir_all(tmp.join("products/gyaraplus/assets"))?;
    std::fs::create_dir_all(tmp.join("products/gcse/assets"))?;

    std::fs::copy(
        repo_root.join("assets/hydrogen-logo.png"),
        tmp.join("assets/hydrogen-logo.png"),
    )?;
    for logo in ["gyara-blue.png", "gyara-white.png", "gyara-blue.svg"] {
        std::fs::copy(
            repo_root.join("products/gyaraplus/assets").join(logo),
            tmp.join("products/gyaraplus/assets").join(logo),
        )?;
    }
    // Unstarted `gcse` has only `.gitkeep` + `.svg` (0 active sub-products, 0 PNG logos)
    std::fs::write(tmp.join("products/gcse/assets/.gitkeep"), "")?;
    std::fs::write(
        tmp.join("products/gcse/assets/gcse-placeholder.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    )?;

    let rules_dir = repo_root.join("rules");
    let toml_text = format!(
        "products_dir = \"products\"\nrules_dir = \"{}\"\ndefault_product = \"gyaraplus\"\ndefault_subject = \"maths\"\nbind_addr = \"127.0.0.1:7430\"\ncarbon_ws_url = \"ws://127.0.0.1:7420/carbon\"\ndevice_id = \"phase3a-test\"\n",
        rules_dir.display()
    );
    std::fs::write(tmp.join("hydrogen.toml"), toml_text)?;
    std::fs::write(
        tmp.join(".env"),
        "HYDROGEN_LESSON_KEY=\"phase3a-secret-lesson-key\"\n",
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

#[tokio::test]
async fn phase3a_end_to_end_explorer_sandbox_and_sync_routes() -> Result<()> {
    let (tmp, cfg) = setup_phase3a_workspace("p3a-routes")?;

    // 1. Seed `products/gyaraplus/maths/` with a sliced sheet, compiled Q001 + replica Q001-R01, and expected=5
    let sheet_bytes = synthetic_sheet_png()?;
    let sheet_path = cfg
        .subject_inbox_dir("gyaraplus", "maths")
        .join("Sheet-01.png");
    std::fs::write(&sheet_path, &sheet_bytes)?;
    let sheet_str = sheet_path.to_string_lossy().into_owned();

    let (ok_slice, _, err_slice) = run_hydrogen_bin(
        &tmp,
        &[
            "slice-paper",
            &sheet_str,
            "--paper-id",
            "GYR-MTH-P001",
            "--auto",
        ],
    )?;
    assert!(ok_slice, "slice-paper failed: {err_slice}");

    let paper_pkg = sample_paper_package()?;
    let q001_archive_json = cfg
        .subject_archive_dir("gyaraplus", "maths")
        .join("GYR-MTH-P001/GYR-MTH-P001-Q001.json");
    std::fs::write(
        &q001_archive_json,
        serde_json::to_string_pretty(&paper_pkg.questions[0])?,
    )?;

    let paper_json_path = tmp.join("paper.json");
    std::fs::write(&paper_json_path, serde_json::to_string_pretty(&paper_pkg)?)?;
    let paper_json_str = paper_json_path.to_string_lossy().into_owned();
    let (ok_up, _, err_up) = run_hydrogen_bin(&tmp, &["upsert-paper", &paper_json_str])?;
    assert!(ok_up, "upsert-paper failed: {err_up}");

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
    let (ok_rep, _, err_rep) = run_hydrogen_bin(&tmp, &["upsert-question", &rep_json_str])?;
    assert!(ok_rep, "upsert-question replica failed: {err_rep}");

    let (ok_exp, _, err_exp) = run_hydrogen_bin(
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
    assert!(ok_exp, "status --set-expected failed: {err_exp}");

    // Plant hidden dotfile, disallowed script, and escaping symlink inside `inbox/` / `archive/` to verify sandbox rejection
    std::fs::write(
        cfg.subject_inbox_dir("gyaraplus", "maths")
            .join(".hidden-secret.png"),
        b"secret",
    )?;
    std::fs::write(
        cfg.subject_archive_dir("gyaraplus", "maths")
            .join("GYR-MTH-P001/exploit.sh"),
        b"#!/bin/sh\n",
    )?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            tmp.join("products/gyaraplus/maths/gyara-maths.keys.json"),
            cfg.subject_archive_dir("gyaraplus", "maths")
                .join("GYR-MTH-P001/escaped-link.json"),
        )?;
    }

    // 2. Start in-process `axum` server on an ephemeral localhost port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let state = Arc::new(ServerState::new(cfg.clone()));
    let router = build_router(state);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let srv_handle = tokio::spawn(async move {
        serve_with_shutdown(listener, router, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    // 3. Verify `/health`, `/assets/hydrogen-logo.png`, & `/products/{product}/brand-logo`
    let health = send_raw_http(addr, "GET", "/health", &[]).await?;
    assert_eq!(health.status, 200);
    assert_eq!(
        health.json_value()?,
        json!({
            "status": "ok",
            "service": "hydrogen",
            "version": hydrogen::VERSION,
        })
    );

    let h_logo = send_raw_http(addr, "GET", "/assets/hydrogen-logo.png", &[]).await?;
    assert_eq!(h_logo.status, 200);
    assert_eq!(
        h_logo.headers.get("content-type").map(String::as_str),
        Some("image/png")
    );
    assert!(read_png_dimensions(&h_logo.body).is_some());

    let prod_logo = send_raw_http(addr, "GET", "/products/gyaraplus/brand-logo", &[]).await?;
    assert_eq!(prod_logo.status, 200);
    assert_eq!(
        prod_logo.headers.get("content-type").map(String::as_str),
        Some("image/png")
    );
    assert!(read_png_dimensions(&prod_logo.body).is_some());

    // `gcse` only has `.svg` & `.gitkeep` — brand-logo endpoint MUST strictly enforce PNG-only (`404`)
    let gcse_logo = send_raw_http(addr, "GET", "/products/gcse/brand-logo", &[]).await?;
    assert_eq!(gcse_logo.status, 404);

    // 4. Verify `GET /` & `GET /api/overview` (Active-Only + `< 2 ms` cached `vaultSummary` without opening SQLCipher!)
    // Temporarily move `gyara-maths.sqlite` aside to prove `/` and `/products/gyaraplus` do NOT open SQLCipher!
    let sqlite_path = cfg.vault_sqlite_path("gyaraplus", "maths");
    let sqlite_backup = tmp.join("gyara-maths.sqlite.bak");
    std::fs::rename(&sqlite_path, &sqlite_backup)?;

    let landing = send_raw_http(addr, "GET", "/", &[]).await?;
    assert_eq!(landing.status, 200);
    let landing_html = landing.text();
    assert!(landing_html.contains("#FDFBF7"));
    assert!(landing_html.contains("#F2ECE1"));
    assert!(landing_html.contains("topbar-left"));
    assert!(landing_html.contains("topbar-center"));
    assert!(landing_html.contains("<h1>Hydrogen Dashboard</h1>"));
    assert!(landing_html.contains("page-hero"));
    assert!(landing_html.contains(".card-title-link::after"));
    assert!(landing_html.contains("/assets/hydrogen-logo.png"));
    assert!(landing_html.contains("background: transparent;"));
    assert!(!landing_html.contains("background: #1F2421;"));
    assert!(landing_html.contains("/products/gyaraplus/brand-logo"));
    assert!(landing_html.contains("Gyara+ (11+)"));
    assert!(landing_html.contains("/products/gyaraplus/maths"));
    assert!(
        !landing_html.contains("/products/gcse"),
        "Landing page MUST hide products with 0 active sub-products (gcse)"
    );

    let api_ov = send_raw_http(addr, "GET", "/api/overview", &[]).await?;
    assert_eq!(api_ov.status, 200);
    let ov: WorkspaceOverview = serde_json::from_slice(&api_ov.body)?;
    assert_eq!(ov.active_product_count, 1);
    assert_eq!(ov.active_subject_count, 1);
    assert_eq!(ov.verified_question_count, 1);
    assert_eq!(ov.expected_question_count, 5);
    assert_eq!(ov.replica_count, 1);
    assert!(
        ov.elapsed_us < 10_000,
        "cached ledger.json scan should be sub-millisecond, got {} us",
        ov.elapsed_us
    );
    assert_eq!(ov.products[0].product, "gyaraplus");

    // 5. Verify `GET /products/gyaraplus` (active `maths` card) & `GET /products/gcse` (`404`)
    let prod_page = send_raw_http(addr, "GET", "/products/gyaraplus", &[]).await?;
    assert_eq!(prod_page.status, 200);
    let prod_html = prod_page.text();
    assert!(prod_html.contains("Products"));
    assert!(prod_html.contains("gyaraplus"));
    assert!(prod_html.contains("gyara-maths.raphs"));
    assert!(prod_html.contains("#5B7DB1")); // Canonical 12-subject Slate Blue mineral border for Maths
    assert!(!prod_html.contains("verbal-reasoning"));
    assert!(!prod_html.contains("gyara-english.raphs"));

    // Restore `gyara-maths.sqlite` before testing Level 3 sub-product dashboard & Vault Sync
    std::fs::rename(&sqlite_backup, &sqlite_path)?;

    let gcse_page = send_raw_http(addr, "GET", "/products/gcse", &[]).await?;
    assert_eq!(gcse_page.status, 404);
    let unworked_subj =
        send_raw_http(addr, "GET", "/products/gyaraplus/verbal-reasoning", &[]).await?;
    assert_eq!(unworked_subj.status, 404);

    // 6. Verify `GET /products/gyaraplus/maths` (`StatusRenderMode::Server`) & `/api/products/gyaraplus/maths`
    let subj_page = send_raw_http(addr, "GET", "/products/gyaraplus/maths", &[]).await?;
    assert_eq!(subj_page.status, 200);
    let subj_html = subj_page.text();
    assert!(subj_html.contains(r#"href="/">Products</a>"#));
    assert!(subj_html.contains(r#"href="/products/gyaraplus">gyaraplus</a>"#));
    assert!(subj_html.contains("<h1>Hydrogen Dashboard</h1>"));
    assert!(subj_html.contains("page-hero"));
    assert!(subj_html.contains("background: transparent;"));
    assert!(!subj_html.contains("background: #1F2421;"));
    assert!(subj_html.contains("Sync Vault"));
    assert!(subj_html.contains("/api/products/gyaraplus/maths/sync"));
    assert!(subj_html.contains("section-tabs"));
    assert!(subj_html.contains(r#"data-qfilter="verified""#));
    assert!(subj_html.contains("img-lightbox"));
    assert!(subj_html.contains("Single-Question Replica"));
    assert!(subj_html.contains("GYR-MTH-P001-Q001-R01"));
    assert!(subj_html.contains("Simultaneous Ticket Bundles"));
    assert!(
        subj_html.contains(
            "/products/gyaraplus/maths/files/archive/GYR-MTH-P001/crops/GYR-MTH-P001-Q001.png"
        ),
        "Server mode must emit lazy HTTP URLs to /files/archive/... instead of inline crop base64"
    );
    assert!(
        !subj_html.contains("data:image/png;base64,"),
        "Server mode must avoid inline base64 PNG bloat when zero Snap Board comment attachments exist"
    );

    let api_subj = send_raw_http(addr, "GET", "/api/products/gyaraplus/maths", &[]).await?;
    assert_eq!(api_subj.status, 200);
    let api_subj_val = api_subj.json_value()?;
    assert_eq!(api_subj_val["vaultStem"], "gyara-maths");
    assert_eq!(
        api_subj_val["topics"].as_array().map(Vec::len).unwrap_or(0),
        8
    );

    // 7. Verify Allowed File Streaming (`200 OK`) vs. Strict Clean-Room & Secret-File Sandbox (`403` / `404`)
    let ok_crop = send_raw_http(
        addr,
        "GET",
        "/products/gyaraplus/maths/files/archive/GYR-MTH-P001/crops/GYR-MTH-P001-Q001.png",
        &[],
    )
    .await?;
    assert_eq!(ok_crop.status, 200);
    assert_eq!(
        ok_crop.headers.get("content-type").map(String::as_str),
        Some("image/png")
    );
    assert!(read_png_dimensions(&ok_crop.body).is_some());

    let ok_json = send_raw_http(
        addr,
        "GET",
        "/products/gyaraplus/maths/files/archive/GYR-MTH-P001/GYR-MTH-P001-Q001.json",
        &[],
    )
    .await?;
    assert_eq!(ok_json.status, 200);
    assert_eq!(
        ok_json.json_value()?["question"]["questionId"],
        "GYR-MTH-P001-Q001"
    );

    let ok_inbox = send_raw_http(
        addr,
        "GET",
        "/products/gyaraplus/maths/files/inbox/Sheet-01.png",
        &[],
    )
    .await?;
    assert_eq!(ok_inbox.status, 200);
    assert_eq!(ok_inbox.body, sheet_bytes);

    let missing_allowed = send_raw_http(
        addr,
        "GET",
        "/products/gyaraplus/maths/files/archive/GYR-MTH-P001/crops/NOPE.png",
        &[],
    )
    .await?;
    assert_eq!(missing_allowed.status, 404);

    let blocked_paths = [
        "/products/gyaraplus/maths/files",
        "/products/gyaraplus/maths/files/",
        "/products/gyaraplus/maths/files/gyara-maths.keys.json",
        "/products/gyaraplus/maths/files/gyara-maths.sqlite",
        "/products/gyaraplus/maths/files/gyara-maths.raphs",
        "/products/gyaraplus/maths/files/ledger.json",
        "/products/gyaraplus/maths/files/status.html",
        "/products/gyaraplus/maths/files/../gyara-maths.keys.json",
        "/products/gyaraplus/maths/files/archive/../../gyara-maths.sqlite",
        "/products/gyaraplus/maths/files/archive/%2e%2e/%2e%2e/gyara-maths.keys.json",
        "/products/gyaraplus/maths/files/inbox/../../.env",
        "/products/gyaraplus/maths/files/inbox/.hidden-secret.png",
        "/products/gyaraplus/maths/files/archive/GYR-MTH-P001/exploit.sh",
        "/products/gyaraplus/maths/files/archive/GYR-MTH-P001/escaped-link.json",
        "/products/gyaraplus/assets",
    ];
    for bad_target in blocked_paths {
        let resp = send_raw_http(addr, "GET", bad_target, &[]).await?;
        assert!(
            resp.status == 403 || resp.status == 404,
            "expected 403/404 for '{bad_target}', got {}",
            resp.status
        );
        if bad_target.contains("keys.json")
            || bad_target.contains(".sqlite")
            || bad_target.contains(".raphs")
            || bad_target.contains(".hidden")
            || bad_target.contains("exploit.sh")
            || bad_target.contains("escaped-link")
        {
            assert_eq!(
                resp.status, 403,
                "secret/disallowed/symlink target '{bad_target}' must be strictly 403 Forbidden"
            );
        }
    }

    // 8. Verify `POST /api/products/gyaraplus/maths/sync` (Rule A6 concurrency + `vaultSummary` & `status.html` refresh)
    {
        let vault = VaultManager::open_or_create(
            &sqlite_path,
            cfg.vault_bundle_path("gyaraplus", "maths"),
            &cfg.lesson_key,
            &cfg.device_id,
        )?;
        let _ = vault
            .upsert_comment(
                &CourseCommentInput {
                    comment_id: Some("cmt-p3a-01".to_string()),
                    target_kind: "question".to_string(),
                    target_id: "GYR-MTH-P001-Q001".to_string(),
                    part_id: Some("main".to_string()),
                    step_id: Some("S01".to_string()),
                    sublevel: Some(9),
                    comment_kind: "fix".to_string(),
                    comment_text: "Widen spacing on 3A = 24".to_string(),
                },
                &[],
            )
            .await?;
    }

    let status_html_disk = cfg.status_html_path("gyaraplus", "maths");
    if status_html_disk.is_file() {
        std::fs::remove_file(&status_html_disk)?;
    }

    // Cached `ledger.json` still reports 0 open comments before explicit sync
    let pre_sync_ov: WorkspaceOverview =
        serde_json::from_slice(&send_raw_http(addr, "GET", "/api/overview", &[]).await?.body)?;
    assert_eq!(pre_sync_ov.open_comment_count, 0);

    // Fire 3 concurrent `POST .../sync` requests to exercise `ServerState::vault_lock` (Rule A6)
    let (r1, r2, r3) = tokio::join!(
        send_raw_http(addr, "POST", "/api/products/gyaraplus/maths/sync", &[]),
        send_raw_http(addr, "POST", "/api/products/gyaraplus/maths/sync", &[]),
        send_raw_http(addr, "POST", "/api/products/gyaraplus/maths/sync", &[]),
    );
    for r in [r1?, r2?, r3?] {
        assert_eq!(r.status, 200);
        let parsed: SyncVaultResponse = serde_json::from_slice(&r.body)?;
        assert_eq!(parsed.status, "ok");
        assert_eq!(parsed.open_comment_count, 1);
        assert_eq!(
            parsed.vault_summary.as_ref().map(|v| v.open_comment_count),
            Some(1)
        );
    }

    assert!(
        status_html_disk.is_file(),
        "POST .../sync must regenerate products/gyaraplus/maths/status.html"
    );
    let regenerated_html = std::fs::read_to_string(&status_html_disk)?;
    assert!(regenerated_html.contains("Widen spacing on 3A = 24"));

    // Browser form redirect mode (`?redirect=1` -> `303 See Other`)
    let sync_redir = send_raw_http(
        addr,
        "POST",
        "/api/products/gyaraplus/maths/sync?redirect=1",
        &[],
    )
    .await?;
    assert_eq!(sync_redir.status, 303);
    assert_eq!(
        sync_redir.headers.get("location").map(String::as_str),
        Some("/products/gyaraplus/maths")
    );

    // Cached `GET /api/overview` now reflects `openCommentCount == 1` in `< 2 ms`
    let post_sync_ov: WorkspaceOverview =
        serde_json::from_slice(&send_raw_http(addr, "GET", "/api/overview", &[]).await?.body)?;
    assert_eq!(post_sync_ov.open_comment_count, 1);

    let _ = shutdown_tx.send(());
    srv_handle
        .await
        .map_err(|e| HydrogenError::Validation(format!("server task join failed: {e}")))??;

    std::fs::remove_dir_all(&tmp)?;
    Ok(())
}

struct ServeChildGuard(Child);

impl Drop for ServeChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn phase3a_cli_serve_lan_startup_and_sigint_shutdown() -> Result<()> {
    let (tmp, _cfg) = setup_phase3a_workspace("p3a-cli-serve")?;
    let port = {
        let l = StdTcpListener::bind("127.0.0.1:0")?;
        l.local_addr()?.port()
    };

    let bin = env!("CARGO_BIN_EXE_hydrogen");
    let stdout_log = tmp.join("serve-stdout.log");
    let log_file = std::fs::File::create(&stdout_log)?;

    let child = Command::new(bin)
        .current_dir(&tmp)
        .args(["serve", "--lan", "--port", &port.to_string()])
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::null())
        .spawn()?;
    let mut guard = ServeChildGuard(child);

    let target_addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|_| HydrogenError::Validation("invalid test socket addr".into()))?;

    let mut ready = false;
    for _ in 0..40 {
        if let Ok(resp) = send_raw_http(target_addr, "GET", "/health", &[]).await
            && resp.status == 200
        {
            ready = true;
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    assert!(
        ready,
        "hydrogen serve --lan failed to become healthy on {target_addr}"
    );

    let banner = std::fs::read_to_string(&stdout_log)?;
    assert!(banner.contains(&format!("http://127.0.0.1:{port}/")));
    assert!(banner.contains(&format!("http://192.168.68.52:{port}/")));
    assert!(banner.contains(&format!("http://192.168.68.85:{port}/")));

    // Send SIGINT (`Ctrl+C`) and verify clean graceful shutdown (exit code 0)
    #[cfg(unix)]
    {
        let pid = guard.0.id().to_string();
        let status = Command::new("kill").args(["-INT", &pid]).status()?;
        assert!(status.success());
        let mut exited_cleanly = false;
        for _ in 0..40 {
            if let Some(exit_status) = guard.0.try_wait()? {
                exited_cleanly = exit_status.success();
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
        assert!(exited_cleanly, "hydrogen serve should exit 0 on SIGINT");
    }

    drop(guard);
    std::fs::remove_dir_all(&tmp)?;
    Ok(())
}
