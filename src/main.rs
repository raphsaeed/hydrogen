//! Unified `hydrogen` CLI entrypoint (zero external argument-parser crates).

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use carbon_protocol::{
    BoardStateSnapshot, CarbonCommand, CarbonEvent, CarbonEventEnvelope, CatalogSelectionKind,
    CatalogSnapshotSummary, ChromeSnapshot, ChromeVisibility, ClientMessage, CommandAck,
    DualTimersSnapshot, ImportantIconStyle, LayoutMode, LayoutSnapshot, LoupeSnapshot,
    NormalizedRect, OpticsSnapshot, PROTOCOL_VERSION, ProgressStatus, ReactiveStateSnapshot,
    ReferenceViewMode, ReminderInterval, ServerMessage, SplitOrientation, SurfaceSpec,
    SurfacesSnapshot, ThemeId, TimerMode, TimerSnapshot, UserInkSummary, decode_client_message,
    encode_server_message,
};
use futures_util::{SinkExt, StreamExt};
use hydrogen::carbon::CarbonClient;
use hydrogen::carbon::catalog::project_full_stage_catalog;
use hydrogen::config::HydrogenConfig;
use hydrogen::error::{HydrogenError, Result};
use hydrogen::theta::{StagePresetConfig, ThetaService};
use hydrogen::vault::VaultManager;
use hydrogen::vault::seed::{GYR_LOGO_BLUE_ID, GYR_LOGO_WHITE_ID};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> ExitCode {
    match run_cli().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run_cli() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let subcmd = args.first().map(String::as_str).unwrap_or("help");

    match subcmd {
        "doctor" => cmd_doctor(&args[1..]).await,
        "init-vault" => cmd_init_vault(&args[1..]).await,
        "catalog" => cmd_catalog(&args[1..]).await,
        "theta" => cmd_theta(&args[1..]),
        "demo" => cmd_demo(&args[1..]).await,
        "-V" | "--version" | "version" => {
            println!("hydrogen v{}", hydrogen::VERSION);
            Ok(())
        }
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        unknown => Err(HydrogenError::Config(format!(
            "unknown subcommand '{unknown}' (run `hydrogen --help`)"
        ))),
    }
}

fn print_help() {
    println!(
        "hydrogen v{} — Multi-Product AI Solution Compiler & L7 .raphs Vault Engine

USAGE:
    hydrogen <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    doctor [--carbon <ws-url>]              Verify Theta (<1ms), gyara-maths.raphs (Schema 2.3.0), & Carbon WS
    init-vault [--product P] [--subject S] [--pack]
                                            Initialize/seed L7 vault + Gyara+ logo blobs (and optionally pack .raphs)
    catalog [--product P] [--subject S] [--carbon-json]
                                            Inspect L7 CourseCatalogSnapshot or emit Carbon Full-Stage JSON
    theta <expr>                            Evaluate an exact expression with 11+ Maths Theta preset
    demo [--carbon <ws-url>]                Run interactive Phase 1 end-to-end demo (Vault + Theta + Carbon WS)
    version                                 Print version",
        hydrogen::VERSION
    );
}

