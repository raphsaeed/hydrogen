# Hydrogen — Phased Build Plan & Execution Checklist (`build-plan.md`)

> **How We Use This File:** Living implementation checklist for **Hydrogen**. As we complete and verify each sub-step, we tick its `[x]` checkbox here.  
> **Immediate North-Star Goal (Phases 1–5):** Stand up all core Rust infrastructure (`memory_core_rs 1.7.0` + `theta_calc` + `Carbon` WebSocket + `axum` minimal HTML/TS UI + CLI), ingest **one real 11+ Maths question paper**, generate verified `Level 9 → 1` solutions **both via Antigravity and via the Rust Gemini 4 engine**, store them in `gyara-maths.raphs`, see everything running live with **Carbon** and **Theta**, and iron out any issues before scaling up.

---

## Progress Overview

| Phase | Milestone Title | Status |
| :---: | :--- | :---: |
| **Phase 1** | Workspace Infrastructure & Core Sibling Wiring (`memory_core_rs` + `theta_calc` + `Carbon`) | `[ ] Pending` |
| **Phase 2** | Domain Blueprints, Quality Gates, PDF Slicer & CLI *(Enables Antigravity Authoring)* | `[ ] Pending` |
| **Phase 3** | Native Rust Gemini 4 Agentic Compiler *(Enables Rust API Single & Batch Authoring)* | `[ ] Pending` |
| **Phase 4** | Minimal Hydrogen Web Console (`axum` + HTML/TS) & Live Carbon/Theta Bridge | `[ ] Pending` |
| **Phase 5** | **Milestone 1 Proving Ground:** 1st Real Question Paper End-to-End & Live Bug-Fix Pass | `[ ] Pending` |
| **Phase 6** | Comment Auto-Processor, On-Demand Replicas (Question & Paper) & Topic Card Builder | `[ ] Pending` |
| **Phase 7** | Clean Customer Release Export, Books/Chapters & Multi-Subject Expansion | `[ ] Pending` |

---

## Phase 1: Workspace Infrastructure & Core Sibling Wiring
*Goal: Create the product directory tree, initialize the Rust project, link `memory_core_rs 1.7.0` and `theta_calc` in-process, connect to Carbon over WebSocket, and verify `gyara-maths.raphs` creation/read/write.*

- [ ] **1.1 Product & Subject Directory Scaffolding:**
  - Create `products/gyara/{maths,english,verbal-reasoning,non-verbal-reasoning}/{inbox,archive}` and `products/gcse/{maths,physics}/{inbox,archive}`.
  - Add `.gitignore` rules protecting raw source papers (`inbox/`, `archive/`, `private-xref.json`), `.env` secrets, and SQLite `-wal`/`-shm` files.
- [ ] **1.2 Rust Project & Configuration Setup (`Cargo.toml` & `src/config.rs`):**
  - Initialize the `hydrogen` Rust crate (`tokio`, `axum`, `serde`, `serde_json`, `clap`, `tokio-tungstenite`, `reqwest`, `tracing`).
  - Wire dependencies to **`theta_calc`** and **`memory_core_rs` (`1.7.0`, Schema `2.3.0`)**.
  - Implement configuration loader (`hydrogen.toml` + environment variables for `GEMINI_API_KEY`, default Lesson Key, Carbon WebSocket URL `ws://127.0.0.1:7420/carbon`, and `axum` port `7430`).
- [ ] **1.3 `L7` Vault Wrapper (`src/vault/`):**
  - Implement thread-safe `VaultManager` (`tokio::sync::Mutex<Memory>`) handling live `<name>.sqlite` + `<name>.keys.json` containers and `.raphs` bundle `pack` / `unpack`.
  - Initialize `gyara-maths.raphs` with `CourseMetaInput` (`course_id: "GYR-MTH"`, `default_answer_format: "mcq"`, `theta_preset: "uk_gcse"`) and seed baseline 11+ Maths `course_topics` (*Number*, *Fractions/Decimals/Percentages*, *Ratio & Proportion*, *Algebra*, *Measurement*, *2D & 3D Geometry*, *Statistics & Probability*) and initial `course_question_types`.
