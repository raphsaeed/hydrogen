//! Unified `hydrogen` CLI entrypoint (zero external argument-parser crates).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use carbon_protocol::{
    BoardStateSnapshot, CarbonCommand, CarbonEvent, CarbonEventEnvelope, CatalogSelectionKind,
    CatalogSnapshotSummary, ChromeSnapshot, ChromeVisibility, ClientInteraction, ClientMessage,
    CommandAck, DualTimersSnapshot, ImportantIconStyle, LayoutMode, LayoutSnapshot, LoupeSnapshot,
    NormalizedRect, OpticsSnapshot, PROTOCOL_VERSION, PaneId, ProgressStatus,
    ReactiveStateSnapshot, ReferenceViewMode, ReminderInterval, ServerMessage, SetFeedbackPayload,
    SplitOrientation, SurfaceSpec, SurfacesSnapshot, ThemeId, TimerMode, TimerSnapshot,
    UserInkSummary, decode_client_message, encode_server_message,
};
use futures_util::{SinkExt, StreamExt};
use hydrogen::base64_util::{encode_base64, read_png_dimensions};
use hydrogen::carbon::catalog::project_full_stage_catalog;
use hydrogen::carbon::preview::{
    PreviewSelection, apply_interactive_event, build_package_preview_batch,
    build_question_preview_batch, force_instant_speed,
};
use hydrogen::carbon::{
    CarbonClient, WatchPersistenceOutcome, handle_watch_persistence_event, inspect_board_bounds,
    load_feedback_entries_for_item,
};
use hydrogen::compiler::{verify_blueprints, verify_question_package};
use hydrogen::config::HydrogenConfig;
use hydrogen::domain::{
    ContainerSegment, PaperPackage, QuestionBlueprint, QuestionPackage, SolutionBlueprint,
    UniversalId,
};
use hydrogen::error::{HydrogenError, Result};
use hydrogen::ingest::{
    CropBox, CropSpec, DEFAULT_RENDER_DPI, NormalizedBBox, PixelBBox, ProductLedger, RgbaBuffer,
    SliceMode, SlicePaperRequest, build_subject_status_report, discover_product_subjects,
    format_terminal_status, refresh_subject_ledger_and_optional_html, slice_paper,
    subject_slug_for_prefix, write_status_html,
};
use hydrogen::rules::RuleLoader;
use hydrogen::server::{resolve_serve_bind_addr, run_server};
use hydrogen::theta::{StagePresetConfig, ThetaService};
use hydrogen::vault::VaultManager;
use hydrogen::vault::seed::{GYR_LOGO_BLUE_ID, GYR_LOGO_WHITE_ID};
use memory_core_rs::{
    CourseAssetInput, CoursePaperInput, CoursePaperSectionInput, CourseStimulusInput,
};
use serde::Deserialize;
use serde_json::{Value, json};
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
        "status" => cmd_status(&args[1..]).await,
        "serve" => cmd_serve(&args[1..]).await,
        "init-vault" => cmd_init_vault(&args[1..]).await,
        "slice-paper" => cmd_slice_paper(&args[1..]).await,
        "theta" => cmd_theta(&args[1..]),
        "verify-question" => cmd_verify_question(&args[1..]),
        "upsert-question" => cmd_upsert_question(&args[1..]).await,
        "upsert-paper" => cmd_upsert_paper(&args[1..]).await,
        "put-asset" => cmd_put_asset(&args[1..]).await,
        "comments" => cmd_comments(&args[1..]).await,
        "catalog" => cmd_catalog(&args[1..]).await,
        "preview-carbon" => cmd_preview_carbon(&args[1..]).await,
        "capture-carbon" => cmd_capture_carbon(&args[1..]).await,
        "pack" => cmd_pack(&args[1..]).await,
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
    doctor [--carbon <ws-url>]              Verify 4-Tier Rules, Theta (<1ms), Vault (Schema 2.3.0), & Carbon WS
    status [--product <slug>] [--subject <slug>] [--set-expected <paper-id> <count>] [--note <paper-id> \"...\"] [--html] [--json]
                                            Inspect active sub-product inbox queue, ledger.json, paper/topic progress & status.html
    serve [--lan] [--port 7430] [--bind <ip:port>]
                                            Start Carbon Warm-Paper Multi-Product & Sub-Product Status Server on :7430
    init-vault [--product P] [--subject S] [--pack]
                                            Initialize/seed L7 vault + PNG brand logo blobs
    slice-paper <pdf-or-png> [OPTIONS]      Render pages & slice clean-room crops into archive/ + ledger.json
        Options: [--paper-id GYR-MTH-P001] [--product P] [--subject S] [--dpi 180] [--pages 1-2]
                 [--pages-only | --auto | --id UID --page N --bbox x,y,w,h | --bbox-px x,y,w,h | --crops-json f.json]
                 [--move-inbox]
    theta <tool> '<json>' | <expr>          Call Theta tool (`calc.eval`, `calc.solve`, ...) or evaluate <expr>
    verify-question <file.json> [OPTIONS]   Run deterministic Theta + Carbon + Warm-Paper + Clean-Room gates
        Options: [--product P] [--subject S] [--tier T] [--json]
    upsert-question <file.json> [OPTIONS]   Verify and atomically upsert question + L9..L1 solution into L7 vault
        Options: [--product P] [--subject S] [--tier T] [--pack] [--html]
    upsert-paper <file.json> [OPTIONS]      Verify and atomically upsert paper, sections, stimuli & questions
        Options: [--product P] [--subject S] [--tier T] [--pack] [--html]
    put-asset <file.png|svg|mp4|mp3> --asset-id <ID> [OPTIONS]
                                            Embed clean regenerated media asset into L7 course_media_assets + blobs
        Options: [--product P] [--subject S] [--alt \"...\"] [--caption \"...\"] [--duration-ms N] [--pack]
    comments [list | show <id> [--export-dir <dir>] | resolve <id> --note \"...\" | delete <id>] [OPTIONS]
                                            Inspect, export attachments & source crops, or resolve Snap Board comments
        Options: [--status open|resolved|all] [--target <id>] [--product P] [--subject S] [--json]
    catalog [--product P] [--subject S] [--carbon-json] [--full-logo]
                                            Inspect L7 CourseCatalogSnapshot or emit Carbon Full-Stage JSON
    preview-carbon [<question-id>] [OPTIONS] Push Split-Board question + (part, step, sublevel) live to Carbon
        Options: [--watch] [--catalog] [--part PART] [--step S01] [--level 9] [--substep 0]
                 [--reveal-answer] [--carbon <ws-url>] [--loopback] [--product P] [--subject S]
    capture-carbon <question-id> [OPTIONS]  Render onto headless Carbon (:7420), verify token/object AABB bounds & save PNG
        Options: [--part P] [--step S01] [--level 9] [--substep 0] [--reveal-answer]
                 [--pane right|left|full] [--include-chrome] [--check-bounds] --out <snap.png>
                 [--carbon <ws-url>] [--loopback] [--product P] [--subject S]
    pack [--product P] [--subject S] [--clean-out <out.raphs> --customer-key <key>]
                                            Pack live SQLCipher container into <stem>.raphs (or clean customer export)
    demo [--carbon <ws-url>] [--watch]      Run interactive end-to-end demo (Vault + Theta + Carbon WS)
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

/// Return all positional arguments (not starting with `-` and not values of `valued_flags`).
fn positional_args<'a>(args: &'a [String], valued_flags: &[&str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if valued_flags.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        out.push(arg.as_str());
    }
    out
}

/// Return the first positional argument (not starting with `--` and not the value of a known `--flag <val>`).
fn first_positional<'a>(args: &'a [String], valued_flags: &[&str]) -> Option<&'a str> {
    positional_args(args, valued_flags).into_iter().next()
}

async fn open_vault_for(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
) -> Result<VaultManager> {
    VaultManager::open_or_create(
        cfg.vault_sqlite_path(product, subject),
        cfg.vault_bundle_path(product, subject),
        &cfg.lesson_key,
        &cfg.device_id,
    )
}