fn workspace_root() -> Result<PathBuf> {
    std::env::current_dir().map_err(HydrogenError::from)
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

async fn cmd_doctor(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let carbon_url = parse_flag_value(args, "--carbon").unwrap_or_else(|| cfg.carbon_ws_url.clone());

    println!("=== Hydrogen Phase 1 Doctor (v{}) ===", hydrogen::VERSION);
    println!("1. Workspace & Rules:");
    println!("   root       : {}", cfg.workspace_root.display());
    println!(
        "   rules/     : {} ({})",
        cfg.rules_dir.display(),
        if cfg.rules_dir.join("gyaraplus/maths/common.md").is_file() {
            "4-tier isolated rules OK"
        } else {
            "MISSING gyaraplus/maths/common.md"
        }
    );

    // 2. Theta check
    let t0 = Instant::now();
    let theta = ThetaService::new(StagePresetConfig::gyara_11plus_maths());
    let eval_out = theta.eval_expr("2 3/4 + 1 1/2")?;
    let mcq_idx = theta.verify_mcq_options(&["3 3/4", "4", "4 1/4", "4 1/2", "5"], "17/4")?;
    let theta_us = t0.elapsed().as_micros();
    println!("2. Theta CAS (`theta_calc` in-process):");
    println!(
        "   eval       : `2 3/4 + 1 1/2` -> exact `{}` ({theta_us} us, MCQ valid idx={mcq_idx})",
        eval_out.exact
    );
    println!("   katex      : {}", eval_out.katex);

    // 3. Vault check
    let product = &cfg.default_product;
    let subject = &cfg.default_subject;
    let sqlite_path = cfg.vault_sqlite_path(product, subject);
    let bundle_path = cfg.vault_bundle_path(product, subject);
    let assets_dir = cfg.product_assets_dir(product);

    let v0 = Instant::now();
    let vault = VaultManager::open_or_create(
        &sqlite_path,
        &bundle_path,
        &cfg.lesson_key,
        &cfg.device_id,
    )?;
    let seeded = vault.seed_gyara_maths(&assets_dir).await?;
    let status = vault.status().await?;
    let check = vault.self_check().await?;
    if !check.ok {
        return Err(HydrogenError::Validation(
            "memory_core_rs self_check reported integrity failure".into(),
        ));
    }
    let c0 = Instant::now();
    let snap = vault.catalog_snapshot().await?;
    let cat_us = c0.elapsed().as_micros();
    let white_asset = vault.get_asset_info(GYR_LOGO_WHITE_ID).await?;
    let blue_asset = vault.get_asset_info(GYR_LOGO_BLUE_ID).await?;
    let vault_ms = v0.elapsed().as_millis();

    println!("3. Course Vault (`memory_core_rs {}` / Schema `{}`):", memory_core_rs::VERSION, status.schema_version);
    println!("   container  : {} ({vault_ms} ms open+seed)", sqlite_path.display());
    println!("   self_check : ok={} · crypto={}", check.ok, status.crypto_version);
    println!(
        "   course_id  : {} · {} topics · {} question types · catalog_snapshot={} us",
        seeded.course_id,
        snap.topics.len(),
        snap.question_types.len(),
        cat_us
    );
    if let (Some(w), Some(b)) = (white_asset, blue_asset) {
        println!(
            "   brand blobs: {} ({}x{} px, {} B) · {} ({}x{} px, {} B)",
            w.asset_id,
            w.width_px.unwrap_or(0),
            w.height_px.unwrap_or(0),
            w.size_bytes,
            b.asset_id,
            b.width_px.unwrap_or(0),
            b.height_px.unwrap_or(0),
            b.size_bytes
        );
    }

    // 4. Carbon probe (live if open, clean standalone note if closed)
    println!("4. Carbon Whiteboard Probe (`{carbon_url}`):");
    match CarbonClient::connect(&carbon_url, &cfg.device_id, Duration::from_millis(800)).await {
        Ok(client) => {
            let pong = client.ping().await?;
            let ready = client.board_ready();
            println!(
                "   LIVE OK    : protocol v{} · gpu={} · viewport={}x{} @ {}x · pong={pong}",
                ready.protocol_version,
                ready.gpu_backend,
                ready.viewport_px[0],
                ready.viewport_px[1],
                ready.scale_factor
            );
        }
        Err(_) => {
            println!(
                "   STANDALONE : No live Carbon window currently listening at {carbon_url} (compile-time carbon-protocol v{} active)",
                PROTOCOL_VERSION
            );
        }
    }

    println!("=== Doctor Check Complete: ALL CORE SYSTEMS READY ===");
    Ok(())
}

async fn cmd_init_vault(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let product = parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject = parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let should_pack = has_flag(args, "--pack");

    let sqlite_path = cfg.vault_sqlite_path(&product, &subject);
    let bundle_path = cfg.vault_bundle_path(&product, &subject);
    let assets_dir = cfg.product_assets_dir(&product);

    let vault = VaultManager::open_or_create(
        &sqlite_path,
        &bundle_path,
        &cfg.lesson_key,
        &cfg.device_id,
    )?;

    if product == "gyaraplus" && subject == "maths" {
        let summary = vault.seed_gyara_maths(&assets_dir).await?;
        println!(
            "Seeded {} in {}: {} assets ({}), {} topics, {} question types",
            summary.course_id,
            sqlite_path.display(),
            summary.embedded_assets.len(),
            summary.embedded_assets.join(", "),
            summary.topic_count,
            summary.question_type_count
        );
    }

    if should_pack {
        let manifest = vault.pack_to_default_bundle().await?;
        println!(
            "Packed self-contained bundle: {} (format v{}, schema v{})",
            bundle_path.display(),
            manifest.format_version,
            manifest.schema_version
        );
    }

    Ok(())
}

async fn cmd_catalog(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let product = parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject = parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let carbon_json = has_flag(args, "--carbon-json");

    let vault = VaultManager::open_or_create(
        cfg.vault_sqlite_path(&product, &subject),
        cfg.vault_bundle_path(&product, &subject),
        &cfg.lesson_key,
        &cfg.device_id,
    )?;

    let snap = vault.catalog_snapshot().await?;
    if carbon_json {
        let logo_uri = vault.read_asset_data_uri(GYR_LOGO_BLUE_ID).await.ok();
        let mut catalog = project_full_stage_catalog(&snap, logo_uri);
        // Truncate massive base64 string unless `--full-logo` is passed so terminal stays readable
        if !has_flag(args, "--full-logo") {
            if let Some(ref mut logo) = catalog.brand_logo
                && logo.len() > 64
            {
                logo.truncate(64);
                logo.push_str("...[truncated-png-base64]");
            }
            for subj in &mut catalog.available_subjects {
                if let Some(ref mut logo) = subj.logo
                    && logo.len() > 64
                {
                    logo.truncate(64);
                    logo.push_str("...[truncated-png-base64]");
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&catalog)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&snap)?);
    }
    Ok(())
}

fn cmd_theta(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(HydrogenError::Config(
            "usage: hydrogen theta \"<expression>\"".into(),
        ));
    }
    let expr = args.join(" ");
    let theta = ThetaService::new(StagePresetConfig::gyara_11plus_maths());
    let t0 = Instant::now();
    let res = theta.eval_expr(&expr)?;
    let elapsed_us = t0.elapsed().as_micros();
    println!("expr   : {expr}");
    println!("exact  : {} ({elapsed_us} us)", res.exact);
    println!("katex  : {}", res.katex);
    if let Some(approx) = res.approx {
        println!("approx : {approx:?}");
    }
    Ok(())
}

