# Hydrogen — Product & System Requirements Specification (`requirements.md`)

> **Document Status:** Locked Pre-Build Baseline (Aligned with `memory.raphs` Schema `2.3.0` / `memory_core_rs 1.7.0` and `Carbon Protocol v0.1` Book Index & Rich Feedback)  
> **Scope:** **Hydrogen** — Multi-Product AI Solution Compiler, Replica Generator, Product Authoring Ledger & `L7` `.raphs` Vault Manager (`Pure Rust CLI + Antigravity + Static HTML`).  
> **Companion Documents:**
> - Standard Operating Procedure: [`sop.md`](./sop.md)
> - Ecosystem Diagram: [`gyara-workflow.png`](./gyara-workflow.png)
> - Overall System Architecture: [`system-architecture.md`](./system-architecture.md)
> - 9-Level Visual & Pedagogical Rules: [`solution-levels.md`](./solution-levels.md)
> - Phased Execution Checklist: [`build-plan.md`](./build-plan.md)
> - Sibling Specifications: [`theta-integration.md`](./theta-integration.md), [`carbon-integration.md`](./carbon-integration.md), [`memory-raphs.md`](./memory-raphs.md)

---

## 1. Purpose & Product Vision

**Hydrogen** is a high-precision AI curriculum compiler and solution authoring system. It ingests raw question papers, books/chapters, topic sheets, worked examples, and question images, and compiles **complete, mathematically verified, visual-first solutions** into a single encrypted `.raphs` course vault per subject.

Every step of every solution is decomposed into **up to 9 calibrated sub-levels of explanation** (`Level 9` = expected exam standard for that question $\to$ `Level 1` = foundational / infant-school intuition), pre-compiled with exact calculations (via **Theta**) and exact 2D/3D whiteboard visuals (for **Carbon**), so that a lightweight on-device voice/board tutor (**SIGMA**) can teach adaptively without ever hallucinating mathematics or geometry.

### 1.1 Multi-Product & Multi-Subject Roadmap
Hydrogen is architected as a reusable curriculum factory across multiple educational product lines and subjects:

1. **Product Line 1 — `gyara` (11+ / Gyaraplus Exam Preparation — *Immediate Launch Focus*):**
   Built first for daily live 11+ tutoring (teaching the founder's son) and packaged as a commercial product across **4 subject vaults**:
   - `gyara-maths.raphs` *(Phase 1 Primary Focus)*
   - `gyara-english.raphs`
   - `gyara-verbal-reasoning.raphs`
   - `gyara-non-verbal-reasoning.raphs`
2. **Subsequent Product Lines (`gcse`, `primary`, `infant`, `alevel`, etc.):**
   - `gcse-maths.raphs`, `gcse-physics.raphs`, and future STEM/core subjects.
   - While **Mathematics and Physics** are the immediate priority, Hydrogen's workspace layout, `memory.raphs` `L7` schema, and compilation pipeline support Infants through A-Level as well as English, Verbal Reasoning (VR), and Non-Verbal Reasoning (NVR).

---

## 2. Ecosystem Components & Strict Responsibility Boundaries

| Component | Role in Ecosystem | Boundary with Hydrogen |
| :--- | :--- | :--- |
| **`papers`** | Raw input sources: PDF exam papers, books/chapters, topic worksheets, worked examples, tests, and image snapshots. | **Input only.** Raw files, scans, cropped snapshots, and **any name/reference to the original source paper/publisher** must **never** appear in the output `.raphs` vault. Only Hydrogen stores a private cross-reference (`private-xref.json`) for verification. |
| **`hydrogen`** *(This Project)* | Heavy AI Solution Compiler Toolkit, Product Ledger, Replica Verifier & Vault Manager (`Pure Rust CLI + Antigravity + status.html`). | **Owns all heavy intelligence & authoring:** PDF/image extraction, product ledger (`ledger.json` + `status.html`), visual/photo regeneration, Theta verification, `9 → 1` Carbon scene & headless visual QA, two-way Carbon `--watch` comment sync, replica/topic workflows, and `.raphs` packaging. |
| **`<product>-<subject>.raphs`** | Single-file encrypted SQLite course vault (e.g., `gyara-maths.raphs`) on `memory.raphs` **Schema `2.3.0` (`L7 Course Layer`)**. | **Hydrogen's primary output & authoring feedback store.** Indexes Books, Chapters, Papers, Sections, Topics, Question Types, Shared Stimuli, Questions, `9 → 1` Solutions, Brand Logo & Media Blobs, and Author Comments; unlocked via **one simple Lesson Key**. **Contains zero student statistics.** |
| **`theta`** (`theta_calc`) | Pure-Rust deterministic symbolic/numerical CAS, unit solver, 313-formula registry & 120 FPS Graph VM. | Linked directly into Hydrogen (`theta_calc`) at **compile time** to verify every step, fraction, root, graph landmark, and MCQ distractor; also used by SIGMA & Carbon at runtime. |
| **`carbon`** (`carbon-shell`) | Standalone 120Hz 2-way interactive GPU whiteboard (`Carbon Protocol v0.1`, port `7420`). | Renders the Full-Stage Dark-Slate Course & Book Index Gallery (`displayMode: "full-stage"`, 3-level drill-down, centered `brandLogo`, `△` important & reminder/AI chips) + Slide-Over Left Band, Split Question \| Working panes, 22 educational 2D macros + 3D stage, and the Multi-Comment Drawer with 1-Click **`Snap Board`** PNG attachments (`authorMode: true` in Hydrogen). |
| **`sigma`** | Small on-device AI tutor with ASR (listen) + TTS (speak) driving lessons for the student. | **Intentionally thin consumer (out of scope to build here).** Reads `<product>-<subject>.raphs` (including embedded `brandLogo` blob) + personal `<student>.raphs` (for learner stats, reminders, due status & important flags), narrates/lightly adapts phrasing via TTS, dispatches Hydrogen's pre-built Carbon steps, and navigates `Level 9 ↔ Level 1` or linked Topic Cards. |

---

## 3. Core Functional Requirements

### FR-1: Product & Subject Workspace Directory Structure (Git-Tracked Assets vs. Git-Ignored Inbox)
Hydrogen organizes product brand assets/documents, raw inputs, archived sources, private cross-references, and compiled `.raphs` vaults in a standardized per-product directory layout. **`products/` itself is tracked in Git** (so product-specific logos, images, and docs live cleanly in version control), while `inbox/`, `archive/`, `private-xref.json`, and `.raphs`/`.sqlite` binaries are Git-ignored:

```text
hydrogen/
├── products/                                        # Tracked in Git (assets, logos, product docs)
│   ├── gyaraplus/                                   # 11+ (Gyara+) Product Line
│   │   ├── assets/                                  # GIT-TRACKED brand logos & product visuals
│   │   │   ├── gyara-white.png                      # Primary dark-slate & subject-card logo (font-independent)
│   │   │   ├── gyara-blue.png                       # Light-background logo variant (font-independent)
│   │   │   ├── gyara-white.svg                      # Vector source asset
│   │   │   └── gyara-blue.svg                       # Vector source asset
│   │   ├── inbox/                                   # GIT-IGNORED: Drop raw PDF papers / chapters / images here
│   │   │   └── maths/
│   │   ├── archive/                                 # GIT-IGNORED: Processed raw source files & crops (NEVER in .raphs)
│   │   │   └── maths/
│   │   ├── private-xref.json                        # GIT-IGNORED: Hydrogen-only map (GYR-MTH-P001 <-> source PDF/page/bbox)
│   │   ├── ledger.json                              # GIT-IGNORED: Product-wide SHA-256 inbox inventory, paper/topic progress & task queue
│   │   ├── status.html                              # GIT-IGNORED: Self-contained Warm-Paper visual status & side-by-side crop dashboard
│   │   ├── gyara-maths.raphs                        # GIT-IGNORED: Live L7 course vault (SQLCipher)
│   │   ├── gyara-english.raphs
│   │   ├── gyara-verbal-reasoning.raphs
│   │   └── gyara-non-verbal-reasoning.raphs
│   └── gcse/                                        # Future GCSE Product Line
│       ├── assets/
│       ├── maths/
│       │   └── gcse-maths.raphs
│       └── physics/
│           └── gcse-physics.raphs
```

---

### FR-2: Universal Hierarchical ID System & Complete Source Anonymity
1. **Universal Identification:** Every book, chapter, paper, section, topic, topic card, question type, question, replica, part, step, and sub-level uses a deterministic, human-readable Universal ID (`1..=128` chars, native SQL `PRIMARY KEY` / `UNIQUE` key in `L7`):
   - **Format:** `<PRODUCT>-<SUBJECT>-<CONTAINER_ID>-<ITEM_ID>[-<REPLICA_ID>][:<PART>:S<STEP>:L<LEVEL>]`
   - **Product Prefixes:** `GYR` (11+ Gyara), `GCSE` (GCSE), `PRI` (Primary), `INF` (Infant), `ALV` (A-Level)
   - **Subject Prefixes:** `MTH` (Maths), `ENG` (English), `VR` (Verbal Reasoning), `NVR` (Non-Verbal Reasoning), `PHY` (Physics)
   - **Examples:**
     - `GYR-MTH-P001-Q014` *(Gyara Maths, Practice Paper 001, Question 14)*
     - `GYR-MTH-P001-Q014-R01` *(Question Replica 1 of Q14)*
     - `GYR-MTH-P001_R01-Q014` *(Question 14 inside Whole-Paper Replica 1 of Paper 001)*
     - `GYR-MTH-B01-CH03-Q008` *(Book 01, Chapter 03, Question 08)*
     - `GYR-MTH-T_RAT01-C01` *(Ratio Topic 01, Concept Card 01)*
2. **Zero Source Paper Leakage:**
   - The compiled `.raphs` file must contain **zero binary scans/crops** from the original paper and **zero textual/metadata references** to the external school, exam board, filename, or source paper title.
   - Only Hydrogen's local `private-xref.json` retains the mapping from `GYR-MTH-P001-Q014` back to the archived original PDF page and bounding box for author cross-checking.

---

### FR-3: Question Fidelity, Answer Formats (`MCQ` Default) & Clean Media Regeneration
1. **High-Fidelity Question Extraction:**
   - Preserve exact mathematical difficulty, question logic, marks (`REAL`, supporting fractional or integer marks), timed paper sections (`course_paper_sections`), shared passages/datasets (`course_stimuli`), and multi-part trees (`(a)`, `(b)`, ...).
2. **Answer Formats (`mcq` Default for Gyara + Full Infant-to-A-Level Support):**
   - Explicitly store `answer_format` per question (`'mcq'`, `'written'`, `'multi_select'`, `'interactive'`, `'spoken'`, `'drawing'`).
   - **Default for Gyara (11+):** Default to **5-Option Multiple-Choice (`A–E`)** matching real-world 11+ exams, while supporting `'written'` where needed.
   - When generating MCQ options (for converted questions or replicas), compute **realistic diagnostic 11+ trap distractors** verified via **Theta** so all 5 choices are distinct and **exactly one** is mathematically valid.
3. **Clean Regeneration of Diagrams, Graphs, Tables, Photos, Audio & Video:**
   - **Graphs, Geometry, Charts & Tables:** Regenerate cleanly using native **Carbon** primitives/macros (`coordinatePlane`, 2D geometry, `smartTable`, `barChart`, `pieChart`, `venn`, `clock`/`mirrorClock`, `barModel`, `cubeAssembly`, `netFold` with NVR markings) or crisp vector SVG.
   - **Real Photographs & Media Assets:** Recreate photographs as clean, high-quality generated images (via Gemini image generation) and store all binary media (images, SVGs, Infant audio, Physics videos) inside deduplicated `blobs` via `course_media_assets` + `course_item_assets`.

---

### FR-4: Question-Relative 9-Level Step Solutions & Heavy Pre-Compilation (`solution-levels.md`)
1. **Question-Relative `9 → 1` Scale:**
   - **`Level 9`** is **always** what is expected at exam standard for *that specific question's step*.
   - **`Levels 8 → 1`** progressively unpack that step down to foundational/infant-school intuition, obeying Rule 0 (Visual-First, zero prose walls) and the Anti-Repetition Law across every drop.
2. **Invisible Safety Net & Completeness Bias:**
   - Sub-level numbers are invisible to the student and driven only by SIGMA. Hydrogen errs toward generating **more** sub-levels (`9` down to `1` whenever distinct visual scaffolds exist) rather than fewer so SIGMA never gets stuck improvising.
3. **Heavy Hydrogen Pre-Compilation:**
   - Every `(step, sub-level)` inside `course_questions.solution_json` stores pre-verified KaTeX (`$...$`), `\htmlId` token anchors, complete Carbon `scene.*` / `focus.*` / `edit.rub` commands & coordinates, `Check ✔` lines, and a plain-English `narration` script for SIGMA.

---

### FR-5: Deterministic Verification via `Theta`
1. Hydrogen links `theta_calc` natively in Rust and verifies **100% of mathematical & physics expressions** before any item enters `gyara-maths.raphs`:
   - Stage-matched presets (`"uk_gcse"`, `allow_mixed_fractions: true`, `angle_mode: "deg"` for 11+ Maths).
   - Zero intermediate rounding drift across steps and multi-part questions (`exportedExact`).
   - Automated check of every step transition, `Check ✔` footer, curve landmark (`calc.analyze_graph`), stats working column (`calc.stats`), and MCQ distractor uniqueness.

---

### FR-6: Native SQLite Course Index (`memory.raphs` Schema `2.3.0` Layer `L7`), Brand Logo Blobs & Zero Student Stats
1. **Native SQLite Course Index (No External Index JSON):**
   - All course hierarchy and Unique IDs live directly inside SQLite `L7` tables in `<product>-<subject>.raphs` (`memory_core_rs 1.7.0`): `course_meta`, `course_books`, `course_chapters`, `course_papers`, `course_paper_sections`, `course_topics`, `course_topic_cards`, `course_question_types`, `course_stimuli`, `course_questions`, `course_question_revisions`, `course_question_topics`, `course_concept_links`, `course_media_assets`, `course_item_assets`, plus `FTS5` & `vec0` search tables.
   - `Memory::course_catalog_snapshot()` projects the full catalog (`< 1 ms`) to power Carbon's **Full-Stage Dark-Slate Gallery** (`displayMode: "full-stage"`) & Slide-Over Index (`chrome.setCatalog`) as well as Hydrogen's Web UI.
2. **Embedded Product Brand Logo Blobs & Subject Palette (`course_media_assets` + `course_meta`):**
   - Hydrogen embeds the product's brand logos from `products/<product>/assets/` directly into `blobs` via `course_media_assets` (for Gyara+: `GYR-LOGO-WHITE` from `gyara-white.png` for `#151A24` dark-slate headers & jewel-toned subject cards, and `GYR-LOGO-BLUE` from `gyara-blue.png` for light surfaces) and records `{ "branding": { "logoWhiteAssetId": "GYR-LOGO-WHITE", "logoLightAssetId": "GYR-LOGO-BLUE", "subjectColor": "#2563EB", "importantIcon": "triangle" } }` inside `course_meta.config_json`.
   - At runtime, SIGMA (and Hydrogen in Author Preview) reads the logo blob from `<product>-<subject>.raphs` and passes it as `brandLogo` / subject `logo` (`data:image/png;base64,...`) in `chrome.setCatalog`, paired with each subject vault's jewel accent (`#2563EB` Maths, `#0D9488` English, `#7C3AED` Verbal Reasoning, `#DB2777` Non-Verbal Reasoning) and `"AI"` violet `tagBadges` on `is_replica = true` papers/questions.
3. **Strict Separation of Learner Statistics:**
   - **Zero learner/classroom statistics** (`completed`, `in-progress`, `attempts`, scores, `important` `△` flags, `reminder: "15d"`, or crimson `dueReminder: true` states) are stored inside `<product>-<subject>.raphs`. Learner progress and spaced-repetition reminders live in each learner's personal `<student>.raphs` vault and are merged at runtime by SIGMA before calling `chrome.setCatalog`.
4. **Single Simple "Lesson Key" & Clean Customer Export:**
   - Course vaults unlock via **one simple Lesson Key** (`Memory::open_with_passphrase`).
   - Publishing a commercial copy calls `Memory::export_clean_course_bundle(out_path, lesson_key)`, which preserves `course_media_assets` (keeping the brand logos intact!) while stripping `course_comments`, `course_comment_attachments`, `course_question_revisions`, unreferenced comment blobs, and authoring `provenance` rows under a fresh Lesson Key.

---

### FR-7: Topics, Books/Chapters & Organic Concept Growth
1. Phase 1 focuses on ingesting **Question Papers** into `gyara-maths.raphs`, tagging every question with its Topic(s) (`course_question_topics`), Question Type (`course_question_types`), and Difficulty (`1..5`), while keeping `course_books`, `course_chapters`, and `course_topic_cards` ready in the database from Day 1.
2. At any time (via Carbon comment or Hydrogen Web UI), the author can generate and link descriptive **Topic / Concept / Worked Example / Prerequisite Cards** (`course_topic_cards` + `course_concept_links`) so any question needing that concept can detour (`flow.detourPush`) to it.

---

### FR-8: Two-Phase Ingestion & On-Demand Replica Generation
1. **Stage 1 — Clean Ingestion First:** Accurately extract an input paper/chapter and compile optimal `9 → 1` solutions without auto-generating unrequested variants.
2. **Stage 2 — On-Demand Replicas:** Triggered by **Paper ID** or **Question ID** (from Hydrogen's Web UI, Carbon's `authorMode` card action `catalog.actionRequested`, or a `"replica"` comment):
   - Generate $N$ replicas of a single question or an entire paper at **Same** or **Higher / Lower** difficulty (defaulting to `MCQ` `A–E`), verified by Theta, with `is_replica = true` and `parent_question_id` / `parent_paper_id` indexed in SQLite.

---

### FR-9: Live Teaching Feedback Loop (Carbon Comments + `"Snap Board"` $\leftrightarrow$ Hydrogen)
1. In Carbon (`chrome.update` with `authorMode: true` for owner sessions via `hydrogen preview-carbon --watch`, `false` for students), comments + 1-click **`Snap Board`** PNG screenshots / uploaded images emit `comment.saved` (`FeedbackEntry`) and are persisted straight into `course_comments` + `course_comment_attachments` inside `gyara-maths.raphs` (while `"create-replica"` card actions record tasks in `ledger.json`).
2. Hydrogen + Antigravity process open comments via `hydrogen comments`:
   - Cross-checks `private-xref.json` (original PDF crop) + attached board screenshot,
   - Updates the target question/step/sub-level (with prior version auto-saved in `course_question_revisions`) or creates requested replicas/topics,
   - Calls `resolve_course_comment(...)`, and extracts `"engine"` notes (`@theta` / `@carbon`) into an ecosystem developer backlog.

---

### FR-10: Product Authoring Ledger (`ledger.json`) & Static Warm-Paper `status.html` Dashboard
Because `<product>-<subject>.raphs` is Clean-Room anonymized and scoped per subject, Hydrogen maintains a lightweight Git-ignored product ledger paired with a zero-server static HTML report (`hydrogen status --html`):
1. **Zero-Drift Product Ledger (`products/<product>/ledger.json`):**
   - Scans `inbox/<subject>/` and `archive/<subject>/` with SHA-256 deduplication so duplicate PDFs are detected automatically even if renamed.
   - Tracks expected question counts per paper (`GYR-MTH-P001`: `50` items), per-question pipeline stage (`cropped` / `verified`), and pending replica/topic tasks without duplicating compiled `.raphs` rows.
2. **Self-Contained Warm-Paper `status.html` (`products/<product>/status.html`):**
   - Merges live `CourseCatalogSnapshot`s across all subject `.sqlite` vaults with `ledger.json` and `private-xref.json`, rendering Subject summary cards, Paper progress bars, Topic coverage tables, side-by-side `archive/` crop vs. compiled stem inspection, and open `Snap Board` comment thumbnails—zero local web server required, while **Carbon (`:7420`)** provides the live 120Hz interactive gallery and `Level 9 → 1` stage.

---

## 4. Technical Stack & Execution Strategy

- **Stack:** **100% Pure Rust** (`hydrogen` CLI linking `theta_calc`, `memory_core_rs 1.7.0`, `carbon-protocol`, and `hayro` + `png` natively in-process) + **Self-Contained Static `status.html`**.
- **Primary Authoring Conductor:** **Antigravity** (interactive & background SSH sessions + parallel subagents) driving `hydrogen` CLI (`status`, `slice-paper`, `theta`, `verify-question`, `capture-carbon`, `put-asset`, `upsert-question`, `comments`, `preview-carbon --watch`, `pack`).
- **Modular Future API Extension (Phase 8):** Pluggable Rust traits (`src/ai/`: `ReasoningModel`, `ImageModel`, `VideoModel`) for optional external API batch automation (`gemini-4-carbon`, `gemini-nano-banana-2.1`, `gemini-omni-1.2-flash`) without coupling vendor SDKs into Hydrogen's core.

---

## 5. Summary Decision Log

| # | Area | Locked Decision |
| :---: | :--- | :--- |
| **1** | **Products & Brand Assets** | `products/` is **tracked in Git** (storing brand logos & product docs like `products/gyaraplus/assets/gyara-{white,blue}.{png,svg}`, embedded as PNG `blobs` in `.raphs`), while `inbox/`, `archive/`, `private-xref.json`, `ledger.json`, `status.html`, and `.raphs` vaults are Git-ignored. |
| **2** | **9-Level Scale** | **Question-relative (`9 → 1`):** `L9` = expected exam working; `L8..L1` = progressive visual/infant scaffolding; invisible to learner; err toward **more** levels. |
| **3** | **Hydrogen vs. SIGMA** | **Heavy Hydrogen + Theta, Thin SIGMA:** Hydrogen pre-compiles all math, 2D/3D Carbon coordinates/macros, brand logos, and narration scripts; SIGMA only narrates, listens, merges `<student>.raphs` stats/reminders, and navigates. |
| **4** | **Source Privacy & Ledger** | Zero source PDFs/crops and zero external paper names in `.raphs`; visuals/photos cleanly regenerated; local source tracking & queue state kept strictly in Git-ignored `private-xref.json` + `ledger.json` + `status.html`. |
| **5** | **Question Format** | Default **5-option MCQ (`A–E`)** for Gyara 11+ (Theta-verified trap distractors) + `written` / `interactive` / `drawing` supported. |
| **6** | **Database Schema** | Native SQLite **Layer `L7` (`course_*` tables)** in `memory.raphs` **Schema `2.3.0` (`memory_core_rs 1.7.0`)**—indexing Books, Chapters, Papers, Sections, Topics, Types, Stimuli, Questions, Brand/Media Blobs & Comments. |
| **7** | **Learner Statistics** | **Strictly excluded from course `.raphs` vaults**; stored in `<student>.raphs` (`important` `△`, `15d` reminders, `#FFF1F2` `dueReminder` states, mastery) and merged by SIGMA at runtime for Carbon's Gallery (`chrome.setCatalog`). |
| **8** | **Lesson Key & Export** | Single simple Lesson Key (`open_with_passphrase`); customer release uses `export_clean_course_bundle` stripping comments & history while preserving brand/media blobs. |
| **9** | **Unified Warm-Paper Theme** | **Carbon (`config.setTheme: "warm-paper"`, `displayMode: "full-stage"`) and Hydrogen's `status.html`** use the unified **Warm-Paper Editorial Theme** with PNG **`GYR-LOGO-BLUE`** and the 12-subject muted mineral palette (`#5B7DB1` Maths, `#4D8B7A` English, `#7A6BA8` VR, `#A8647A` NVR). |
| **10** | **Authoring Stack** | **Pure-Rust `hydrogen` CLI + Antigravity + Carbon `:7420` `--watch` + Static `status.html`** (with Zero-Install Browser WASM delivery in Phase 7 and modular API providers in Phase 8). |