async fn cmd_doctor(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let carbon_url =
        parse_flag_value(args, "--carbon").unwrap_or_else(|| cfg.carbon_ws_url.clone());

    let rule_loader = RuleLoader::new(&cfg.rules_dir);
    let rulebook = rule_loader.load(&cfg.default_product, &cfg.default_subject, None)?;

    println!("=== Hydrogen Doctor (v{}) ===", hydrogen::VERSION);
    println!("1. Workspace & 4-Tier Isolated Rules:");
    println!("   root       : {}", cfg.workspace_root.display());
    println!(
        "   rulebook   : {} ({} tiers loaded · course_id={} · stage={})",
        cfg.rules_dir.display(),
        rulebook.sections.len(),
        rulebook.frontmatter.course_id,
        rulebook.frontmatter.stage_label
    );

    // 2. Theta check
    let t0 = Instant::now();
    let theta = ThetaService::new(rulebook.frontmatter.theta.to_stage_preset());
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
    let assets_dir = cfg.product_assets_dir(product);

    let v0 = Instant::now();
    let vault = open_vault_for(&cfg, product, subject).await?;
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

    println!(
        "3. Course Vault (`memory_core_rs {}` / Schema `{}`):",
        memory_core_rs::VERSION,
        status.schema_version
    );
    println!(
        "   container  : {} ({vault_ms} ms open+seed)",
        sqlite_path.display()
    );
    println!(
        "   self_check : ok={} · crypto={}",
        check.ok, status.crypto_version
    );
    println!(
        "   course_id  : {} · {} topics · {} question types · {} papers · {} questions · catalog_snapshot={} us",
        seeded.course_id,
        snap.topics.len(),
        snap.question_types.len(),
        snap.papers.len(),
        snap.questions.len(),
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

    // 4. Carbon probe
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
    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let should_pack = has_flag(args, "--pack");

    let sqlite_path = cfg.vault_sqlite_path(&product, &subject);
    let bundle_path = cfg.vault_bundle_path(&product, &subject);
    let assets_dir = cfg.product_assets_dir(&product);

    let vault = open_vault_for(&cfg, &product, &subject).await?;

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

async fn cmd_slice_paper(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = [
        "--product",
        "--subject",
        "--paper-id",
        "--dpi",
        "--pages",
        "--id",
        "--page",
        "--bbox",
        "--bbox-px",
        "--crops-json",
        "--min-gap",
        "--start-seq",
    ];
    let src_arg = first_positional(args, &valued_flags).ok_or_else(|| {
        HydrogenError::Config("usage: hydrogen slice-paper <pdf-or-png> [options]".into())
    })?;

    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let paper_id = parse_flag_value(args, "--paper-id");
    let dpi = match parse_flag_value(args, "--dpi") {
        Some(s) => s
            .parse::<f32>()
            .map_err(|_| HydrogenError::Config(format!("invalid --dpi '{s}'")))?,
        None => DEFAULT_RENDER_DPI,
    };
    let page_range = match parse_flag_value(args, "--pages") {
        Some(spec) => Some(parse_page_range(&spec)?),
        None => None,
    };
    let move_inbox = has_flag(args, "--move-inbox");

    let mode = if has_flag(args, "--pages-only") {
        SliceMode::PagesOnly
    } else if let Some(manifest_path) = parse_flag_value(args, "--crops-json") {
        let raw = std::fs::read_to_string(&manifest_path)?;
        let crops: Vec<CropSpec> = serde_json::from_str(&raw)?;
        SliceMode::ExplicitCrops(crops)
    } else if let Some(uid) = parse_flag_value(args, "--id") {
        let page = parse_flag_value(args, "--page")
            .unwrap_or_else(|| "1".to_string())
            .parse::<u32>()
            .map_err(|_| HydrogenError::Config("invalid --page integer".into()))?;
        let bbox = if let Some(norm_str) = parse_flag_value(args, "--bbox") {
            CropBox::Normalized(parse_norm_bbox(&norm_str)?)
        } else if let Some(px_str) = parse_flag_value(args, "--bbox-px") {
            CropBox::Pixels(parse_pixel_bbox(&px_str)?)
        } else {
            return Err(HydrogenError::Config(
                "--id requires either `--bbox x,y,w,h` (normalized 0..1) or `--bbox-px x,y,w,h`"
                    .into(),
            ));
        };
        SliceMode::ExplicitCrops(vec![CropSpec {
            universal_id: uid,
            page,
            bbox,
        }])
    } else {
        let min_gap_px = parse_flag_value(args, "--min-gap")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(32);
        let start_question_no = parse_flag_value(args, "--start-seq")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        SliceMode::AutoWhitespaceBands {
            min_gap_px,
            min_band_height_px: 48,
            pad_px: 14,
            start_question_no,
        }
    };

    let t0 = Instant::now();
    let summary = slice_paper(
        &cfg,
        &SlicePaperRequest {
            source_path: PathBuf::from(src_arg),
            product: product.clone(),
            subject: subject.clone(),
            paper_id,
            dpi,
            page_range,
            mode,
            move_source_to_archive: move_inbox,
        },
    )?;
    let elapsed_ms = t0.elapsed().as_millis();

    println!(
        "Sliced paper {} in {elapsed_ms} ms ({} total PDF pages, {} rendered pages, {} crops):",
        summary.paper_id,
        summary.total_pdf_pages,
        summary.rendered_pages.len(),
        summary.sliced_items.len()
    );
    println!(
        "  archived_source : {}",
        summary.archived_source_path.display()
    );
    println!("  ledger          : {}", summary.ledger_path.display());
    for item in &summary.sliced_items {
        println!(
            "  crop            : {} -> page {} ({}x{} px) @ {}",
            item.universal_id, item.page, item.width_px, item.height_px, item.crop_path
        );
    }

    if let Some(html_path) =
        refresh_subject_ledger_and_optional_html(&cfg, &product, &subject, has_flag(args, "--html"))
            .await?
    {
        println!("  status_html     : {}", html_path.display());
    }
    Ok(())
}

fn parse_page_range(spec: &str) -> Result<(u32, u32)> {
    if let Some((a, b)) = spec.split_once('-') {
        let start = a
            .trim()
            .parse::<u32>()
            .map_err(|_| HydrogenError::Config(format!("invalid start page in '{spec}'")))?;
        let end = b
            .trim()
            .parse::<u32>()
            .map_err(|_| HydrogenError::Config(format!("invalid end page in '{spec}'")))?;
        if start == 0 || end < start {
            return Err(HydrogenError::Config(format!(
                "invalid page range '{spec}' (pages are 1-based and end >= start)"
            )));
        }
        Ok((start, end))
    } else {
        let p = spec
            .trim()
            .parse::<u32>()
            .map_err(|_| HydrogenError::Config(format!("invalid page number '{spec}'")))?;
        if p == 0 {
            return Err(HydrogenError::Config("page numbers are 1-based".into()));
        }
        Ok((p, p))
    }
}

fn parse_norm_bbox(s: &str) -> Result<NormalizedBBox> {
    let vals: Vec<f32> = s
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<f32>()
                .map_err(|_| HydrogenError::Config(format!("invalid float in --bbox '{s}'")))
        })
        .collect::<Result<Vec<f32>>>()?;
    if vals.len() != 4 {
        return Err(HydrogenError::Config(format!(
            "expected 4 comma-separated floats `x,y,w,h` in --bbox, got '{s}'"
        )));
    }
    Ok(NormalizedBBox {
        x: vals[0],
        y: vals[1],
        w: vals[2],
        h: vals[3],
    })
}

fn parse_pixel_bbox(s: &str) -> Result<PixelBBox> {
    let vals: Vec<u32> = s
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| HydrogenError::Config(format!("invalid integer in --bbox-px '{s}'")))
        })
        .collect::<Result<Vec<u32>>>()?;
    if vals.len() != 4 {
        return Err(HydrogenError::Config(format!(
            "expected 4 comma-separated integers `x,y,w,h` in --bbox-px, got '{s}'"
        )));
    }
    Ok(PixelBBox {
        x: vals[0],
        y: vals[1],
        w: vals[2],
        h: vals[3],
    })
}