async fn cmd_demo(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let external_carbon = parse_flag_value(args, "--carbon");

    println!("==================================================================");
    println!("  HYDROGEN PHASE 1 END-TO-END DEMO (Vault + Theta + Carbon v0.1)  ");
    println!("==================================================================\n");

    // Step 1: Initialize & Pack `gyara-maths.raphs`
    let sqlite_path = cfg.vault_sqlite_path("gyaraplus", "maths");
    let bundle_path = cfg.vault_bundle_path("gyaraplus", "maths");
    let assets_dir = cfg.product_assets_dir("gyaraplus");

    let vault = VaultManager::open_or_create(
        &sqlite_path,
        &bundle_path,
        &cfg.lesson_key,
        &cfg.device_id,
    )?;
    let seeded = vault.seed_gyara_maths(&assets_dir).await?;
    let manifest = vault.pack_to_default_bundle().await?;
    let (white_logo_uri, blue_logo_uri) = vault.read_gyara_brand_logos().await?;

    println!("[Step 1/3] Encrypted L7 Vault (`gyara-maths.raphs`):");
    println!("  - SQLite Container : {}", sqlite_path.display());
    println!(
        "  - Packed Bundle    : {} (Schema v{}, DEK={})",
        bundle_path.display(),
        manifest.schema_version,
        manifest.dek_id
    );
    println!(
        "  - Embedded Logos   : {} ({} chars data URI) & {} ({} chars data URI)",
        GYR_LOGO_WHITE_ID,
        white_logo_uri.len(),
        GYR_LOGO_BLUE_ID,
        blue_logo_uri.len()
    );
    println!(
        "  - Seeded Taxonomy  : {} 11+ Maths Strands · {} Problem Archetypes\n",
        seeded.topic_count, seeded.question_type_count
    );

    // Step 2: Solve & Verify a Sample 11+ Algebraic Word Problem via In-Process `Theta`
    let theta = ThetaService::new(StagePresetConfig::gyara_11plus_maths());
    let t0 = Instant::now();
    let solved = theta.solve_equations(&["a + b = 52", "a - b = 14"], &["a", "b"], true)?;
    let sol_map = solved
        .solutions
        .as_ref()
        .and_then(|v| v.first())
        .ok_or_else(|| HydrogenError::Theta("expected unique simultaneous solution".into()))?;
    let val_a = sol_map
        .get("a")
        .map(|t| t.exact.as_str())
        .unwrap_or("?");
    let val_b = sol_map
        .get("b")
        .map(|t| t.exact.as_str())
        .unwrap_or("?");
    let check_a = theta.eval_expr("(52 + 14) / 2")?;
    let valid_choice = theta.verify_mcq_options(&["19", "26", "33", "38", "66"], val_a)?;
    let theta_us = t0.elapsed().as_micros();

    println!("[Step 2/3] Deterministic `Theta` (`theta_calc`) Verification ({theta_us} us):");
    println!(
        "  - 11+ System       : `a + b = 52`, `a - b = 14` -> solved a={val_a}, b={val_b} (method={})",
        solved.method.as_deref().unwrap_or("exact")
    );
    println!("  - Exact Value `a`  : `{}` | KaTeX: `{}`", check_a.exact, check_a.katex);
    println!(
        "  - 5-Option MCQ Gate: Options [A:19, B:26, C:33, D:38, E:66] -> Sole exact match = Option {} ('33')\n",
        (b'A' + valid_choice as u8) as char
    );

    // Step 3: Connect to Carbon (live URL if supplied/open, else spin up local loopback stage)
    let (target_url, mock_handle) = if let Some(url) = external_carbon {
        (url, None)
    } else {
        let (url, handle) = spawn_demo_loopback_carbon().await?;
        (url, Some(handle))
    };

    let client = CarbonClient::connect(&target_url, &cfg.device_id, Duration::from_secs(3)).await?;
    let pong_ts = client.ping().await?;
    let snap = vault.catalog_snapshot().await?;
    let ack = client
        .push_author_catalog(&snap, Some(blue_logo_uri))
        .await?;
    let state = client.get_state(false).await?;

    println!("[Step 3/3] Live `Carbon Protocol v{}` Handshake & Full-Stage Gallery Push:", PROTOCOL_VERSION);
    println!(
        "  - Connected        : {} (GPU={}, viewport={}x{}, pong={pong_ts})",
        client.ws_url(),
        client.board_ready().gpu_backend,
        client.board_ready().viewport_px[0],
        client.board_ready().viewport_px[1]
    );
    println!(
        "  - Atomic Batch Ack : ok={} (actions={:?})",
        ack.ok, ack.action_ids
    );
    if let Some(cat) = state.catalog {
        println!(
            "  - Carbon State()   : displayMode={:?}, viewLevel={:?}, subjects={}, topics={}, authorMode={}",
            cat.display_mode,
            cat.view_level,
            cat.subject_count,
            cat.topic_count,
            state.chrome.author_mode
        );
    }

    if let Some(mut rx) = client.take_event_receiver().await
        && let Ok(Some(ev)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await
    {
        println!(
            "  - Inbound Event    : seq={} `{}` received cleanly!",
            ev.seq,
            ev.event.event_name()
        );
    }

    if let Some(h) = mock_handle {
        h.abort();
    }

    println!("\n=== Phase 1 End-to-End Demo Succeeded ===");
    Ok(())
}

async fn spawn_demo_loopback_carbon() -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let url = format!("ws://{addr}/carbon");

    let handle = tokio::spawn(async move {
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
                gpu_backend: "Loopback-StageRuntime-120Hz".to_string(),
                viewport_px: [2560, 1600],
                scale_factor: 2.0,
            },
        });
        if let Ok(raw) = encode_server_message(&ready) {
            let _ = ws.send(Message::Text(raw.into())).await;
        }

        let mut cat_summary: Option<CatalogSnapshotSummary> = None;
        let mut author_mode = false;

        while let Some(Ok(Message::Text(txt))) = ws.next().await {
            let Ok(msg) = decode_client_message(txt.as_str()) else {
                continue;
            };
            match msg {
                ClientMessage::Ping { id, .. } => {
                    let pong = ServerMessage::Pong {
                        request_id: id,
                        timestamp_ms: 1763838000555,
                    };
                    if let Ok(raw) = encode_server_message(&pong) {
                        let _ = ws.send(Message::Text(raw.into())).await;
                    }
                }
                ClientMessage::Batch { id, commands, .. } => {
                    for cmd in &commands {
                        match cmd {
                            CarbonCommand::ChromeUpdate(p) => {
                                if let Some(am) = p.author_mode {
                                    author_mode = am;
                                }
                            }
                            CarbonCommand::SetCatalog(c) => {
                                cat_summary = Some(CatalogSnapshotSummary {
                                    open: c.open,
                                    display_mode: c.display_mode,
                                    view_level: c.view_level.unwrap_or_default(),
                                    active_tab: c.active_tab,
                                    active_subject_id: c.active_subject_id.clone(),
                                    active_paper_id: c.active_paper_id.clone(),
                                    active_item_id: c.active_item_id.clone(),
                                    subject_count: c.available_subjects.len(),
                                    paper_count: c.papers.len(),
                                    topic_count: c.topics.len(),
                                    question_count: c.papers.iter().map(|p| p.items.len()).sum(),
                                });
                            }
                            _ => {}
                        }
                    }
                    let ack = ServerMessage::Ack(CommandAck {
                        request_id: id,
                        ok: true,
                        action_ids: vec!["chrome.update".into(), "chrome.setCatalog".into()],
                        error: None,
                    });
                    if let Ok(raw) = encode_server_message(&ack) {
                        let _ = ws.send(Message::Text(raw.into())).await;
                    }

                    let ev = ServerMessage::Event(CarbonEventEnvelope {
                        seq: 2,
                        timestamp_ms: 1763838000600,
                        actor: None,
                        event: CarbonEvent::CatalogItemSelected {
                            kind: CatalogSelectionKind::Topic,
                            item_id: "GYR-MTH-T-ALG".to_string(),
                            subject_id: Some("GYR-MTH".to_string()),
                            paper_id: None,
                            topic_id: Some("GYR-MTH-T-ALG".to_string()),
                        },
                    });
                    if let Ok(raw) = encode_server_message(&ev) {
                        let _ = ws.send(Message::Text(raw.into())).await;
                    }
                }
                ClientMessage::GetState { id, .. } => {
                    let timer = TimerSnapshot {
                        mode: TimerMode::Countdown,
                        running: false,
                        configured_sec: None,
                        elapsed_sec: 0,
                        remaining_sec: 0,
                        overtime_sec: 0,
                        warning_active: false,
                        expired: false,
                        formatted: "00:00".to_string(),
                    };
                    let plain = SurfaceSpec::default();
                    let snap = BoardStateSnapshot {
                        protocol_version: PROTOCOL_VERSION.to_string(),
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
                            left: plain.clone(),
                            right: plain,
                            detour: None,
                        },
                        region_space: Vec::new(),
                        objects: Vec::new(),
                        optics: OpticsSnapshot {
                            camera_scale: 1.0,
                            camera_center: [0.5, 0.5],
                            loupe: LoupeSnapshot {
                                visible: false,
                                x: 0.5,
                                y: 0.5,
                                radius_px: 120.0,
                                magnification: 2.0,
                            },
                        },
                        chrome: ChromeSnapshot {
                            mode: ChromeVisibility::Full,
                            title: "Gyara+ 11+ Maths".to_string(),
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
                            author_mode,
                            part_tabs: Vec::new(),
                            active_part_id: None,
                            total_steps: 0,
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
                        catalog: cat_summary.clone(),
                        telemetry: None,
                    };
                    let state_msg = ServerMessage::State {
                        request_id: id,
                        snapshot: Box::new(snap),
                    };
                    if let Ok(raw) = encode_server_message(&state_msg) {
                        let _ = ws.send(Message::Text(raw.into())).await;
                    }
                }
                _ => {}
            }
        }
    });

    Ok((url, handle))
}
