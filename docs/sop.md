# Hydrogen — Standard Operating Procedure & Engineering Playbook (`sop.md`)

> **Purpose of this document:** Everything needed for the founder or an AI agent (Antigravity) to pick up **Hydrogen** cold—today or months from now—and immediately know **every non-negotiable boundary, repository path, way of working, headless/LAN Carbon workflow, `ledger.json` & `status.html` convention, CLI command, and hard-won engineering trick**.

---

## 1. 60-Second Executive Orientation & Non-Negotiable Laws

**Hydrogen** (`/home/raphspark/projects/hydrogen`) is the pure-Rust curriculum compiler toolkit, `Level 9 → Level 1` visual solution verifier, product authoring ledger, and encrypted `L7` `.raphs` course vault manager.

Rather than running heavy web servers or brittle hardcoded prompt chains, Hydrogen pairs **Antigravity** (operating interactively and in background SSH sessions with parallel subagents) with a fast, deterministic, zero-bloat **`hydrogen` Rust CLI** backed by in-process **Theta** (`theta_calc`), **`memory.raphs`** (`memory_core_rs 1.7.0`, Schema `2.3.0`), and **Carbon** (`Carbon Protocol v0.1`).

### 1.1 Ecosystem Component Matrix & Strict Independence Boundary

| Component | Role | DGX Spark / Git Location | Port / Binding | Strict Boundary Rule |
| :--- | :--- | :--- | :---: | :--- |
| **`Hydrogen`** *(This Repo)* | Pure-Rust Authoring CLI, PDF/PNG Slicer, Product Ledger (`ledger.json` + `status.html`), `:7430` `Hydrogen Dashboard` Server, 4-Tier `RuleLoader`, Quality Gate, Two-Way Carbon `--watch` Bridge & `L7` Vault Manager | `/home/raphspark/projects/hydrogen`<br>`git@github.com:raphsaeed/hydrogen.git` (`main`) | `:7430` (`hydrogen serve`) + CLI + outbound WS to `:7420` | **Edit only inside `/home/raphspark/projects/hydrogen`.** Never runs during student lessons; never stores learner stats in course vaults. |
| **`Carbon`** | 120Hz Interactive GPU/WASM Whiteboard & Headless Stage (`carbon-shell`, `carbon-protocol`) | DGX Spark: `/home/raphspark/projects/carbon` *(prebuilt)*<br>MacBook: `~/projects/carbon`<br>`ssh://git@github.com/raphsaeed/carbon.git` (`main`) | `:7420` (`/carbon`, `/health`, `/` WebGL2) | **STRICTLY READ-ONLY SIBLING.** Never modify files inside `/home/raphspark/projects/carbon`. Consume via `carbon-protocol` crate and `ws://...:7420/carbon`. |
| **`Theta`** (`theta_calc`) | Deterministic Symbolic/Rational CAS, 313-Formula Solver & Graph VM (`calc-api`) | `/home/raphspark/projects/theta`<br>Crate path: `crates/calc-api` | In-process Rust (`< 50 µs`) | **STRICTLY READ-ONLY SIBLING.** Never modify files inside `/home/raphspark/projects/theta`. |
| **`memory.raphs`** (`memory_core_rs`) | Encrypted SQLCipher `4.14.0` + `sqlite-vec` `0.1.9` + FTS5 Vault Engine | `ssh://git@github.com/raphs-app/raphs.git`<br>Pinned tag: **`memory-core-1.7.0`** (Schema `2.3.0`) | In-process Rust (`Mutex<Memory>`) | **Immutable tagged crate.** Enforce Conformance Rule A6 (single `Memory` handle per `.sqlite` file per process). |
| **`SIGMA`** | Runtime Voice & Adaptive Tutoring Conductor (merges `Course L7` + `Student L1–L6`) | Separate runtime repo | Client / `wss://` | **STRICTLY READ-ONLY / SEPARATE REPO.** Never modify from Hydrogen. |

### 1.2 Five Non-Negotiable Engineering Laws

1. **Strict Repository Independence:** Never create, edit, rebuild, or delete files inside `/home/raphspark/projects/theta`, `/home/raphspark/projects/carbon`, `/home/raphspark/projects/plugin-vertexai`, `SIGMA`, `gyara`, or any other sibling repo.
2. **Pure, Lean, Panic-Free Rust (`Edition 2024`):**
   - `Cargo.toml` enforces `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`, and `panic = "deny"` across **all** targets (`lib`, `bin`, unit tests, and `tests/*.rs`).
   - Minimal dependencies: zero `clap`, `anyhow`, `config`, pdfium/poppler C++ bindings, or system OpenSSL C bindings.
3. **Clean-Room `.raphs` Anonymization Law:**
   - Raw source PDFs (`products/**/inbox/`), rendered pages & crops (`products/**/archive/`), `ledger.json`, `status.html`, and any external school, exam board, publisher, or source PDF filename **MUST NEVER** enter `<product>-<subject>.raphs` or Git.
   - All diagrams/graphs/tables/clocks/nets are regenerated natively in Carbon (`scene.place2D` / `scene.place3D`); real photographs/videos are recreated cleanly as generated assets in `course_media_assets`.
4. **Warm-Paper Theme & 5 Curated Font Voices + `PennstanderMath` Law:**
   - Every board command targets Carbon's light editorial `"warm-paper"` surface (`#FDFBF7` Left / `#FDFCF9` Right, primary ink `#1F2421`, semantic tokens `c1..c8`, diagram strokes `<= 2.2 pt`, transparent diagram labels, PNG-only `brandLogo`/`logo` `GYR-LOGO-BLUE`).
   - Wrap **100% of math, variables, fractions, exponents, currencies, and units in `$...$` or `\(...\)`** (rendered automatically by Carbon in **`PennstanderMath`** across all 5 curated voices: `handwritten`, `typeset`, `casual`, `marker`, `typewriter`). **Never** emit `"classic-serif"` unless explicitly requested.