fn cmd_theta(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(HydrogenError::Config(
            "usage: hydrogen theta <tool> '<json>' | hydrogen theta \"<expression>\"".into(),
        ));
    }

    let cfg = HydrogenConfig::load(workspace_root()?).ok();
    let stage = if let Some(ref c) = cfg {
        let product =
            parse_flag_value(args, "--product").unwrap_or_else(|| c.default_product.clone());
        let subject =
            parse_flag_value(args, "--subject").unwrap_or_else(|| c.default_subject.clone());
        RuleLoader::new(&c.rules_dir)
            .load(&product, &subject, None)
            .map(|r| r.frontmatter.theta.to_stage_preset())
            .unwrap_or_else(|_| StagePresetConfig::gyara_11plus_maths())
    } else {
        StagePresetConfig::gyara_11plus_maths()
    };

    let theta = ThetaService::new(stage);
    let first = args[0].as_str();

    if matches!(
        first,
        "calc.eval"
            | "calc.solve"
            | "calc.formula"
            | "calc.transform"
            | "calc.stats"
            | "calc.analyze_graph"
    ) {
        let json_str = args.get(1).ok_or_else(|| {
            HydrogenError::Config(format!("usage: hydrogen theta {first} '<json-args>'"))
        })?;
        let mut req_val: Value = serde_json::from_str(json_str)?;
        if let Some(obj) = req_val.as_object_mut() {
            let st = theta.stage();
            obj.entry("preset".to_string())
                .or_insert_with(|| Value::String(st.preset.clone()));
            if matches!(first, "calc.eval" | "calc.solve") {
                obj.entry("angle_mode".to_string())
                    .or_insert_with(|| Value::String(st.angle_mode.clone()));
                obj.entry("anchor_tokens".to_string())
                    .or_insert_with(|| Value::Bool(st.anchor_tokens));
            }
            if first == "calc.eval" {
                obj.entry("allow_mixed_fractions".to_string())
                    .or_insert_with(|| Value::Bool(st.allow_mixed_fractions));
            }
        }
        let t0 = Instant::now();
        let out = theta.dispatch_tool(first, &req_val)?;
        let elapsed_us = t0.elapsed().as_micros();
        eprintln!("// {first} completed in {elapsed_us} us");
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let expr = args.join(" ");
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BareBlueprintsFile {
    question: QuestionBlueprint,
    solution: SolutionBlueprint,
}

fn cmd_verify_question(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = ["--product", "--subject", "--tier"];
    let file_arg = first_positional(args, &valued_flags).ok_or_else(|| {
        HydrogenError::Config("usage: hydrogen verify-question <file.json> [options]".into())
    })?;

    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let tier = parse_flag_value(args, "--tier");
    let emit_json = has_flag(args, "--json");

    let rulebook = RuleLoader::new(&cfg.rules_dir).load(&product, &subject, tier.as_deref())?;
    let raw = std::fs::read_to_string(file_arg)?;
    let val: Value = serde_json::from_str(&raw)?;

    let t0 = Instant::now();
    let report = if val.get("title").is_some() {
        let pkg: QuestionPackage = serde_json::from_value(val)?;
        verify_question_package(&pkg, Some(&rulebook))
    } else {
        let bare: BareBlueprintsFile = serde_json::from_value(val)?;
        verify_blueprints(&bare.question, &bare.solution, Some(&rulebook))
    };
    let elapsed_us = t0.elapsed().as_micros();

    if emit_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if report.ok {
        println!(
            "VERIFIED OK : {} ({} Theta checks · {} Carbon commands · {elapsed_us} us)",
            report.question_id, report.theta_checks_run, report.carbon_commands_checked
        );
    }

    let _ = report.into_result()?;
    Ok(())
}

async fn ensure_parent_paper_exists(
    vault: &VaultManager,
    paper_id: &str,
    default_answer_format: &str,
    calculator_mode: &str,
) -> Result<()> {
    if vault.get_paper(paper_id).await?.is_some() {
        return Ok(());
    }
    let parsed = UniversalId::parse(paper_id)?;
    let (paper_no, rep_opt) = match parsed.container {
        Some(ContainerSegment::Paper { number, replica }) => (number, replica),
        _ => {
            return Err(HydrogenError::Validation(format!(
                "invalid paperId '{paper_id}'"
            )));
        }
    };
    let (code, title, is_replica) = match rep_opt {
        Some(r) => (
            format!("P{paper_no:03}_R{r:02}"),
            format!("Practice Paper {paper_no:02}-ai ({r:02})"),
            true,
        ),
        None => (
            format!("P{paper_no:03}"),
            format!("Practice Paper {paper_no:02}"),
            false,
        ),
    };
    let input = CoursePaperInput {
        paper_id: paper_id.to_string(),
        code,
        title,
        badge: None,
        duration_sec: Some(3000),
        calculator_mode: calculator_mode.to_string(),
        default_answer_format: default_answer_format.to_string(),
        tier: None,
        total_marks: None,
        is_replica,
        parent_paper_id: None,
        sort_order: i64::from(paper_no) * 10,
        meta_json: json!({}),
    };
    vault.upsert_paper(&input).await
}

async fn cmd_upsert_question(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = ["--product", "--subject", "--tier"];
    let file_arg = first_positional(args, &valued_flags).ok_or_else(|| {
        HydrogenError::Config("usage: hydrogen upsert-question <file.json> [options]".into())
    })?;

    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let tier = parse_flag_value(args, "--tier");
    let should_pack = has_flag(args, "--pack");

    let rulebook = RuleLoader::new(&cfg.rules_dir).load(&product, &subject, tier.as_deref())?;
    let raw = std::fs::read_to_string(file_arg)?;
    let pkg: QuestionPackage = serde_json::from_str(&raw)?;

    // Enforce deterministic quality gate BEFORE touching the encrypted vault
    let gate_report = verify_question_package(&pkg, Some(&rulebook)).into_result()?;
    let l7_input = pkg.to_course_question_input()?;

    let vault = open_vault_for(&cfg, &product, &subject).await?;
    if product == "gyaraplus" && subject == "maths" {
        let _ = vault
            .seed_gyara_maths(&cfg.product_assets_dir(&product))
            .await?;
    }
    if let Some(pid) = l7_input.paper_id.as_deref() {
        ensure_parent_paper_exists(
            &vault,
            pid,
            &rulebook.frontmatter.defaults.answer_format,
            &l7_input.calculator_mode,
        )
        .await?;
    }

    vault.upsert_question(&l7_input).await?;
    println!(
        "Upserted verified question {} ({} · {} marks · sublevels {}..={} · {} Theta checks) -> {}",
        l7_input.question_id,
        l7_input.title,
        l7_input.marks,
        l7_input.min_sublevel,
        l7_input.max_sublevel,
        gate_report.theta_checks_run,
        vault.sqlite_path().display()
    );

    if should_pack {
        let manifest = vault.pack_to_default_bundle().await?;
        println!(
            "Packed bundle: {} (schema v{})",
            vault.bundle_path().display(),
            manifest.schema_version
        );
    }

    // Drop single VaultManager handle BEFORE syncing sub-product ledger / status.html (Rule A6)
    drop(vault);
    if let Some(html_path) =
        refresh_subject_ledger_and_optional_html(&cfg, &product, &subject, has_flag(args, "--html"))
            .await?
    {
        println!("Refreshed status dashboard: {}", html_path.display());
    }
    Ok(())
}

async fn cmd_upsert_paper(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = ["--product", "--subject", "--tier"];
    let file_arg = first_positional(args, &valued_flags).ok_or_else(|| {
        HydrogenError::Config("usage: hydrogen upsert-paper <file.json> [options]".into())
    })?;

    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let tier = parse_flag_value(args, "--tier");
    let should_pack = has_flag(args, "--pack");

    let rulebook = RuleLoader::new(&cfg.rules_dir).load(&product, &subject, tier.as_deref())?;
    let raw = std::fs::read_to_string(file_arg)?;
    let paper_pkg: PaperPackage = serde_json::from_str(&raw)?;

    // Validate paper ID & clean-room title, plus every included question BEFORE writing
    let parsed_pid = UniversalId::parse(&paper_pkg.paper.paper_id)?;
    if !matches!(parsed_pid.container, Some(ContainerSegment::Paper { .. }))
        || parsed_pid.item.is_some()
    {
        return Err(HydrogenError::Validation(format!(
            "invalid paperId '{}'",
            paper_pkg.paper.paper_id
        )));
    }
    if paper_pkg.paper.title.to_ascii_lowercase().contains(".pdf") {
        return Err(HydrogenError::Validation(
            "clean-room violation: paper.title must not contain raw PDF filenames".into(),
        ));
    }

    let mut verified_inputs = Vec::with_capacity(paper_pkg.questions.len());
    for q_pkg in &paper_pkg.questions {
        let _ = verify_question_package(q_pkg, Some(&rulebook)).into_result()?;
        verified_inputs.push(q_pkg.to_course_question_input()?);
    }

    let vault = open_vault_for(&cfg, &product, &subject).await?;
    if product == "gyaraplus" && subject == "maths" {
        let _ = vault
            .seed_gyara_maths(&cfg.product_assets_dir(&product))
            .await?;
    }

    vault
        .upsert_paper(&CoursePaperInput::from(&paper_pkg.paper))
        .await?;
    for sec in &paper_pkg.sections {
        vault
            .upsert_paper_section(&CoursePaperSectionInput::from(sec))
            .await?;
    }
    for stim in &paper_pkg.stimuli {
        vault
            .upsert_stimulus(&CourseStimulusInput::from(stim))
            .await?;
    }
    for q_in in &verified_inputs {
        vault.upsert_question(q_in).await?;
    }

    println!(
        "Upserted paper {} ('{}'): {} sections, {} stimuli, {} verified questions -> {}",
        paper_pkg.paper.paper_id,
        paper_pkg.paper.title,
        paper_pkg.sections.len(),
        paper_pkg.stimuli.len(),
        verified_inputs.len(),
        vault.sqlite_path().display()
    );

    if should_pack {
        let manifest = vault.pack_to_default_bundle().await?;
        println!(
            "Packed bundle: {} (schema v{})",
            vault.bundle_path().display(),
            manifest.schema_version
        );
    }

    // Drop single VaultManager handle BEFORE syncing sub-product ledger / status.html (Rule A6)
    drop(vault);
    if let Some(html_path) =
        refresh_subject_ledger_and_optional_html(&cfg, &product, &subject, has_flag(args, "--html"))
            .await?
    {
        println!("Refreshed status dashboard: {}", html_path.display());
    }
    Ok(())
}

async fn cmd_status(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let mut product = cfg.default_product.clone();
    let mut explicit_subject: Option<String> = None;
    let mut emit_html = false;
    let mut emit_json = false;
    let mut expected_updates: Vec<(String, u32)> = Vec::new();
    let mut note_updates: Vec<(String, String)> = Vec::new();

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--product" => {
                idx += 1;
                product = args
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| HydrogenError::Config("--product requires <slug>".into()))?;
            }
            "--subject" => {
                idx += 1;
                explicit_subject =
                    Some(args.get(idx).cloned().ok_or_else(|| {
                        HydrogenError::Config("--subject requires <slug>".into())
                    })?);
            }
            "--set-expected" => {
                let paper_id = args.get(idx + 1).cloned().ok_or_else(|| {
                    HydrogenError::Config(
                        "usage: --set-expected <paper-id> <expected-question-count>".into(),
                    )
                })?;
                let count_str = args.get(idx + 2).ok_or_else(|| {
                    HydrogenError::Config(
                        "usage: --set-expected <paper-id> <expected-question-count>".into(),
                    )
                })?;
                let count = count_str.parse::<u32>().map_err(|_| {
                    HydrogenError::Config(format!(
                        "invalid integer '{count_str}' for --set-expected"
                    ))
                })?;
                expected_updates.push((paper_id, count));
                idx += 2;
            }
            "--note" => {
                let paper_id = args.get(idx + 1).cloned().ok_or_else(|| {
                    HydrogenError::Config("usage: --note <paper-id> \"<text>\"".into())
                })?;
                let text = args.get(idx + 2).cloned().ok_or_else(|| {
                    HydrogenError::Config("usage: --note <paper-id> \"<text>\"".into())
                })?;
                note_updates.push((paper_id, text));
                idx += 2;
            }
            "--html" => emit_html = true,
            "--json" => emit_json = true,
            unknown => {
                return Err(HydrogenError::Config(format!(
                    "unrecognized status option '{unknown}' (run `hydrogen --help`)"
                )));
            }
        }
        idx += 1;
    }

    for (pid, exp) in &expected_updates {
        let parsed = UniversalId::parse(pid)?;
        let paper_subj = subject_slug_for_prefix(&parsed.subject_code);
        if let Some(ref req_subj) = explicit_subject
            && req_subj != paper_subj
        {
            return Err(HydrogenError::Validation(format!(
                "paper '{pid}' belongs to subject '{paper_subj}', which conflicts with --subject '{req_subj}'"
            )));
        }
        let ledger_path = cfg.ledger_path(&product, paper_subj);
        let mut ledger = ProductLedger::load_or_new(&ledger_path, &product, paper_subj)?;
        ledger.set_expected_questions(pid, *exp)?;
        ledger.save_atomic(&ledger_path)?;
    }

    for (pid, note) in &note_updates {
        let parsed = UniversalId::parse(pid)?;
        let paper_subj = subject_slug_for_prefix(&parsed.subject_code);
        if let Some(ref req_subj) = explicit_subject
            && req_subj != paper_subj
        {
            return Err(HydrogenError::Validation(format!(
                "paper '{pid}' belongs to subject '{paper_subj}', which conflicts with --subject '{req_subj}'"
            )));
        }
        let ledger_path = cfg.ledger_path(&product, paper_subj);
        let mut ledger = ProductLedger::load_or_new(&ledger_path, &product, paper_subj)?;
        ledger.add_paper_note(pid, note.clone())?;
        ledger.save_atomic(&ledger_path)?;
    }

    let subjects_to_report = match explicit_subject {
        Some(ref s) => vec![s.clone()],
        None => discover_product_subjects(&cfg, &product)?,
    };

    if subjects_to_report.is_empty() {
        if emit_json {
            println!("[]");
        } else {
            println!(
                "=== Hydrogen Status: `{product}` ===\nNo active sub-products under {}/ yet.",
                cfg.product_dir(&product).display()
            );
        }
        return Ok(());
    }

    let mut reports = Vec::with_capacity(subjects_to_report.len());
    for subj in &subjects_to_report {
        reports.push(build_subject_status_report(&cfg, &product, subj, emit_html).await?);
    }

    if emit_json {
        if reports.len() == 1 {
            println!("{}", serde_json::to_string_pretty(&reports[0])?);
        } else {
            println!("{}", serde_json::to_string_pretty(&reports)?);
        }
    } else {
        for (i, rep) in reports.iter().enumerate() {
            if i > 0 {
                println!();
            }
            print!("{}", format_terminal_status(rep));
        }
    }

    if emit_html {
        for rep in &reports {
            let html_path = write_status_html(&cfg, rep)?;
            println!(
                "\nGenerated Warm-Paper HTML dashboard: {}",
                html_path.display()
            );
        }
    }
    Ok(())
}

