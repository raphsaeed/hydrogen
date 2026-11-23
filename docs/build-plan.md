# Hydrogen — Phased Build Plan & Execution Checklist (`build-plan.md`)

> **How We Use This File:** Living implementation checklist for **Hydrogen**. As we complete and verify each sub-step, we tick its `[x]` checkbox here.  
> **Core Execution Model:** **Antigravity** (interactive & background SSH sessions with parallel subagents) drives Hydrogen's deterministic pure-Rust CLI (`hydrogen`), in-process **`theta_calc`**, prebuilt headless **Carbon** (`:7420`), encrypted **`memory.raphs`** (`memory_core_rs 1.7.0`), and the per-product authoring ledger (`ledger.json` + static `status.html`). **Carbon (`:7420`)** serves directly as the live interactive gallery, `Level 9 → 1` stepper, and `Snap Board` comment stage via `hydrogen preview-carbon --watch`.

---

## Progress Overview

| Phase | Milestone Title | Status |
| :---: | :--- | :---: |
| **Phase 1** | Workspace Infrastructure & Core Sibling Wiring (`memory_core_rs` + `theta_calc` + `Carbon`) | `[x] Complete` |
| **Phase 2** | Domain Blueprints, Quality Gates, PDF Slicer & Core CLI | `[x] Complete` |
| **Phase 3** | Product Authoring Ledger (`ledger.json` + `status.html`), Two-Way Carbon `--watch` Sync & Visual/Media CLI | `[x] Complete` |
| **Phase 4** | **Milestone 1 Proving Ground:** 1st Real 11+ Maths Paper (`GYR-MTH-P001`) End-to-End | `[ ] Next` |
| **Phase 5** | Live Teaching Feedback (`Snap Board` Fixes), Replicas & Organic Topic Cards | `[ ] Pending` |
| **Phase 6** | Clean Customer Release Pipeline, Books/Chapters & Multi-Subject Vault Rollout | `[ ] Pending` |
| **Phase 7** | Zero-Install Web Browser Delivery (`carbon-web` WASM + `theta.wasm` + Web Vault Gateway) | `[ ] Pending` |
| **Phase 8** | Modular Multi-Model API & Batch Automation (`src/ai/` Providers & Batch Runner) | `[ ] Pending` |

---

## Phase 1: Workspace Infrastructure & Core Sibling Wiring
*Goal: Create the product directory tree, initialize the Rust project, link `memory_core_rs 1.7.0` and `theta_calc` in-process, connect to Carbon over WebSocket, and verify `gyara-maths.raphs` creation/read/write.*