5. **Zero-Drift Split Between `.raphs` Vaults & Local Sub-Product `ledger.json`:**
   - `products/<product>/<subject>/<stem>.sqlite` / `.raphs` is the single source of truth for compiled curriculum, `9 → 1` solutions, PNG/media blobs, revisions, and `course_comments` (**zero learner stats**).
   - Git-ignored `products/<product>/<subject>/ledger.json` tracks raw `inbox/` SHA-256 inventory, expected paper question counts, item provenance, and queued authoring tasks for that sub-product; `hydrogen status --html` merges both into `products/<product>/<subject>/status.html` (showing only active/worked subjects).

---

## 2. Machines, Network Topology & Prebuilt Headless Carbon (`DGX Spark` ↔ `MacBook`)

### 2.1 Live Environment & Endpoint Map

| Host / Service | Machine | Address / Path | Purpose |
| :--- | :--- | :--- | :--- |
| **Hydrogen Workspace** | DGX Spark (`Linux aarch64`) | `/home/raphspark/projects/hydrogen` | Primary development, Antigravity SSH sessions, CLI compilation, `ledger.json`, `status.html`, and `.raphs` vaults |
| **Hydrogen Status Server (`Phase 3A`)** | DGX Spark (`Linux aarch64`) | `http://127.0.0.1:7430/` *(local)*<br>`http://192.168.68.52:7430/` *(Ethernet)*<br>`http://192.168.68.85:7430/` *(Wi-Fi)* | `hydrogen serve --lan --port 7430` (`axum` Carbon Warm-Paper Multi-Product $\to$ Sub-Product $\to$ `status.html` Explorer + `inbox/`/`archive/` file viewer) |
| **Prebuilt Headless Carbon** | DGX Spark (`Linux aarch64`) | `/home/raphspark/projects/carbon/target/release/carbon-shell` | Local headless `Carbon Protocol v0.1` server (`ws://127.0.0.1:7420/carbon`), `getState` token bounds, deterministic CPU `captureBoard` PNGs, & LAN WebGL2 viewer |
| **DGX Carbon LAN Viewer** | MacBook / iPad Browser | `http://192.168.68.52:7420/` *(Ethernet)*<br>`http://192.168.68.85:7420/` *(Wi-Fi)* | Zero-install 120Hz `engine.wasm` + WebGL2 mirror of DGX Spark's headless Carbon stage |
| **Native MacBook Carbon** | Apple Silicon Mac | `ws://192.168.68.64:7420/carbon` | Native macOS Metal `carbon-shell --lan --split` window on the founder's MacBook |

### 2.2 Operating Prebuilt Headless Carbon on DGX Spark

We have direct read/execute access to the prebuilt **Carbon** headless server at `/home/raphspark/projects/carbon/target/release/carbon-shell` (**never** modify files inside `/home/raphspark/projects/carbon`; read `/home/raphspark/projects/carbon/docs/carbon-integration.md` and [`docs/carbon-integration.md`](./carbon-integration.md) for the full wire contract).

#### 1. Start Headless Carbon (with LAN WebGL2 Viewer Enabled)
If port `7420` is not already listening, start `carbon-shell` in the background:
```bash
/home/raphspark/projects/carbon/target/release/carbon-shell --headless --lan --split --port 7420
```

#### 2. Verify Health, Token Bounds & Deterministic PNG Capture
- **Health check:**
  ```bash
  curl -s http://127.0.0.1:7420/health
  # Returns: {"status":"ok","protocol":"0.1"}
  ```