async fn cmd_serve(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let mut lan = false;
    let mut port_override: Option<u16> = None;
    let mut bind_override: Option<String> = None;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--lan" => lan = true,
            "--port" => {
                idx += 1;
                let p_str = args
                    .get(idx)
                    .ok_or_else(|| HydrogenError::Config("--port requires <port>".into()))?;
                let p = p_str.parse::<u16>().map_err(|_| {
                    HydrogenError::Config(format!("invalid --port integer '{p_str}'"))
                })?;
                if p == 0 {
                    return Err(HydrogenError::Config("--port must be >= 1".into()));
                }
                port_override = Some(p);
            }
            "--bind" => {
                idx += 1;
                let b_str = args
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| HydrogenError::Config("--bind requires <ip:port>".into()))?;
                bind_override = Some(b_str);
            }
            unknown => {
                return Err(HydrogenError::Config(format!(
                    "unrecognized serve option '{unknown}' (usage: `hydrogen serve [--lan] [--port 7430] [--bind <ip:port>]`)"
                )));
            }
        }
        idx += 1;
    }

    let bind_addr =
        resolve_serve_bind_addr(&cfg.bind_addr, lan, port_override, bind_override.as_deref())?;
    run_server(cfg, bind_addr).await
}

async fn cmd_put_asset(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = [
        "--asset-id",
        "--product",
        "--subject",
        "--alt",
        "--caption",
        "--duration-ms",
    ];
    let file_arg = first_positional(args, &valued_flags).ok_or_else(|| {
        HydrogenError::Config(
            "usage: hydrogen put-asset <file.png|svg|mp4|mp3> --asset-id <ID> [--alt \"...\"] [--caption \"...\"]"
                .into(),
        )
    })?;
    let asset_id = parse_flag_value(args, "--asset-id").ok_or_else(|| {
        HydrogenError::Config("--asset-id <ID> is required for `put-asset`".into())
    })?;
    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let alt_text = parse_flag_value(args, "--alt");
    let caption_text = parse_flag_value(args, "--caption");
    let duration_ms = match parse_flag_value(args, "--duration-ms") {
        Some(s) => Some(
            s.parse::<i64>()
                .map_err(|_| HydrogenError::Config(format!("invalid --duration-ms '{s}'")))?,
        ),
        None => None,
    };
    let should_pack = has_flag(args, "--pack");

    validate_clean_asset_id(&asset_id)?;
    if let Some(ref alt) = alt_text {
        ensure_no_clean_room_leak(alt, "--alt")?;
    }
    if let Some(ref cap) = caption_text {
        ensure_no_clean_room_leak(cap, "--caption")?;
    }

    let file_path = PathBuf::from(file_arg);
    if !file_path.is_file() {
        return Err(HydrogenError::Validation(format!(
            "asset file does not exist: {}",
            file_path.display()
        )));
    }
    let bytes = std::fs::read(&file_path)?;
    if bytes.is_empty() {
        return Err(HydrogenError::Validation(format!(
            "asset file is empty: {}",
            file_path.display()
        )));
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (media_kind, mime_type) = classify_media_extension(&ext)?;

    let (width_px, height_px) = if mime_type == "image/png" {
        match read_png_dimensions(&bytes) {
            Some((w, h)) => (Some(w), Some(h)),
            None => {
                return Err(HydrogenError::Validation(format!(
                    "file '{}' has .png extension but lacks a valid PNG IHDR header",
                    file_path.display()
                )));
            }
        }
    } else {
        (None, None)
    };

    let clean_filename = format!("{asset_id}.{ext}");
    let input = CourseAssetInput {
        asset_id: asset_id.clone(),
        media_kind: media_kind.to_string(),
        mime_type: mime_type.to_string(),
        original_filename: Some(clean_filename),
        width_px,
        height_px,
        duration_ms,
        alt_text,
        caption_text,
        captions_json: None,
    };

    let vault = open_vault_for(&cfg, &product, &subject).await?;
    let blob_id = vault.put_asset(&input, &bytes).await?;
    let stored = vault.get_asset_info(&asset_id).await?.ok_or_else(|| {
        HydrogenError::Validation(format!("stored asset '{asset_id}' missing after put_asset"))
    })?;

    println!(
        "Embedded clean asset {} ({} · {} · {} B · sha256={} · blob={}) -> {}",
        stored.asset_id,
        stored.media_kind,
        stored.mime_type,
        stored.size_bytes,
        &stored.sha256[..stored.sha256.len().min(16)],
        blob_id,
        vault.sqlite_path().display()
    );

    if should_pack {
        let manifest = vault.pack_to_default_bundle().await?;
        println!(
            "Packed bundle: {} (schema v{})",
            vault.bundle_path().display(),
            manifest.schema_version
        );
    }
    Ok(())
}

fn validate_clean_asset_id(asset_id: &str) -> Result<()> {
    if asset_id.is_empty()
        || asset_id.len() > 128
        || !asset_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(HydrogenError::Validation(format!(
            "invalid --asset-id '{asset_id}' (expected 1..=128 ASCII `[A-Za-z0-9_-]+`)"
        )));
    }
    ensure_no_clean_room_leak(asset_id, "--asset-id")
}