- [ ] **1.4 In-Process `Theta` Integration (`src/theta/`):**
  - Implement `ThetaService` wrapping `theta_calc` (`calc_eval`, `calc_solve`, `calc_formula`, `calc_transform`, `calc_stats`, `calc_analyze_graph`) with stage presets (`allow_mixed_fractions: true`, `angle_mode: "deg"`, `anchor_tokens: true` for 11+ Maths).
  - Add smoke tests verifying exact rational arithmetic, mixed fractions, and `\htmlId` KaTeX generation.
- [ ] **1.5 `Carbon` WebSocket Client & Ping/State Handshake (`src/carbon/`):**
  - Implement async `CarbonClient` connecting to `ws://127.0.0.1:7420/carbon`, receiving `board.ready`, sending `ping` / `getState` / `batch`, and streaming live `ServerMessage::Event`s.

> **Phase 1 Exit Check:** `cargo test` passes; `hydrogen doctor` confirms `theta_calc` (`< 1 ms`), opens/creates `gyara-maths.raphs` at Schema `2.3.0`, and handshakes with a running `carbon-shell`.

---

## Phase 2: Domain Blueprints, Quality Gates, PDF Slicer & CLI *(Antigravity Path)*
*Goal: Lock the strongly-typed `question_json` and `solution_json` schemas, build the deterministic verification gates, build the private PDF/image slicer, and expose `hydrogen` CLI commands so Antigravity can process test questions/papers right away.*

- [ ] **2.1 Universal ID & Blueprint Types (`src/domain/`):**
  - Implement `UniversalId` parser/builder (`GYR-MTH-P001-Q014`, replicas `-R01`, chapters `B01-CH03`, cards `-C01`, sub-addressing `:a:S01:L09`).
  - Implement `QuestionBlueprint` (`question_json`) and `SolutionBlueprint` (`solution_json`: multi-part state, `Steps × Sub-Levels 9..1`, Carbon commands, Theta checks, MCQ trap diagnostics, and SIGMA narration scripts) matching [`solution-levels.md`](./solution-levels.md) §6.
- [ ] **2.2 Deterministic Quality Gate (`src/compiler/gate.rs`):**
  - **Theta Math Gate:** Verify every `thetaVerification` check, multi-part `exportedExact` chain, `Check ✔` equivalence, and 5-option (`A–E`) MCQ uniqueness (all 5 distinct, exactly 1 equal to `answerKey.exactValue`).
  - **Carbon Command & KaTeX Gate:** Verify no unescaped ASCII math leaks outside `$...$`, valid canonical Carbon §5.1–§5.2 enums, valid `targetId` / `\htmlId` ordering, and optional `carbon-shell --headless` dry-run `ack.ok == true`.
  - **Pedagogical & Clean-Room Lint:** Enforce contiguous `9 → min_sublevel` numbering, anti-prose-wall word caps on board callouts, non-identical adjacent levels, and zero external paper names/raw scans.
- [ ] **2.3 Clean-Room PDF/Image Slicer & `private-xref.json` (`src/ingest/`):**
  - Render PDF pages to high-DPI working images in `archive/` (never inside `.raphs`), slice individual question/diagram crops, and record `{ universal_id, source_file, page, bbox }` in `private-xref.json`.
- [ ] **2.4 Complete `hydrogen` CLI (`src/main.rs`):**
  - Implement CLI commands:
    - `hydrogen init-vault`
    - `hydrogen slice-paper <pdf-or-image>`
    - `hydrogen theta <tool> <json>`
    - `hydrogen verify-question <file.json>`
    - `hydrogen upsert-question <file.json>` / `hydrogen upsert-paper <file.json>`
    - `hydrogen catalog` / `hydrogen preview-carbon <question-id> [--step S01 --level 9]`
    - `hydrogen pack`

> **Phase 2 Exit Check:** Using `hydrogen` CLI from Antigravity, we can slice a sample question, verify its `Level 9 → 1` JSON through Theta + Carbon gates, store it in `gyara-maths.raphs`, and preview it on Carbon.

---

## Phase 3: Native Rust Gemini 4 Agentic Compiler *(Rust API Path)*
*Goal: Build the multi-turn Gemini 4 API + Theta tool-calling + Carbon visual self-check loop directly in Rust so the Rust engine produces Antigravity-grade solutions autonomously for single questions and full papers.*

- [ ] **3.1 Gemini 4 Multimodal & Tool-Calling Client (`src/compiler/gemini.rs`):**
  - Implement async Rust client for Gemini 4 API supporting PDF/image vision inputs, high-thinking configuration, structured JSON outputs, function/tool calling, and image generation (for recreating real photographs cleanly).