- **Driver / Pipeline WebSocket (`Carbon Protocol v0.1`):** `ws://127.0.0.1:7420/carbon`
  - Send `command` / `batch` envelopes and verify `{"type":"ack","ok":true}`.
  - Inspect layout & `\htmlId` token bounds:
    ```json
    {"version":"0.1","type":"getState","id":"st-1","includeTokens":true,"includeAttachmentData":false}
    ```
  - Render deterministic PNG snapshots (via Carbon's windowless CPU `FrameDrawList` rasterizer):
    ```json
    {"version":"0.1","type":"captureBoard","id":"cap-1","pane":"right","cropToUserInk":false,"includeChrome":false}
    ```
- **Live MacBook / iPad Browser Stage (Optional Visual Check):**
  - Ethernet: `http://192.168.68.52:7420/`
  - Wi-Fi: `http://192.168.68.85:7420/`

#### 3. Stop Headless Carbon When Finished
```bash
pkill -f "carbon-shell --headless"
```

### 2.3 Operating the Hydrogen Dashboard Server (`:7430`) on DGX Spark

`hydrogen serve` runs the pure-Rust `axum` **Hydrogen Dashboard** (Carbon Warm-Paper Multi-Product $\to$ Sub-Product $\to$ Status Explorer) on port **`7430`** alongside Carbon on **`7420`**.

#### 1. Start the Hydrogen Dashboard (`:7430`)
- **Foreground (interactive SSH / terminal — prints Local + LAN URLs, stops cleanly on `Ctrl+C`):**
  ```bash
  cd /home/raphspark/projects/hydrogen
  ./target/release/hydrogen serve --lan --port 7430
  # Or during development:
  export PATH="$HOME/.cargo/bin:$PATH"
  cargo run --quiet -- serve --lan --port 7430
  ```
- **Localhost-only mode (no LAN exposure, binds `127.0.0.1:7430` from `hydrogen.toml`):**
  ```bash
  ./target/release/hydrogen serve
  ```
- **Background daemon (detached SSH session):**
  ```bash
  nohup /home/raphspark/projects/hydrogen/target/release/hydrogen serve --lan --port 7430 \
    >/tmp/hydrogen-serve.log 2>&1 &
  ```

#### 2. Browse & Verify Live Endpoints (`MacBook` / `iPad` / `DGX Spark`)
- **MacBook / iPad Browser (Ethernet):** `http://192.168.68.52:7430/`
- **MacBook / iPad Browser (Wi-Fi):** `http://192.168.68.85:7430/`
- **Local DGX Spark:** `http://127.0.0.1:7430/`
- **Health & JSON Overview Checks:**
  ```bash
  curl -s http://127.0.0.1:7430/health
  # Returns: {"service":"hydrogen","status":"ok","version":"0.1.0"}

  curl -s http://127.0.0.1:7430/api/overview
  ```
- **On-Demand Vault Sync:** Click **`Sync Vault`** in the web UI (or `curl -s -X POST http://127.0.0.1:7430/api/products/gyaraplus/maths/sync`) to refresh cached `vaultSummary` in `ledger.json` and regenerate `status.html`.

#### 3. Stop the Hydrogen Dashboard (`:7430`)
- **If running in foreground:** Press **`Ctrl+C`** (triggers graceful `SIGINT` shutdown).
- **If running in background / another shell:**
  ```bash
  pkill -INT -f "hydrogen serve"
  # Confirm port 7430 is closed:
  ss -tlnp | grep ':7430\b' || echo "Hydrogen Dashboard (:7430) stopped."
  ```

---

## 3. Standard Ways of Working (Day-to-Day Authoring Playbook)

### Workflow A — Starting / Resuming Any Session (`hydrogen status` & `hydrogen serve`)
Whenever starting a new Antigravity session (or checking active product / sub-product progress):
1. Run `hydrogen status --product gyaraplus --html` (or launch `hydrogen serve --lan --port 7430` in `Phase 3A` to browse all active products & sub-products live from MacBook/iPad at `http://192.168.68.52:7430/`).
2. Inspect the terminal summary or Carbon Warm-Paper dashboard (`products/<product>/<subject>/status.html`):
   - **Zero-clutter discovery:** Products with `0` active sub-products and untouched subjects never appear—only worked folders (e.g. `products/gyaraplus/maths/` or `products/gcse/{maths,physics}/`) are shown,
   - Cached `vaultSummary` in `ledger.json` enables `< 2 ms` multi-product card scanning without decrypting SQLCipher vaults on every page view,
   - Section tabs (`Overview & Papers`, `Questions & Crops`, `Topics`, `Replicas`, `Comments & Tasks`, `Sources`), instant question filter chips (`All`, `Verified`, `Sliced / Pending`, `Replicas`, `Open Comments`), search box, and click-to-zoom crop/`Snap Board` lightbox,
   - Which raw PDFs/images are sitting unprocessed in `products/<product>/<subject>/inbox/` (checked by SHA-256 so renamed duplicates are flagged automatically),
   - Which papers are partially compiled (e.g. `GYR-MTH-P001`: `2/50` verified, next uncompiled items `Q002..Q014`),
   - Replica lineage (`single_question` vs. `whole_paper` replicas + original parent & source references),
   - Open `Snap Board` comments (`fix`, `replica`, `topic`, `engine`) and queued tasks in `ledger.json`.

### Workflow B — Ingesting & Compiling a Paper via Antigravity
1. **Render Pages & Map Clean Bounding Boxes:**
   - Render pages at `180 DPI` (`hydrogen slice-paper <pdf> --paper-id GYR-MTH-P001 --pages-only`).
   - Visually inspect page PNGs in `archive/.../pages/`, determine exact `[x, y, w, h]` normalized boxes for each question (`Q001..QN`), and slice crops into `archive/.../crops/` + `ledger.json` (`hydrogen slice-paper <pdf> --paper-id GYR-MTH-P001 --crops-json ...`).
2. **Ground Every Calculation & Distractor in `Theta`:**
   - Read the 4-tier rulebook (`rules/common.md` $\to$ `rules/gyaraplus/common.md` $\to$ `rules/gyaraplus/maths/{common,levels}.md`) and the clean crop PNG.
   - Run `hydrogen theta ...` (`calc.eval`, `calc.solve`, `calc.transform`, `calc.formula`, `calc.stats`, `calc.analyze_graph`) for every step, exact intermediate value, and all 5 `A–E` MCQ choices.
   - If a question requires a recreated clean photograph, illustration, audio clip, or short video, generate the clean asset and embed it via `hydrogen put-asset`.
3. **Author `QuestionPackage` (`Steps × Sub-Levels 9 → 1`) & Pass Both Gates:**
   - Save the compiled package to `products/<product>/<subject>/archive/<paper_id>/<QID>.json` (fanning out parallel Antigravity subagents across independent questions when compiling full papers).
   - **Gate 1 (Deterministic `< 1 ms`):** `hydrogen verify-question <QID>.json` (verifies Theta math, MCQ uniqueness, Carbon protocol types, Warm-Paper colors, 5 font voices + `PennstanderMath`, `\htmlId` execution order, non-repetitive contiguous `9..1` levels, and Clean-Room anonymization).
   - **Gate 2 (Headless Carbon Layout & Visual Check):** `hydrogen capture-carbon <QID> --check-bounds --out /tmp/<QID>.png` (verifies zero token/object bounding-box collisions on `:7420` and renders the actual board PNG for visual inspection).
4. **Atomic Vault Upsert & Dashboard Refresh:**
   - Run `hydrogen upsert-question <QID>.json --pack` (or `upsert-paper`), then `hydrogen status --html` to refresh `ledger.json` and `products/<product>/<subject>/status.html`.

### Workflow C — Live Interactive Preview & `Snap Board` Comment Capture on Carbon
1. Connect Hydrogen's live two-way `--watch` bridge to Carbon:
   - Against local DGX headless Carbon (viewed on MacBook/iPad at `http://192.168.68.52:7420/`):
     `cargo run --quiet -- preview-carbon --catalog --watch --carbon ws://127.0.0.1:7420/carbon`
   - Or directly against native MacBook Carbon (`carbon-shell --lan --split`):
     `cargo run --quiet -- preview-carbon --catalog --watch --carbon ws://192.168.68.64:7420/carbon`
2. Navigate Subject Cards $\to$ Papers/Topics $\to$ Questions, switch `(a)/(b)` Part Tabs, step `|◀ / ◀ / ▶`, click `Break step down`, tap `L9..L1` sublevel chips, or toggle `Reveal Answer`.
3. Draw marker annotations on the board, open Carbon's Comment Drawer (`💬`), click **`Snap Board`**, choose `fix` / `replica` / `topic` / `engine`, and click Save — `--watch` persists the comment and ref-counted PNG blob straight into `gyara-maths.sqlite` (`course_comments`) and `ledger.json` in real time!

### Workflow D — Resolving Comments, Creating Replicas & Topic Cards
1. Run `hydrogen comments list` to view open feedback.
2. Run `hydrogen comments show <comment-id> --export-dir /tmp/cmt` to inspect the comment alongside its `Snap Board` PNG and `ledger.json` original crop.
3. Patch the question JSON, verify via `verify-question` + `capture-carbon`, `upsert-question` (automatically recording the prior snapshot in `course_question_revisions`), and run `hydrogen comments resolve <comment-id> --note "..."`.

---

## 4. Workspace Directory Structure & Git Tracking Boundary

```text
/home/raphspark/projects/hydrogen/
├── .cargo/config.toml                      # `git-fetch-with-cli = true` (uses SSH keys for private GitHub deps)
├── .env                                    # GIT-IGNORED: Local secrets & endpoints (HYDROGEN_LESSON_KEY, CARBON_WS_URL)
├── .env.example                            # GIT-TRACKED: Template for `.env`
├── .gitignore                              # Ignores target/, .env*, *.sqlite*, *.keys.json, *.raphs, inbox/, archive/, ledger.json, status.html
├── Cargo.toml                              # Rust Edition 2024 manifest (forbid unsafe; deny unwrap/expect/panic)
├── hydrogen.toml                           # Workspace defaults (products_dir, rules_dir, bind_addr :7430, carbon_ws_url :7420)
├── README.md                               # Top-level index
├── assets/
│   └── hydrogen-logo.png                   # GIT-TRACKED: Hydrogen application logo (landing page & status header)
├── docs/                                   # Master specifications & operational playbooks
│   ├── sop.md                              # <--- THIS DOCUMENT (Ways of working, CLI playbook, Carbon headless, tricks)
│   ├── system-architecture.md              # 5-component topology, product ledger & strict conductor boundaries
│   ├── requirements.md                     # Product requirements & decision log
│   ├── build-plan.md                       # Living Phase 1 -> Phase 8 execution checklist
│   ├── solution-levels.md                  # Question-relative `Level 9 -> Level 1` spec & JSON schemas
│   ├── carbon-visual-theme.md              # Warm-Paper palette, 12-subject mineral colors & 5 Curated Font Voices
│   ├── carbon-integration.md               # Full `Carbon Protocol v0.1` command, 2D/3D macro & event reference
│   ├── theta-integration.md                # `theta_calc` tools, presets, units & 313-formula catalog
│   ├── memory-raphs.md                     # `memory.raphs` Schema 2.3.0 / `memory_core_rs 1.7.0` spec
│   └── gyara-workflow.png                  # Ecosystem workflow diagram
├── rules/                                  # GIT-TRACKED: Strictly isolated 4-tier pedagogical & visual rules
│   ├── README.md
│   ├── common.md                           # Tier 1: Global clarity, Warm-Paper tokens, 5 font voices, KaTeX hygiene
│   ├── gyaraplus/                          # Tier 2..4: 11+ (`common.md` + `{maths,english,verbal-reasoning,non-verbal-reasoning}/{common,levels}.md`)
│   ├── gcse/                               # Tier 2..4: GCSE (`common.md` + `maths/{common,levels,foundation,higher}.md` + `physics/{common,levels}.md`)
│   ├── alevel/                             # Tier 2..4: A-Level (`maths`, `physics`)
│   ├── primary/                            # Tier 2..4: Primary KS2 (`maths`)
│   └── infant/                             # Tier 2..4: Infant EYFS/KS1 (`maths`)
├── products/                               # Multi-product root (`assets/` TRACKED; sub-product folders GIT-IGNORED)
│   ├── gyaraplus/
│   │   ├── assets/                         # GIT-TRACKED shared brand logos: `gyara-blue.png`, `gyara-white.png`, `.svg`
│   │   └── maths/                          # Sub-product folder (created ONLY when working on `gyaraplus/maths`)
│   │       ├── inbox/                      # GIT-IGNORED: Drop raw source PDFs / worksheets here (e.g. Practice-Paper-01.pdf)
│   │       ├── archive/GYR-MTH-P001/       # GIT-IGNORED: Archived source PDF, 180 DPI `pages/`, `crops/`, & compiled `<QID>.json`
│   │       ├── ledger.json                 # GIT-IGNORED: Unified sub-product ledger (SHA-256 sources, pages/bbox crops, progress, replica lineage & tasks)
│   │       ├── status.html                 # GIT-IGNORED: Self-contained Warm-Paper dashboard strictly for `gyaraplus/maths`
│   │       ├── gyara-maths.sqlite          # GIT-IGNORED: Live SQLCipher 4.14 L7 working container (+ `.keys.json` sidecar)
│   │       └── gyara-maths.raphs           # GIT-IGNORED: Deliverable single-file `.raphs` sub-product bundle
│   └── gcse/
│       └── assets/                         # Sub-product folders (`maths/`, `physics/`) created only when worked on
├── src/
│   ├── lib.rs                              # Crate module exports & `VERSION`
│   ├── main.rs                             # Zero-dependency CLI dispatcher & interactive two-way `--watch` loop
│   ├── config.rs                           # Pure-Rust `OS env -> .env -> hydrogen.toml` loader
│   ├── error.rs                            # Unified `HydrogenError` (`thiserror`)
│   ├── base64_util.rs                      # Zero-dep RFC 4648 Base64 codec, `data:` URI formatter & PNG `IHDR` dimension reader
│   ├── rules/mod.rs                        # Product-isolated 4-tier `RuleLoader` + minimal YAML frontmatter parser
│   ├── domain/
│   │   ├── id.rs                           # `UniversalId` parser/builder (`GYR-MTH-P001-Q014:a:S01:L09`, `-R01`, `B01-CH03`, `T_RAT-C01`)
│   │   └── blueprint.rs                    # `QuestionBlueprint`, `SolutionBlueprint` (`9..1`), `QuestionPackage`, `PaperPackage`
│   ├── theta/mod.rs                        # In-process `ThetaService` (`theta_calc`), mixed-fraction normalizer & `A-E` MCQ uniqueness gate
│   ├── vault/
│   │   ├── mod.rs                          # Thread-safe `VaultManager` (`tokio::sync::Mutex<Memory>`, Rule A6)
│   │   └── seed.rs                         # Idempotent `GYR-MTH` metadata, `GYR-LOGO-BLUE`/`WHITE` PNG blobs, 8 topics & 10 question types
│   ├── carbon/
│   │   ├── mod.rs                          # Async `CarbonClient` WS driver (`connect`, `ping`, `send_batch`, `get_state`, `capture_board`, `send_interaction`, events)
│   │   ├── bounds.rs                       # Deterministic `getState(includeTokens: true)` AABB overlap & out-of-pane clipping inspector
│   │   ├── catalog.rs                      # `CourseCatalogSnapshot` -> `SetCatalogParams` (`full-stage`, 12-subject mineral palette)
│   │   ├── feedback.rs                     # Two-way `--watch` persistence (`comment.saved`, `comment.deleted`, `create-replica`, `chrome.setFeedback`)
│   │   └── preview.rs                      # Split-board `(part, step, sublevel, substep, answer)` batch builder & live event reducer
│   ├── ingest/
│   │   ├── slicer.rs                       # Pure-Rust `hayro` PDF rasterizer + `png` cropper (`PagesOnly`, `AutoWhitespaceBands`, `ExplicitCrops`)
│   │   ├── ledger.rs                       # Unified sub-product `ledger.json` (`vaultSummary`, SHA-256 sources, page/bbox crops, progress, replica lineage & tasks)
│   │   └── report.rs                       # Dual-mode Carbon Warm-Paper `status.html` renderer (`StandaloneFile` & `Server` modes, tabs, filters, lightbox)
│   ├── server/                             # `axum` `:7430` Multi-Product & Sub-Product Status Explorer (`hydrogen serve`)
│   │   ├── mod.rs                          # `ServerState` (Rule A6 per-vault lock), bind/LAN resolver, & file-path sandbox
│   │   ├── scanner.rs                      # `< 2 ms` cached `ledger.json` (`vaultSummary`) workspace & product roll-up scanner
│   │   ├── views.rs                        # Carbon Warm-Paper Landing (`/`) & Product (`/products/{product}`) HTML views
│   │   └── routes.rs                       # `axum` HTML, PNG logo, sandboxed `inbox/`/`archive/` file, JSON & Vault Sync routes
│   └── compiler/
│       └── gate.rs                         # Deterministic Quality Gate (Theta + Carbon + Warm-Paper + 5 Font Voices + KaTeX + Clean-Room)
└── tests/
    ├── phase2_pipeline.rs                  # End-to-end integration test (Rules -> Slice -> Gate -> Vault -> Carbon WS -> Clean Export)
    ├── phase3_ledger_and_carbon.rs         # End-to-end Phase 3 test (SHA-256 ledger, status.html, two-way `--watch`, put-asset, comments & headless captureBoard)
    └── phase3a_server.rs                   # End-to-end Phase 3A `:7430` server, active-only cards, sandbox & Vault Sync test suite
```

---

## 5. Toolchain, Configuration & Daily Build / Test Commands

### 5.1 Host Toolchain Prerequisites on DGX Spark
Always prepend `~/.cargo/bin` to `PATH` in non-interactive shells:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```
- **Rust Compiler:** `rustc 1.98+` (`Edition 2024`)
- **System Build Tools (used once by `memory_core_rs` to compile SQLite `3.51.3`, SQLCipher `4.14.0`, `sqlite-vec` `0.1.9`, and vendored OpenSSL):** `/usr/bin/cc` and `/usr/bin/perl`
- **Private Git Dependencies:** `.cargo/config.toml` sets `git-fetch-with-cli = true` so Cargo uses SSH keys for `raphs-app/raphs` and `raphsaeed/carbon`.

### 5.2 Configuration Precedence (`OS Env` → `.env` → `hydrogen.toml` → Defaults)

| Variable (`.env` / OS Env) | `hydrogen.toml` Key | Default / Example | Purpose |
| :--- | :--- | :--- | :--- |
| `HYDROGEN_LESSON_KEY` | *(secret — `.env` only)* | *(required)* | Passphrase unlocking `<stem>.sqlite` / `<stem>.raphs` |
| `HYDROGEN_BIND_ADDR` | `bind_addr` | `127.0.0.1:7430` | Default bind address for `hydrogen serve` (`--lan` overrides IP to `0.0.0.0`; `--port` / `--bind` override at CLI) |
| `CARBON_WS_URL` | `carbon_ws_url` | `ws://127.0.0.1:7420/carbon` | Default Carbon WebSocket endpoint (`ws://127.0.0.1:7420/carbon` for DGX headless or `ws://192.168.68.64:7420/carbon` for MacBook) |
| `HYDROGEN_DEVICE_ID` | `device_id` | `hydrogen-dgx-spark` | HLC device attribution stamped in `.raphs` `provenance` |
| `HYDROGEN_DEFAULT_PRODUCT` | `default_product` | `gyaraplus` | Default product directory under `products/` & `rules/` |
| `HYDROGEN_DEFAULT_SUBJECT` | `default_subject` | `maths` | Default subject slug (`gyaraplus` + `maths` → `gyara-maths.raphs`) |

### 5.3 Daily Build, Zero-Warning Lint & Test Suite

```bash
export PATH="$HOME/.cargo/bin:$PATH"

# 1. Zero-warning Clippy lint across lib, bin, unit tests & integration tests (MANDATORY before every commit)
cargo clippy --all-targets -- -D warnings

# 2. Full unit + end-to-end integration test suite
cargo test

# 3. Optimized release binary (`target/release/hydrogen`)
cargo build --release
```

---

## 6. Complete `hydrogen` CLI Operations Playbook

All commands assume `cd /home/raphspark/projects/hydrogen && export PATH="$HOME/.cargo/bin:$PATH"` (use `cargo run --quiet -- <subcmd>` during development or `./target/release/hydrogen <subcmd>` once built).

### 6.1 System Health, Product Ledger Status & Vault Initialization
```bash
# Verify 4-tier rules, in-process Theta (< 500 us), Vault Schema 2.3.0 + PNG logos, and Carbon WS
cargo run --quiet -- doctor
cargo run --quiet -- doctor --carbon ws://127.0.0.1:7420/carbon
cargo run --quiet -- doctor --carbon ws://192.168.68.64:7420/carbon

# Initialize / idempotently seed `gyara-maths.sqlite` (+ optional `--pack` into `gyara-maths.raphs`)
cargo run --quiet -- init-vault --product gyaraplus --subject maths --pack

# Inspect active sub-product inbox queue, paper/topic progress & open comments (+ emit `products/gyaraplus/maths/status.html`)
cargo run --quiet -- status --product gyaraplus --set-expected GYR-MTH-P001 50 --html

# Launch Hydrogen Dashboard on `:7430` (`--lan` exposes `http://192.168.68.52:7430/` & `.85:7430/`; stop with Ctrl+C or `pkill -INT -f "hydrogen serve"`)
cargo run --quiet -- serve --lan --port 7430
cargo run --quiet -- serve --bind 127.0.0.1:7430
```

### 6.2 Clean-Room PDF / Image Slicing (`inbox/` → `archive/` + `ledger.json`)
```bash
# 1. Render PDF pages to 180 DPI PNGs in `products/gyaraplus/maths/archive/GYR-MTH-P001/pages/` for inspection
cargo run --quiet -- slice-paper products/gyaraplus/maths/inbox/Practice-Paper-01.pdf \
  --paper-id GYR-MTH-P001 --dpi 180 --pages 1-6 --pages-only

# 2. Slice a single question crop by normalized [0..1] bbox (`x,y,w,h`) into `archive/.../crops/` + `ledger.json`
cargo run --quiet -- slice-paper products/gyaraplus/maths/inbox/Practice-Paper-01.pdf \
  --paper-id GYR-MTH-P001 --id GYR-MTH-P001-Q001 --page 2 --bbox 0.06,0.05,0.88,0.22

# 3. Slice a batch of crops from a JSON manifest (`Vec<CropSpec>`)
cargo run --quiet -- slice-paper products/gyaraplus/maths/inbox/Practice-Paper-01.pdf \
  --paper-id GYR-MTH-P001 --crops-json /tmp/p001-crops.json
```

### 6.3 Deterministic `Theta` (`theta_calc`) CLI Queries
```bash
# Quick expression evaluation (uses stage preset: mixed fractions + degrees + \htmlId tokens)
cargo run --quiet -- theta "2 3/4 + 1 1/2"

# Full JSON tool calls (`calc.eval`, `calc.solve`, `calc.formula`, `calc.transform`, `calc.stats`, `calc.analyze_graph`)
cargo run --quiet -- theta calc.solve '{"equations":["2*(N - 3) = N"],"solve_for":["N"],"trace":true}'
cargo run --quiet -- theta calc.transform '{"op":"mixed_fraction","expr":"39/20"}'
cargo run --quiet -- theta calc.analyze_graph '{"expr":"x^2 - 4*x + 3","x_min":-2,"x_max":6}'
```

### 6.4 Quality Gate, Headless Carbon Capture, Media Assets & Vault Upsert
```bash
# Run deterministic Theta + Carbon + Warm-Paper + 5-Voice + KaTeX + Clean-Room gates (`< 1 ms`)
cargo run --quiet -- verify-question products/gyaraplus/maths/archive/GYR-MTH-P001/GYR-MTH-P001-Q015.json

# (Phase 3.3) Render onto headless Carbon (:7420), check object/token AABB bounds, and save PNG snapshot
cargo run --quiet -- capture-carbon GYR-MTH-P001-Q015 --part a --step S01 --level 9 \
  --check-bounds --out /tmp/q15-s1-l9.png --carbon ws://127.0.0.1:7420/carbon

# (Phase 3.3) Embed clean regenerated PNG/SVG/MP4/audio asset into `.raphs` `course_media_assets` + `blobs`
cargo run --quiet -- put-asset /tmp/clean-photo.png --asset-id GYR-MTH-P001-Q018-IMG01 --alt "Wooden prism blocks"

# Verify and atomically upsert a single question or whole paper into `gyara-maths.sqlite` (+ `--pack`)
cargo run --quiet -- upsert-question products/gyaraplus/maths/archive/GYR-MTH-P001/GYR-MTH-P001-Q001.json --pack
cargo run --quiet -- upsert-paper products/gyaraplus/maths/archive/GYR-MTH-P001/paper.json --pack

# Inspect L7 catalog snapshot (or Carbon `chrome.setCatalog` Full-Stage JSON)
cargo run --quiet -- catalog
cargo run --quiet -- catalog --carbon-json

# Pack author `.raphs` bundle + export commercial clean customer bundle (strips comments, revisions & provenance)
cargo run --quiet -- pack \
  --clean-out /tmp/gyara-maths-release.raphs \
  --customer-key "customer-release-key-2026"
```

### 6.5 Live Two-Way Carbon `--watch` Session & `Snap Board` Comments CLI
```bash
# Push a specific Question + (Part, Step, Sub-Level) onto Carbon
cargo run --quiet -- preview-carbon GYR-MTH-P001-Q015 --part a --step S01 --level 9 \
  --carbon ws://192.168.68.64:7420/carbon

# Launch interactive two-way `--watch` session (streams Gallery/Steps/L9..L1 AND saves `Snap Board` comments into `.sqlite` + `ledger.json`!)
cargo run --quiet -- preview-carbon --catalog --watch --carbon ws://192.168.68.64:7420/carbon

# Inspect, export attachments/source crops, resolve, or delete `Snap Board` comments from CLI
cargo run --quiet -- comments list [--status open|resolved|all] [--target GYR-MTH-P001-Q015]
cargo run --quiet -- comments show <comment-id> --export-dir /tmp/comment-review
cargo run --quiet -- comments resolve <comment-id> --note "Adjusted barModel label spacing in S02:L03"
cargo run --quiet -- comments delete <comment-id>
```

---

## 7. Authoring & Verification Quick-Reference Cheat-Sheet

### 7.1 Universal Hierarchical ID Grammar (`1..=128` ASCII chars)
- **Course:** `GYR-MTH`, `GYR-ENG`, `GYR-VR`, `GYR-NVR`, `GCSE-MTH`, `GCSE-PHY`, `ALV-MTH`, `PRI-MTH`, `INF-MTH`
- **Paper & Replicas:** `GYR-MTH-P001`, whole-paper replica `GYR-MTH-P001_R01`
- **Paper Question & Item Replica:** `GYR-MTH-P001-Q014`, `GYR-MTH-P001-Q014-R01`
- **Section & Stimulus:** `GYR-MTH-P001-SEC01`, `GYR-MTH-P001-STIM01`
- **Book & Chapter:** `GYR-MTH-B01`, `GYR-MTH-B01-CH03`, `GYR-MTH-B01-CH03-Q008`
- **Topic, Card & Question Type:** `GYR-MTH-T_ALG`, `GYR-MTH-T_RAT01-C01`, `GYR-MTH-QT-ALG-WORD`
- **Fine-Grained Sub-Address:** `GYR-MTH-P001-Q014:a:S01:L09` *(used for targeting; never stored on `question.questionId` itself)*

### 7.2 Carbon's 5 Curated Font Voices (`PennstanderMath` Everywhere)
| Voice (`scene.write.style`) | Font Face + `$...$` Math | Where to Use | Typical Scale & Tilt |
| :--- | :--- | :--- | :--- |
| **`"handwritten"`** *(default)* | `Pennstander` + `PennstanderMath` | Right-Pane worked steps (`zone: "spine"`) & Left-Pane `E.g.` examples | `scale: 1.0` (`21 pt` Right), `0°` |
| **`"typeset"`** | `Lato` + `PennstanderMath` | Left-Pane question stems (`#0F172A`) & `"NEVER EVER"` rule cards (`#FDF2F8` / `#9F1239`) | `scale: 1.0` (`16.8 pt` Left), `0°` |
| **`"casual"`** | `Single Day` + `PennstanderMath` | `substeps[]` breakdown cards (`#EFF6FF` / `#3B82F6`) & yellow `RE-CHECK` sticky notes (`#FEF9C3` / dashed `#CA8A04`) | `scale: 0.88..0.95`, `rotationDeg: -2.0..+1.5` |
| **`"marker"`** | `Patrick Hand` + `PennstanderMath` | Bold classroom step headlines & key takeaways (`#1D4ED8`, `#0D9488`, `#15803D`, `#B45309`) | `scale: 0.95..1.10`, `rotationDeg: -1.5..+1.5` |
| **`"typewriter"`** | `Special Elite` + `PennstanderMath` | Solid corner stamps (`NON-CALC`, `EXAMPLE`, `KEY TRAP` — `#FFFFFF` on `#1565C0` or `#991B1B`) | `sizePt: 14..16`, `rotationDeg: -3.5` or `0°` |

---

## 8. Hard-Won Tricks, Techniques & Pitfalls

### 8.1 Zero `unwrap()` / `expect()` / `panic!()` — Including Inside Tests!
- `Cargo.toml` sets `unwrap_used = "deny"`, `expect_used = "deny"`, and `panic = "deny"`, and `cargo clippy --all-targets -- -D warnings` lints unit tests and `tests/*.rs` with the exact same strictness.
- **Technique:** Write every `#[test]` and `#[tokio::test]` signature as `fn my_test() -> Result<()>` and use `?` or `.ok_or_else(|| HydrogenError::Validation("...".into()))?` instead of `.unwrap()` / `.expect()`.

### 8.2 Rust Raw Strings Containing `#RRGGBB` Hex Colors
- Embedding JSON or HTML/CSS with `"#1565C0"` inside single-hash `r#"..."#` prematurely closes the raw string literal at `"#`!
- **Technique:** Always use double-hash raw strings `r##"{ "color": "#1565C0" }"##` in tests and HTML templates.

### 8.3 Execution-Order `BoardSymbolTable` Resolution in `gate.rs`
- `gate.rs` simulates Carbon's scene symbol table in strict execution order:
  1. `question.leftPaneCarbonCommands`
  2. Preceding steps' **`Level 9`** commands (`steps[0..i]` @ `L9`)
  3. Current step's sublevel commands (`L9..min_sublevel`, each validated independently atop prior steps' `L9` state)
  4. `step.substeps[]` (validated atop current step's `L9` state) and `finalAnswer.carbonCommands`
- **Pitfall to Avoid:** Lower sub-levels (`L8..L1`) of Step 2 may anchor `below` an object placed in Step 1's `Level 9`, **never** an object that existed only in Step 1's `Level 8`! Similarly, `speechTokens` (`"obj_id:tok_id"`) and `focus.annotate` `targetIds` must reference an object or `\htmlId{tok_id}{...}` already registered earlier in that sequence.

### 8.4 `focus.annotate` Checkmarks, `"NEVER EVER"` Cards & Prose Word Caps
- **No Duplicate Checkmarks:** Never include `✓`, `✔`, `✅`, or `\checkmark` inside `focus.annotate.label` when `"kind": "check"` — Carbon draws a crisp corner tick automatically (`gate.rs` rejects literal ticks).
- **`"NEVER EVER"` Rule Cards:** Whenever `scene.write.content` contains `"NEVER EVER"`, `style` **must** be `"typeset"` (`Lato` clean wrap), never `"handwritten"` or `"casual"`.
- **Anti-Prose-Wall Caps:** Right-Pane `scene.write` blocks are capped at `45` prose words outside `$...$` (`18` words on `focus.annotate` labels). Put full conversational tutor explanations in `narration` (for SIGMA TTS), keeping the Carbon board visual and spacious.
- **Anti-Repetition Across Levels `9 → 1`:** Adjacent sub-levels (`N` and `N-1`) must never have identical `carbonCommands` or identical `narration`.

### 8.5 `Theta` Mixed Fractions, MCQ Choice Extraction & Tool Aliases
- **Mixed Fractions:** In `gyaraplus/maths`, `allow_mixed_fractions: true` is active. `ThetaService` normalizes plain-ASCII mixed numbers (`"2 3/4"` → `"(2 + 3/4)"`) automatically when no LaTeX `\` is present, and natively parses LaTeX `2\frac{3}{4}` or `mixed(2, 3, 4)`.
- **MCQ Choice Values with Currency / Units:** `gate.rs` strips `$...$`, `\pounds`, `£`, `\mathbf{...}`, and `\text{...}` from `choices[].label` before verifying 5-way distinctness in `Theta`. When an MCQ option label contains richer prose (e.g. `"$2\\text{ hours } 15\\text{ mins}$"`), populate `answerKey.choiceValues` (`{"A": "135", ...}`) so `Theta` verifies exact mathematical equivalence unambiguously.
- **Tool Name Aliases:** `theta_calc::dispatch_tool_json` accepts both dotted names (`calc.eval`, `calc.solve`, ...) and underscore aliases (`calc_eval`, `calc_solve`, ...).

### 8.6 `memory.raphs` Conformance Rule A6 (Single `Memory` Handle)
- Never open two `Memory` connections to the same `<stem>.sqlite` file simultaneously in one process. Always share `&VaultManager` (or `Arc<VaultManager>`), which serializes SQLCipher transactions safely behind a single `tokio::sync::Mutex<Memory>`.

---

## 9. Cold-Boot Restart Checklist

Run these 6 commands on DGX Spark whenever resuming work after a break or reboot:

```bash
cd /home/raphspark/projects/hydrogen
export PATH="$HOME/.cargo/bin:$PATH"

# 1. Check clean repository state
git status -sb

# 2. Start prebuilt headless Carbon in background (if port 7420 is not already listening)
curl -s http://127.0.0.1:7420/health || \
  /home/raphspark/projects/carbon/target/release/carbon-shell --headless --lan --split --port 7420 >/tmp/carbon-headless.log 2>&1 &

# 3. Run full zero-warning Clippy & test suite
cargo clippy --all-targets -- -D warnings && cargo test

# 4. Run end-to-end Hydrogen Doctor check (Rules + Theta + L7 Vault + Carbon WS)
cargo run --quiet -- doctor --carbon ws://127.0.0.1:7420/carbon

# 5. Verify reference compiled blueprints in `archive/`
cargo run --quiet -- verify-question products/gyaraplus/maths/archive/GYR-MTH-P001/GYR-MTH-P001-Q001.json
cargo run --quiet -- verify-question products/gyaraplus/maths/archive/GYR-MTH-P001/GYR-MTH-P001-Q015.json

# 6. Optional: Launch Hydrogen Dashboard (:7430 -> http://192.168.68.52:7430/) & interactive Carbon `--watch` (:7420 -> http://192.168.68.52:7420/)
./target/release/hydrogen serve --lan --port 7430 >/tmp/hydrogen-serve.log 2>&1 &
cargo run --quiet -- preview-carbon --catalog --watch --carbon ws://127.0.0.1:7420/carbon

# 7. When finished for the day, stop background servers cleanly
pkill -INT -f "hydrogen serve"
pkill -f "carbon-shell --headless"
```