fn ensure_no_clean_room_leak(text: &str, field: &str) -> Result<()> {
    let lower = text.to_ascii_lowercase();
    for forbidden in [
        ".pdf",
        "ledger.json",
        "status.html",
        "private-xref",
        "inbox/",
        "archive/",
        "gl assessment",
        "iseb",
        "csse",
        "aqa",
        "edexcel",
    ] {
        if lower.contains(forbidden) {
            return Err(HydrogenError::Validation(format!(
                "clean-room violation in {field}: forbidden source token '{forbidden}'"
            )));
        }
    }
    Ok(())
}

fn classify_media_extension(ext: &str) -> Result<(&'static str, &'static str)> {
    match ext {
        "png" => Ok(("image", "image/png")),
        "jpg" | "jpeg" => Ok(("image", "image/jpeg")),
        "webp" => Ok(("image", "image/webp")),
        "gif" => Ok(("image", "image/gif")),
        "svg" => Ok(("svg", "image/svg+xml")),
        "mp4" => Ok(("video", "video/mp4")),
        "webm" => Ok(("video", "video/webm")),
        "mov" => Ok(("video", "video/quicktime")),
        "mp3" => Ok(("audio", "audio/mpeg")),
        "wav" => Ok(("audio", "audio/wav")),
        "ogg" => Ok(("audio", "audio/ogg")),
        "m4a" => Ok(("audio", "audio/mp4")),
        other => Err(HydrogenError::Config(format!(
            "unsupported asset extension '.{other}' (expected png, jpg, webp, svg, mp4, webm, mp3, wav, ogg, m4a)"
        ))),
    }
}