- [ ] **3.2 Multi-Step Agentic Question Compiler (`src/compiler/agent_loop.rs`):**
  - **Step A (Extract & Classify):** Read cropped source image $\to$ extract clean KaTeX stem, multi-part structure, topic/type tags, difficulty (`1–5`), MCQ options (or synthesize 4 realistic trap distractors if source was written), and figure specification (regenerating graphs/geometry/tables in Carbon or recreating photos via image gen + `put_course_asset`).
  - **Step B (Theta-Grounded Solve):** Expose `theta_calc` tools directly to Gemini 4 so every intermediate value, fraction, root, landmark, and distractor is computed deterministically.
  - **Step C (`Level 9 → 1` Carbon Synthesis):** Author contiguous sub-levels (`9` down to `1`) with Carbon commands and SIGMA narration scripts.
  - **Step D (Self-Check & Auto-Repair Loop):** Run Phase 2's Quality Gate (+ optional headless Carbon `captureBoard` visual inspection to catch overlapping labels); automatically feed any error back to Gemini 4 to repair before committing to `gyara-maths.raphs`.
- [ ] **3.3 Paper-Level Orchestrator & Progress Events (`src/compiler/jobs.rs`):**
  - Coordinate multi-question paper ingestion with configurable concurrency, checkpoint resume (skipping already-verified questions if interrupted), and real-time job progress broadcasting.

> **Phase 3 Exit Check:** Running `hydrogen compile-paper` in pure Rust compiles questions end-to-end through Gemini 4 + `theta_calc` + Carbon gate into `gyara-maths.raphs` with zero manual intervention.

---

## Phase 4: Minimal Hydrogen Web Console (`axum` + HTML/TS) & Live Carbon Bridge
*Goal: Deliver the clean, fast 4-view browser interface (`http://127.0.0.1:7430`) connecting Hydrogen, `gyara-maths.raphs`, Theta, and a live Carbon window.*

- [ ] **4.1 `axum` Local HTTP & WebSocket Server (`src/server/`):**
  - REST endpoints for vault selection, `CourseCatalogSnapshot`, question/level inspection, private source-crop serving (`private-xref.json`), job triggers (ingest, regenerate, comment fix, replica), and live WebSocket progress/event push.
- [ ] **4.2 Bidirectional Live Carbon Author Bridge (`src/carbon/bridge.rs`):**
  - Push `chrome.update` (`authorMode: true`) + `chrome.setCatalog` (Left-Band Book Index built from `CourseCatalogSnapshot`) to Carbon.
  - Handle live Carbon events:
    - `catalog.itemSelected` $\to$ load clicked Question or Topic Card onto Carbon,
    - `comment.saved` $\to$ persist comment + `Snap Board` PNG blob straight into `gyara-maths.raphs` (`upsert_course_comment`) and notify the Web UI live,
    - `comment.deleted` $\to$ remove comment in `gyara-maths.raphs`.
- [ ] **4.3 Minimal 4-View HTML/TypeScript Frontend (`web/`):**
  - **View 1 — Vault Browser & Coverage Dashboard:** Filter/group by Paper, Book/Chapter, Topic (*Algebra*, *Geometry*, ...), Question Type, Difficulty (`1–5`), `MCQ`/`Written`, and QA/Comment badges.
  - **View 2 — Question, `9 → 1` & Source Crop Inspector:** Side-by-side private PDF crop vs. compiled question; interactive Step & Sub-Level (`9..1`) selector with 1-click **"Send / Step Live on Carbon"**, **"Regenerate with Prompt"**, and **"Create Replica"** buttons.
  - **View 3 — Comments & Screenshots Inbox:** View open/resolved Carbon comments + `Snap Board` screenshots, edit notes, trigger fixes, and inspect `course_question_revisions` diffs.
  - **View 4 — Ingest, Live Job Queue & Export:** Upload/select inbox paper, watch live per-question progress bars, and trigger `export_clean_course_bundle`.

> **Phase 4 Exit Check:** Opening `http://127.0.0.1:7430` loads `gyara-maths.raphs`, displays the catalog, previews steps/levels live onto Carbon, and receives a `Snap Board` comment saved from Carbon in real time.

---