### Pinned Sibling Dependencies, Paths & Environment Notes
1. **`memory.raphs` (`memory_core_rs 1.7.0`, Schema `2.3.0` Layer `L7`):**
   - **Git Repository & Immutable Tag:** `ssh://git@github.com/raphs-app/raphs.git` at tag **`memory-core-1.7.0`** ([Release Page](https://github.com/raphs-app/raphs/releases/tag/memory-core-1.7.0)).
   - **Rust `Cargo.toml` Dependency Pin:**
     ```toml
     memory_core_rs = { git = "ssh://git@github.com/raphs-app/raphs.git", tag = "memory-core-1.7.0" }
     ```
   - **Host Build Prerequisites (Verified):** `export PATH="$HOME/.cargo/bin:$PATH"` (`rustc 1.98+`, Edition 2024), `/usr/bin/cc`, and `/usr/bin/perl` (compiles SQLite `3.51.3`, SQLCipher `4.14.0`, `sqlite-vec` `0.1.9`, and vendored OpenSSL directly from source).
2. **`Theta` (`theta_calc` / `calc-api`):**
   - **Local Workspace Path:** `/home/raphspark/projects/theta`
   - **Strict Boundary Rule:** **READ-ONLY dependency — NEVER modify `/home/raphspark/projects/theta`.**
   - **Rust `Cargo.toml` Dependency Pin:**
     ```toml
     theta_calc = { package = "calc-api", path = "/home/raphspark/projects/theta/crates/calc-api", default-features = false }
     ```
3. **First Staged 11+ Maths Practice Paper (Git-Ignored Local Input):**
   - **Local Path:** `/home/raphspark/projects/hydrogen/products/gyaraplus/maths/inbox/Practice-Paper-01.pdf`
   - **Clean-Room Mapping:** Ingested into `gyara-maths.raphs` under neutral course paper ID `GYR-MTH-P001`; raw PDF, crops, and authoring tracking remain strictly local inside `products/gyaraplus/maths/` (`inbox/` $\to$ `archive/` + `ledger.json` + `status.html`, excluded from Git and `.raphs`).
4. **`Carbon` (`carbon-protocol`, Prebuilt DGX Spark Headless Server & MacBook Native Window):**
   - **Strict Boundary Rule:** **READ-ONLY sibling — NEVER modify `/home/raphspark/projects/carbon`.**
   - **Compile-Time Wire Safety (`carbon-protocol` Only):**
     ```toml
     carbon-protocol = { git = "ssh://git@github.com/raphsaeed/carbon.git", branch = "main" }
     ```
   - **Prebuilt Local Headless Carbon on DGX Spark (`/home/raphspark/projects/carbon/target/release/carbon-shell`):**
     - Start: `/home/raphspark/projects/carbon/target/release/carbon-shell --headless --lan --split --port 7420`
     - Health: `curl -s http://127.0.0.1:7420/health` (`{"status":"ok","protocol":"0.1"}`)
     - WebSocket: `ws://127.0.0.1:7420/carbon` (supports `batch`, `getState` with `includeTokens: true`, and deterministic CPU-rasterized `captureBoard` PNG snapshots)
     - Live Browser Viewer (MacBook / iPad): `http://192.168.68.52:7420/` (Ethernet) or `http://192.168.68.85:7420/` (Wi-Fi)
     - Stop: `pkill -f "carbon-shell --headless"`
   - **Native MacBook Carbon Window:** `ws://192.168.68.64:7420/carbon` (`carbon-shell --lan --split` on macOS Metal).

### Phase 1 Sub-Steps
- [x] **1.0 Pedagogical & Visual Rules Hierarchy (`rules/`):**
  - Created 4-tier strictly product-isolated rules tree (`rules/common.md` $\to$ `rules/<product>/common.md` $\to$ `rules/<product>/<subject>/{common,levels}.md`) covering `gyaraplus` (`maths` Algebra-First, `english`, `verbal-reasoning`, `non-verbal-reasoning`), `gcse` (`maths` Foundation/Higher + `physics`), `alevel` (`maths` + `physics`), `primary`, and `infant`.
- [x] **1.1 Product & Subject Directory Scaffolding & Brand Assets:**
  - [x] Place Git-tracked `Gyara+` brand logos in `products/gyaraplus/assets/` (`gyara-white.png`, `gyara-blue.png`, `gyara-white.svg`, `gyara-blue.svg`) and ensure `.gitignore` keeps `products/` tracked while ignoring `products/**/inbox/`, `products/**/archive/`, `products/**/ledger.json`, `products/**/status.html`, `.env`, `.keys.json`, and SQLite/`.raphs` binaries.
  - [x] Standardize sub-product isolation (`products/<product>/<subject>/{inbox,archive,ledger.json,status.html,<stem>.sqlite,<stem>.raphs}` created on demand per worked subject, with shared brand assets in `products/<product>/assets/`).
- [x] **1.2 Lean Pure-Rust Project & Configuration Setup (`Cargo.toml` & `src/config.rs`):**
  - Initialized `hydrogen` crate (Edition 2024, `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`), avoiding `clap`/`config`/`anyhow` bloat (pure `std::env::args` CLI + pure-Rust `.env` & `hydrogen.toml` loader).
  - Wired dependencies to **`theta_calc`**, **`memory_core_rs`** (`memory-core-1.7.0`), and **`carbon-protocol`**.
- [x] **1.3 `L7` Vault Wrapper & Brand Logo Blob Seeding (`src/vault/`):**
  - Implemented thread-safe `VaultManager` (`tokio::sync::Mutex<Memory>`, Conformance Rule A6) handling live `<name>.sqlite` + `<name>.keys.json` containers, `.raphs` bundle `pack` / `unpack`, and `export_clean_course_bundle`.
  - Initialized `gyara-maths.raphs` (`course_id: "GYR-MTH"`, `default_answer_format: "mcq"`, `theta_preset: "uk_gcse"`), embedded `GYR-LOGO-BLUE` (`gyara-blue.png`) and `GYR-LOGO-WHITE` (`gyara-white.png`) PNG blobs into `course_media_assets` + `course_meta.config_json.branding`, and seeded 8 canonical 11+ Maths strands (`course_topics`) and 10 problem archetypes (`course_question_types`).
- [x] **1.4 In-Process `Theta` Integration (`src/theta/`):**
  - Implemented `ThetaService` wrapping `theta_calc` (`calc_eval`, `calc_solve`, `calc_formula`, `calc_transform`, `calc_stats`, `calc_analyze_graph`, `dispatch_tool_json`) with stage presets (`allow_mixed_fractions: true`, `angle_mode: "deg"`, `anchor_tokens: true`), exact equivalence checks, and 5-option `A–E` MCQ distractor gate.
- [x] **1.5 `Carbon` WebSocket Client, Full-Stage Catalog Projector & Handshake (`src/carbon/`):**
  - Implemented async `CarbonClient` (`ws://...:7420/carbon` + `project_full_stage_catalog`) verifying `board.ready`, `ping`/`pong`, atomic `config.setTheme("warm-paper")` + `chrome.update` + `chrome.setCatalog` (`displayMode: "full-stage"`, 12-subject muted mineral palette, `GYR-LOGO-BLUE` PNG data URI), `getState`, and live `ServerMessage::Event` streaming.

---

## Phase 2: Domain Blueprints, Quality Gates, PDF Slicer & Core CLI
*Goal: Lock the strongly-typed `question_json` and `solution_json` schemas, build the deterministic verification gates, build the private PDF/image slicer, and expose `hydrogen` CLI commands for slicing, verifying, storing, and previewing questions.*

- [x] **2.1 Universal ID, RuleLoader & Blueprint Types (`src/rules/` & `src/domain/`):**
  - Implemented 4-tier `RuleLoader` (`src/rules/`) parsing YAML frontmatter (`theta` preset, `defaults`, `level_9_policy`) with strict product isolation.
  - Implemented `UniversalId` parser/builder (`GYR-MTH-P001-Q014`, replicas `-R01` / `P001_R01`, chapters `B01-CH03`, cards `-C01`, sub-addressing `:a:S01:L09`).
  - Implemented `QuestionBlueprint` (`question_json`), `SolutionBlueprint` (`solution_json`: `Steps × Sub-Levels 9..1`), `QuestionPackage`, and `PaperPackage`.
- [x] **2.2 Deterministic Quality Gate (`src/compiler/gate.rs`):**
  - **Theta Math Gate:** Verifies every `thetaVerification` check, multi-part `exportedExact` chain, `answerKey`, and 5-option (`A–E`) MCQ uniqueness.
  - **Carbon Command, Warm-Paper Theme, 5 Font Voices & KaTeX Gate ([`carbon-visual-theme.md`](./carbon-visual-theme.md)):** Verifies `$...$` `PennstanderMath` wrapping, the 5 curated font voices (`handwritten`, `typeset`, `casual`, `marker`, `typewriter`; rejects `classic-serif`), `c1..c8` editorial tokens, diagram strokes `<= 2.2 pt`, zero duplicate `✓` inside `"kind": "check"`, PNG-only logos, and execution-order `targetId` / `\htmlId` resolution.
  - **Pedagogical & Clean-Room Lint:** Enforces contiguous `9 → min_sublevel` numbering, non-identical adjacent levels, board prose word caps, and zero external paper names/raw scans.
- [x] **2.3 Clean-Room PDF/Image Slicer (`src/ingest/slicer.rs` & `ledger.rs`):**
  - Pure-Rust `hayro` PDF rasterizer + `png` cropper writing pages/crops into `archive/` and `{ universal_id, source_file, page, bbox, bboxPx, cropPath }` straight into git-ignored `ledger.json`.
- [x] **2.4 Core `hydrogen` CLI & Reference Questions (`src/main.rs`):**
  - Implemented `doctor`, `init-vault`, `slice-paper`, `theta`, `verify-question`, `upsert-question`, `upsert-paper`, `catalog`, `preview-carbon [--watch]`, `pack`, and `demo`; verified end-to-end with `GYR-MTH-P001-Q001` and `GYR-MTH-P001-Q015`.

---

## Phase 3: Product Authoring Ledger (`ledger.json` + `status.html`), Two-Way Carbon `--watch` Sync & Visual/Media CLI
*Goal: Track product-wide ingestion queues, paper progress, topics, and tasks in a local git-ignored `ledger.json` + static Warm-Paper `status.html`, make `preview-carbon --watch` persist `Snap Board` comments two-way into `.raphs`, and add headless Carbon PNG capture + bounds checking + media/comment CLI commands.*

- [x] **3.1 Sub-Product Authoring Ledger & Static `status.html` Dashboard (`src/ingest/ledger.rs`, `src/ingest/report.rs` & `.gitignore`):**
  - Implement git-ignored `products/<product>/<subject>/ledger.json` tracking everything outside Clean-Room `.raphs` vaults:
    - **SHA-256 Inbox & Archive Inventory:** Scans `products/<product>/<subject>/{inbox,archive}/`, detecting renamed duplicate files automatically and tracking each source file's `paper_id` and stage (`queued`, `sliced`, `in_progress`, `completed`).
    - **Paper Expectations & Checklist:** Expected question counts per paper (e.g. `GYR-MTH-P001`: `50` questions), sliced vs. verified item counts, missing question numbers, item source/crop provenance, and QA notes (auto-updated on `slice-paper`, `upsert-question`, and `upsert-paper`).
    - **Authoring Work Queue:** Queued replica requests, planned topic/concept cards, and backlog items.
  - Implement `hydrogen status [--product <slug>] [--subject <slug>] [--set-expected <paper-id> <count>] [--html]` discovering only active sub-products and merging live `inbox/` scans + unified `ledger.json` (including crop geometry & replica lineage) + `CourseCatalogSnapshot` & open `course_comments`:
    - Prints a crisp terminal briefing for Antigravity sessions, and
    - Generates a self-contained Warm-Paper `products/<product>/<subject>/status.html` dashboard (vault overview card, paper progress bars, topic coverage matrix, side-by-side `archive/` crop vs. compiled stem table, and open `Snap Board` comment thumbnails).
- [x] **3.2 Two-Way Live Carbon `--watch` Event Persistence (`src/vault/mod.rs` & `src/main.rs`):**
  - Upgrade `VaultManager` with comment CRUD & attachment helpers (`list_comments`, `get_comment`, `upsert_comment`, `resolve_comment`, `delete_comment`, `read_comment_attachment`).
  - Upgrade `hydrogen preview-carbon --watch` so Carbon (`:7420`) acts as a complete two-way authoring interface:
    - `comment.saved` $\to$ decodes base64 `Snap Board` PNG / uploaded attachments into `CommentAttachmentBytes`, persists straight into `gyara-maths.sqlite` (`upsert_course_comment`), and updates `chrome.setFeedback` / open comment counts live,
    - `comment.deleted` $\to$ removes the comment from `gyara-maths.sqlite`,
    - `catalog.actionRequested` (`"create-replica"`) $\to$ records a pending replica task in `products/<product>/<subject>/ledger.json`.
- [x] **3.3 Headless Carbon Visual/Bounds Verifier, Media Asset & Comment CLI (`src/carbon/` & `src/main.rs`):**
  - Extend `CarbonClient` (`src/carbon/mod.rs`) with `capture_board(...)` (`ClientMessage::CaptureBoard` $\to$ `ServerMessage::Capture` PNG bytes + bounds) and deterministic `getState(includeTokens: true)` AABB overlap & out-of-pane clipping inspection against prebuilt headless Carbon (`ws://127.0.0.1:7420/carbon`).
  - Add CLI subcommands for Antigravity workflows:
    - `hydrogen capture-carbon <question-id> [--part P] [--step S01] [--level 9] [--pane right|left] [--check-bounds] --out <snap.png>`
    - `hydrogen put-asset <file.png|svg|mp4|mp3> --asset-id <ID> [--alt "..."] [--caption "..."]`
    - `hydrogen comments [list | show <id> [--export-dir <dir>] | resolve <id> --note "..."]`

> **Phase 3 Exit Check:** `cargo clippy --all-targets -- -D warnings` and `cargo test` pass with zero new heavy dependencies; `hydrogen status --html` generates `products/gyaraplus/maths/status.html`; `hydrogen capture-carbon GYR-MTH-P001-Q015 --check-bounds` verifies bounds and saves a crisp headless Carbon PNG; `preview-carbon --watch` persists a simulated/live `Snap Board` comment round-trip into `gyara-maths.sqlite`.

---

## Phase 4: Milestone 1 Live Proving Ground — 1st Real 11+ Maths Paper (`GYR-MTH-P001`) End-to-End
*Goal: Process the entire first real 11+ Maths paper (`Practice-Paper-01.pdf` $\to$ `GYR-MTH-P001`) via Antigravity + Hydrogen CLI + headless Carbon visual verification, walk through it live on Carbon, and polish every ecosystem detail.*

- [ ] **4.1 Complete Clean-Room Slicing of `Practice-Paper-01.pdf` (`GYR-MTH-P001`):**
  - Render all 24 pages of `/home/raphspark/projects/hydrogen/products/gyaraplus/maths/inbox/Practice-Paper-01.pdf` at `180 DPI`, visually map every numbered question (`Q001..QN`) and diagram, slice clean per-question crops into `products/gyaraplus/maths/archive/GYR-MTH-P001/crops/` + `ledger.json`, and record the full item checklist in `ledger.json`.
- [ ] **4.2 Full-Paper `Level 9 → 1` Authoring & Verification via Antigravity:**
  - Author complete `QuestionPackage` JSONs (`products/gyaraplus/maths/archive/GYR-MTH-P001/GYR-MTH-P001-Q*.json`) across all questions of `GYR-MTH-P001` (using parallel Antigravity subagents where helpful), obeying:
    - 4-tier `gyaraplus/maths` Algebra-First rules (`rules/gyaraplus/maths/{common,levels}.md`),
    - Deterministic `theta_calc` verification on every step & 5-option `A–E` MCQ distractor set,
    - Carbon's 5 Curated Font Voices + `$...$` `PennstanderMath`, 2D/3D macros, and regenerated clean diagrams/assets (`put-asset`),
    - Zero issues in `hydrogen verify-question` AND headless `hydrogen capture-carbon --check-bounds`,
  - Upsert the complete paper + all verified questions into `gyara-maths.raphs` (`upsert-paper` / `upsert-question --pack`) and refresh `status.html`.
- [ ] **4.3 Live End-to-End Walkthrough on Carbon (`preview-carbon --watch`):**
  - Drive Carbon (`:7420`, via LAN WebGL2 `http://192.168.68.52:7420/` or native MacBook `ws://192.168.68.64:7420/carbon`):
    - Verify Full-Stage Gallery (`GYR-LOGO-BLUE`, `Practice Paper 01`, `Topics`, `Stats` tabs, search & filters),
    - Step through `Levels 9 → 1`, `Break step down`, and `Reveal Answer` across arithmetic, fractions/percentages, algebra, ratios, charts/tables, and 2D/3D geometry,
    - Save **`Snap Board`** comments + marker sketches on Carbon and verify instant persistence into `gyara-maths.sqlite` and `status.html`.
- [ ] **4.4 Ecosystem Polish & Rule Refinement Pass:**
  - Resolve every `fix` comment from the walkthrough, refine `rules/` or `gate.rs` lints where discovered, and lock `GYR-MTH-P001` as the gold-standard teaching paper.

---

## Phase 5: Live Teaching Feedback (`Snap Board` Fixes), Replicas & Organic Topic Cards
*Goal: Turn live teaching feedback from Carbon into streamlined Antigravity workflows for comment fixes, on-demand question/paper replicas, and reusable `Level 9 → 1` concept/prerequisite cards.*

- [ ] **5.1 Structured Comment-to-Fix Workflow (`course_comments` + `course_question_revisions`):**
  - Streamline `hydrogen comments` bundle export (`Snap Board` PNG + `ledger.json` crop + current `<QID>.json`) so Antigravity patches the exact affected `(part, step, sublevel)`, verifies through `gate.rs` + `capture-carbon`, upserts (auto-snapshotting prior JSON in `course_question_revisions`), logs `@theta`/`@carbon` engine tickets to `docs/ecosystem-backlog.md`, and resolves the comment with `resolution_note`.
- [ ] **5.2 Question & Whole-Paper Replica Pipeline (`-R01`, `P001_R01`):**
  - Add replica scaffolding & deterministic replica gates (`hydrogen verify-replica` checking parent archetype parity, fresh numbers/contexts, `is_replica = true`, `parent_question_id` / `parent_paper_id` links, Theta-verified `A–E` distractors, and zero parent-number text leaks) so Antigravity processes queued replicas from `ledger.json` at `same` / `higher` / `easier` difficulty.
- [ ] **5.3 Organic Topic, Worked-Example & Prerequisite Card Pipeline (`course_topic_cards` + `course_concept_links`):**
  - Implement strongly-typed `TopicCardPackage` (`concept`, `worked_example`, `formula`, `reference_sheet`, `prerequisite_refresher` with `9 → 1` Carbon sublevels), `gate.rs` verification, `hydrogen verify-card` / `upsert-card` CLI, `status.html` topic card indexing, and Carbon `flow.detourPush` (`scope: "solution"`) live preview.

---

## Phase 6: Clean Customer Release Pipeline, Books/Chapters & Multi-Subject Vault Rollout
*Goal: Package commercial single-key `.raphs` vaults and expand authoring across all `Gyara+` and `GCSE` subjects.*

- [ ] **6.1 Commercial Customer `.raphs` Release Verification:**
  - Add automated clean-export audit (`hydrogen verify-clean-bundle <out.raphs> --customer-key <key>`) asserting zero `course_comments`, zero `course_question_revisions`, zero authoring `provenance`, zero clean-room leaks, intact brand/media `blobs`, and `< 1 ms` `course_catalog_snapshot()` unlock.
- [ ] **6.2 Books, Chapters & Shared Stimuli Workflow (`course_books` / `course_chapters` / `course_stimuli`):**
  - Add `BookPackage` / `ChapterPackage` blueprints and CLI commands (`upsert-book`, `upsert-chapter`, `upsert-stimulus`) for structured textbook chapters (`B01-CH03`: `Concept → Worked Examples → Exercises`) and shared passages/datasets (`STIM01`).
- [ ] **6.3 Multi-Subject & Multi-Product Vault Profiles:**
  - Add vault seeders (`src/vault/seed.rs`) and subject-specific `gate.rs` validators for:
    - `gyara-non-verbal-reasoning.raphs` (`GYR-NVR`: `netFold`, `cubeAssembly`, `faceMarkings`, 2D spatial transforms),
    - `gyara-verbal-reasoning.raphs` (`GYR-VR`: code keys, letter/word rules),
    - `gyara-english.raphs` (`GYR-ENG`: reading comprehension `course_stimuli` + `chrome.setDictionary`),
    - `gcse-maths.raphs` (`GCSE-MTH` Foundation & Higher overlays) and `gcse-physics.raphs` (`GCSE-PHY`: `PhysicsRig`, `calc.formula` FIFA workflow, SI unit checks).

---

## Phase 7: Zero-Install Web Browser Delivery (`carbon-web` WASM + `theta.wasm` + Web Vault Gateway)
*Goal: Let individual students and parents (on Mac, Windows, iPad Safari, and Chromebooks) log in and use the full 120Hz interactive Carbon whiteboard, Full-Stage Course Gallery, and `Level 9 → 1` lessons directly in a standard web browser with zero native app installation—built from the **exact same `main` branches**.*

- [ ] **7.1 Single-Codebase Browser WASM Targets (`carbon-web` + `theta-wasm`, Zero Forked Branches):**
  - Package Carbon's `wasm32-unknown-unknown` `StageRuntime` (`engine.wasm` + HTML/WebGL2 host + Full-Stage Course Gallery) and pure-Rust `theta_calc` (`theta.wasm` for client-side 120Hz `@param` sliders & CAS) as static browser bundles directly from `main` alongside native `carbon-shell`.
- [ ] **7.2 Server-Side `.raphs` Vault Streaming Gateway (Zero Vault Piracy):**
  - Serve `<product>-<subject>.raphs` from a lightweight Rust web gateway alongside **SIGMA**: keep `.raphs` strictly server-side and stream only the merged `chrome.setCatalog` view (`Course L7` + `<student>.raphs` `L1–L6` progress, `△` important flags, and `15d` / `#FFF1F2` `dueReminder` states) plus active `(question, step, level)` Carbon JSON commands over `wss://`.
- [ ] **7.3 Individual User Login, Cross-Device Sync & Instant "Try Free in Browser" Funnel:**
  - Add zero-install *"Try a Free Paper"* browser entry links, parent/student authentication, automatic cross-device `<student>.raphs` sync (laptop $\leftrightarrow$ iPad browser), and browser WebAudio/WebRTC voice tutoring with **SIGMA**.

> **Phase 7 Exit Check:** An individual student on an iPad, Chromebook, Windows PC, or Mac opens the web URL in Chrome/Safari with zero installation, browses the Full-Stage `Gyara+` Course Gallery, works through an interactive `Level 9 → 1` Maths paper with live stylus/mouse ink and audio tutoring, and sees progress and reminders persist across devices.

---

## Phase 8: Modular Multi-Model API & Batch Automation (`src/ai/` Providers & Batch Runner)
*Goal: Add a clean, pluggable, provider-agnostic Rust AI/Media layer (`src/ai/`) for external model APIs, automated media generation, and unattended high-volume batch runs.*

- [ ] **8.1 Clean Model-Agnostic Provider Traits (`src/ai/mod.rs`):**
  - Define simple, decoupled Rust traits (`ReasoningModel`, `ImageModel`, `VideoModel`) isolated from `domain/`, `gate.rs`, and `vault/` so models and endpoints can be swapped cleanly via configuration.
- [ ] **8.2 Direct Vertex AI Adapter (`src/ai/vertex.rs`):**
  - Pure-Rust (`reqwest` + `rustls-tls`) Vertex AI provider supporting Service Account / API Key auth for `ReasoningModel` (`gemini-4-carbon`), `ImageModel` (`gemini-nano-banana-2.1`), and `VideoModel` (`gemini-omni-1.2-flash`) piped into `VaultManager::put_asset`.
- [ ] **8.3 Checkpointed Batch Runner (`src/compiler/jobs.rs`):**
  - Multi-step `Step A → D` autonomous compiler (`Extract → Theta Solve → L9..L1 Synthesis → Gate + Headless captureBoard Self-Check → Auto-Repair`) and concurrent `compile-paper` CLI orchestration.