async fn cmd_comments(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = [
        "--product",
        "--subject",
        "--status",
        "--target",
        "--export-dir",
        "--note",
    ];
    let pos = positional_args(args, &valued_flags);
    let action = pos.first().copied().unwrap_or("list");

    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let emit_json = has_flag(args, "--json");

    let vault = open_vault_for(&cfg, &product, &subject).await?;

    match action {
        "list" => {
            let status_raw = parse_flag_value(args, "--status").unwrap_or_else(|| "open".into());
            let status_filter = match status_raw.as_str() {
                "all" => None,
                "open" | "pending" => Some("open"),
                "resolved" | "done" => Some("resolved"),
                other => {
                    return Err(HydrogenError::Config(format!(
                        "invalid --status '{other}' (expected open, resolved, or all)"
                    )));
                }
            };
            let target_filter = parse_flag_value(args, "--target");
            let comments = vault
                .list_comments(status_filter, target_filter.as_deref())
                .await?;
            if emit_json {
                println!("{}", serde_json::to_string_pretty(&comments)?);
            } else if comments.is_empty() {
                println!(
                    "No {} comments in {}.",
                    status_filter.unwrap_or("stored"),
                    vault.sqlite_path().display()
                );
            } else {
                println!(
                    "Comments in {} ({} match(es)):",
                    vault.sqlite_path().display(),
                    comments.len()
                );
                for c in &comments {
                    println!(
                        "  - {} [{} · {}] -> {}:{}{}{}{} ({} attachment(s)): {}",
                        c.comment_id,
                        c.status,
                        c.comment_kind,
                        c.target_kind,
                        c.target_id,
                        c.part_id
                            .as_deref()
                            .map(|p| format!(":{p}"))
                            .unwrap_or_default(),
                        c.step_id
                            .as_deref()
                            .map(|s| format!(":{s}"))
                            .unwrap_or_default(),
                        c.sublevel.map(|l| format!(":L{l:02}")).unwrap_or_default(),
                        c.attachments.len(),
                        c.comment_text
                    );
                }
            }
            Ok(())
        }
        "show" => {
            let comment_id = pos.get(1).copied().ok_or_else(|| {
                HydrogenError::Config(
                    "usage: hydrogen comments show <comment-id> [--export-dir <dir>]".into(),
                )
            })?;
            let comment = vault.get_comment(comment_id).await?.ok_or_else(|| {
                HydrogenError::Validation(format!(
                    "comment '{comment_id}' not found in {}",
                    vault.sqlite_path().display()
                ))
            })?;

            let ledger = ProductLedger::load_or_new(
                &cfg.ledger_path(&product, &subject),
                &product,
                &subject,
            )?;
            let ledger_item = ledger.items.get(&comment.target_id);

            if emit_json {
                println!("{}", serde_json::to_string_pretty(&comment)?);
            } else {
                println!("Comment ID   : {}", comment.comment_id);
                println!(
                    "Status       : {} (kind={})",
                    comment.status, comment.comment_kind
                );
                println!(
                    "Target       : {}:{} (part={:?}, step={:?}, sublevel={:?})",
                    comment.target_kind,
                    comment.target_id,
                    comment.part_id,
                    comment.step_id,
                    comment.sublevel
                );
                println!("Text         : {}", comment.comment_text);
                if let Some(ref note) = comment.resolution_note {
                    println!("Resolution   : {note}");
                }
                if let Some(item) = ledger_item
                    && let Some(ref crop_rel) = item.crop_path
                {
                    println!(
                        "Source Crop  : {} (page {:?}, dims={:?})",
                        crop_rel, item.page, item.crop_dimensions
                    );
                }
                println!("Attachments  : {}", comment.attachments.len());
                for att in &comment.attachments {
                    println!(
                        "  - {} ({} · {} · {} B · {:?}x{:?} px · pane={:?})",
                        att.attachment_id,
                        att.source,
                        att.mime_type,
                        att.size_bytes,
                        att.width_px,
                        att.height_px,
                        att.pane
                    );
                }
            }

            if let Some(export_dir_str) = parse_flag_value(args, "--export-dir") {
                let export_dir = PathBuf::from(export_dir_str);
                std::fs::create_dir_all(&export_dir)?;

                let meta_path = export_dir.join("comment.json");
                std::fs::write(&meta_path, serde_json::to_string_pretty(&comment)?)?;

                for att in &comment.attachments {
                    let bytes = vault.read_comment_attachment(&att.attachment_id).await?;
                    let ext = match att.mime_type.as_str() {
                        "image/png" => "png",
                        "image/jpeg" => "jpg",
                        "image/svg+xml" => "svg",
                        "application/pdf" => "pdf",
                        _ => "bin",
                    };
                    let safe_att = att.attachment_id.replace(['/', '\\', ':'], "_");
                    let att_out = export_dir.join(format!("{safe_att}.{ext}"));
                    std::fs::write(&att_out, bytes)?;
                    println!("Exported attachment -> {}", att_out.display());
                }

                if let Some(item) = ledger_item
                    && let Some(ref crop_rel) = item.crop_path
                {
                    let src_crop = cfg.subject_dir(&product, &subject).join(crop_rel);
                    if src_crop.is_file() {
                        let dst_crop =
                            export_dir.join(format!("source-crop-{}.png", comment.target_id));
                        std::fs::copy(&src_crop, &dst_crop)?;
                        println!("Exported source crop -> {}", dst_crop.display());
                    }
                }

                if comment.target_kind == "question"
                    && let Some(cq) = vault.get_question(&comment.target_id).await?
                {
                    let q_out = export_dir.join(format!("{}-vault.json", comment.target_id));
                    std::fs::write(&q_out, serde_json::to_string_pretty(&cq)?)?;
                    println!("Exported vault question -> {}", q_out.display());
                }
            }
            Ok(())
        }
        "resolve" => {
            let comment_id = pos.get(1).copied().ok_or_else(|| {
                HydrogenError::Config(
                    "usage: hydrogen comments resolve <comment-id> [--note \"...\"]".into(),
                )
            })?;
            let note = parse_flag_value(args, "--note");
            let updated = vault.resolve_comment(comment_id, note.as_deref()).await?;
            if !updated {
                return Err(HydrogenError::Validation(format!(
                    "comment '{comment_id}' not found in {}",
                    vault.sqlite_path().display()
                )));
            }
            println!(
                "Resolved comment {comment_id}{}",
                note.as_deref()
                    .map(|n| format!(" (note: '{n}')"))
                    .unwrap_or_default()
            );
            drop(vault);
            let _ =
                refresh_subject_ledger_and_optional_html(&cfg, &product, &subject, false).await?;
            Ok(())
        }
        "delete" => {
            let comment_id = pos.get(1).copied().ok_or_else(|| {
                HydrogenError::Config("usage: hydrogen comments delete <comment-id>".into())
            })?;
            let removed = vault.delete_comment(comment_id).await?;
            if !removed {
                return Err(HydrogenError::Validation(format!(
                    "comment '{comment_id}' not found in {}",
                    vault.sqlite_path().display()
                )));
            }
            println!("Deleted comment {comment_id}");
            drop(vault);
            let _ =
                refresh_subject_ledger_and_optional_html(&cfg, &product, &subject, false).await?;
            Ok(())
        }
        other => Err(HydrogenError::Config(format!(
            "unknown `comments` action '{other}' (expected list, show, resolve, or delete)"
        ))),
    }
}

async fn cmd_capture_carbon(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = [
        "--part",
        "--step",
        "--level",
        "--substep",
        "--pane",
        "--out",
        "--carbon",
        "--product",
        "--subject",
    ];
    let target_arg = first_positional(args, &valued_flags).ok_or_else(|| {
        HydrogenError::Config(
            "usage: hydrogen capture-carbon <question-id> [--part P] [--step S01] [--level 9] [--pane right|left] [--check-bounds] --out <snap.png>"
                .into(),
        )
    })?;

    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let part_id = parse_flag_value(args, "--part");
    let step_id = parse_flag_value(args, "--step");
    let level = match parse_flag_value(args, "--level") {
        Some(s) => s
            .parse::<u8>()
            .map_err(|_| HydrogenError::Config(format!("invalid --level '{s}'")))?,
        None => 9,
    };
    let substep = match parse_flag_value(args, "--substep") {
        Some(s) => Some(
            s.parse::<u32>()
                .map_err(|_| HydrogenError::Config(format!("invalid --substep '{s}'")))?,
        ),
        None => None,
    };
    let reveal_answer = has_flag(args, "--reveal-answer");
    let check_bounds = has_flag(args, "--check-bounds");
    let include_chrome = has_flag(args, "--include-chrome");
    let out_path = parse_flag_value(args, "--out").map(PathBuf::from);
    let use_loopback = has_flag(args, "--loopback");
    let explicit_carbon = parse_flag_value(args, "--carbon");

    if out_path.is_none() && !check_bounds {
        return Err(HydrogenError::Config(
            "capture-carbon requires `--out <snap.png>` and/or `--check-bounds`".into(),
        ));
    }

    let pane_opt = match parse_flag_value(args, "--pane").as_deref() {
        None | Some("full") | Some("all") => None,
        Some("right") => Some(PaneId::Right),
        Some("left") => Some(PaneId::Left),
        Some("board") => Some(PaneId::Board),
        Some(other) => {
            return Err(HydrogenError::Config(format!(
                "invalid --pane '{other}' (expected right, left, or full)"
            )));
        }
    };

    let sel = PreviewSelection {
        part_id,
        step_id,
        level,
        substep,
        reveal_answer,
    };

    let mut built =
        load_preview_batch_for_target(&cfg, &product, &subject, target_arg, &sel).await?;
    force_instant_speed(&mut built.commands);

    let (target_url, mock_handle) = if use_loopback {
        let (url, h) = spawn_demo_loopback_carbon().await?;
        (url, Some(h))
    } else {
        let url = explicit_carbon.unwrap_or_else(|| cfg.carbon_ws_url.clone());
        (url, None)
    };

    let client = CarbonClient::connect(&target_url, &cfg.device_id, Duration::from_secs(5)).await?;
    let _ = client.send_batch(built.commands).await?;

    if let Some(ref out) = out_path {
        let captured = client
            .capture_board(pane_opt, false, include_chrome)
            .await?;
        if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(out, &captured.png_bytes)?;
        println!(
            "Captured {} (part='{}', step='{}', L{}) -> {} ({}x{} px, {} B)",
            built.question_id,
            built.part_id,
            built.step_id,
            built.level,
            out.display(),
            captured.width_px,
            captured.height_px,
            captured.png_bytes.len()
        );
    }

    if check_bounds {
        let state = client.get_state(true).await?;
        let bounds_report = inspect_board_bounds(&state, pane_opt);
        if bounds_report.ok {
            println!(
                "BOUNDS OK   : {} ({} objects · {} tokens verified with zero collisions/clipping)",
                built.question_id, bounds_report.objects_checked, bounds_report.tokens_checked
            );
        }
        if let Some(h) = mock_handle {
            h.abort();
        }
        let _ = bounds_report.into_result()?;
        return Ok(());
    }

    if let Some(h) = mock_handle {
        h.abort();
    }
    Ok(())
}

async fn load_preview_batch_for_target(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    target_arg: &str,
    sel: &PreviewSelection,
) -> Result<hydrogen::carbon::preview::BuiltPreviewBatch> {
    let as_path = Path::new(target_arg);
    if as_path.is_file() {
        let raw = std::fs::read_to_string(as_path)?;
        let pkg: QuestionPackage = serde_json::from_str(&raw)?;
        return build_package_preview_batch(&pkg, sel);
    }

    let sqlite_path = cfg.vault_sqlite_path(product, subject);
    let bundle_path = cfg.vault_bundle_path(product, subject);
    if sqlite_path.is_file() || bundle_path.is_file() {
        let vault = open_vault_for(cfg, product, subject).await?;
        if let Some(cq) = vault.get_question(target_arg).await? {
            return build_question_preview_batch(&cq, sel);
        }
    }

    if let Some(archive_json) =
        find_archive_question_json(&cfg.subject_archive_dir(product, subject), target_arg)?
    {
        let raw = std::fs::read_to_string(archive_json)?;
        let pkg: QuestionPackage = serde_json::from_str(&raw)?;
        return build_package_preview_batch(&pkg, sel);
    }

    Err(HydrogenError::Validation(format!(
        "question '{target_arg}' not found in vault {} or archive/",
        sqlite_path.display()
    )))
}