## Phase 5: Milestone 1 Live Proving Ground — First Real 11+ Paper End-to-End
*Goal: Put the entire system (`Papers → Hydrogen → gyara-maths.raphs ↔ Carbon + Theta`) through a real-world test on Paper #1, comparing Antigravity-authored and Rust-API-authored solutions and fixing every integration kink.*

- [ ] **5.1 Ingest Real 11+ Maths Paper #1 (`GYR-MTH-P001`):**
  - Drop the first real 11+ Maths paper into `products/gyara/maths/inbox/`, slice all questions, and verify `private-xref.json` crops & 100% source-name stripping.
- [ ] **5.2 Dual-Path Solution Authoring Comparison:**
  - Author a subset of questions interactively via **Antigravity + `hydrogen` CLI** and the rest (or full paper) via the **Rust Web UI (`axum` + Gemini 4 Agentic Loop)** into `gyara-maths.raphs`.
- [ ] **5.3 Live End-to-End Walkthrough on Carbon + Theta:**
  - Launch `carbon-shell --split` alongside `hydrogen serve`:
    - Test Carbon's **Slide-Over Left Band** (`Papers`, `Topics`, `Stats` tabs, search & filters),
    - Step through **Levels `9 → 1`** across arithmetic, fractions/percentages, algebra, ratios, and 2D/3D geometry diagrams on Carbon,
    - Test saving **`Snap Board` comments + marker sketches** on Carbon and verifying they appear immediately in `gyara-maths.raphs` and Hydrogen's Comments Inbox.
- [ ] **5.4 Polish & Ecosystem Issue Resolution:**
  - Fix any layout, prompt, KaTeX, macro, or integration issues discovered across Hydrogen, Theta, Carbon, and `gyara-maths.raphs` so Paper #1 is 100% teaching-ready.

---

## Phase 6: Comment Auto-Processor, On-Demand Replicas & Organic Topic Builder
*Goal: Turn live teaching feedback into automated vault improvements, replicas, and reusable concept lessons.*

- [ ] **6.1 Automated Comment Processor & Scheduled Job (`hydrogen process-comments`):**
  - Process open `course_comments` (`fix` $\to$ cross-check private crop + `Snap Board` image and patch affected step/level; `engine` $\to$ append actionable ticket to `docs/ecosystem-backlog.md`; mark `resolved` with `resolution_note`).
  - Support running both on-click in View 3 and on a daily cron/schedule.
- [ ] **6.2 On-Demand Question & Whole-Paper Replica Generator:**
  - Given any `question_id` or `paper_id`, count $N$, and target difficulty (`same`, `higher`, `easier`), generate clean isomorphic/context-transfer replicas (default `mcq` `A–E` verified by Theta) and index them into `gyara-maths.raphs`.
- [ ] **6.3 Organic Topic / Concept / Prerequisite Card Authoring:**
  - Implement generator for `course_topic_cards` (`concept`, `worked_example`, `formula`, `prerequisite_refresher` with `9 → 1` levels) and wire `course_concept_links` so questions link cleanly to reusable concepts.

---

## Phase 7: Customer Release Pipeline, Books/Chapters & Multi-Subject Rollout
*Goal: Prepare commercial single-key `.raphs` exports and expand beyond 11+ Maths Papers.*

- [ ] **7.1 Clean Customer `.raphs` Export & Lesson Key Flow:**
  - Wire `Memory::export_clean_course_bundle` into CLI & Web UI, verifying zero authoring comments, zero revision snapshots, zero source leaks, and clean unlock with a single Lesson Key.
- [ ] **7.2 Books & Chapters Ingestion Workflow (`course_books` / `course_chapters` / `course_stimuli`):**
  - Support ingesting structured textbook chapters (Concept $\to$ Worked Examples $\to$ Exercise Questions) and shared reading/data stimuli.
- [ ] **7.3 Multi-Subject & Multi-Product Profiles:**
  - Add subject compiler profiles for `gyara-non-verbal-reasoning.raphs` (`netFold`, `cubeAssembly`, 2D spatial transforms), `gyara-verbal-reasoning.raphs`, `gyara-english.raphs` (`course_stimuli` comprehension passages + `chrome.setDictionary`), and `gcse-{maths,physics}.raphs` (`PhysicsRig`, `calc.formula` FIFA workflow, `reference_sheet` cards).
