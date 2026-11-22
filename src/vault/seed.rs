//! Baseline `Gyara+` 11+ Maths (`GYR-MTH`) course metadata, brand logo assets, topics, and question types.

use std::path::Path;

use memory_core_rs::{
    CourseAssetInput, CourseMetaInput, CourseQuestionTypeInput, CourseTopicInput, Memory,
};
use serde_json::json;

use crate::base64_util::read_png_dimensions;
use crate::error::{HydrogenError, Result};

/// Canonical asset ID for the primary `Gyara+` PNG logo (`gyara-blue.png`, strictly PNG-only per Carbon spec).
pub const GYR_LOGO_BLUE_ID: &str = "GYR-LOGO-BLUE";
/// Optional backup PNG logo asset ID (`gyara-white.png`).
pub const GYR_LOGO_WHITE_ID: &str = "GYR-LOGO-WHITE";

/// Summary of what was seeded or verified inside `gyara-maths.raphs`.
#[derive(Debug, Clone)]
pub struct SeedSummary {
    pub course_id: String,
    pub embedded_assets: Vec<String>,
    pub topic_count: usize,
    pub question_type_count: usize,
}

/// Idempotently seed `GYR-MTH` course metadata, PNG-only brand logo blobs (`gyara-blue.png`, `gyara-white.png`),
/// baseline 11+ Maths topics, and problem archetypes into `mem`.
pub fn seed_gyara_maths(mem: &mut Memory, actor: &str, assets_dir: &Path) -> Result<SeedSummary> {
    let mut embedded_assets = Vec::new();

    // 1. Embed Gyara+ PNG brand logos into content-addressed `blobs` + `course_media_assets`.
    // Note: Per `docs/carbon-visual-theme.md` §2.1, brand & subject logos are STRICTLY PNG ONLY (never SVG).
    let logo_specs = [
        (
            GYR_LOGO_BLUE_ID,
            "gyara-blue.png",
            "Gyara+ wordmark (royal blue, emerald & coral calligraphy for Warm-Paper theme)",
            true,
        ),
        (
            GYR_LOGO_WHITE_ID,
            "gyara-white.png",
            "Gyara+ wordmark (alternate PNG render)",
            false,
        ),
    ];

    for (asset_id, filename, alt_text, required) in logo_specs {
        let file_path = assets_dir.join(filename);
        if !file_path.is_file() {
            if required {
                return Err(HydrogenError::Validation(format!(
                    "required Gyara+ PNG brand asset missing at {}",
                    file_path.display()
                )));
            }
            continue;
        }
        let bytes = std::fs::read(&file_path)?;
        let (width_px, height_px) = match read_png_dimensions(&bytes) {
            Some((w, h)) => (Some(w), Some(h)),
            None => {
                return Err(HydrogenError::Validation(format!(
                    "brand logo {} is not a valid PNG file (strictly PNG only)",
                    file_path.display()
                )));
            }
        };

        let asset_input = CourseAssetInput {
            asset_id: asset_id.to_string(),
            media_kind: "image".to_string(),
            mime_type: "image/png".to_string(),
            original_filename: Some(filename.to_string()),
            width_px,
            height_px,
            duration_ms: None,
            alt_text: Some(alt_text.to_string()),
            caption_text: None,
            captions_json: None,
        };
        mem.put_course_asset(actor, None, &asset_input, &bytes)?;
        embedded_assets.push(asset_id.to_string());
    }

    // 2. Set `course_meta` header + branding configuration (Light, Soft `warm-paper` theme + `gyara-blue.png`).
    let meta = CourseMetaInput {
        course_id: "GYR-MTH".to_string(),
        product_code: "GYR".to_string(),
        subject_code: "MTH".to_string(),
        title: "11+ Maths".to_string(),
        subtitle: Some("Practice Papers, Chapters & Topics".to_string()),
        stage_label: Some("11+ (Year 5-6)".to_string()),
        default_answer_format: "mcq".to_string(),
        theta_preset: Some("uk_gcse".to_string()),
        course_version: "0.1.0".to_string(),
        config_json: json!({
            "branding": {
                "primaryLogoAssetId": GYR_LOGO_BLUE_ID,
                "logoLightAssetId": GYR_LOGO_BLUE_ID,
                "logoWhiteAssetId": GYR_LOGO_WHITE_ID,
                "theme": "warm-paper",
                "subjectColor": "#5B7DB1",
                "importantIcon": "triangle",
                "palette": {
                    "maths": "#5B7DB1",
                    "english": "#4D8B7A",
                    "verbalReasoning": "#7A6BA8",
                    "nonVerbalReasoning": "#A8647A"
                }
            },
            "pedagogy": {
                "algebraFirstAtExamLevels": true,
                "defaultMcqChoices": 5
            }
        }),
    };
    mem.set_course_meta(actor, None, &meta)?;

    // 3. Clean up any legacy hyphenated topic IDs (`GYR-MTH-T-*`) before seeding the 8 canonical
    //    `rules/gyaraplus/maths/common.md` underscore IDs (`GYR-MTH-T_NUM` .. `GYR-MTH-T_LOG`).
    for legacy_id in [
        "GYR-MTH-T-NUM",
        "GYR-MTH-T-FDP",
        "GYR-MTH-T-RAT",
        "GYR-MTH-T-ALG",
        "GYR-MTH-T-MEA",
        "GYR-MTH-T-GEO",
        "GYR-MTH-T-STA",
        "GYR-MTH-T-LOG",
    ] {
        let _ = mem.delete_topic(actor, None, legacy_id)?;
    }

    // 4. Seed the 8 canonical 11+ Maths curriculum strands (`course_topics`).
    let baseline_topics = [
        (
            "GYR-MTH-T_NUM",
            "NUM",
            "Number, Place Value & Operations",
            "Place value, BODMAS, factors/HCF/LCM, primes, negative numbers, powers/roots, and rounding.",
            10,
        ),
        (
            "GYR-MTH-T_FDP",
            "FDP",
            "Fractions, Decimals & Percentages",
            "Mixed fractions, fracCancel, exact equivalences, percentage changes, and reverse percentages.",
            20,
        ),
        (
            "GYR-MTH-T_RAT",
            "RAT",
            "Ratio, Proportion & Scaling",
            "Ratio sharing, unitary scaling, recipes/maps, direct/inverse proportion, and shifting ratios.",
            30,
        ),
        (
            "GYR-MTH-T_ALG",
            "ALG",
            "Algebra, Equations & Sequences",
            "Linear equations with initials, substitution, simultaneous elimination, clearing fractions, and nth term.",
            40,
        ),
        (
            "GYR-MTH-T_MEA",
            "MEA",
            "Measurement, Time, Speed & Money",
            "Metric units, time/timetables, Speed-Distance-Time, perimeter/area/volume, and money problems.",
            50,
        ),
        (
            "GYR-MTH-T_GEO",
            "GEO",
            "2D & 3D Geometry, Angles & Coordinate Grids",
            "Angle laws, polygons, coordinates/transformations, bearings, 3D solids, and cube nets.",
            60,
        ),
        (
            "GYR-MTH-T_STA",
            "STA",
            "Statistics, Charts, Venn Diagrams & Probability",
            "Mean/median/mode/range, pieChart, barChart, lineChart, two-way tables, Venn sets, and probability.",
            70,
        ),
        (
            "GYR-MTH-T_LOG",
            "LOG",
            "Word & Logic Puzzles",
            "Catch-up/rate chunks, working backwards, combinatorics, and calendar/clock logic puzzles.",
            80,
        ),
    ];

    for (topic_id, code, title, description, sort_order) in baseline_topics {
        let input = CourseTopicInput {
            topic_id: topic_id.to_string(),
            parent_topic_id: None,
            category: "strand".to_string(),
            code: code.to_string(),
            title: title.to_string(),
            description: Some(description.to_string()),
            sort_order,
            meta_json: json!({}),
        };
        mem.upsert_topic(actor, None, &input)?;
    }

    // 5. Seed initial 11+ Maths problem archetypes (`course_question_types`).
    let question_types = [
        (
            "GYR-MTH-QT-ALG-WORD",
            "ALG-WORD",
            "Algebraic Word & Age/Coin Problem",
            "Multi-step word problems solved cleanly by letting initials represent unknowns and forming linear equations.",
            Some("GYR-MTH-T_ALG"),
            10,
        ),
        (
            "GYR-MTH-QT-ARITH",
            "ARITH",
            "Number Properties & BODMAS Fluency",
            "Exact arithmetic, factors/LCM/HCF, prime factorisation, and digit/place-value puzzles.",
            Some("GYR-MTH-T_NUM"),
            20,
        ),
        (
            "GYR-MTH-QT-FDP",
            "FDP",
            "Fraction, Decimal & Percentage Equivalence",
            "Multi-stage fraction remainders, reverse percentages, and mixed-number operations.",
            Some("GYR-MTH-T_FDP"),
            30,
        ),
        (
            "GYR-MTH-QT-RATIO",
            "RATIO",
            "Ratio Sharing & Shifting Proportions",
            "Sharing quantities, scaling parts, and before/after ratio transfers.",
            Some("GYR-MTH-T_RAT"),
            40,
        ),
        (
            "GYR-MTH-QT-RATES",
            "RATES",
            "Speed-Distance-Time, Clocks & Work Rates",
            "Journey stages, average speed, clock-face angles/mirror clocks, and combined tap/worker rates.",
            Some("GYR-MTH-T_MEA"),
            50,
        ),
        (
            "GYR-MTH-QT-GEO2D",
            "GEO2D",
            "2D Angles, Compound Area & Coordinates",
            "Missing angles, shaded/compound shapes, and 4-quadrant coordinate transformations.",
            Some("GYR-MTH-T_GEO"),
            60,
        ),
        (
            "GYR-MTH-QT-GEO3D",
            "GEO3D",
            "3D Solids, Nets & Painted/Stacked Cubes",
            "Volume, surface area, cube nets, and 3D spatial counting.",
            Some("GYR-MTH-T_GEO"),
            70,
        ),
        (
            "GYR-MTH-QT-DATA",
            "DATA",
            "Averages, Venn Sets, Charts & Probability",
            "Reverse mean, pie/bar/table interpretation, 2- or 3-set Venn diagrams, and exact fraction probability.",
            Some("GYR-MTH-T_STA"),
            80,
        ),
        (
            "GYR-MTH-QT-SEQ",
            "SEQ",
            "Sequences, Function Machines & Custom Operators",
            "Linear/triangular/square sequences, inverse machines, and novel symbol definitions.",
            Some("GYR-MTH-T_ALG"),
            90,
        ),
        (
            "GYR-MTH-QT-LOGIC",
            "LOGIC",
            "Catch-Up, Combinatorics & Working-Backwards Puzzles",
            "Closing-the-gap problems, handshake/counting combinations, calendar shifts, and multi-clue deduction.",
            Some("GYR-MTH-T_LOG"),
            100,
        ),
    ];

    for (type_id, code, title, description, default_topic_id, sort_order) in question_types {
        let input = CourseQuestionTypeInput {
            type_id: type_id.to_string(),
            code: code.to_string(),
            title: title.to_string(),
            description: Some(description.to_string()),
            default_topic_id: default_topic_id.map(str::to_string),
            sort_order,
        };
        mem.upsert_question_type(actor, None, &input)?;
    }

    let topic_count = mem.list_topics()?.len();
    let question_type_count = mem.list_question_types()?.len();

    Ok(SeedSummary {
        course_id: "GYR-MTH".to_string(),
        embedded_assets,
        topic_count,
        question_type_count,
    })
}