fn find_archive_question_json(archive_subj_dir: &Path, qid: &str) -> Result<Option<PathBuf>> {
    if !archive_subj_dir.is_dir() {
        return Ok(None);
    }
    let expected_name = format!("{qid}.json");
    let direct = archive_subj_dir.join(&expected_name);
    if direct.is_file() {
        return Ok(Some(direct));
    }
    for res in std::fs::read_dir(archive_subj_dir)? {
        let entry = res?;
        let p = entry.path();
        if p.is_dir() {
            let candidate = p.join(&expected_name);
            if candidate.is_file() {
                return Ok(Some(candidate));
            }
        }
    }
    Ok(None)
}

async fn cmd_catalog(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let carbon_json = has_flag(args, "--carbon-json");

    let vault = open_vault_for(&cfg, &product, &subject).await?;
    let snap = vault.catalog_snapshot().await?;

    if carbon_json {
        let logo_uri = vault.read_asset_data_uri(GYR_LOGO_BLUE_ID).await.ok();
        let mut catalog = project_full_stage_catalog(&snap, logo_uri);
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

async fn cmd_preview_carbon(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let valued_flags = [
        "--part",
        "--step",
        "--level",
        "--substep",
        "--carbon",
        "--product",
        "--subject",
    ];
    let explicit_qid = first_positional(args, &valued_flags).map(str::to_string);
    let watch = has_flag(args, "--watch");
    let open_catalog_first = has_flag(args, "--catalog") || explicit_qid.is_none();

    if explicit_qid.is_none() && !has_flag(args, "--catalog") {
        return Err(HydrogenError::Config(
            "usage: hydrogen preview-carbon <question-id> [--watch] [--step S01 --level 9] (or `--catalog --watch`)".into(),
        ));
    }

    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());
    let part_id = parse_flag_value(args, "--part");
    let step_id = parse_flag_value(args, "--step");
    let level = match parse_flag_value(args, "--level") {
        Some(s) => s
            .parse::<u8>()
            .map_err(|_| HydrogenError::Config(format!("invalid --level '{s}'")))?,
        None => 9,
    };
    let substep = match parse_flag_value(args, "--substep") {
        Some(s) => Some(
            s.parse::<u32>()
                .map_err(|_| HydrogenError::Config(format!("invalid --substep '{s}'")))?,
        ),
        None => None,
    };
    let reveal_answer = has_flag(args, "--reveal-answer");
    let use_loopback = has_flag(args, "--loopback");
    let explicit_carbon = parse_flag_value(args, "--carbon");

    let vault = open_vault_for(&cfg, &product, &subject).await?;
    let snap = vault.catalog_snapshot().await?;
    let logo_uri = vault.read_asset_data_uri(GYR_LOGO_BLUE_ID).await.ok();

    let mut current_qid = match explicit_qid {
        Some(q) => q,
        None => snap
            .questions
            .first()
            .map(|q| q.question_id.clone())
            .ok_or_else(|| {
                HydrogenError::Validation(format!(
                    "no compiled questions found in {}",
                    vault.sqlite_path().display()
                ))
            })?,
    };

    let mut sel = PreviewSelection {
        part_id,
        step_id,
        level,
        substep,
        reveal_answer,
    };

    let (target_url, mock_handle) = if use_loopback {
        let (url, h) = spawn_demo_loopback_carbon().await?;
        (url, Some(h))
    } else {
        let url = explicit_carbon.unwrap_or_else(|| cfg.carbon_ws_url.clone());
        (url, None)
    };

    let client = CarbonClient::connect(&target_url, &cfg.device_id, Duration::from_secs(3)).await?;

    // Always synchronize Carbon's Full-Stage Course Gallery so the top-left Gallery icon works live
    let _ = client.push_author_catalog(&snap, logo_uri).await?;

    if !open_catalog_first {
        let cq = vault.get_question(&current_qid).await?.ok_or_else(|| {
            HydrogenError::Validation(format!(
                "question '{current_qid}' not found in {}",
                vault.sqlite_path().display()
            ))
        })?;
        let mut built = build_question_preview_batch(&cq, &sel)?;
        let stored_feedback =
            load_feedback_entries_for_item(&vault, &built.question_id, true).await?;
        built
            .commands
            .push(CarbonCommand::SetFeedback(SetFeedbackPayload::List {
                entries: stored_feedback,
            }));
        let ack = client.send_batch(built.commands.clone()).await?;
        let state = client.get_state(false).await?;

        println!(
            "Previewed {} (part='{}', step='{}' [{}/{}], level={}, archetype='{}') on {}:",
            built.question_id,
            built.part_id,
            built.step_id,
            built.step_index + 1,
            built.total_steps,
            built.level,
            built.archetype,
            client.ws_url()
        );
        println!(
            "  batch_ack  : ok={} ({} commands dispatched)",
            ack.ok,
            built.commands.len()
        );
        println!(
            "  board_mode : layout={:?}, theme={:?}, authorMode={}",
            state.layout.mode, state.theme, state.chrome.author_mode
        );
        println!("  narration  : {}", built.narration);
        if !watch && !use_loopback {
            println!(
                "  interactive: pass `--watch` to navigate parts, steps, breakdowns, L9..L1 chips, and Gallery live!"
            );
        }
    } else {
        println!(
            "Opened Full-Stage Course Gallery ({} papers, {} questions) on {}",
            snap.papers.len(),
            snap.questions.len(),
            client.ws_url()
        );
    }

    if watch {
        run_interactive_watch_loop(
            &cfg,
            &product,
            &subject,
            &client,
            &vault,
            &mut current_qid,
            &mut sel,
        )
        .await?;
    }

    if let Some(h) = mock_handle {
        h.abort();
    }
    Ok(())
}

async fn run_interactive_watch_loop(
    cfg: &HydrogenConfig,
    product: &str,
    subject: &str,
    client: &CarbonClient,
    vault: &VaultManager,
    current_qid: &mut String,
    sel: &mut PreviewSelection,
) -> Result<()> {
    let snap = vault.catalog_snapshot().await?;
    let mut full_questions = Vec::with_capacity(snap.questions.len());
    for q_card in &snap.questions {
        if let Some(cq) = vault.get_question(&q_card.question_id).await? {
            full_questions.push(cq);
        }
    }

    let Some(mut rx) = client.take_event_receiver().await else {
        return Ok(());
    };

    println!(
        "\n[Watch] Live two-way session active ({} questions loaded). Navigate Gallery/Steps/L9..L1 or save Snap Board comments in Carbon (Ctrl+C to stop)...",
        full_questions.len()
    );

    while let Some(ev) = rx.recv().await {
        if let CarbonEvent::PromptAnswered {
            ref prompt_id,
            ref value,
        } = ev.event
        {
            println!("[Watch] Prompt answered: {prompt_id} = {value}");
        }

        match handle_watch_persistence_event(
            &ev.event,
            cfg,
            product,
            subject,
            vault,
            client,
            current_qid,
        )
        .await
        {
            Ok(Some(WatchPersistenceOutcome::CommentSaved {
                comment_id,
                target_id,
                attachment_count,
                open_count_for_item,
            })) => {
                println!(
                    "[Watch] Persisted Snap Board comment {comment_id} -> {target_id} ({attachment_count} attachment(s), {open_count_for_item} open)"
                );
            }
            Ok(Some(WatchPersistenceOutcome::CommentDeleted {
                comment_id,
                item_id,
                deleted,
                open_count_for_item,
            })) => {
                println!(
                    "[Watch] Deleted comment {comment_id} on {item_id} (removed={deleted}, {open_count_for_item} open)"
                );
            }
            Ok(Some(WatchPersistenceOutcome::ReplicaQueued {
                task_id,
                target_kind,
                target_id,
            })) => {
                println!(
                    "[Watch] Queued create-replica task {task_id} -> {target_kind}:{target_id} in ledger.json"
                );
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("[Watch] persistence error: {err}");
            }
        }

        if apply_interactive_event(&ev.event, current_qid, sel, &full_questions)
            && let Some(cq) = full_questions
                .iter()
                .find(|q| q.question_id == *current_qid)
        {
            match build_question_preview_batch(cq, sel) {
                Ok(mut built) => {
                    if let Ok(stored_feedback) =
                        load_feedback_entries_for_item(vault, &built.question_id, true).await
                    {
                        built
                            .commands
                            .push(CarbonCommand::SetFeedback(SetFeedbackPayload::List {
                                entries: stored_feedback,
                            }));
                    }
                    let _ = client.send_batch(built.commands).await?;
                    println!(
                        "[Watch] -> {} (part='{}', step='{}' [{}/{}], L{} '{}', substep={:?}, answer={})",
                        built.question_id,
                        built.part_id,
                        built.step_id,
                        built.step_index + 1,
                        built.total_steps,
                        built.level,
                        built.archetype,
                        sel.substep,
                        sel.reveal_answer
                    );
                }
                Err(err) => {
                    eprintln!("[Watch] warning: {err}");
                }
            }
        }
    }

    Ok(())
}

async fn cmd_pack(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let product =
        parse_flag_value(args, "--product").unwrap_or_else(|| cfg.default_product.clone());
    let subject =
        parse_flag_value(args, "--subject").unwrap_or_else(|| cfg.default_subject.clone());

    let vault = open_vault_for(&cfg, &product, &subject).await?;
    let manifest = vault.pack_to_default_bundle().await?;
    println!(
        "Packed L7 vault bundle: {} (format v{}, schema v{}, dek_id={})",
        vault.bundle_path().display(),
        manifest.format_version,
        manifest.schema_version,
        manifest.dek_id
    );

    if let Some(clean_out) = parse_flag_value(args, "--clean-out") {
        let customer_key =
            parse_flag_value(args, "--customer-key").unwrap_or_else(|| cfg.lesson_key.clone());
        let out_path = PathBuf::from(clean_out);
        vault.export_clean_bundle(&out_path, &customer_key).await?;
        println!(
            "Exported clean customer bundle (comments/revisions stripped): {}",
            out_path.display()
        );
    }

    Ok(())
}

async fn cmd_demo(args: &[String]) -> Result<()> {
    let cfg = HydrogenConfig::load(workspace_root()?)?;
    let external_carbon = parse_flag_value(args, "--carbon");

    println!("==================================================================");
    println!("  HYDROGEN END-TO-END DEMO (Vault + Theta + Carbon Protocol v0.1) ");
    println!("==================================================================\n");

    let sqlite_path = cfg.vault_sqlite_path("gyaraplus", "maths");
    let bundle_path = cfg.vault_bundle_path("gyaraplus", "maths");
    let assets_dir = cfg.product_assets_dir("gyaraplus");

    let vault =
        VaultManager::open_or_create(&sqlite_path, &bundle_path, &cfg.lesson_key, &cfg.device_id)?;
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

    let theta = ThetaService::new(StagePresetConfig::gyara_11plus_maths());
    let t0 = Instant::now();
    let solved = theta.solve_equations(&["a + b = 52", "a - b = 14"], &["a", "b"], true)?;
    let sol_map = solved
        .solutions
        .as_ref()
        .and_then(|v| v.first())
        .ok_or_else(|| HydrogenError::Theta("expected unique simultaneous solution".into()))?;
    let val_a = sol_map.get("a").map(|t| t.exact.as_str()).unwrap_or("?");
    let val_b = sol_map.get("b").map(|t| t.exact.as_str()).unwrap_or("?");
    let check_a = theta.eval_expr("(52 + 14) / 2")?;
    let valid_choice = theta.verify_mcq_options(&["19", "26", "33", "38", "66"], val_a)?;
    let theta_us = t0.elapsed().as_micros();

    println!("[Step 2/3] Deterministic `Theta` (`theta_calc`) Verification ({theta_us} us):");
    println!(
        "  - 11+ System       : `a + b = 52`, `a - b = 14` -> solved a={val_a}, b={val_b} (method={})",
        solved.method.as_deref().unwrap_or("exact")
    );
    println!(
        "  - Exact Value `a`  : `{}` | KaTeX: `{}`",
        check_a.exact, check_a.katex
    );
    println!(
        "  - 5-Option MCQ Gate: Options [A:19, B:26, C:33, D:38, E:66] -> Sole exact match = Option {} ('33')\n",
        (b'A' + valid_choice as u8) as char
    );

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

    println!(
        "[Step 3/3] Live `Carbon Protocol v{}` Handshake & Full-Stage Gallery Push:",
        PROTOCOL_VERSION
    );
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

    if has_flag(args, "--watch") {
        let mut current_qid = snap
            .questions
            .first()
            .map(|q| q.question_id.clone())
            .unwrap_or_else(|| "GYR-MTH-P001-Q001".to_string());
        let mut sel = PreviewSelection::default();
        run_interactive_watch_loop(
            &cfg,
            "gyaraplus",
            "maths",
            &client,
            &vault,
            &mut current_qid,
            &mut sel,
        )
        .await?;
    } else if let Some(mut rx) = client.take_event_receiver().await
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

    println!("\n=== End-to-End Demo Succeeded ===");
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
        let mut active_item_id: Option<String> = None;

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
                    let mut action_ids = Vec::with_capacity(commands.len());
                    for cmd in &commands {
                        action_ids.push(cmd.op_name().to_string());
                        match cmd {
                            CarbonCommand::ChromeUpdate(p) => {
                                if let Some(am) = p.author_mode {
                                    author_mode = am;
                                }
                                if let Some(ref iid) = p.active_item_id {
                                    active_item_id = Some(iid.clone());
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
                        action_ids,
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
                            item_id: "GYR-MTH-T_ALG".to_string(),
                            subject_id: Some("GYR-MTH".to_string()),
                            paper_id: None,
                            topic_id: Some("GYR-MTH-T_ALG".to_string()),
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
                            active_item_id: active_item_id.clone(),
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
                ClientMessage::CaptureBoard { id, .. } => {
                    let png = RgbaBuffer {
                        width: 320,
                        height: 200,
                        data: vec![253u8; 320 * 200 * 4],
                    }
                    .encode_png()
                    .unwrap_or_default();
                    let cap_msg = ServerMessage::Capture {
                        request_id: id,
                        mime_type: "image/png".to_string(),
                        width_px: 320,
                        height_px: 200,
                        bounds: NormalizedRect::new(0.0, 0.0, 1.0, 1.0),
                        data_base64: encode_base64(&png),
                    };
                    if let Ok(raw) = encode_server_message(&cap_msg) {
                        let _ = ws.send(Message::Text(raw.into())).await;
                    }
                }
                ClientMessage::Interact {
                    id, interaction, ..
                } => {
                    let ack = ServerMessage::Ack(CommandAck {
                        request_id: id,
                        ok: true,
                        action_ids: vec!["interact-1".to_string()],
                        error: None,
                    });
                    if let Ok(raw) = encode_server_message(&ack) {
                        let _ = ws.send(Message::Text(raw.into())).await;
                    }
                    let maybe_ev = match interaction {
                        ClientInteraction::SaveComment { entry } => {
                            Some(CarbonEvent::CommentSaved { entry })
                        }
                        ClientInteraction::DeleteComment {
                            comment_id,
                            item_id,
                            part_id,
                        } => Some(CarbonEvent::CommentDeleted {
                            comment_id,
                            item_id,
                            part_id,
                        }),
                        ClientInteraction::RequestCatalogAction {
                            action,
                            target_kind,
                            target_id,
                        } => Some(CarbonEvent::CatalogActionRequested {
                            action,
                            target_kind,
                            target_id,
                        }),
                        _ => None,
                    };
                    if let Some(event) = maybe_ev {
                        let env = ServerMessage::Event(CarbonEventEnvelope {
                            seq: 3,
                            timestamp_ms: 1763838000700,
                            actor: None,
                            event,
                        });
                        if let Ok(raw) = encode_server_message(&env) {
                            let _ = ws.send(Message::Text(raw.into())).await;
                        }
                    }
                }
                _ => {}
            }
        }
    });

    Ok((url, handle))
}
