# memory.raphs 2.0: Architecture, Schema Specification & Client API Reference

> **Specification & Version Status (Updated 2026-11-21 — Schema `2.3.0` / Core Crate `1.7.0`).** This is the comprehensive specification, table reference, and client integration guide for `memory.raphs`. It supersedes draft 1 of 2026-09-24 (which described schema 1.3.0 and a retired Python library). The authoritative code and schema definitions live in `services/memory-format/` (schema `2.3.0`) and `services/memory-core-rs/` (crate `1.7.0`). Whenever a new schema migration or crate release is cut, update the version block below and regenerate `memory-raphs.html` via `docs/architecture/build_memory_raphs_html.py`.

- **Schema Version:** `2.3.0` (backward-compatible auto-migrations from `2.0.0`, `2.0.1`, `2.0.2`, `2.0.3`, `2.0.4`, `2.1.0`, `2.2.0`)
- **Core Crate & Release Tag:** `memory_core_rs 1.7.0` (git tag `memory-core-1.7.0`) · Python wheel `memory_raphs 1.7.0`
- **Container / Sidecar / Bundle Formats:** Container `2.0.0` (`<name>.sqlite`) · Key Sidecar `1.0.0` (`<name>.keys.json`) · Bundle `2.0.0` (`<name>.raphs`)
- **Crypto Specification:** `sqlcipher4-aes256-envelope-v1` (32-byte raw DEK wrapped with AES-256-GCM under Argon2id 64 MiB / 3-iteration passphrase KDF and 52-char Crockford Base32 recovery key)
- **Pinned Engines & Embedder:** SQLite `3.51.3` · SQLCipher `4.14.0` · `sqlite-vec` `0.1.9` · FTS5 (`porter unicode61 remove_diacritics 2`) · `nomic-embed-text-v1.5` (`768` dims)
- **Schema Object Counts:** **57 Tables** (**51 standard tables** + **6 virtual tables**) and **14 Views** (10 locality-filtered prompt-safe views + 4 state/clock views)
- **Source of Truth:** `services/memory-format/` (contract `2.3.0`), `services/memory-core-rs/` (core `1.7.0`), ADR 0026, `docs/vision/memory-build-plan.md`
- **Audience:** any client application (Visage, Hydrogen, and SIGMA in Rust/Tauri, or Python, C/C++, Swift, and Kotlin consumers) and engineers evaluating or integrating `memory.raphs`

---

## 0. In one paragraph

memory.raphs keeps one person's memory — or an encrypted course & curriculum vault — in **one encrypted SQLite file** (SQLCipher 4, with `sqlite-vec` for semantic search and FTS5 for keywords), with a small **key sidecar** next to it and a **`.raphs` bundle** for backup and transport. Version 2.0 rebuilt the format and the code as a module of three layers: an **engine** (SQLite, SQLCipher, sqlite-vec) compiled into the core; a **contract** of plain SQL and JSON that is the one source of truth and enforces most of its own rules with triggers and views; and a **core** in Rust, `memory_core_rs`, that is the only implementation. Visage, Hydrogen, and SIGMA link that crate directly. Anything not written in Rust uses a packaging of the same crate (a C ABI, generated Swift and Kotlin bindings, a Python wheel) that holds no logic of its own. The Python library of 1.3.0 is frozen and retires; there is no second implementation to keep in step and nothing to compare against except the file itself.

**What a client calls, and nothing else is required:** `Memory::create` or `Memory::open_with_passphrase` once at start-up, `set_embedder` and `set_client` once after that, then per turn `build_context` before the brain and `process_turn` after it, `confirm_pending` / `confirm_memory` and `confirm_lexicon` when the person taps yes, `remember`, `forget` and `recall` for explicit commands, `due_reminders` / `complete_reminder` / `snooze_reminder` for scheduled reminders, `course_catalog_snapshot` / `query_question_summaries` / `get_question` / `upsert_question` / `export_clean_course_bundle` for L7 course vaults, and `pack` for Back up. Everything else in the crate is there for settings screens and tools. One number to pin: the release tag, which is the crate version (`memory-core-1.7.0`). The file format version (`2.3.0`) lives inside the file and the client never pins it.

### Quick Specification Summary

| Specification Item | Current Value | Notes |
|---|---|---|
| **Schema Version** | `2.3.0` | Stored in `metadata.schema_version` and `schema_migrations`. Older `2.x` files migrate automatically on open. |
| **Core Crate Version** | `1.7.0` (`memory-core-1.7.0`) | Semver; within major `1.x`, public API only adds (no breaking renames or signature changes). |
| **Container / Sidecar / Bundle** | `2.0.0` / `1.0.0` / `2.0.0` | `<name>.sqlite` + `<name>.keys.json` on disk; `<name>.raphs` zip archive for backup/restore. |
| **Encryption & Key Wrapping** | `sqlcipher4-aes256-envelope-v1` | SQLCipher 4 raw 32-byte DEK; wrapped in sidecar via AES-256-GCM by Argon2id (`m=65536, t=3, p=1`) passphrase + 32-byte recovery key. |
| **Compiled-In Engines** | SQLite `3.51.3`, SQLCipher `4.14.0`, `sqlite-vec` `0.1.9` | Statically compiled into `memory_core_rs` with vendored OpenSSL; asserted against `engine.json` on every open. |
| **Vector & Keyword Search** | `vec0` (`float[768]`) + FTS5 (`porter unicode61`) | Reciprocal Rank Fusion (RRF) over `nomic-embed-text-v1.5` embeddings and stemmed FTS5 keyword matches; graceful keyword-only fallback when offline. |
| **Database Tables** | **57 total** (51 standard + 6 virtual) | 32 domain/layer tables (L1–L7 + queue), 4 housekeeping tables, 3 client/consent/audit tables, 11 vocabulary lookup tables (+ `sqlite_sequence` when L7 questions exist), 2 `vec0` tables, 4 `fts5` tables. |
| **Database Views** | **14 total** (10 prompt-safe + 4 state/clock) | `prompt_safe_{atoms,personality,facets,people,turns}_{local,cloud}`, `current_atoms`, `active_personality`, `current_facets`, `clock_horizon`. |
| **Language Packagings** | Rust (`memory_core_rs`), Python (`memory_raphs`), C ABI, Swift & Kotlin (UniFFI) | Single Rust implementation; C, Swift, Kotlin, and Python bindings are thin JSON-in/JSON-out wrappers over `memory_core_rs::facade`. |

### How to Get & Access `memory.raphs` (`Schema 2.3.0` · `Core 1.7.0`)

Everything is distributed through the **`raphs-app/raphs`** GitHub repository under the immutable release tag **`memory-core-1.7.0`**:

- **GitHub Repository:** `https://github.com/raphs-app/raphs` (`git@github.com:raphs-app/raphs.git`)
- **Official Release & Pre-Built Assets:** `https://github.com/raphs-app/raphs/releases/tag/memory-core-1.7.0`
- **Repository Paths:**
  - `services/memory-format/` — Contract & Schema `2.3.0` (`schema.sql`, `migrations/`, `schemas/*.v2.schema.json`, `conformance.md`, `formats.md`)
  - `services/memory-core-rs/` — Rust Core `1.7.0` (`memory_core_rs` library + `memory` reference CLI)
  - `services/memory-core-py/` — Python `abi3` package (`memory_raphs` `1.7.0`)

#### 1. Rust Clients (Visage, Hydrogen, SIGMA, Tauri, or Native Rust)
Add `memory_core_rs` directly to `Cargo.toml` pinned to the `memory-core-1.7.0` tag. The `2.3.0` SQL contract, JSON schemas, and all forward migrations (`2.0.0` → `2.3.0`) are embedded into the crate at compile time — no external `.sql` files need to be shipped:

```toml
[dependencies]
# SSH (when the machine or CI runner has SSH access to github.com/raphs-app/raphs):
memory_core_rs = { git = "ssh://git@github.com/raphs-app/raphs.git", tag = "memory-core-1.7.0" }

# Or HTTPS:
# memory_core_rs = { git = "https://github.com/raphs-app/raphs.git", tag = "memory-core-1.7.0" }
```

#### 2. Python Clients (`memory_raphs` Pre-Built Wheels)
Download the pre-built `cp310-abi3` wheel for your OS/architecture from the `memory-core-1.7.0` GitHub Release (no Rust toolchain required):

```sh
gh release download memory-core-1.7.0 --repo raphs-app/raphs --pattern "memory_raphs-1.7.0-*.whl"
pip install memory_raphs-1.7.0-*.whl
```

#### 3. Swift, Kotlin & C/C++ Clients (Pre-Built Bindings & C Header)
Download the generated UniFFI bindings (`memory-core-1.7.0-bindings.zip`) or C ABI header (`memory_raphs.h`) directly from the release:

```sh
gh release download memory-core-1.7.0 --repo raphs-app/raphs \
  --pattern "memory-core-1.7.0-bindings.zip" \
  --pattern "memory_raphs.h" \
  --pattern "memory-core-1.7.0-source.tar.gz"
```

#### Published Release Assets on `memory-core-1.7.0`

| Asset Name | Target / Consumer | Contents |
|---|---|---|
| `memory-core-1.7.0-source.tar.gz` | All clients / Auditors | Complete `services/memory-format/` (schema `2.3.0`) and `services/memory-core-rs/` (crate `1.7.0`) source tree |
| `memory-core-1.7.0-bindings.zip` | Swift & Kotlin apps | Generated UniFFI Swift and Kotlin bindings (`MemoryFile`, `MemoryError`) |
| `memory_raphs.h` | C / C++ applications | C ABI header for `libmemory_core_rs` (`cdylib` / `staticlib`) |
| `memory_raphs-1.7.0-cp310-abi3-macosx_11_0_arm64.whl` | Python 3.10+ (macOS Apple Silicon) | Pre-built Python wheel with bundled SQLCipher 4.14.0 & `sqlite-vec` 0.1.9 |
| `memory_raphs-1.7.0-cp310-abi3-macosx_10_12_x86_64.whl` | Python 3.10+ (macOS Intel) | Pre-built Python wheel |
| `memory_raphs-1.7.0-cp310-abi3-manylinux_2_28_x86_64.whl` | Python 3.10+ (Linux `x86_64`) | Pre-built Python wheel |
| `memory_raphs-1.7.0-cp310-abi3-manylinux_2_28_aarch64.whl` | Python 3.10+ (Linux `aarch64`) | Pre-built Python wheel |
| `memory_raphs-1.7.0-cp310-abi3-win_amd64.whl` | Python 3.10+ (Windows `x86_64`) | Pre-built Python wheel |

---

## 1. The layered picture

```
                 VISAGE  (desktop · Rust)                 OTHER CONSUMERS  (any language)

1  PEOPLE        ┌───────────────────────────────────┐   ┌───────────────────────────────────┐
                 │ the person at the desk:           │   │ a developer, a script, an agent,  │
                 │ hold-key · speech · typing · Board│   │ a Swift or Kotlin app (later)     │
                 └─────────────────┬─────────────────┘   └─────────────────┬─────────────────┘
                                   ▼                                       ▼
2  APP           ┌───────────────────────────────────┐   ┌───────────────────────────────────┐
                 │ Tauri shell (Rust + webview)      │   │ Python program · C or C++ program │
                 │ face · Board · settings           │   │ · Swift or Kotlin app             │
                 │ brain: Gemma via llama-server, or │   │                                   │
                 │ a cloud model · whisper · piper   │   │                                   │
                 └─────────────────┬─────────────────┘   └─────────────────┬─────────────────┘
                                   ▼ links the crate                       ▼ calls a packaging
3  PACKAGING     ┌───────────────────────────────────┐   ┌───────────────────────────────────┐
                 │ none needed: memory_core_rs is a  │   │ memory_raphs wheel (PyO3, abi3)   │
                 │ dependency in Visage's Cargo.toml │   │ C ABI (include/memory_raphs.h)    │
                 │                                   │   │ UniFFI → Swift, Kotlin (generated)│
                 └─────────────────┬─────────────────┘   └─────────────────┬─────────────────┘
                                   └───────────────────┬───────────────────┘
                 ═══ memory.raphs · the module (ADR 0026) ═══════════════════════════════════════
                                                       ▼
4  CORE          ┌───────────────────────────────────────────────────────────────────────────┐
                 │ memory_core_rs 1.7.0 (Rust) · the one implementation                      │
                 │ create / open · write protocol · consent · remember / recall / forget     │
                 │ grounding · policy kit · L7 course vaults · bundle / clean export · check │
                 └─────────────────────────────────────┬─────────────────────────────────────┘
                                                       ▼ embedded at build time
5  CONTRACT      ┌───────────────────────────────────────────────────────────────────────────┐
                 │ memory-format 2.3.0 · schema.sql · migrations/ · schemas/*.v2.schema.json │
                 │ engine.json · embedding.json · facets/registry.json · policy text         │
                 │ conformance.md (rules A–L) · formats.md · triggers and views enforce it   │
                 └─────────────────────────────────────┬─────────────────────────────────────┘
                                                       ▼ compiled in
6  ENGINE        ┌───────────────────────────────────────────────────────────────────────────┐
                 │ SQLite 3.51.3 · SQLCipher 4.14.0 · sqlite-vec 0.1.9 · FTS5 · JSON         │
                 │ statically linked into the core: no system library on macOS or Windows    │
                 └─────────────────────────────────────┬─────────────────────────────────────┘
                                                       ▼ one file per person
7  FILES         ┌───────────────────────────────────────────────────────────────────────────┐
                 │ <name>.sqlite (encrypted, raw 32-byte key) · <name>.keys.json (sidecar)   │
                 │ <name>.raphs bundle = manifest.json + memory.sqlite + keys.json           │
                 └───────────────────────────────────────────────────────────────────────────┘
                 ═══════════════════════════════════════════════════════════════════════════════

SIDE SERVICES    embeddings     llama-server + nomic-embed-text-v1.5 (768 dims) over HTTP; keyword-only when down
                 keys           the passphrase lives in the OS keychain; the recovery key is shown once, never stored
                 conversations  L6 inside the memory container (local-only default, 30-day retention, kept = forever)
```

How to read it:

- **Layers 1 to 3 are consumers.** Visage is the first. It links the crate; there is nothing between Visage and the core. Consumers in other languages call a packaging, and every packaging is JSON in, JSON out, no decisions.
- **Layer 4 is the only code that touches a memory file.** Lifecycle, the write protocol, consent, recall, grounding, the recording policy, bundles and the self-check all live here, once.
- **Layer 5 is the contract.** Plain SQL and JSON, generated and checked in CI, embedded into the core at build time. Triggers and views make most rules impossible to break; `conformance.md` lists the rest, numbered so a review can cite them.
- **Layer 6 is pinned.** The core asserts the engine versions at start-up and refuses to run on anything else, so two machines never disagree about what a file means.
- **Layer 7 is the data.** The live file, its key sidecar, and the bundle that carries both.

---

## 2. Terms

| Term | Meaning |
|---|---|
| container | The live database, `<name>.sqlite`: SQLCipher 4, WAL mode, schema 2.3.0. One file per person (or one encrypted course vault per subject). |
| key sidecar | `<name>.keys.json` next to the container. Holds wrapped copies of the file's data-encryption key: one wrapped by the passphrase, one by the recovery key. |
| bundle | `<name>.raphs`: a zip of `manifest.json`, `memory.sqlite` (an encrypted export of the container) and `keys.json` (the sidecar). Self-contained: backup, export, transport, or customer course distribution. |
| `memory_core_rs` | The Rust crate at `services/memory-core-rs`, version 1.7.0. The one implementation. Its binary `memory` is the reference client. |
| `memory-format` | The contract directory at `services/memory-format`: `schema.sql` (generated), migrations, JSON Schemas, engine and embedding pins, the facet registry, the policy text, `conformance.md`, `formats.md`. |
| course vault | An encrypted `.sqlite` / `.raphs` container populated with Layer L7 (`course_*` tables, e.g., `gyara-maths.raphs`, `gcse-maths.raphs`): books, chapters, papers, timed sections, topics, `9 -> 1` topic cards and questions/solutions, content-addressed media assets, and authoring review comments. Empty (`0` overhead) in personal companion vaults; read-only to learners at study time. |
| atom | One remembered thing in `memory_atoms`: an event, a fact, a routine or a milestone (optionally a reminder with `remind_at` / `completed_at`), with a vector, a keyword index row, importance, confidence, sensitivity and provenance. |
| lexicon | The L4 table `personality_profile`: how the person communicates. Preferences, routines, phrases, word approximations, and Visage's dictation dictionary (section 10). |
| facet | A small typed identity attribute under a namespace (`raphs:locale`, `raphs:timezone`, `visage:...`), declared in the facet registry. |
| locality | Where the brain that reads a prompt runs: `Local` (this device) or `Cloud`. It selects which rows may be rendered. |
| sensitivity | A column on every row a prompt might see: `normal` (any brain), `local_only` (a local brain only), `never_prompt` (stored, never rendered). |
| actor | Who is writing: the person (`user`), the application on a rule (`visage`), the model through the router (`companion`, stamped by the core), the contract itself (`raphs:contract`, migrations only). |
| client | A registered application in `clients`, with consent scopes. Visage registers itself once and acts under its client id. |
| clock | A hybrid logical clock stamped on every row, `<13-digit ms>-<4 hex counter>-<device id>`. Sorts as text; the foundation for sync between devices. |
| conversation, turn | A conversation is one exchange between the person and the companion; a turn is one thing one of them said, `person` or `companion`, written as it happens. Conversations are local-only by default and kept by the memory for a retention period unless the person keeps them. |
| provenance | Append-only audit and change log: every create, update and delete, by whom, from which device, at which clock. Never the content itself. |

---

## 3. What the file looks like (schema 2.3.0)

The field-level truth is `services/memory-format/schemas/<table>.v2.schema.json`; this section is the map. Everything below is created by `schema.sql`, which the core applies when it creates a file.

### 3.1 The seven layers and complete table inventory (57 tables, 14 views)

A `memory.raphs` container at schema `2.3.0` defines **57 tables** (**51 standard SQLite tables** + **6 virtual tables**) and **14 views**, grouped into seven layers (`L1`–`L7`), the policy confirmation queue, housekeeping/audit tables, and vocabulary lookup tables:

| Layer / Category | Count | Purpose | Tables / Views |
|---|---|---|---|
| **L1 Identity** | 3 tables | Who the person is, typed facets, and never-prompted sensitive fields | `identity_core` (one row), `identity_facets`, `sensitive_identity` |
| **L1 People** | 2 tables | Family, friends, carers, colleagues, customers, and atom links | `people`, `atom_people` |
| **L2 Episodic & L3 Semantic** | 3 tables + 2 virtual | Events, distilled facts, routines, milestones, and scheduled reminders | `memory_atoms`, `atom_revisions`, `atom_stats`, `atom_embeddings` (`vec0`), `atoms_fts` (`fts5`) |
| **L4 Personality (Lexicon)** | 1 table | Communication style, preferences, routines, and dictation dictionary | `personality_profile` |
| **L5 Sources & Blobs** | 4 tables | Original documents and media stored inside the container in 16 MiB chunks | `source_documents`, `blobs`, `blob_chunks`, `atom_sources` |
| **Policy Kit Queue** | 1 table | Held memory/reminder proposals awaiting user confirmation (since 2.0.4) | `pending_proposals` |
| **L6 Conversations** | 2 tables + 1 virtual | Turn-by-turn conversation transcripts and stemmed keyword index (since 2.1.0) | `conversations`, `turns`, `turns_fts` (`fts5`) |
| **L7 Course & Curriculum** | 17 tables + 3 virtual | Encrypted Course & Solution Vaults: books, chapters, papers, sections, topics, `9 -> 1` cards/questions/solutions, media assets, comments & clean export (since 2.3.0; empty in personal companion vaults) | `course_meta`, `course_books`, `course_chapters`, `course_papers`, `course_paper_sections`, `course_topics`, `course_topic_cards`, `course_question_types`, `course_stimuli`, `course_questions`, `course_question_revisions`, `course_question_topics`, `course_concept_links`, `course_media_assets`, `course_item_assets`, `course_comments`, `course_comment_attachments`, `course_questions_fts` (`fts5`), `course_topic_cards_fts` (`fts5`), `course_embeddings` (`vec0`) |
| **Clients, Consent & Audit** | 3 tables | Registered applications, consent scope grants, and append-only audit log | `clients`, `consent_scopes`, `provenance` |
| **Housekeeping & Clocks** | 4 tables | Schema versions, container metadata, HLC state, and per-tx write context | `metadata`, `schema_migrations`, `hlc_state`, `_context` |
| **Vocabularies** | 11 tables | Extensible (`ns:value`) and fixed contract lookup tables | `vocab_sensitivity`, `vocab_atom_type`, `vocab_entry_type`, `vocab_frequency`, `vocab_document_type`, `vocab_sensitive_field_type`, `vocab_trust_scope`, `vocab_consent_scope`, `vocab_provenance_action`, `vocab_client_kind`, `vocab_turn_role` |
| **Views** | 14 views | 10 locality-filtered prompt-safe views + 4 active-state and clock views | `prompt_safe_{atoms,personality,facets,people,turns}_{local,cloud}`, `current_atoms`, `active_personality`, `current_facets`, `clock_horizon` |

#### Complete Table & View Catalog (Schema `2.3.0`)

| # | Name | Kind | Primary Key | Audited in `provenance`? | Consent Scope (Read / Write) | Description & Key Columns |
|---|---|---|---|---|---|---|
| 1 | `identity_core` | `TABLE` | `id` (UUID, 1 row) | Yes (`create`/`update`/`delete`) | `identity.read` / `identity.write` | Core person profile: `display_name`, `full_name`, `pronouns`, `date_of_birth`, `diagnoses`, `sensory_profile`, `communication`, `known_triggers`, `calming_strategies`, `interests`, `photo`. |
| 2 | `identity_facets` | `TABLE` | `id` (UUID) | Yes (supersedes on change) | `identity.read` / `identity.write` | Extensible attributes (`namespace`, `name`, JSON `value`, `sensitivity`, `valid_from`, `valid_to`, `superseded_by`). Unique partial index `ux_facets_current` on active `(identity_id, namespace, name)`. |
| 3 | `sensitive_identity` | `TABLE` | `id` (UUID) | Yes (content-free audit) | `admin` / `admin` | Never-prompted fields (`field_type`, `label`, `value`, `priority`), including `access_code` (PINs/passcodes), `medical_id`, `address`, `emergency_contact`. |
| 4 | `people` | `TABLE` | `id` (UUID) | Yes | `people.read` / `people.write` | Trust circle and contacts: `display_name`, `full_name`, `relationship`, `contact_phone`, `contact_email`, `trust_scopes` (JSON array), `is_emergency_contact`, `priority`, `sensitivity`, `notes`. |
| 5 | `atom_people` | `TABLE` | `(atom_id, person_id)` | Yes (`atom_id/person_id`) | `people.read` / `people.write` | Many-to-many link between `memory_atoms` and `people` (`ON DELETE CASCADE`). |
| 6 | `memory_atoms` | `TABLE` | `seq` (int), `id` (UUID `UNIQUE`) | Yes (+ snapshot in `atom_revisions`) | `atoms.read` / `atoms.write` (`atoms.propose`) | Core memories & reminders: `atom_type`, `title`, `content`, `emotion`, `importance`, `confidence`, `occurred_at`, `remind_at`, `completed_at`, `location`, `tags`, `media_refs`, `origin_ref`, `sensitivity`, `valid_from`, `valid_to`, `superseded_by`. |
| 7 | `atom_revisions` | `TABLE` | `id` (UUID), `UNIQUE(atom_id, version)` | Populated by `memory_atoms_au` trigger | `atoms.read` / trigger-managed | Pre-update snapshot of `memory_atoms` (`version`, `title`, `content`, `occurred_at`, `remind_at`, `completed_at`, `replaced_at`, `replaced_by`, `hlc`). |
| 8 | `atom_stats` | `TABLE` | `atom_id` (UUID) | No (salience on read) | `atoms.read` | Recall salience counters: `recall_count`, `last_recalled` (Unix ms). Updated automatically during `recall()`. |
| 9 | `atom_embeddings` | `VIRTUAL TABLE (vec0)` | `id` (UUID) | Maintained with `memory_atoms` | `atoms.read` / `atoms.write` | `sqlite-vec` index: `embedding float[768]` plus metadata filter columns `sensitivity`, `atom_type`, `occurred_at`, `is_current`. |
| 10 | `atoms_fts` | `VIRTUAL TABLE (fts5)` | `rowid` (`memory_atoms.seq`) | Maintained by triggers | `atoms.read` / trigger-managed | External-content FTS5 keyword index over `title, content` with `porter unicode61 remove_diacritics 2` tokenizer. |
| 11 | `personality_profile` | `TABLE` | `id` (UUID) | Yes (supersedes on change) | `personality.read` / `personality.write` | L4 communication lexicon & dictation dictionary: `entry_type`, `observation`, `meaning`, `context`, `language` (BCP-47), `frequency`, `confidence`, `tags`, `notes`, `is_active`, `sensitivity`, `valid_to`, `superseded_by`. |
| 12 | `blobs` | `TABLE` | `id` (UUID), `sha256` (`UNIQUE`) | Yes (`create`/`delete`, immutable) | `sources.read` / `sources.write` (or `course.read` / `course.write` via L7 helpers) | Content-addressed immutable binary metadata (`sha256`, `mime_type`, `size_bytes`, `chunk_count`). Garbage-collected via 3-way ref-counting only when zero references remain across `source_documents`, `course_media_assets`, and `course_comment_attachments`. |
| 13 | `blob_chunks` | `TABLE` | `(blob_id, seq)` | Immutable (guarded by triggers) | `sources.read` / `sources.write` (or `course.read` / `course.write` via L7 helpers) | Raw binary chunks (`data BLOB`, up to 16 MiB per chunk) belonging to `blobs`. |
| 14 | `source_documents` | `TABLE` | `id` (UUID) | Yes | `sources.read` / `sources.write` | L5 document records (`document_type`, `title`, `description`, `blob_id`, `original_filename`, `occurred_at`, `tags`, `sensitivity`, `retention_note`). |
| 15 | `atom_sources` | `TABLE` | `(atom_id, source_id)` | Yes (`atom_id/source_id`) | `sources.read` / `sources.write` | Many-to-many link between `memory_atoms` and `source_documents` (`ON DELETE CASCADE`). |
| 16 | `pending_proposals` | `TABLE` | `id` (UUID) | Yes | `atoms.read` / `atoms.propose` (`atoms.write`) | Policy kit confirmation queue: `title`, `content`, `atom_type`, `important`, `occurred_at`, `remind_at`, `source`, `status` (`open`, `confirmed`, `dismissed`), `resolved_at`, `resolved_atom_id`. |
| 17 | `conversations` | `TABLE` | `id` (UUID) | Yes | `conversations.read` / `conversations.write` | L6 conversation headers: `title`, `started_at`, `ended_at`, `kept` (`0`/`1`), `sensitivity` (default `local_only`). |
| 18 | `turns` | `TABLE` | `seq` (int), `id` (UUID `UNIQUE`) | Immutable (`UNIQUE(conversation_id, n)`) | `conversations.read` / `conversations.write` | Append-only conversation turns: `conversation_id`, `n` (`>= 1`), `role` (`person`, `companion`, `system`), `text` (`<= 64 KiB`), `recorded` (`0` for refused secret placeholder, `1` normal), `occurred_at`, `source`, `meta` (JSON object `<= 2 KB`). |
| 19 | `turns_fts` | `VIRTUAL TABLE (fts5)` | `rowid` (`turns.seq`) | Maintained by triggers | `conversations.read` | External-content FTS5 keyword index over `turns.text` (`porter unicode61 remove_diacritics 2`). |
| 20 | `clients` | `TABLE` | `id` (UUID) | Yes | `admin` | Registered client applications/agents/services (`name`, `kind`, `revoked_at`). |
| 21 | `consent_scopes` | `TABLE` | `(client_id, scope)` | Yes (`client_id/scope`) | `admin` | Per-client scope grants (`granted_at`, `granted_by`, `revoked_at`). |
| 22 | `provenance` | `TABLE` | `seq` (int) | Append-only audit table | Owner / `atoms.read` | Content-free audit and change log: `table_name`, `row_id`, `action` (`create`, `update`, `delete`), `row_version`, `changed_by`, `client_id`, `device_id`, `hlc`, `changed_at`, `source`, `notes`. |
| 23 | `metadata` | `TABLE` | `id` (`CHECK (id = 1)`) | Housekept on write | Any open handle | Single-row container header: `schema_version` (`2.3.0`), `container_format_version` (`2.0.0`), `crypto_version`, `dek_id`, `embedding_model_id`, `embedding_model_version`, `embedding_dimensions` (`768`), `origin_device_id`, `created_at`, `last_modified_at`. |
| 24 | `schema_migrations` | `TABLE` | `version` (`TEXT`) | Contract-managed | Any open handle | Applied schema versions (`2.0.0` through `2.3.0`) and `applied_at` timestamps. |
| 25 | `hlc_state` | `TABLE` | `device_id` (`TEXT`) | Binding-maintained | Internal | Last Hybrid Logical Clock (`last_hlc`) issued per `device_id` across restarts. |
| 26 | `_context` | `TABLE` | `id` (`CHECK (id = 1)`) | Ephemeral per-tx | Internal (`Memory::write`) | Per-transaction write attribution (`actor`, `client_id`, `device_id`, `hlc`, `source`) read by triggers and cleared before commit. |
| 27–37 | `vocab_*` (11 tables) | `TABLE` | `value` (`TEXT`) | Guarded by triggers | Read-only (`ns:value` extensible on 5 tables) | **Extensible (5):** `vocab_atom_type`, `vocab_entry_type`, `vocab_document_type`, `vocab_sensitive_field_type`, `vocab_consent_scope`. **Fixed (6):** `vocab_sensitivity`, `vocab_frequency`, `vocab_trust_scope`, `vocab_provenance_action`, `vocab_client_kind`, `vocab_turn_role`. |
| 38 | `course_meta` | `TABLE` | `id` (`CHECK (id = 1)`) | Yes (`course_id`) | `course.read` / `course.write` | Single-row course vault header: `course_id` (`1..128`), `product_code`, `subject_code`, `title`, `subtitle`, `stage_label`, `default_answer_format`, `theta_preset`, `course_version`, `config_json`. |
| 39 | `course_books` | `TABLE` | `book_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Course books: `code` (`UNIQUE`), `title`, `description`, `tier`, `sort_order`, `meta_json`. |
| 40 | `course_chapters` | `TABLE` | `chapter_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Chapters within a book (`book_id` `ON DELETE CASCADE`): `chapter_no`, `title`, `description`, `primary_topic_id` (`ON DELETE SET NULL`), `sort_order`. |
| 41 | `course_papers` | `TABLE` | `paper_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Practice/mock papers: `code` (`UNIQUE`), `title`, `badge`, `duration_sec`, `calculator_mode`, `default_answer_format`, `tier`, `total_marks`, `is_replica`, `parent_paper_id` (`ON DELETE SET NULL`), `sort_order`, `meta_json`. |
| 42 | `course_paper_sections` | `TABLE` | `section_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Timed sub-sections within a paper (`paper_id` `ON DELETE CASCADE`): `section_no`, `title`, `instructions`, `duration_sec`, `calculator_mode`. |
| 43 | `course_topics` | `TABLE` | `topic_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Self-referential curriculum tree (`parent_topic_id` `ON DELETE SET NULL`): `category`, `code` (`UNIQUE`), `title`, `description`, `sort_order`, `meta_json`. |
| 44 | `course_topic_cards` | `TABLE` | `card_id` (`TEXT`, `1..128`), `seq` (`UNIQUE`) | Yes | `course.read` / `course.write` | Concept lessons, worked examples, formulas, reference sheets & prerequisite refreshers: `topic_id`, `chapter_id`, `card_kind`, `title`, `summary_text`, `min_sublevel`, `max_sublevel`, `card_json`, `sort_order`, `seq`. |
| 45 | `course_question_types` | `TABLE` | `type_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Problem archetypes: `code` (`UNIQUE`), `title`, `description`, `default_topic_id` (`ON DELETE SET NULL`), `sort_order`. |
| 46 | `course_stimuli` | `TABLE` | `stimulus_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Shared passages, datasets, diagrams, experiments, code keys, audio/video clips: `stimulus_kind`, `title`, `body_text`, `stimulus_json`. |
| 47 | `course_questions` | `TABLE` | `seq` (`AUTOINCREMENT`), `question_id` (`UNIQUE`, `1..128`) | Yes (+ snapshot in `course_question_revisions`) | `course.read` / `course.write` | Core questions & `9 -> 1` solutions: `container_kind`, `paper_id`, `section_id`, `chapter_id`, `stimulus_id`, `seq_no`, `label`, `title`, `answer_format`, `question_type_id`, `difficulty` (`1..5`), `tier`, `marks`, `calculator_mode`, `recommended_time_sec`, `part_count`, `step_count`, `min_sublevel`, `max_sublevel`, `is_replica`, `parent_question_id`, `qa_status`, `stem_text`, `question_json`, `solution_json`. |
| 48 | `course_question_revisions` | `TABLE` | `id` (`TEXT`), `UNIQUE(question_id, version)` | Populated by `course_questions_au` trigger | `course.read` / trigger-managed | Pre-update snapshot written whenever `question_json` or `solution_json` changes (`version`, `question_json`, `solution_json`, `replaced_at`, `replaced_by`, `hlc`). Stripped on clean export. |
| 49 | `course_question_topics` | `TABLE` | `(question_id, topic_id)` | Yes (`question_id/topic_id`) | `course.read` / `course.write` | Many-to-many question-to-topic mapping with `is_primary` (`0`/`1`, `ON DELETE CASCADE`). |
| 50 | `course_concept_links` | `TABLE` | `(source_kind, source_id, target_kind, target_id, link_kind)` | Yes (`source_kind:source_id->target_kind:target_id:link_kind`) | `course.read` / `course.write` | Directed concept/formula/prerequisite/worked-example links between questions/topics/cards, with optional `step_hint`. |
| 51 | `course_media_assets` | `TABLE` | `asset_id` (`TEXT`, `1..128`) | Yes (+ 3-way blob GC trigger) | `course.read` / `course.write` | Course media metadata over deduplicated `blobs` (`blob_id` `ON DELETE RESTRICT`): `media_kind`, `mime_type`, `original_filename`, `width_px`, `height_px`, `duration_ms`, `alt_text`, `caption_text`, `captions_json`. |
| 52 | `course_item_assets` | `TABLE` | `(item_kind, item_id, asset_id, part_id, step_id, sublevel)` | Yes | `course.read` / `course.write` | Usage map binding `course_media_assets` to questions, topic cards, or stimuli at a specific `role`, `part_id`, `step_id`, and `sublevel` (`0..9`). |
| 53 | `course_comments` | `TABLE` | `comment_id` (`TEXT`, `1..128`) | Yes | `course.read` / `course.write` | Authoring/QA review comments (`target_kind`, `target_id`, `part_id`, `step_id`, `sublevel`, `comment_kind`, `status` in `'open' \| 'resolved'`, `comment_text`, `resolution_note`, `resolved_at`). Stripped on clean export. |
| 54 | `course_comment_attachments` | `TABLE` | `attachment_id` (`TEXT`, `1..128`) | Yes (+ 3-way blob GC trigger) | `course.read` / `course.write` | Board-capture/upload/clipboard screenshots attached to `course_comments` (`blob_id` `ON DELETE RESTRICT`, `source`, `pane`, `mime_type`, `filename`, `width_px`, `height_px`, `sort_order`). Stripped on clean export. |
| 55 | `course_questions_fts` | `VIRTUAL TABLE (fts5)` | `rowid` (`course_questions.seq`) | Maintained by triggers | `course.read` / trigger-managed | External-content FTS5 keyword index over `question_id, title, stem_text` (`porter unicode61 remove_diacritics 2`). |
| 56 | `course_topic_cards_fts` | `VIRTUAL TABLE (fts5)` | `rowid` (`course_topic_cards.seq`) | Maintained by triggers | `course.read` / trigger-managed | External-content FTS5 keyword index over `card_id, title, summary_text` (`porter unicode61 remove_diacritics 2`). |
| 57 | `course_embeddings` | `VIRTUAL TABLE (vec0)` | `item_key` (`'question:<id>'` or `'card:<id>'`) | Maintained with questions & cards | `course.read` / `course.write` | `sqlite-vec` 768-float cosine index (`embedding float[768]`) with filter columns `item_kind`, `difficulty`, `is_replica`. |
| V1–V10 | `prompt_safe_*_{local,cloud}` | `VIEW` | — | Read-only views | Layer `.read` scope | 10 locality-enforced views (`atoms`, `personality`, `facets`, `people`, `turns` × `local`, `cloud`). `local` allows `normal` + `local_only`; `cloud` allows `normal` only and strips `people` contact fields. |
| V11–V14 | `current_atoms`, `active_personality`, `current_facets`, `clock_horizon` | `VIEW` | — | Read-only views | Internal / `.read` | Active non-superseded rows (`valid_to IS NULL AND superseded_by IS NULL`) and `clock_horizon` (max `hlc` across the entire file, including all audited `course_*` tables). |

### 3.2 Conventions on every row

- **Ids** are UUID v7 strings (time-ordered), 36 characters, lower-case, on `L1`–`L6` tables; Layer `L7` (`course_*`) tables use deterministic human-readable Universal IDs (`1..=128` characters, e.g., `'GYR-MTH-P001-Q014'`). `metadata`, `_context`, and `course_meta` are single rows with `id = 1`.
- **Timestamps** are integer Unix milliseconds, UTC. Text timestamps are rejected by `typeof` checks.
- **Every row carries** `created_at`, `created_by`, `updated_at`, `updated_by`, `version`, `hlc` and `device_id`. Triggers fill `updated_*`, `version` and the audit rows; a writer never sets them.
- **Write attribution** is a single-row table, `_context`, set inside every transaction (actor, client id, device id, a fresh clock, a source pointer) and cleared before commit. Triggers read it. A write without an actor or a clock is refused by the file itself. The core does all of this in `Memory::write`; Visage never touches `_context`.
- **Provenance** rows are written by triggers for every create, update and delete, including link tables, with the row's version, clock, device and client. Salience (`atom_stats`) is the one unaudited table.
- **Vocabularies** are lookup tables, not CHECK lists: `vocab_atom_type`, `vocab_entry_type`, `vocab_sensitivity`, `vocab_document_type`, `vocab_consent_scope` and so on (appendix B). Core values are owned by the contract. A consumer adds its own as `namespace:value` with a core `base`, per file, and every rule, view and renderer treats the value as its base.
- **Sensitivity** replaces the `local-only` tag of 1.3: a column on atoms, people, lexicon entries, facets and sources, filtered by the prompt views.
- **No `user_id` anywhere.** Isolation is one file per person.
- **Deletes are real deletes.** The row and everything that hangs off it go; provenance keeps that a deletion happened.

### 3.3 Identity: `identity_core`, `identity_facets`, `sensitive_identity`

`identity_core` is one row per file, created by `Memory::set_identity(display_name, language, actor)`. It carries `display_name`, optional `full_name`, `pronouns`, `date_of_birth`, and the care-specific JSON columns RAPHS uses (`diagnoses`, `known_triggers`, `calming_strategies`, `interests`, `sensory_profile`, `communication`). Visage fills `display_name` and `communication.language` on first run and leaves the rest empty. `Memory::update_identity` changes fields through an `IdentityPatch`.

`identity_facets` is where new attributes go without a schema change. A facet has a `namespace`, a `name`, a JSON `value` validated against the facet's schema (the contract's small JSON Schema subset, conformance F1; the core refuses a schema outside it), a `sensitivity`, and validity: a new value supersedes the old one, so the history is kept. The contract ships the `raphs` namespace with `locale`, `timezone` and `preferred_name`. Visage declares its own namespace, `visage`, in `services/memory-format/facets/registry.json` before writing under it (rule F1), and registers the definitions at runtime with `Memory::register_facet` so the core can validate them.

`sensitive_identity` holds `field_type` in `emergency_contact | medical_id | address | nhs_number | gp_surgery | medication | allergy | access_code` with `label`, `value`, `priority`. `access_code` (since 2.0.3) is the one sanctioned place for a PIN, passcode or door code the person needs; see 9.3. It is never rendered into any prompt, local or cloud (rule C2), needs the `admin` scope, and is filled only from an explicit settings form.

### 3.4 People

`PersonInput`: `display_name`, `relationship` (free text: "mother", "colleague"), optional `full_name`, `contact_phone`, `contact_email`, `trust_scopes` (values from `medical, financial, education, daily_care, emergency_decision, personal`), `is_emergency_contact`, `priority`, `sensitivity`, `notes`. `atom_people(atom_id, person_id)` links a memory to the people in it; the link rows are audited with the compound id `atom_id/person_id`.

A prompt sees people through `prompt_safe_people_{local,cloud}` and the renderer emits exactly `person: <display_name> (<relationship>)`. Contact details never reach a cloud brain; the cloud view does not expose them.

### 3.5 Atoms: `memory_atoms`, `atom_embeddings`, `atoms_fts`, `atom_revisions`, `atom_stats`

What Visage supplies (`AtomInput`, which also implements `Default`):

| Field | Notes |
|---|---|
| `atom_type` | `episodic`, `semantic`, `routine`, `milestone`, or a namespaced value based on one of them |
| `title`, `content` | Short title; the content is what is embedded and recalled |
| `emotion`, `location`, `tags`, `media_refs` | Optional |
| `importance` 0..1 (default 0.5), `confidence` 0..1 (default 0.8) | |
| `occurred_at` | Unix ms. When it happened, not when it was written. Absent means the turn time |
| `remind_at`, `completed_at` | Optional Unix ms (since 2.2.0). When a reminder is scheduled to fire (`remind_at`) or was completed (`completed_at`). Indexed by `idx_atoms_remind_at` while active |
| `origin_ref` | Authorship class: `user`, `companion`, `dictation`, `import`. The core forces `companion` for model proposals |
| `sensitivity` | `normal` (default), `local_only`, `never_prompt` |
| `people`, `sources` | Ids to link in the same transaction |

What the file adds: `valid_from`, `valid_to`, `superseded_by` (an atom is never edited for a change of meaning; `Memory::supersede` ends the old one and inserts the new one in one transaction), the row conventions of 3.2, and `seq`, a file-local integer the FTS5 index needs that is never exposed or synced.

`atom_embeddings` is a `vec0` table: the 768-float vector plus metadata columns `sensitivity`, `atom_type`, `occurred_at` and `is_current`, kept in step by triggers so a vector search can filter exactly like the views. `atoms_fts` is an external-content FTS5 index over title and content, with English stemming (porter over unicode61, since 2.0.2), so "swims" finds "swimming". `atom_revisions` snapshots an atom before each update. `atom_stats` counts recalls (salience), written on read, excluded from provenance and sync.

An atom stored while the embedding server is unreachable has no vector yet: it is findable by keyword, and `Memory::backfill_vectors` completes it when the server returns (rule B6). If the embedding model in `metadata` ever differs from the one the core is built for, `Memory::reembed_all` recomputes every vector before recall is trusted.

**Reminders lifecycle (since schema 2.2.0 / crate 1.6.0, conformance K):** Any atom may carry `remind_at: Option<u64>` (scheduled fire time in Unix ms UTC) and `completed_at: Option<u64>` (completion time in Unix ms UTC), indexed by the partial index `idx_atoms_remind_at ON memory_atoms(remind_at) WHERE remind_at IS NOT NULL AND valid_to IS NULL`.
- `Memory::due_reminders(until_ms, locality)` returns active atoms from `prompt_safe_atoms_{local,cloud}` where `remind_at <= until_ms`, ordered by `remind_at ASC, id ASC`.
- `Memory::upcoming_reminders(after_ms, until_ms, limit, locality)` returns active atoms where `remind_at > after_ms` (and `<= until_ms` if given), ordered by `remind_at ASC, id ASC`.
- `Memory::complete_reminder(actor, client_id, source, atom_id, completed_at_ms)` sets `completed_at = Some(completed_at_ms)` and clears `remind_at = None` in an audited transaction. The pre-completion snapshot is saved in `atom_revisions`, and the atom remains searchable in `memory_atoms` as a completed past memory.
- `Memory::snooze_reminder(actor, client_id, source, atom_id, remind_at_ms)` reschedules `remind_at = Some(remind_at_ms)` and clears `completed_at = None`.
- `Memory::cancel_reminder(actor, client_id, source, atom_id)` clears `remind_at = None` without marking `completed_at` (leaving the atom intact; `forget` deletes the whole atom).

An illustrative atom as the core returns it:

```json
{
  "id": "019968a2-1f4e-7c6b-9d4a-3e2f1a0b5c7d",
  "atom_type": "episodic",
  "title": "Space museum trip",
  "content": "Alex spent the whole afternoon at the space museum and was calm and happy the entire time.",
  "emotion": "happy",
  "importance": 0.7,
  "confidence": 0.8,
  "occurred_at": 1789992000000,
  "remind_at": null,
  "completed_at": null,
  "location": "London",
  "tags": ["outing", "special-interest"],
  "media_refs": [],
  "origin_ref": "companion",
  "sensitivity": "normal",
  "valid_from": null,
  "valid_to": null,
  "superseded_by": null,
  "created_at": 1790006531000,
  "created_by": "companion",
  "updated_at": 1790006531000,
  "updated_by": "companion",
  "version": 1,
  "hlc": "1790006531000-0000-mac-visage",
  "device_id": "mac-visage",
  "has_vector": true
}
```

### 3.6 The lexicon: `personality_profile`

`LexiconInput`: `entry_type` (appendix B; Visage uses `preference`, `routine`, `preferred_phrase`, `word_approximation`), `observation` (what was seen or heard), optional `meaning`, `context`, `language` (a BCP-47 tag when the entry is language-specific, which the dictation dictionary is), `frequency`, `first_observed`, `last_observed`, `confidence`, `tags`, `notes`, `is_active` (default true), `sensitivity`.

Corrections **supersede** (rule B11): `Memory::supersede_lexicon_entry` ends the old entry and inserts the new one in one transaction. Entries are never edited in place for a change of meaning or spelling. `is_active` switches an entry off without deleting it. Prompts read `prompt_safe_personality_{local,cloud}` and render `profile.<entry_type>: <observation> — <meaning>` (meaning omitted when empty), active entries only.

### 3.7 Sources: `source_documents`, `blobs`, `blob_chunks`, `atom_sources`

`SourceInput`: `document_type` (appendix B), `title`, optional `description`, `mime_type`, `original_filename`, `occurred_at`, `tags`, `sensitivity`, `retention_note`. `Memory::add_source(..., bytes)` stores the bytes inside the file as a `blobs` row (sha256, size, mime) with 16 MiB `blob_chunks`, deduplicated by digest; there is no `blobs/` directory any more. Chunks are immutable: a changed document is a new blob. `Memory::read_blob` returns the bytes after verifying the digest. Since schema `2.3.0`, `blobs` uses **3-way reference counting** across `source_documents`, `course_media_assets`, and `course_comment_attachments`: deleting a source, course asset, or comment attachment removes the underlying `blobs` row (and its `blob_chunks`) only when zero references remain across all three tables. `atom_sources(atom_id, source_id)` says which document an atom was distilled from. A kept Visage conversation becomes a `conversation_export` source.

### 3.7b Policy confirmation queue: `pending_proposals` (since 2.0.4, extended in 2.2.0)

When `Memory::process_turn` holds a `REMEMBER:` or `REMIND:` proposal for confirmation (under `RecordingPolicy::ConfirmImportant` or `RecordingPolicy::ConfirmAll`), the proposal is persisted in `pending_proposals` (`id`, `title`, `content`, `atom_type`, `important`, `occurred_at`, `remind_at`, `source`, `status` in `'open' | 'confirmed' | 'dismissed'`, `resolved_at`, `resolved_atom_id`) so it survives application restarts.
- `Memory::pending_proposals()` lists all open (`status = 'open'`) items.
- `Memory::confirm_pending(actor, client_id, proposal_id, atom_type_override)` writes the atom (preserving `occurred_at` and `remind_at`) and marks the row `confirmed` with `resolved_atom_id`.
- `Memory::dismiss_pending(actor, client_id, proposal_id)` marks the row `dismissed`; dismissed items are not re-queued on the same UTC day (conformance I3).

### 3.8 Conversations: `conversations`, `turns`, `turns_fts` (since 2.1.0)

Decision 1 was reversed on 2026-09-28: conversations live inside the memory, as a layer of their own, so there is one store, one key and one security story for everything the person and the companion say to each other. A conversation is an audited row (`title`, `started_at`, `ended_at`, `kept`, `sensitivity`, default `local_only`). Its turns are append-only rows (`n` from 1, `role` `person`, `companion` or `system` for the application's own lines, `text` capped at 64 KiB, `occurred_at`, `source`, and `meta`, one client-owned JSON object of at most 2 KB that a brain never sees) with a stemmed keyword index; `system` lines read back where they happened but stay out of the prompt views and out of search unless asked; they are immutable and not individually audited, the conversation's row carries the audit. A turn that looks like a secret is stored as a placeholder with `recorded = 0`, never searchable. Retention is the memory's: the `raphs:conversation_retention_days` facet (absent means 30, 0 means forever) is applied when the file is opened; a kept conversation is never removed by it. Prompt views `prompt_safe_turns_{local,cloud}` are the only way a search reads turns; a cloud brain sees nothing until the person sets a conversation's sensitivity to `normal`. `build_context` does not include turns: Visage passes its own rolling window and calls `search_turns` for questions about the past.

### 3.8b Course & Curriculum: `course_*`, `course_*_fts`, `course_embeddings` (L7, since 2.3.0)

Layer `L7` (conformance L) turns any `memory.raphs` container into an encrypted **Course & Solution Vault** (e.g., `gyara-maths.raphs`, `gcse-maths.raphs`, `gyara-english.raphs`) consumed by **`Hydrogen`** (authoring/QA studio) and **`SIGMA`** / learner clients, while remaining completely empty (`0` overhead) inside personal companion vaults:

- **Deterministic Universal IDs & Zero Student State:** Every `course_*` entity uses a human-readable Universal ID (`TEXT PRIMARY KEY`, `1..=128` characters, e.g., `'GYR-MTH-P001-Q014'`) under `PRAGMA foreign_keys = ON`. Course vaults are read-only to learners at study time; per-learner progress and statistics live in each learner's personal `.raphs` vault, never in `L7`.
- **Containers, Taxonomy & `9 -> 1` Walkthroughs:** `course_meta` (`id = 1`) identifies the course (`course_id`, `product_code`, `subject_code`, `stage_label`, `default_answer_format`, `theta_preset`, `course_version`, `config_json`). Containers (`course_books`, `course_chapters`, `course_papers`, `course_paper_sections`), taxonomy (`course_topics`, `course_topic_cards`, `course_question_types`), and shared passages/datasets (`course_stimuli`) organize `course_questions`, where `question_json` and `solution_json` store the Carbon/Theta `9 -> 1` sub-level scaffolding. Updating `question_json` or `solution_json` automatically snapshots the prior version into `course_question_revisions` via `course_questions_au`.
- **`< 1 ms` Left-Band Queries & Hybrid RRF Search:** `Memory::course_catalog_snapshot()` and `Memory::query_question_summaries(filter)` project only indexed scalar columns (never reading heavy `question_json`, `solution_json`, or `card_json`), returning 1,000+ Left-Band cards with primary/linked topic IDs and open-comment counts in under `1 ms`. `Memory::search_course(query, filter, limit)` fuses `course_embeddings` (`vec0` 768-float cosine over `'question:<id>'` and `'card:<id>'`) with `course_questions_fts` and `course_topic_cards_fts` via Reciprocal Rank Fusion (`k = 60`), falling back automatically to FTS5 when no embedder is attached.
- **3-Way Content-Addressed Media & Review Comments:** `course_media_assets` (`course_item_assets` for part/step/sublevel bindings) and `course_comment_attachments` (screenshots on `course_comments`) store binary payloads in `blobs` / `blob_chunks` deduplicated by SHA-256, sharing 3-way reference-counted garbage collection with `source_documents`.
- **Clean Customer Bundle Export:** `Memory::export_clean_course_bundle(out_bundle_path, lesson_key_passphrase)` exports a new `.raphs` bundle encrypted under a fresh random DEK wrapped with `lesson_key_passphrase`, stripping `course_comments`, `course_comment_attachments`, `course_question_revisions`, unreferenced comment blobs, `provenance`, and `hlc_state`, then running `VACUUM` + integrity/FK verification while leaving the live authoring vault untouched.

### 3.9 Housekeeping: provenance, clients, metadata, clocks, views

- **`provenance`** is the append-only, content-free audit and change log: `seq`, `table_name`, `row_id`, `action` (`create`, `update`, `delete`), `row_version`, `changed_by`, `client_id`, `device_id`, `hlc`, `changed_at`, `source`, `notes`. Visage's Memory Log screen reads it via `Memory::list_provenance(limit, before_seq)`. `source` carries the pointer to the moment (`visage:session/<id>/turn/<n>`).
- **`clients`** and **`consent_scopes`**: registered applications and what each may do. Grants and revocations are audited. See 5.3.
- **`metadata`** (one row, `id = 1`): `schema_version` `2.3.0`, `container_format_version` `2.0.0`, `crypto_version` `sqlcipher4-aes256-envelope-v1`, `dek_id`, `origin_device_id`, `embedding_model_id`, `embedding_model_version`, `embedding_dimensions` (`768`), `created_at`, `last_modified_at`. **`schema_migrations`** lists every version applied (`2.0.0` through `2.3.0`).
- **`hlc_state`** persists the last clock issued per device; **`clock_horizon`** is the newest clock in the file across every table, so a device that was behind seeds its clock past everything it can see.
- **Prompt views** are static and per locality; they are the only way a prompt reads the file (rule C1). The core uses them; Visage never writes its own SELECT for a prompt.

---

## 4. The container: file, keys, bundle

### 4.1 On disk

```
<dir>/<name>.sqlite         the live container (plus -wal / -shm while open)
<dir>/<name>.keys.json      the key sidecar (schemas/keys.v1.schema.json)
```

A container without its sidecar cannot be opened by anyone. Treat both files as one unit: back them up together, move them together, delete them together.

### 4.2 Keys

- The file is encrypted with a random 32-byte **data-encryption key** (DEK), applied to SQLCipher as a raw key. The DEK is never stored in clear anywhere.
- The sidecar holds the DEK **wrapped** (AES-256-GCM, bound to the file's `dek_id`) under two keys: one derived from the **passphrase** with Argon2id (64 MiB, 3 iterations), and the **recovery key** itself, 32 random bytes shown to the person once as 52 Crockford base32 characters in groups of four.
- `Memory::create(path, passphrase, device_id)` returns the open memory and the recovery key text. `Memory::open_with_passphrase` and `Memory::open_with_recovery` open it. `Memory::change_passphrase(path, current, new)` rewrites the sidecar only; the database is untouched and there is no re-encryption. `Memory::change_passphrase_with_recovery(path, recovery, new_passphrase, actor, device_id)` sets a new passphrase using the recovery key and records an audited touch on the file. `Memory::rekey(path, passphrase, actor)` is the slow path for a suspected key compromise: a new data-encryption key, the whole file re-encrypted, a new recovery key returned; it is written so that an interruption at any point leaves a file the next open completes.
- For Visage the passphrase is a high-entropy string generated once and kept in the OS keychain (macOS Keychain, Windows Credential Manager, the `keyring` crate). The person never types it. The recovery key is what they write down; offer to show it again only by re-authenticating to the OS.
- Opening asserts the engine pins, applies `PRAGMA foreign_keys = ON`, runs `quick_check`, verifies `metadata.dek_id` against the sidecar, refuses a schema newer than the core knows, and migrates an older 2.x file forward before anything else runs.

### 4.3 The `.raphs` bundle

`Memory::pack(out)` writes a zip of three entries: `manifest.json` (validated against `schemas/manifest.v2.schema.json`), `memory.sqlite` (a `sqlcipher_export` copy, still encrypted under the same DEK) and `keys.json` (the sidecar). `bundle::read_manifest` inspects one; `bundle::unpack(bundle, dest)` verifies every hash and the `dek_id` and writes a new container plus sidecar. Whoever holds the passphrase or the recovery key can open a bundle anywhere; nothing else is needed. For Visage the bundle is what Back up, Export and any future sync produce.

### 4.4 Where things live on disk (proposal for Visage)

```
<app data>/memory/<profile>/
    memory.sqlite            the live container (includes L1–L6, conversations & turns)
    memory.keys.json         its sidecar
<user-chosen folder>/
    <name>-2026-10-19.raphs  bundles produced by Back up / Export
```

The reference client keeps its device id at `~/.raphs/device-id`. Visage should generate its own once per install and keep it in app data; the id appears in every row's `hlc` and `device_id`, so it should be short, stable and not personal (`mac-visage-7f3a`, not a hostname with a name in it).

---

## 5. The core: what the crate gives Visage

### 5.1 Lifecycle and threading

`Memory` owns one connection, the device clock, the sidecar identity, the acting client and an optional embedder. It is `Send` but not `Sync`: keep exactly one `Memory` per file per process (rule A6) and either own it in one task behind a channel or put it in a `Mutex` in Tauri state, as the packagings do. Every write goes through `Memory::write`, which runs the whole B1 protocol (begin immediate, set `_context`, statements, clear, commit, rollback on failure). The public methods call it; Visage never needs to.

### 5.2 The surface

| Area | Methods on `Memory` | Notes |
|---|---|---|
| Lifecycle | `create`, `open_with_passphrase`, `open_with_recovery`, `change_passphrase`, `change_passphrase_with_recovery` (a restored file gets its own passphrase from the recovery key), `rekey`, `destroy`, `status`, `self_check`, `list_provenance` | `status` reports versions, counts, clock horizon, integrity; `rekey` is the slow path (new data-encryption key, safe at every interruption); `destroy` is "forget everything" for the container and its sidecar |
| Identity | `set_identity`, `identity`, `identity_id`, `update_identity`, `register_facet`, `set_facet`, `current_facets`, `facet_history`, `add_sensitive_field`, `get_sensitive_field`, `list_sensitive_fields`, `delete_sensitive_field` | Sensitive fields need `admin` |
| People | `add_person`, `update_person`, `delete_person`, `get_person`, `list_people`, `people_for_prompt`, `link_person`, `unlink_person` | |
| Atoms & Reminders | `remember`, `supersede`, `forget`, `get_atom`, `list_atoms`, `due_reminders`, `upcoming_reminders`, `complete_reminder`, `snooze_reminder`, `cancel_reminder`, `pending_vectors`, `backfill_vectors`, `reembed_all` | `remember` takes an optional vector; without one it uses the attached embedder or stores the atom with a pending vector |
| Recall and grounding | `recall(RecallQuery, vector?)` (`keyword_only` for a search box), `build_context(query, locality, atom_limit, lexicon_limit)` | Defaults: 5 atoms, 10 lexicon lines; `build_context` prepends currently due reminders |
| Lexicon | `add_lexicon_entry`, `supersede_lexicon_entry`, `set_lexicon_active`, `delete_lexicon_entry`, `get_lexicon_entry`, `list_lexicon`, `lexicon_for_prompt` | |
| Sources | `add_source`, `add_source_chunked`, `read_blob`, `get_source`, `get_source_text`, `list_sources`, `list_sources_of`, `delete_source`, `link_source` | Bytes live inside the file in 16 MiB chunks |
| Conversations | `start_conversation`, `append_turn`, `end_conversation`, `keep_conversation`, `set_conversation_sensitivity`, `rename_conversation`, `get_conversation`, `list_conversations`, `conversation_turns`, `conversation_turns_page`, `search_turns`, `forget_conversation`, `purge_expired_conversations`, `conversation_retention_days`, `set_conversation_retention_days` | Section 3.8; scopes `conversations.read` and `conversations.write` |
| Course & Curriculum (L7) | `set_course_meta`, `course_meta`, `upsert_book`, `list_books`, `delete_book`, `upsert_chapter`, `list_chapters`, `delete_chapter`, `upsert_paper`, `get_paper`, `list_papers`, `delete_paper`, `upsert_paper_section`, `list_paper_sections`, `delete_paper_section`, `upsert_topic`, `list_topics`, `delete_topic`, `upsert_topic_card`, `get_topic_card`, `list_topic_cards`, `delete_topic_card`, `upsert_question_type`, `list_question_types`, `delete_question_type`, `upsert_stimulus`, `get_stimulus`, `list_stimuli`, `delete_stimulus`, `upsert_question`, `get_question`, `get_question_revisions`, `delete_question`, `query_question_summaries`, `course_catalog_snapshot`, `search_course`, `put_course_asset`, `get_course_asset_info`, `read_course_asset_bytes`, `delete_course_asset`, `upsert_course_comment`, `list_course_comments`, `read_comment_attachment_bytes`, `resolve_course_comment`, `delete_course_comment`, `export_clean_course_bundle` | Section 3.8b; scopes `course.read` and `course.write` |
| Policy kit | `process_turn`, `pending_proposals`, `confirm_pending`, `dismiss_pending`, `confirm_memory`, `confirm_lexicon`; free functions `parse_signals`, `parse_timestamp_ms`, `format_iso8601_utc`, `route_memory`, `route_lexicon`, `refuse_reason`, `confirm_topic` | Section 5.5 |
| Consent | `register_client`, `find_client`, `list_clients`, `set_client`, `client`, `client_scopes`, `grant_scope`, `revoke_scope`, `revoke_client` | Section 5.3; `client_scopes`, `find_client` and `list_clients` are admin calls |
| Bundle | `pack`, `export_clean_course_bundle`; `bundle::read_manifest`, `bundle::unpack` | Sections 3.8b & 4.3 |
| Embeddings | `set_embedder(Box<dyn Embedder>)`, `has_embedder`; `LlamaServer::new(url, timeout)`, `.healthy()` | Section 7.1 |
| Crate root | `VERSION`, `supported_schema_versions()`, `now_ms()`, `new_id()` | |

Every public item is documented (`cargo doc --open` in the crate). Within crate major 1 the surface only grows: nothing is renamed, removed or changes signature.

#### Complete Rust API & Data Types Reference (`memory_core_rs 1.7.0`)

| Domain | Key Structs / Enums | Method Signature on `Memory` (or Free Function) | Scope Required |
|---|---|---|---|
| **Lifecycle & Audit** | `Status`, `CheckReport`, `ProvenanceRow`, `WriteCtx` | `Memory::create(path, passphrase, device_id) -> Result<(Memory, String)>`<br>`Memory::open_with_passphrase(path, passphrase, device_id) -> Result<Memory>`<br>`Memory::open_with_recovery(path, recovery, device_id) -> Result<Memory>`<br>`Memory::change_passphrase(path, current, new) -> Result<()>`<br>`Memory::change_passphrase_with_recovery(path, recovery, new_passphrase, actor, device_id) -> Result<()>`<br>`Memory::rekey(path, passphrase, actor) -> Result<String>`<br>`Memory::destroy(path) -> Result<()>`<br>`m.status() -> Result<Status>` · `m.self_check() -> Result<CheckReport>`<br>`m.list_provenance(limit, before_seq) -> Result<Vec<ProvenanceRow>>` | Owner / Open handle |
| **Identity & Facets** | `Identity`, `IdentityPatch`, `Facet`, `FacetDef`, `SensitiveField` | `m.set_identity(display_name, language, actor) -> Result<String>`<br>`m.identity() -> Result<Option<Identity>>`<br>`m.update_identity(actor, client_id, patch) -> Result<()>`<br>`m.register_facet(def)` · `m.set_facet(actor, client_id, ns, name, val, sensitivity, source) -> Result<String>`<br>`m.current_facets(locality) -> Result<Vec<Facet>>` · `m.facet_history(ns, name) -> Result<Vec<Facet>>`<br>`m.add_sensitive_field(actor, field_type, label, value, priority) -> Result<String>`<br>`m.get_sensitive_field(id) -> Result<Option<SensitiveField>>` · `m.list_sensitive_fields() -> Result<Vec<SensitiveField>>`<br>`m.delete_sensitive_field(actor, id) -> Result<bool>` | `identity.read` / `identity.write`<br>(`admin` for `sensitive_identity`) |
| **People** | `PersonInput`, `Person`, `PersonForPrompt` | `m.add_person(actor, client_id, input) -> Result<String>`<br>`m.update_person(actor, client_id, id, input) -> Result<bool>`<br>`m.delete_person(actor, client_id, id) -> Result<bool>`<br>`m.get_person(id) -> Result<Option<Person>>` · `m.list_people() -> Result<Vec<Person>>`<br>`m.people_for_prompt(locality) -> Result<Vec<PersonForPrompt>>`<br>`m.link_person(actor, client_id, atom_id, person_id)` · `m.unlink_person(...)` | `people.read` / `people.write` |
| **Atoms & Reminders** | `AtomInput` (`Default`), `Atom` | `m.remember(actor, client_id, source, input, vector) -> Result<String>`<br>`m.supersede(actor, client_id, source, old_id, input, vector) -> Result<String>`<br>`m.forget(actor, client_id, id) -> Result<bool>`<br>`m.get_atom(id) -> Result<Option<Atom>>` · `m.list_atoms(include_history) -> Result<Vec<Atom>>`<br>`m.due_reminders(until_ms, locality) -> Result<Vec<Atom>>`<br>`m.upcoming_reminders(after_ms, until_ms, limit, locality) -> Result<Vec<Atom>>`<br>`m.complete_reminder(actor, client_id, source, atom_id, completed_at_ms) -> Result<bool>`<br>`m.snooze_reminder(actor, client_id, source, atom_id, remind_at_ms) -> Result<bool>`<br>`m.cancel_reminder(actor, client_id, source, atom_id) -> Result<bool>`<br>`m.pending_vectors() -> Result<usize>` · `m.backfill_vectors(actor) -> Result<usize>` · `m.reembed_all(actor) -> Result<usize>` | `atoms.read` / `atoms.write` |
| **Recall & Grounding** | `Locality` (`Local`, `Cloud`), `RecallQuery`, `RecallHit`, `Recall`, `GroundingContext`, `ContextSections` | `m.recall(query, vector) -> Result<Recall>`<br>`m.build_context(query, locality, atom_limit, lexicon_limit) -> Result<GroundingContext>`<br>`ctx.render() -> String` · `ctx.render_sections() -> ContextSections` · `ctx.used_ids() -> Vec<String>` | `atoms.read` (+ `identity.read`, `people.read`, `personality.read` for `build_context`) |
| **Lexicon (L4)** | `LexiconInput`, `LexiconEntry` | `m.add_lexicon_entry(actor, client_id, source, input) -> Result<String>`<br>`m.supersede_lexicon_entry(actor, client_id, source, old_id, input) -> Result<String>`<br>`m.set_lexicon_active(actor, client_id, id, active) -> Result<bool>`<br>`m.delete_lexicon_entry(actor, client_id, id) -> Result<bool>`<br>`m.get_lexicon_entry(id) -> Result<Option<LexiconEntry>>` · `m.list_lexicon(include_history) -> Result<Vec<LexiconEntry>>`<br>`m.lexicon_for_prompt(locality) -> Result<Vec<LexiconEntry>>` | `personality.read` / `personality.write` |
| **Sources & Blobs (L5)** | `SourceInput`, `Source`, `BlobInfo` | `m.add_source(actor, client_id, source, input, bytes) -> Result<String>`<br>`m.read_blob(blob_id) -> Result<Vec<u8>>` · `m.get_source_text(id) -> Result<Option<String>>`<br>`m.get_source(id) -> Result<Option<Source>>` · `m.list_sources() -> Result<Vec<Source>>`<br>`m.list_sources_of(document_type) -> Result<Vec<Source>>`<br>`m.delete_source(actor, client_id, id) -> Result<bool>` · `m.link_source(actor, client_id, atom_id, source_id)` | `sources.read` / `sources.write` |
| **Conversations (L6)** | `Conversation`, `Turn`, `TurnRecord`, `TurnHit` | `m.start_conversation(actor, client_id, title, sensitivity) -> Result<String>`<br>`m.append_turn(actor, client_id, conv_id, Turn { role, text, occurred_at, source, meta }) -> Result<TurnRecord>`<br>`m.end_conversation(actor, client_id, id, ended_at) -> Result<bool>`<br>`m.keep_conversation(actor, client_id, id, kept) -> Result<bool>`<br>`m.set_conversation_sensitivity(actor, client_id, id, sensitivity) -> Result<bool>`<br>`m.rename_conversation(actor, client_id, id, title) -> Result<bool>`<br>`m.get_conversation(id)` · `m.list_conversations(limit)` · `m.conversation_turns(id, include_system)` · `m.conversation_turns_page(id, include_system, limit, after_n)`<br>`m.search_turns(query, locality, limit, include_system) -> Result<Vec<TurnHit>>`<br>`m.forget_conversation(actor, client_id, id) -> Result<bool>` · `m.purge_expired_conversations(actor, now_ms) -> Result<usize>`<br>`m.conversation_retention_days() -> Result<u32>` · `m.set_conversation_retention_days(actor, client_id, days)` | `conversations.read` / `conversations.write` |
| **Course & Curriculum (L7)** | `CourseMetaInput`, `CourseMeta`, `CourseBookInput`, `CourseBook`, `CourseChapterInput`, `CourseChapter`, `CoursePaperInput`, `CoursePaper`, `CoursePaperSectionInput`, `CoursePaperSection`, `CourseTopicInput`, `CourseTopic`, `CourseTopicCardInput`, `CourseTopicCard`, `CourseTopicCardSummary`, `CourseQuestionTypeInput`, `CourseQuestionType`, `CourseStimulusInput`, `CourseStimulus`, `CourseQuestionTopicLink`, `CourseConceptLinkInput`, `CourseConceptLink`, `CourseItemAssetInput`, `CourseItemAsset`, `CourseQuestionInput` (`Default`), `CourseQuestion`, `CourseQuestionRevision`, `CourseQuestionFilter` (`Default`), `CourseQuestionSummary`, `CourseCatalogSnapshot`, `CourseSearchHit`, `CourseAssetInput`, `CourseAsset`, `CommentAttachmentBytes`, `CourseCommentAttachment`, `CourseCommentInput`, `CourseComment` | `m.set_course_meta(actor, client_id, &meta) -> Result<()>` · `m.course_meta() -> Result<Option<CourseMeta>>`<br>`m.upsert_book(actor, client_id, &input) -> Result<()>` · `m.list_books() -> Result<Vec<CourseBook>>` · `m.delete_book(actor, client_id, book_id) -> Result<bool>`<br>`m.upsert_chapter(actor, client_id, &input) -> Result<()>` · `m.list_chapters(book_id) -> Result<Vec<CourseChapter>>` · `m.delete_chapter(actor, client_id, chapter_id) -> Result<bool>`<br>`m.upsert_paper(actor, client_id, &input) -> Result<()>` · `m.get_paper(paper_id) -> Result<Option<CoursePaper>>` · `m.list_papers() -> Result<Vec<CoursePaper>>` · `m.delete_paper(actor, client_id, paper_id) -> Result<bool>`<br>`m.upsert_paper_section(actor, client_id, &input) -> Result<()>` · `m.list_paper_sections(paper_id) -> Result<Vec<CoursePaperSection>>` · `m.delete_paper_section(actor, client_id, section_id) -> Result<bool>`<br>`m.upsert_topic(actor, client_id, &input) -> Result<()>` · `m.list_topics() -> Result<Vec<CourseTopic>>` · `m.delete_topic(actor, client_id, topic_id) -> Result<bool>`<br>`m.upsert_topic_card(actor, client_id, &input, vector) -> Result<()>` · `m.get_topic_card(card_id) -> Result<Option<CourseTopicCard>>` · `m.list_topic_cards(topic_id, kind) -> Result<Vec<CourseTopicCard>>` · `m.delete_topic_card(actor, client_id, card_id) -> Result<bool>`<br>`m.upsert_question_type(actor, client_id, &input) -> Result<()>` · `m.list_question_types() -> Result<Vec<CourseQuestionType>>` · `m.delete_question_type(actor, client_id, type_id) -> Result<bool>`<br>`m.upsert_stimulus(actor, client_id, &input) -> Result<()>` · `m.get_stimulus(stimulus_id) -> Result<Option<CourseStimulus>>` · `m.list_stimuli() -> Result<Vec<CourseStimulus>>` · `m.delete_stimulus(actor, client_id, stimulus_id) -> Result<bool>`<br>`m.upsert_question(actor, client_id, &input, vector) -> Result<()>` · `m.get_question(question_id) -> Result<Option<CourseQuestion>>` · `m.get_question_revisions(question_id) -> Result<Vec<CourseQuestionRevision>>` · `m.delete_question(actor, client_id, question_id) -> Result<bool>`<br>`m.query_question_summaries(&filter) -> Result<Vec<CourseQuestionSummary>>` · `m.course_catalog_snapshot() -> Result<CourseCatalogSnapshot>` · `m.search_course(query, &filter, limit) -> Result<Vec<CourseSearchHit>>`<br>`m.put_course_asset(actor, client_id, &meta, bytes) -> Result<String>` · `m.get_course_asset_info(asset_id) -> Result<Option<CourseAsset>>` · `m.read_course_asset_bytes(asset_id) -> Result<Vec<u8>>` · `m.delete_course_asset(actor, client_id, asset_id) -> Result<bool>`<br>`m.upsert_course_comment(actor, client_id, &input, new_attachments) -> Result<String>` · `m.list_course_comments(status, target_id) -> Result<Vec<CourseComment>>` · `m.read_comment_attachment_bytes(attachment_id) -> Result<Vec<u8>>` · `m.resolve_course_comment(actor, client_id, comment_id, resolution_note) -> Result<bool>` · `m.delete_course_comment(actor, client_id, comment_id) -> Result<bool>`<br>`m.export_clean_course_bundle(out_bundle_path, lesson_key_passphrase) -> Result<()>` | `course.read` / `course.write` |
| **Policy Kit & Queue** | `RecordingPolicy`, `TurnContext`, `TurnOutcome`, `PendingMemory`, `PendingLexicon`, `SkippedProposal`, `ToolCall` | `m.process_turn(reply, &TurnContext) -> Result<TurnOutcome>`<br>`m.pending_proposals() -> Result<Vec<PendingMemory>>`<br>`m.confirm_pending(actor, client_id, id, atom_type) -> Result<String>`<br>`m.dismiss_pending(actor, client_id, id) -> Result<bool>`<br>`m.confirm_memory(actor, client_id, pending)` · `m.confirm_lexicon(actor, client_id, pending)`<br>`parse_signals(reply)` · `parse_timestamp_ms(s)` · `format_iso8601_utc(ms)` · `refuse_reason(s)` | `atoms.propose` / `atoms.write` / `personality.write` |
| **Consent & Bundle** | `ClientSession`, `ClientRow`, `Manifest` | `m.register_client(name, kind, actor)` · `m.set_client(id)` · `m.grant_scope(client_id, scope, actor)` · `m.revoke_scope(...)` · `m.revoke_client(...)` · `m.find_client(name)` · `m.list_clients()`<br>`m.pack(bundle_path)` · `bundle::read_manifest(bundle_path)` · `bundle::unpack(bundle_path, dest_sqlite)` | `admin` (consent)<br>Owner (bundle) |

#### Exact Course & Curriculum (`L7`) Rust Structs & `Memory` Signatures (`services/memory-core-rs/src/course.rs`)

All 30 L7 structs below derive `Debug, Clone, Serialize, Deserialize` and are re-exported at both `memory_core_rs::*` and `memory_core_rs::course::*` (`Value` is `serde_json::Value`, `BTreeMap` is `std::collections::BTreeMap`, `Path` is `std::path::Path`, and `Result<T>` is `memory_core_rs::Result<T>`). L7 domain enums are represented as `String` fields validated by SQLite `CHECK` constraints (see Appendix B for exact allowed string values):

```rust
// Re-exported at crate root: `use memory_core_rs::{CourseMetaInput, CourseMeta, ...};`

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseMetaInput {
    pub course_id: String,
    pub product_code: String,
    pub subject_code: String,
    pub title: String,
    #[serde(default)] pub subtitle: Option<String>,
    #[serde(default)] pub stage_label: Option<String>,
    #[serde(default)] pub default_answer_format: String, // default: "mcq"
    #[serde(default)] pub theta_preset: Option<String>,
    #[serde(default)] pub course_version: String,        // default: "0.1.0"
    #[serde(default)] pub config_json: Value,            // default: {} (JSON object)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseMeta {
    pub course_id: String,
    pub product_code: String,
    pub subject_code: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub stage_label: Option<String>,
    pub default_answer_format: String,
    pub theta_preset: Option<String>,
    pub course_version: String,
    pub config_json: Value,
    pub created_at: u64,
    pub created_by: String,
    pub updated_at: u64,
    pub updated_by: String,
    pub version: i64,
    pub hlc: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseBookInput {
    pub book_id: String,
    pub code: String,
    pub title: String,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub tier: Option<String>,
    #[serde(default)] pub sort_order: i64,
    #[serde(default)] pub meta_json: Value,              // default: {} (JSON object)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseBook {
    pub book_id: String,
    pub code: String,
    pub title: String,
    pub description: Option<String>,
    pub tier: Option<String>,
    pub sort_order: i64,
    pub meta_json: Value,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseChapterInput {
    pub chapter_id: String,
    pub book_id: String,
    pub chapter_no: i64,
    pub title: String,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub primary_topic_id: Option<String>,
    #[serde(default)] pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseChapter {
    pub chapter_id: String,
    pub book_id: String,
    pub chapter_no: i64,
    pub title: String,
    pub description: Option<String>,
    pub primary_topic_id: Option<String>,
    pub sort_order: i64,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePaperInput {
    pub paper_id: String,
    pub code: String,
    pub title: String,
    #[serde(default)] pub badge: Option<String>,
    #[serde(default)] pub duration_sec: Option<i64>,
    #[serde(default)] pub calculator_mode: String,       // default: "non-calculator"
    #[serde(default)] pub default_answer_format: String, // default: "mcq"
    #[serde(default)] pub tier: Option<String>,
    #[serde(default)] pub total_marks: Option<f64>,
    #[serde(default)] pub is_replica: bool,
    #[serde(default)] pub parent_paper_id: Option<String>,
    #[serde(default)] pub sort_order: i64,
    #[serde(default)] pub meta_json: Value,              // default: {} (JSON object)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePaper {
    pub paper_id: String,
    pub code: String,
    pub title: String,
    pub badge: Option<String>,
    pub duration_sec: Option<i64>,
    pub calculator_mode: String,
    pub default_answer_format: String,
    pub tier: Option<String>,
    pub total_marks: Option<f64>,
    pub is_replica: bool,
    pub parent_paper_id: Option<String>,
    pub sort_order: i64,
    pub meta_json: Value,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePaperSectionInput {
    pub section_id: String,
    pub paper_id: String,
    pub section_no: i64,
    pub title: String,
    #[serde(default)] pub instructions: Option<String>,
    #[serde(default)] pub duration_sec: Option<i64>,
    #[serde(default)] pub calculator_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePaperSection {
    pub section_id: String,
    pub paper_id: String,
    pub section_no: i64,
    pub title: String,
    pub instructions: Option<String>,
    pub duration_sec: Option<i64>,
    pub calculator_mode: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseTopicInput {
    pub topic_id: String,
    #[serde(default)] pub parent_topic_id: Option<String>,
    pub category: String,
    pub code: String,
    pub title: String,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub sort_order: i64,
    #[serde(default)] pub meta_json: Value,              // default: {} (JSON object)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseTopic {
    pub topic_id: String,
    pub parent_topic_id: Option<String>,
    pub category: String,
    pub code: String,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub meta_json: Value,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseTopicCardInput {
    pub card_id: String,
    #[serde(default)] pub topic_id: Option<String>,
    #[serde(default)] pub chapter_id: Option<String>,
    pub card_kind: String,
    pub title: String,
    #[serde(default)] pub summary_text: String,
    #[serde(default)] pub min_sublevel: Option<i64>,
    #[serde(default)] pub max_sublevel: Option<i64>,
    pub card_json: Value,
    #[serde(default)] pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseTopicCard {
    pub card_id: String,
    pub topic_id: Option<String>,
    pub chapter_id: Option<String>,
    pub card_kind: String,
    pub title: String,
    pub summary_text: String,
    pub min_sublevel: Option<i64>,
    pub max_sublevel: Option<i64>,
    pub card_json: Value,
    pub sort_order: i64,
    pub has_vector: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseTopicCardSummary {
    pub card_id: String,
    pub topic_id: Option<String>,
    pub chapter_id: Option<String>,
    pub card_kind: String,
    pub title: String,
    pub summary_text: String,
    pub min_sublevel: Option<i64>,
    pub max_sublevel: Option<i64>,
    pub sort_order: i64,
    pub open_comment_count: i64,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseQuestionTypeInput {
    pub type_id: String,
    pub code: String,
    pub title: String,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub default_topic_id: Option<String>,
    #[serde(default)] pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseQuestionType {
    pub type_id: String,
    pub code: String,
    pub title: String,
    pub description: Option<String>,
    pub default_topic_id: Option<String>,
    pub sort_order: i64,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseStimulusInput {
    pub stimulus_id: String,
    pub stimulus_kind: String,
    #[serde(default)] pub title: Option<String>,
    #[serde(default)] pub body_text: String,
    pub stimulus_json: Value,                            // JSON object
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseStimulus {
    pub stimulus_id: String,
    pub stimulus_kind: String,
    pub title: Option<String>,
    pub body_text: String,
    pub stimulus_json: Value,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseQuestionTopicLink {
    pub topic_id: String,
    #[serde(default)] pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseConceptLinkInput {
    pub target_kind: String,
    pub target_id: String,
    pub link_kind: String,
    #[serde(default)] pub step_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseConceptLink {
    pub source_kind: String,
    pub source_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub link_kind: String,
    pub step_hint: Option<String>,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseItemAssetInput {
    pub asset_id: String,
    #[serde(default)] pub role: String,                  // default: "diagram"
    #[serde(default)] pub part_id: String,               // default: ""
    #[serde(default)] pub step_id: String,               // default: ""
    #[serde(default)] pub sublevel: i64,                 // default: 0 (0 = stem / all sublevels, 1..=9 = sublevel)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseItemAsset {
    pub asset_id: String,
    pub blob_id: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub media_kind: String,
    pub mime_type: String,
    pub original_filename: Option<String>,
    pub width_px: Option<i64>,
    pub height_px: Option<i64>,
    pub duration_ms: Option<i64>,
    pub alt_text: Option<String>,
    pub caption_text: Option<String>,
    pub captions_json: Option<Value>,
    pub role: String,
    pub part_id: String,
    pub step_id: String,
    pub sublevel: i64,
}

/// Also implements `Default` (`container_kind: "paper"`, `seq_no: 1`, `answer_format: "mcq"`,
/// `difficulty: 3`, `marks: 1.0`, `calculator_mode: "non-calculator"`, `part_count: 1`,
/// `step_count: 1`, `min_sublevel: 1`, `max_sublevel: 9`, `qa_status: "verified"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseQuestionInput {
    pub question_id: String,
    pub container_kind: String,
    #[serde(default)] pub paper_id: Option<String>,
    #[serde(default)] pub section_id: Option<String>,
    #[serde(default)] pub chapter_id: Option<String>,
    #[serde(default)] pub stimulus_id: Option<String>,
    #[serde(default)] pub seq_no: i64,
    pub label: String,
    pub title: String,
    #[serde(default)] pub answer_format: String,
    #[serde(default)] pub question_type_id: Option<String>,
    #[serde(default)] pub difficulty: i64,
    #[serde(default)] pub tier: Option<String>,
    #[serde(default)] pub marks: f64,
    #[serde(default)] pub calculator_mode: String,
    #[serde(default)] pub recommended_time_sec: Option<i64>,
    #[serde(default)] pub part_count: i64,
    #[serde(default)] pub step_count: i64,
    #[serde(default)] pub min_sublevel: i64,
    #[serde(default)] pub max_sublevel: i64,
    #[serde(default)] pub is_replica: bool,
    #[serde(default)] pub parent_question_id: Option<String>,
    #[serde(default)] pub qa_status: String,
    pub stem_text: String,
    pub question_json: Value,                            // JSON object
    pub solution_json: Value,                            // JSON object
    #[serde(default)] pub topics: Vec<CourseQuestionTopicLink>,
    #[serde(default)] pub concept_links: Vec<CourseConceptLinkInput>,
    #[serde(default)] pub item_assets: Vec<CourseItemAssetInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseQuestion {
    pub question_id: String,
    pub container_kind: String,
    pub paper_id: Option<String>,
    pub section_id: Option<String>,
    pub chapter_id: Option<String>,
    pub stimulus_id: Option<String>,
    pub seq_no: i64,
    pub label: String,
    pub title: String,
    pub answer_format: String,
    pub question_type_id: Option<String>,
    pub difficulty: i64,
    pub tier: Option<String>,
    pub marks: f64,
    pub calculator_mode: String,
    pub recommended_time_sec: Option<i64>,
    pub part_count: i64,
    pub step_count: i64,
    pub min_sublevel: i64,
    pub max_sublevel: i64,
    pub is_replica: bool,
    pub parent_question_id: Option<String>,
    pub qa_status: String,
    pub stem_text: String,
    pub question_json: Value,
    pub solution_json: Value,
    pub has_vector: bool,
    pub open_comment_count: i64,
    pub topics: Vec<CourseQuestionTopicLink>,
    pub concept_links: Vec<CourseConceptLink>,
    pub assets: Vec<CourseItemAsset>,
    pub created_at: u64,
    pub created_by: String,
    pub updated_at: u64,
    pub updated_by: String,
    pub version: i64,
    pub hlc: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseQuestionRevision {
    pub id: String,
    pub question_id: String,
    pub version: i64,
    pub question_json: Value,
    pub solution_json: Value,
    pub replaced_at: u64,
    pub replaced_by: String,
    pub hlc: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CourseQuestionFilter {
    #[serde(default)] pub paper_id: Option<String>,
    #[serde(default)] pub section_id: Option<String>,
    #[serde(default)] pub chapter_id: Option<String>,
    #[serde(default)] pub topic_id: Option<String>,
    #[serde(default)] pub question_type_id: Option<String>,
    #[serde(default)] pub difficulty: Option<i64>,
    #[serde(default)] pub answer_format: Option<String>,
    #[serde(default)] pub qa_status: Option<String>,
    #[serde(default)] pub is_replica: Option<bool>,
    #[serde(default)] pub has_open_comments: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseQuestionSummary {
    pub question_id: String,
    pub container_kind: String,
    pub paper_id: Option<String>,
    pub section_id: Option<String>,
    pub chapter_id: Option<String>,
    pub stimulus_id: Option<String>,
    pub seq_no: i64,
    pub label: String,
    pub title: String,
    pub answer_format: String,
    pub question_type_id: Option<String>,
    pub difficulty: i64,
    pub tier: Option<String>,
    pub marks: f64,
    pub calculator_mode: String,
    pub recommended_time_sec: Option<i64>,
    pub part_count: i64,
    pub step_count: i64,
    pub min_sublevel: i64,
    pub max_sublevel: i64,
    pub is_replica: bool,
    pub parent_question_id: Option<String>,
    pub qa_status: String,
    pub primary_topic_id: Option<String>,
    pub topic_ids: Vec<String>,
    pub open_comment_count: i64,
    pub updated_at: u64,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseCatalogSnapshot {
    pub meta: Option<CourseMeta>,
    pub books: Vec<CourseBook>,
    pub chapters: Vec<CourseChapter>,
    pub papers: Vec<CoursePaper>,
    pub paper_sections: Vec<CoursePaperSection>,
    pub topics: Vec<CourseTopic>,
    pub topic_cards: Vec<CourseTopicCardSummary>,
    pub question_types: Vec<CourseQuestionType>,
    pub questions: Vec<CourseQuestionSummary>,
    pub open_comment_counts: BTreeMap<String, i64>,      // keys: "<target_kind>:<target_id>"
    pub total_open_comments: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseSearchHit {
    pub item_kind: String,                               // "question" | "topic_card"
    pub item_id: String,
    pub title: String,
    pub label: Option<String>,
    pub snippet: String,
    pub paper_id: Option<String>,
    pub chapter_id: Option<String>,
    pub topic_id: Option<String>,
    pub difficulty: Option<i64>,
    pub score: f64,
    pub vector_rank: Option<usize>,
    pub keyword_rank: Option<usize>,
    pub distance: Option<f64>,
    pub used_vector: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseAssetInput {
    pub asset_id: String,
    pub media_kind: String,
    pub mime_type: String,
    #[serde(default)] pub original_filename: Option<String>,
    #[serde(default)] pub width_px: Option<i64>,
    #[serde(default)] pub height_px: Option<i64>,
    #[serde(default)] pub duration_ms: Option<i64>,
    #[serde(default)] pub alt_text: Option<String>,
    #[serde(default)] pub caption_text: Option<String>,
    #[serde(default)] pub captions_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseAsset {
    pub asset_id: String,
    pub blob_id: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub chunk_count: i64,
    pub media_kind: String,
    pub mime_type: String,
    pub original_filename: Option<String>,
    pub width_px: Option<i64>,
    pub height_px: Option<i64>,
    pub duration_ms: Option<i64>,
    pub alt_text: Option<String>,
    pub caption_text: Option<String>,
    pub captions_json: Option<Value>,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: i64,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentAttachmentBytes {
    #[serde(default)] pub attachment_id: Option<String>, // None / empty => auto-generated UUIDv7
    #[serde(default)] pub source: String,                // default: "board-capture"
    #[serde(default)] pub pane: Option<String>,
    pub mime_type: String,
    #[serde(default)] pub filename: Option<String>,
    #[serde(default)] pub width_px: Option<i64>,
    #[serde(default)] pub height_px: Option<i64>,
    #[serde(default)] pub sort_order: i64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseCommentAttachment {
    pub attachment_id: String,
    pub comment_id: String,
    pub blob_id: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub source: String,
    pub pane: Option<String>,
    pub mime_type: String,
    pub filename: Option<String>,
    pub width_px: Option<i64>,
    pub height_px: Option<i64>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseCommentInput {
    #[serde(default)] pub comment_id: Option<String>,    // None / empty => auto-generated UUIDv7
    pub target_kind: String,
    pub target_id: String,
    #[serde(default)] pub part_id: Option<String>,
    #[serde(default)] pub step_id: Option<String>,
    #[serde(default)] pub sublevel: Option<i64>,
    #[serde(default)] pub comment_kind: String,          // default: "fix"
    pub comment_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseComment {
    pub comment_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub part_id: Option<String>,
    pub step_id: Option<String>,
    pub sublevel: Option<i64>,
    pub comment_kind: String,
    pub status: String,
    pub comment_text: String,
    pub resolution_note: Option<String>,
    pub resolved_at: Option<u64>,
    pub attachments: Vec<CourseCommentAttachment>,
    pub created_at: u64,
    pub created_by: String,
    pub updated_at: u64,
    pub updated_by: String,
    pub version: i64,
    pub hlc: String,
}

// Exact L7 methods on `memory_core_rs::Memory`:
impl Memory {
    // 1. Course Metadata & Containers
    pub fn set_course_meta(&mut self, actor: &str, client_id: Option<&str>, meta: &CourseMetaInput) -> Result<()>;
    pub fn course_meta(&self) -> Result<Option<CourseMeta>>;

    pub fn upsert_book(&mut self, actor: &str, client_id: Option<&str>, input: &CourseBookInput) -> Result<()>;
    pub fn list_books(&self) -> Result<Vec<CourseBook>>;
    pub fn delete_book(&mut self, actor: &str, client_id: Option<&str>, book_id: &str) -> Result<bool>;

    pub fn upsert_chapter(&mut self, actor: &str, client_id: Option<&str>, input: &CourseChapterInput) -> Result<()>;
    pub fn list_chapters(&self, book_id: Option<&str>) -> Result<Vec<CourseChapter>>;
    pub fn delete_chapter(&mut self, actor: &str, client_id: Option<&str>, chapter_id: &str) -> Result<bool>;

    pub fn upsert_paper(&mut self, actor: &str, client_id: Option<&str>, input: &CoursePaperInput) -> Result<()>;
    pub fn get_paper(&self, paper_id: &str) -> Result<Option<CoursePaper>>;
    pub fn list_papers(&self) -> Result<Vec<CoursePaper>>;
    pub fn delete_paper(&mut self, actor: &str, client_id: Option<&str>, paper_id: &str) -> Result<bool>;

    pub fn upsert_paper_section(&mut self, actor: &str, client_id: Option<&str>, input: &CoursePaperSectionInput) -> Result<()>;
    pub fn list_paper_sections(&self, paper_id: &str) -> Result<Vec<CoursePaperSection>>;
    pub fn delete_paper_section(&mut self, actor: &str, client_id: Option<&str>, section_id: &str) -> Result<bool>;

    // 2. Topics, Topic Cards, Question Types & Shared Stimuli
    pub fn upsert_topic(&mut self, actor: &str, client_id: Option<&str>, input: &CourseTopicInput) -> Result<()>;
    pub fn list_topics(&self) -> Result<Vec<CourseTopic>>;
    pub fn delete_topic(&mut self, actor: &str, client_id: Option<&str>, topic_id: &str) -> Result<bool>;

    pub fn upsert_topic_card(&mut self, actor: &str, client_id: Option<&str>, input: &CourseTopicCardInput, vector: Option<&[f32]>) -> Result<()>;
    pub fn get_topic_card(&self, card_id: &str) -> Result<Option<CourseTopicCard>>;
    pub fn list_topic_cards(&self, topic_id: Option<&str>, kind: Option<&str>) -> Result<Vec<CourseTopicCard>>;
    pub fn delete_topic_card(&mut self, actor: &str, client_id: Option<&str>, card_id: &str) -> Result<bool>;

    pub fn upsert_question_type(&mut self, actor: &str, client_id: Option<&str>, input: &CourseQuestionTypeInput) -> Result<()>;
    pub fn list_question_types(&self) -> Result<Vec<CourseQuestionType>>;
    pub fn delete_question_type(&mut self, actor: &str, client_id: Option<&str>, type_id: &str) -> Result<bool>;

    pub fn upsert_stimulus(&mut self, actor: &str, client_id: Option<&str>, input: &CourseStimulusInput) -> Result<()>;
    pub fn get_stimulus(&self, stimulus_id: &str) -> Result<Option<CourseStimulus>>;
    pub fn list_stimuli(&self) -> Result<Vec<CourseStimulus>>;
    pub fn delete_stimulus(&mut self, actor: &str, client_id: Option<&str>, stimulus_id: &str) -> Result<bool>;

    // 3. Questions, Revisions, Fast Left-Band Catalog & Hybrid RRF Search
    pub fn upsert_question(&mut self, actor: &str, client_id: Option<&str>, input: &CourseQuestionInput, vector: Option<&[f32]>) -> Result<()>;
    pub fn get_question(&self, question_id: &str) -> Result<Option<CourseQuestion>>;
    pub fn get_question_revisions(&self, question_id: &str) -> Result<Vec<CourseQuestionRevision>>;
    pub fn delete_question(&mut self, actor: &str, client_id: Option<&str>, question_id: &str) -> Result<bool>;

    pub fn query_question_summaries(&self, filter: &CourseQuestionFilter) -> Result<Vec<CourseQuestionSummary>>;
    pub fn course_catalog_snapshot(&self) -> Result<CourseCatalogSnapshot>;
    pub fn search_course(&self, query: &str, filter: &CourseQuestionFilter, limit: usize) -> Result<Vec<CourseSearchHit>>;

    // 4. Content-Addressed Media Assets (3-Way Ref-Counted `blobs`)
    pub fn put_course_asset(&mut self, actor: &str, client_id: Option<&str>, meta: &CourseAssetInput, bytes: &[u8]) -> Result<String>;
    pub fn get_course_asset_info(&self, asset_id: &str) -> Result<Option<CourseAsset>>;
    pub fn read_course_asset_bytes(&self, asset_id: &str) -> Result<Vec<u8>>;
    pub fn delete_course_asset(&mut self, actor: &str, client_id: Option<&str>, asset_id: &str) -> Result<bool>;

    // 5. Authoring Comments + Screenshot Attachments & Clean Release Export
    pub fn upsert_course_comment(&mut self, actor: &str, client_id: Option<&str>, input: &CourseCommentInput, new_attachments: &[CommentAttachmentBytes]) -> Result<String>;
    pub fn list_course_comments(&self, status: Option<&str>, target_id: Option<&str>) -> Result<Vec<CourseComment>>;
    pub fn read_comment_attachment_bytes(&self, attachment_id: &str) -> Result<Vec<u8>>;
    pub fn resolve_course_comment(&mut self, actor: &str, client_id: Option<&str>, comment_id: &str, resolution_note: Option<&str>) -> Result<bool>;
    pub fn delete_course_comment(&mut self, actor: &str, client_id: Option<&str>, comment_id: &str) -> Result<bool>;

    pub fn export_clean_course_bundle(&self, out_bundle_path: &Path, lesson_key_passphrase: &str) -> Result<()>;
}
```

### 5.3 Consent: clients and scopes

The core, not the file, enforces consent (rule B10). A `Memory` opened without a client acts as the file's owner and is unrestricted; that is how the CLI and the settings screens work. An application registers itself once, `register_client("Visage", "application", "user")`, keeps the returned id, and calls `set_client(Some(id))` after every open. From then on every read and write is checked against the client's unrevoked scopes and refused with `Error::Consent` when one is missing, and every write is attributed to that client in provenance. The `client_id` parameter on the write methods is for tools acting on behalf of another client; Visage passes `None` everywhere and the session's client is recorded. Scopes are the vocabulary in appendix B; `admin` means everything, including sensitive fields and consent management. The person can revoke a scope or a whole client at any time and the running session loses it at once. Grants and revocations are audited like every other write.

Visage needs: `identity.read`, `identity.write`, `people.read`, `people.write`, `atoms.read`, `atoms.propose`, `atoms.write`, `personality.read`, `personality.write`, `sources.read`, `sources.write`, `conversations.read`, `conversations.write`. It should not hold `admin` in normal operation; the settings forms that touch `sensitive_identity` run as the owner (`set_client(None)`) for the duration of the form and switch back.

### 5.4 Recall and grounding

`recall` runs a vector nearest-neighbour search whose filters mirror the locality view (sensitivity, `is_current` unless history is asked for, atom types expanded through their base) and a keyword search over FTS5, fuses them with reciprocal rank fusion, and loads the hits through the view. With no embedder reachable it runs keyword-only and sets `degraded = true`; the reply should say so (rule C3). Salience is recorded on every hit.

`build_context(query, locality, atom_limit, lexicon_limit)` returns a `GroundingContext`: identity, current facets, people, active lexicon entries and recalled atoms (prepending any currently due reminders up to `now_ms()` before filling remaining slots with recalled atoms), all through the views for that locality, plus `degraded`. `render()` produces the exact prompt lines, golden-tested, so a local and a cloud brain see identical text for the same rows:

```
Person: Alex
language: en-GB
locale: en-GB
profile.preference: British spelling — always
profile.word_approximation: raffs — RAPHS
person: Sam (mother)
memory: Space museum trip — Alex spent the whole afternoon at the space museum and was calm and happy the entire time.
```

If an atom carries `remind_at` or `completed_at`, its rendered memory line appends ` [remind_at: <ISO-8601-UTC>]` or ` [completed_at: <ISO-8601-UTC>]` so the model can distinguish a scheduled reminder from a completed past memory. `render_sections()` gives the same lines split into `person`, `lexicon`, `people` and `memories`, for an application that wants the recalled block beside the question; `render()` is exactly those sections joined, so the golden format is never duplicated. `used_ids()` lists every row that went in, for the Board's "what I used" panel.

### 5.5 The policy kit

The brain signals what it wants remembered with plain lines after its reply (the grammar is a contract asset, `policy/memory-instructions.md`, to paste into the system prompt):

```
REMEMBER: <short title> | <what to remember>
REMEMBER: !<short title> | <what to remember>          leading ! = important, needs confirmation
REMEMBER: <short title> | <what to remember> | <kind>  kind = episodic (default), semantic, routine, milestone
REMIND: [!] <ISO-8601-UTC or epoch_ms> | <short title> | <what to remind>
LEXICON: <entry_type> | <what you observed> | <what it means>
TOOL: <tool_name> | <key>=<value>,<key>=<value>
```

`Memory::process_turn(reply, TurnContext { actor, client_id, source, policy, occurred_at })` strips the lines, parses them strictly (a malformed line is dropped, never repaired; at most 3 memory/reminder and 3 lexicon proposals per turn; fields capped at 2000 characters), refuses anything that looks like a secret or a payment card number, resolves the kind (`REMIND:` resolves to `routine` with `remind_at` and `occurred_at` set to the parsed timestamp; `REMEMBER:` uses the third part when the line names one or `episodic` otherwise; an unknown kind is dropped with a reason), routes every proposal under the recording policy, saves what is allowed as the companion (never as the person, `origin_ref = companion`), and returns a `TurnOutcome`: the clean `reply` to speak, `saved` ids, `pending_memories` and `pending_lexicon` for confirmation cards, `skipped` duplicates, `dropped` with reasons, `tools`, and how many lines were dropped. `confirm_pending` / `confirm_memory` and `confirm_lexicon` write a pending item when the person says yes; the card may change a pending memory's `atom_type` first.

| `RecordingPolicy` | An event (`episodic`) | A fact, routine or milestone, a `REMIND:`, a confirm topic, or a `!` |
|---|---|---|
| `Silent` | Saved in the background | Saved in the background |
| `ConfirmImportant` (default) | Saved in the background | Held for the person or guardian |
| `ConfirmAll` | Held | Held |
| `Off` | Dropped | Dropped |

Confirm topics are the contract's word list (`policy/confirm-topics.json`: health, family status, money, legal; whole-word, case-insensitive), because small models rarely write a kind or a `!` and an allergy or a child's birth must raise a card. Before routing, the core compares a proposal with the current memories and the queue by fixed rules (conformance I2: identical content to any current memory is a duplicate whatever the kinds; near matches count within the kind for an explicit kind and across every memory for a kind-less line; only a line the brain marks `episodic` may repeat an event on another day) and reports a duplicate in `TurnOutcome.skipped` instead of storing it; an explicit "remember this" from the person is never deduplicated. Held proposals are stored in the `pending_proposals` table: once, across restarts, read with `pending_proposals()`, resolved with `confirm_pending` or `dismiss_pending`, never re-queued while open nor for a day after a dismissal. Visage renders its cards from that queue, not from the last turn.

Lexicon proposals are **always** held for confirmation, whatever the policy, because a wrong guess about what someone means is harmful (ADR 0016).

### 5.6 Packagings, for consumers that are not Rust

Visage links `memory_core_rs` directly, while non-Rust consumers use one of the thin packagings built over `memory_core_rs::facade` (JSON in, JSON out, `Kind` error classification: `Argument`, `Io`, `Engine`, `Key`, `Format`, `Consent`, `Panic`, `Refused`):

- **C ABI (`include/memory_raphs.h`, built as `cdylib` and `staticlib`):**
  - `memory_raphs_version()`, `memory_raphs_last_error()`, `memory_raphs_string_free(ptr)`
  - `memory_raphs_create(path, passphrase, device_id, out_handle, out_recovery)`
  - `memory_raphs_open(path, passphrase, device_id, out_handle)` and `memory_raphs_close(handle)`
  - `memory_raphs_remember(handle, actor, json, out_id)` (accepts `AtomInput` JSON including `remind_at` / `completed_at`, plus optional `vector` and `source`)
  - `memory_raphs_recall(handle, json, out_json)`
  - `memory_raphs_forget(handle, actor, atom_id, out_removed)`
  - `memory_raphs_status(handle, out_json)`
  - `memory_raphs_context(handle, query, locality, atom_limit, lexicon_limit, out_json)`
  - `memory_raphs_course_op(handle, op, json, out_json)` (executes any L7 `facade::course_op_json` operation)
- **UniFFI (`cargo build --features uniffi` → Swift, Kotlin, Python):**
  - Exposes `MemoryFile` (`open`, `remember`, `recall`, `forget`, `status`, `context`, `course_op`) plus top-level `create_memory`, `version`, `supported_schema_versions`, and `MemoryError`.
- **Python (`services/memory-core-py`, PyO3 `abi3` wheel `import memory_raphs`):**
  - Exposes `MemoryFile` (`create`, `open`, `remember`, `recall`, `forget`, `status`, `context`, `course_op`, `course_catalog_snapshot`, `put_course_asset`, `read_course_asset_bytes`, `export_clean_course_bundle`, `close`).
  - Pre-built wheels for macOS (`aarch64`, `x86_64`), Windows (`x86_64`), and Linux (`x86_64`, `aarch64`) published on every `memory-core-*` GitHub Release.

---

## 6. Where Visage sits

### 6.1 Depending on the crate

```toml
[dependencies]
memory_core_rs = { git = "ssh://git@github.com/raphs-app/raphs.git", tag = "memory-core-1.7.0" }
```

Visage, Hydrogen, SIGMA, and any Rust consumer pin one number: the release tag on `github.com/raphs-app/raphs`, which is the crate version (`memory-core-1.7.0` is crate `1.7.0`, published at `https://github.com/raphs-app/raphs/releases/tag/memory-core-1.7.0`). The release workflow runs the full contract and engine checks before a tag is published. The file format version (schema `2.3.0`) is written inside every file and reported by `status`; the client reads it, never pins it, and a newer file simply refuses to open with `Error::SchemaVersion`. Pin a tag, never a branch.

Build prerequisites (the engine is compiled from source, once, then cached):

| Platform | Needs |
|---|---|
| macOS | Xcode command line tools |
| Windows | MSVC Build Tools, Perl (Strawberry Perl) and NASM on `PATH` for the vendored OpenSSL |
| Linux | a C compiler and Perl |

A library build compiles 74 crates in total, with no TLS stack, no argument parser and no Python; the crate readme lists every direct dependency and why it is there.

`.github/workflows/memory-core.yml` in the RAPHS repo is a working setup for Linux and Windows; macOS is built and tested on the developer's machine.

### 6.2 Start-up

1. Read or create the device id (4.4).
2. Read the passphrase from the keychain; on first run generate it, `Memory::create`, show the recovery key once with a "I have written it down" confirmation, then `set_identity(display_name, Some("en-GB"), "user")`.
3. Otherwise `Memory::open_with_passphrase(path, passphrase, device_id)`. A `Error::SchemaVersion` means the file is newer than this Visage: tell the person to update Visage; never downgrade a file.
4. Attach the embedder if it answers: `LlamaServer::new("http://localhost:8081", Duration::from_secs(30))`, `healthy()`, `set_embedder`. Without it the memory works keyword-only and `backfill_vectors` catches up later.
5. On first run register the client and grant the scopes in 5.3; every run, `set_client(Some(&visage_client_id))`.
6. Run `self_check` in the background and surface anything other than `ok` in Settings.

### 6.3 One turn, end to end

```rust
use std::time::Duration;
use memory_core_rs::{Locality, Memory, RecordingPolicy, TurnContext, now_ms};

// ground the question for the brain that will answer it
let ctx = memory.build_context(&question, Locality::Local, 5, 10)?;
let system_prompt = format!("{constitution}\n\n{}\n\n{}", memory_core_rs::schema::MEMORY_INSTRUCTIONS, ctx.render());
board.show_used(ctx.used_ids());
if ctx.degraded { board.note("memory is keyword-only until the embedding server is back"); }

// ask the brain, then let the core route what it proposed
let reply = brain.complete(&system_prompt, &history, &question).await?;
let outcome = memory.process_turn(&reply, &TurnContext {
    actor: "user",
    client_id: None,                             // the client set once with set_client is recorded
    source: Some(&format!("visage:session/{session_id}/turn/{turn}")),
    policy: settings.recording_policy,           // RecordingPolicy::ConfirmImportant by default
    occurred_at: Some(now_ms()),
})?;
piper.speak(&outcome.reply);                     // the reply with the signalling lines removed
for p in outcome.pending_memories { board.confirm_card(p); }   // yes -> memory.confirm_pending("user", None, &p.id, None)
for l in outcome.pending_lexicon  { board.confirm_card(l); }   // always a card; yes -> memory.confirm_lexicon(...)
```

Explicit commands bypass the model entirely: "remember this" is `memory.remember("user", None, Some(source), &AtomInput { .. }, None)`; "forget that" is `memory.forget("user", None, &atom_id)`; "what do you remember about X" is `memory.recall(&RecallQuery::new("X"), None)` shown on the Board with ids.

### 6.4 What Visage builds on top

These are product, not memory, and live in Visage:

- The confirmation cards, the Memory Log (provenance, newest first, model-authored first, one-tap delete and edit) and the Board's "what I used" panel.
- The memory switch (off / this session only / always), incognito, and "forget everything" (`Memory::destroy(path)` for the container, `-wal`/`-shm` and sidecar, rule D3).
- Conversation retention controls (Keep ON/OFF via `keep_conversation`, retention days via `set_conversation_retention_days`, section 8).
- The dictation dictionary editor and the Whisper prompt built from it (section 10).
- Key storage in the keychain and the recovery-key screen.

---

## 7. Services Visage needs to run

### 7.1 Embeddings: a second llama-server

| Item | Value |
|---|---|
| Server | `llama-server` (llama.cpp), the same binary family Visage already runs for Gemma; one model per process, so a second instance on its own port |
| Model | `nomic-embed-text-v1.5` GGUF, 768 dimensions; the only production embedder (rule E1); model id and version are recorded in the file |
| Endpoint | `POST /v1/embeddings` over plain HTTP (the core carries no TLS stack; the server is on this machine); the core adds the `search_document: ` and `search_query: ` prefixes itself; health at `GET /health` |
| Where the core looks | The URL passed to `LlamaServer::new`; the CLI defaults to `http://localhost:8081` or `MEMORY_EMBED_URL` |
| When it is down | Everything still works: atoms are stored with pending vectors, recall is keyword-only and says so, `backfill_vectors` completes the vectors later |

Two machines opening the same file against the same model return identical hits for the same question (rule E2). There is no mock embedder anywhere; the first real test is the person's own file.

### 7.2 Keys

The passphrase in the OS keychain; the recovery key on paper. Visage never logs either, never writes them to disk, never puts them on a command line.

### 7.3 Whisper and Piper

Unchanged. One addition: the Whisper prompt built from the dictation dictionary (10.4).

### 7.4 Not needed for the first version

Sync between devices (the clocks and provenance are the foundation; the applier and merge policy are a later decision, rule G), guardian keys (reserved in the sidecar format), any cloud service.

---

## 8. Short-term versus long-term

| Tier | Where | Lifetime | Who decides |
|---|---|---|---|
| Rolling window | RAM, the messages list sent to the brain each turn | This conversation | Code |
| Conversation | The memory, L6: the conversation and its turns, written as they happen | The retention setting (default thirty days), or forever once kept | The memory's rule; the person with "keep this" |
| Atoms and lexicon entries | The memory | Forever, until forgotten | Person explicitly, or the person accepting a proposal (section 9) |

Visage has no session store of its own: resuming a conversation after a restart reads its turns back from the memory. Incognito means no turns are written and no proposals are made, rolling window only.

---

## 9. Who decides what goes where

### 9.1 The three deciders, in strict priority order

1. **The person, explicitly.** "Remember this." "Forget that." "Keep this conversation." "Add X to my dictionary." A settings form. A Board action. These always win and are executed by code with no model in the loop.
2. **Deterministic rules in code.** The session log follows the retention setting. Dictation vocabulary comes only from explicit adds or accepted corrections. `sensitive_identity` is filled only by a form and never leaves the device. Sensitivity decides what a cloud brain may see. Provenance is written on every change. The rules are written down and tested.
3. **The model, as a proposer only.** The brain may suggest memories. It never writes. A proposal is parsed, refused, routed and gated by the core's policy kit; it is saved in the background (routine), held for confirmation (important, and every lexicon entry), or dropped (policy off, malformed, or a secret). Model-authored atoms are stamped `companion` so they can be reviewed and reversed.

### 9.2 The routing table

| What the content is | Goes to | Who may write it | Confirmation | Reaches a cloud brain? |
|---|---|---|---|---|
| Name, pronouns, language | `identity_core` | Person via Settings | n/a | Name and language yes |
| Locale, time zone, preferred name, Visage's own attributes | `identity_facets` | Person via Settings; Visage on a rule | n/a | Per facet sensitivity |
| Address, medical, official ids, emergency contacts, access codes | `sensitive_identity` | Person via a form only | n/a | **Never** |
| A person in the person's life | `people` | Person; model may propose | Always confirm | Display name and relationship only |
| An event with a time | `memory_atoms` `episodic` | Person; model proposes | Routine: silent. Marked `!`: confirm | Unless `local_only` |
| A fact, project context | `memory_atoms` `semantic` | Person; model proposes | Confirm under `ConfirmImportant` | Same |
| A recurring habit | `memory_atoms` `routine` | Person; model proposes | Confirm under `ConfirmImportant` | Same |
| A scheduled reminder (`REMIND:`) | `memory_atoms` (`remind_at` set, `routine`) | Person; model proposes via `REMIND:` | Confirm under `ConfirmImportant` | Same |
| A milestone | `memory_atoms` `milestone` | Person; model proposes with `!` | Confirm | Same |
| A preference or standing instruction | `personality_profile` `preference` | Person; model proposes | Always confirm | Yes, it shapes replies |
| Dictation vocabulary | `personality_profile` `preferred_phrase` / `word_approximation` | Person; accepted corrections | Explicit or one-tap accept | Per entry sensitivity |
| The conversation and its turns | `conversations` + `turns` (L6) | Code, per retention setting (`raphs:conversation_retention_days`) | n/a | No by default (`local_only`); only if set to `normal` |
| A conversation the person keeps forever | `conversations` (`kept = 1`) or `source_documents` (`conversation_export`) | Person ("keep this") | Explicit | Only if sensitivity is `normal` |
| A file or screen glance the person shares | Nowhere by default; `source_documents` if they say keep | Person | Explicit | No |
| Anything arriving through an agent | Same tables, actor `<agent name>`, its own client with its own scopes | Never silent | Always confirm | Same rules |

### 9.3 What never goes in, and the two doors for secrets

- Secrets: passwords, passphrases, API keys, tokens, private keys, PINs, passcodes, door and access codes, sort codes, account and card numbers, long credential strings. The core refuses them on every write path (`Error::Refused`), whether the brain proposed the line or Visage called `remember` for an explicit "remember this". There is no override.
- **Door 1, conversation**, is where they are refused. When Alex says "remember my door code is 2580", nothing is written; Visage's reply points to the form.
- **Door 2, the settings form**, is where a code the person genuinely needs goes: Visage switches to the owner for the length of the form (`set_client(None)`), calls `add_sensitive_field` with type `access_code`, and switches back. The row lives in `sensitive_identity`, encrypted like everything else; no prompt view reads that table, so no brain, local or cloud, ever sees it, and the audit log records that a field exists, never its value.
- **Reveal**: Alex asks "what is my door code" or taps a button; Visage recognises this itself, without the model, asks the operating system to re-authenticate (Touch ID, Windows Hello), reads the row with `list_sensitive_fields`, shows it on screen, and never sends it to speech or to the model.
- Anything from an incognito session, or while the memory switch is off.
- Dictated text. Dictation into other applications is ephemeral; only vocabulary the person chose to learn is kept.
- Other people's sensitive details volunteered in passing: at minimum a confirmation card, by default dropped.
- Transient chatter: the model is told not to propose it, and the core caps proposals per turn so a chatty model cannot flood the log.

### 9.4 Locality

`build_context` takes the brain's locality. `Local` sees `normal` and `local_only` rows; `Cloud` sees `normal` only, and never contact details; `never_prompt` rows are stored and never rendered; `sensitive_identity` is never rendered anywhere. This is enforced by the views, not by Visage code. Visage's job is to pass the right locality and to set `sensitivity` on what it writes (the "local-only by default for" setting in section 11 does that at proposal time).

### 9.5 Actor and pointer conventions

| Field | Visage value |
|---|---|
| actor | `user` for explicit actions, `visage` for rule-driven writes (dictation dictionary, session promotion); the core stamps `companion` on accepted proposals; `<agent-name>` if content ever comes from an agent |
| `origin_ref` on atoms | `user`, `companion` (forced by the core), `dictation`, `import` |
| `source` (into `provenance.source`) | `visage:session/<session-id>/turn/<n>`, so any memory traces back to the moment it came from even after the session is purged |
| `client_id` | Set once per open with `set_client`; pass `None` on every call and the core records the session's client |

### 9.6 Review and undo

- "What do you remember about X?" is a recall shown on the Board with ids.
- "Forget that" is `forget`: the atom, its revisions, links, salience, vector and index row go; provenance keeps that a deletion happened, with the clock.
- The Memory Log lists provenance newest first with one-tap delete and edit (an edit is a supersede).
- "Forget everything" is `Memory::destroy(path)` (removes the container, `-wal`/`-shm`, and sidecar).

### 9.7 A caveat about small models

Gemma-class models will sometimes emit a `REMEMBER` line for something trivial, mis-title it, or put a reply sentence into the signalling area. The defences are in the core: strict parsing, the per-turn caps, refusals, `ConfirmImportant` as the default, and the model never choosing `origin_ref`, the actor, or an importance above the default. If the proposals are noisy, the person turns the policy `Off` and keeps explicit "remember this", which needs no model at all.

---

## 10. The dictation dictionary

### 10.1 What it is

A per-person vocabulary that makes dictation and replies spell things the person's way: names of people and products, project jargon, non-English words in English sentences, house spellings, and "when I say X, write Y" corrections. Per language, optionally per scope.

### 10.2 Where it lives: L4, two entry types

| Entry type | `observation` | `meaning` | `language` | `context` | Use |
|---|---|---|---|---|---|
| `preferred_phrase` | The written form wanted: `Raphs`, `Visage` | Optional gloss | `en-GB` | `global` or an application id | Bias Whisper; teach the brain the spelling |
| `word_approximation` | What Whisper tends to produce: `raffs` | The intended form: `RAPHS` | `en-GB` | Same | Deterministic replacement after transcription |

`language` is a real column in 2.0 (a BCP-47 tag), so no tag convention is needed. `is_active` switches an entry off without deleting it; `frequency` and `last_observed` record use; `confidence` is 1.0 for person-added entries and lower for suggested-then-accepted ones. A correction to an existing entry is a supersede, never an edit, so the history of "how did Alex want this spelt" is kept. Lookups scan the active entries, which is fine up to a few hundred; if a dictionary outgrows that, Visage asks RAPHS for a dedicated table in a schema point release rather than forking one.

### 10.3 How entries get in

Only these three ways; the model never adds one silently.

1. **Explicit.** "Add Syngenta to my dictionary." An editor on the Board or in Settings. Import from a text list.
2. **Accepted correction.** The person fixes a dictated word on the Board (or Visage sees the same correction twice). Visage asks once: "Always write it as *RAPHS*?" Yes creates a `word_approximation` entry.
3. **Accepted model suggestion.** A `LEXICON: preferred_phrase | <term> | <gloss>` line is held for confirmation like every lexicon proposal.

The actor is `user` for 1 and 2; the core stamps 3 as `companion`.

### 10.4 How entries are used

1. **Whisper biasing.** Before each transcription, Visage builds a short comma-separated list of active `preferred_phrase` terms for the selected language, most recently used first, capped to fit the prompt window, and sends it as the `prompt` field. This is the single biggest accuracy win for proper nouns.
2. **Post-transcription correction.** Apply `word_approximation` entries as whole-word, case-preserving replacements before the text is inserted anywhere. Deterministic, no model.
3. **Brain context.** `build_context` already renders active entries as `profile.preferred_phrase: ...` lines, so replies and Board drafts match. Entries marked `local_only` stay with the local brain.
4. **Piper.** Nothing initially.

### 10.5 What is not stored

The dictated text itself. The only durable trace of dictation is a vocabulary entry the person explicitly accepted.

---

## 11. Visage settings this implies

| Setting | Values | Effect |
|---|---|---|
| Memory | off / this session only / always | Gates every write; "this session only" keeps the rolling window and nothing else |
| Incognito | on / off | No session log, no proposals, nothing written |
| Conversations | Keep conversation on / off; keep for N days (default 30); forever | `keep_conversation` per conversation; `set_conversation_retention_days` (0 = forever) writes the `raphs:conversation_retention_days` facet, applied by the memory on open; kept conversations are exempt |
| Recording policy | silent / confirm important / confirm all / off | `RecordingPolicy` passed to `process_turn` |
| Local-only by default for | checklist: people's contact details, health, finances, work project names | Sets `sensitivity = local_only` on proposals in those categories |
| Cloud brain may see memory | yes / no | If no, `build_context(.., Locality::Cloud, ..)` is never called; the cloud brain gets the name and language only |
| Dictionary | editor; per language; per scope; import; export | Section 10 |
| Back up / Export | writes a `.raphs` bundle | Section 4.3 |
| Recovery key | show again (after OS re-authentication) | Section 4.2 |
| Codes and PINs | a form (add, reveal after OS re-authentication, remove) | Section 9.3: `access_code` sensitive fields; never in conversation |
| Forget everything | one action, typed confirmation | `Memory::destroy(path)`: deletes container, `-wal`/`-shm`, and sidecar |

---

## 12. Integration plan (Stage 4 of the build plan)

There is no fixture and no intermediate stage. The first real file is the developer's own, made on macOS and on Windows through Visage, with the CLI as the reference client beside it.

| Step | Visage | Check |
|---|---|---|
| 1 Build | Add the dependency; build on macOS and Windows (6.1) | `cargo build` green on both; `memory engines` from the crate prints the pinned versions |
| 2 First file | Start-up sequence (6.2): keychain, create, recovery key, identity, client, scopes | `memory status <file>` and `memory check <file>` from the CLI report `ok`; `memory clients` shows Visage and its scopes |
| 3 Read path | `build_context` into the system prompt; Board shows `used_ids`; degraded notice | A question about something remembered through the CLI grounds a Visage reply |
| 4 Write path | `process_turn`, confirmation cards, explicit remember / forget, Memory Log | Atoms written by Visage appear in `memory atoms` and `memory recall` from the CLI with complete provenance |
| 5 Embeddings | Second llama-server; `set_embedder`; `backfill_vectors` on reconnect | `memory backfill` reports nothing pending; recall order is identical from Visage and from the CLI |
| 6 Dictionary and conversations | Editor, Whisper prompt, correction pass; L6 conversations (`start_conversation`, `append_turn`, `keep_conversation`) | Entries render in `memory context`; conversations and turns appear in `memory conversations` |
| 7 Bundle | Back up / Export with `pack`; restore with `unpack` | A bundle made on the Mac opens on Windows with the same passphrase, and the other way round |

Exit: the developer uses Visage with their own memory on both machines for a period, and every problem found is reproduced as a failing test in the core or the contract before it is fixed.

---

## 13. Decisions

Answered by Visage on 2026-09-25 against the hand-over prompt:

1. **Conversations live inside the memory** (decision 1 reversed by Raphael on 2026-09-28, schema 2.1.0): one store, one key, one security story; Visage writes each turn as it happens and keeps no session store of its own.
2. **Recovery key** shown the first time it matters, re-shown after OS re-authentication.
3. **File layout** as section 4.4; **single profile** for the first version.
4. **No `visage:` facet namespace and no vocabulary additions** for the first version.
5. **Threading**: `Mutex<Memory>` in Tauri state.
6. **Release cadence**: Visage pins `memory-core-*` tags and is told by message when one is cut.

Still open:

- **Sync** between two devices. Out of scope until one person uses two devices with Visage; the clocks and provenance are already in every row, the applier and the merge policy are the open design (build plan 4.5).

---

## Appendix A. Where to look in the RAPHS repo

| Topic | Path |
|---|---|
| The decision | `docs/adr/0026-memory-core-module.md` |
| The plan and its checklist | `docs/vision/memory-build-plan.md` |
| Table DDL (generated) and its generator | `services/memory-format/schema.sql`, `services/memory-format/tools/gen_schema.py` |
| Field-level contracts | `services/memory-format/schemas/*.v2.schema.json`, `keys.v1.schema.json`, `manifest.v2.schema.json`, `facet_registry.v1.schema.json` |
| Rules a binding must honour | `services/memory-format/conformance.md` |
| File, key sidecar and bundle formats | `services/memory-format/formats.md` |
| Engine and embedding pins | `services/memory-format/engine.json`, `services/memory-format/embedding.json` |
| Facet registry | `services/memory-format/facets/registry.json` |
| Signalling grammar for the system prompt | `services/memory-format/policy/memory-instructions.md` (embedded as `schema::MEMORY_INSTRUCTIONS`) |
| Migrations | `services/memory-format/migrations/` |
| The core | `services/memory-core-rs/src/` (`store`, `atoms`, `recall`, `grounding`, `policy`, `identity`, `people`, `lexicon`, `sources`, `conversations`, `course`, `bundle`, `check`, `keys`, `engine`, `ffi`, `uniffi_api`, `facade`) |
| Crate readme, public surface and compatibility table | `services/memory-core-rs/readme.md` |
| The reference client | `services/memory-core-rs/src/bin/memory.rs` |
| C header | `services/memory-core-rs/include/memory_raphs.h` |
| Python wheel | `services/memory-core-py/` |
| CI and release | `.github/workflows/memory-core.yml`, `.github/workflows/memory-release.yml` |
| Earlier decisions still in force | ADR 0002 (L1 and the data contract), 0003 (memory library), 0005 (bundle), 0013 §10 (proactive recording), 0016 (communication lexicon), 0017 §6 (confirm before save), 0018 (native inference behind seams) |

## Appendix B. Vocabulary (core values, schema 2.3.0)

- `sensitivity`: `normal`, `local_only`, `never_prompt`
- `atom_type`: `episodic`, `semantic`, `routine`, `milestone`
- `personality_profile.entry_type`: `echolalia_script`, `preferred_phrase`, `aac_symbol`, `gesture`, `vocal_pattern`, `word_approximation`, `social_script`, `routine`, `coping_strategy`, `behavioural_pattern`, `sensory_response`, `emotional_expression`, `preference`
- `personality_profile.frequency`: `daily`, `weekly`, `occasionally`, `rarely`, `once`
- `sensitive_identity.field_type`: `emergency_contact`, `medical_id`, `address`, `nhs_number`, `gp_surgery`, `medication`, `allergy`, `access_code`
- `people.trust_scopes[]`: `medical`, `financial`, `education`, `daily_care`, `emergency_decision`, `personal`
- `source_documents.document_type`: `photo`, `video`, `audio`, `medical_report`, `school_report`, `therapy_report`, `official_document`, `personal_document`, `conversation_export`, `other`
- `consent_scopes`: `identity.read`, `identity.write`, `people.read`, `people.write`, `atoms.read`, `atoms.propose`, `atoms.write`, `personality.read`, `personality.write`, `sources.read`, `sources.write`, `conversations.read`, `conversations.write`, `course.read`, `course.write`, `admin`
- `clients.kind`: `application`, `agent`, `service`
- `turns.role`: `person`, `companion`, `system`
- `provenance.action`: `create`, `update`, `delete`
- `RecordingPolicy`: `silent`, `confirm_important`, `confirm_all`, `off`
- facets shipped by the contract (`raphs` namespace): `locale`, `timezone`, `preferred_name`, `conversation_retention_days`
- L7 `course_*` `CHECK` domains (schema `2.3.0`):
  - `answer_format` / `default_answer_format`: `mcq`, `written`, `multi_select`, `interactive`, `spoken`, `drawing`
  - `calculator_mode`: `non-calculator`, `allowed`
  - `course_topic_cards.card_kind`: `concept`, `worked_example`, `formula`, `reference_sheet`, `prerequisite_refresher`
  - `course_stimuli.stimulus_kind`: `passage`, `dataset`, `diagram`, `experiment`, `code_key`, `audio_clip`, `video_clip`
  - `course_questions.container_kind`: `paper`, `chapter`, `topic`
  - `course_questions.qa_status`: `draft`, `verified`, `needs_review`, `approved`
  - `course_concept_links.source_kind`: `question`, `topic`, `topic_card`; `target_kind`: `topic`, `topic_card`; `link_kind`: `prerequisite`, `concept`, `formula`, `worked_example`
  - `course_media_assets.media_kind`: `image`, `svg`, `audio`, `video`, `animation`, `other`
  - `course_item_assets.item_kind`: `question`, `topic_card`, `stimulus`
  - `course_comments.target_kind`: `question`, `paper`, `book`, `chapter`, `topic`, `topic_card`; `comment_kind`: `fix`, `replica`, `topic`, `note`, `engine`; `status`: `open`, `resolved`
  - `course_comment_attachments.source`: `board-capture`, `upload`, `clipboard`

Consumer values on extensible `vocab_*` tables are `namespace:value` with a core base, added per file; they behave as their base everywhere.

## Appendix C. The reference client

`memory` is the crate's binary, built with `cargo build --release --features cli` (the feature keeps its argument parser out of a library consumer's build). Every public operation is reachable from it, so anything Visage, Hydrogen, or SIGMA writes can be inspected, and anything the CLI writes can be read by them.

```
memory engines            the bundled engine versions, checked against engine.json
memory create             a new container and sidecar; prints the recovery key once
memory status             versions, counts, clock horizon, integrity, as JSON
memory check              integrity, foreign keys, the contract's structure, sidecar match
memory passphrase         change the passphrase (sidecar only, or --recovery)
memory remember           store an atom (--remind-at <ISO-8601-or-epoch> for a reminder); embeds when server answers
memory recall             vector + keyword search fused; keyword-only when the server is down
memory forget             erase an atom
memory atoms              list atoms
memory due-reminders      list active reminders due at or before --until (default now)
memory upcoming-reminders list active reminders scheduled after --after
memory complete-reminder  mark a reminder completed (sets completed_at, clears remind_at)
memory snooze-reminder    reschedule a reminder to a new remind_at timestamp
memory cancel-reminder    clear remind_at on an atom without deleting it
memory backfill           embed atoms stored while the server was unreachable
memory context            what a brain of the given locality would receive for a question
memory turn               process a model reply under the recording policy
memory people · lexicon · facet · sensitive · sources · clients · pending · provenance · conversations
memory course meta             print course_meta as JSON
memory course set-meta         set course_meta from JSON string or @file.json
memory course catalog          print the full Left-Band CourseCatalogSnapshot as JSON
memory course questions        query lightweight CourseQuestionSummary rows (--paper, --chapter, --topic, --question-type, --difficulty, --qa-status)
memory course question         print one complete CourseQuestion (with question_json, solution_json, topics, links, and assets)
memory course upsert-question  upsert a question from JSON string or @file.json (--embed-url)
memory course revisions        print revision snapshots for a question
memory course delete-question  delete a question
memory course search           hybrid RRF search across course questions and topic cards (--limit, --embed-url)
memory course put-asset        store a media file into deduplicated blobs + course_media_assets (--id, --kind, --mime, --file, --alt)
memory course read-asset       write a course media asset's verified bytes to --out
memory course comments         list authoring comments (--status, --target)
memory course resolve-comment  mark an authoring comment resolved (--note)
memory course export-clean     export a clean customer-distributable .raphs course bundle under a lesson key passphrase
memory course op               run any L7 facade::course_op_json operation (<op> [json|@file.json])
memory pack · unpack · manifest
memory rekey              replace the data-encryption key; prints the new recovery key once
memory destroy            forget everything: container, -wal/-shm and sidecar (typed confirmation, or --yes)
memory bench              timings on the live file (read-only) and, with --writes, on a temporary copy
```

When stdin is not a terminal the client reads the passphrase from it, one line per prompt, so a script never puts it on a command line.

`memory --version` prints the crate version; `memory status` prints the schema, container, crypto and engine versions of a file.

## Appendix D. Schema & Crate Release History

| Crate Version | Git Tag | Schema Version | Opens & Auto-Migrates | Summary of Changes |
|---|---|---|---|---|
| `1.0.0` – `1.1.0` | `memory-core-1.1.0` | `2.0.1` | `2.0.0` → `2.0.1` | Initial 2.0 container, sidecar, bundle, L1–L5 tables, additive consent scopes, `render_sections()`, `REMEMBER` kind slot. |
| `1.2.0` | `memory-core-1.2.0` | `2.0.2` | `2.0.0`..`2.0.1` → `2.0.2` | Stemmed FTS5 keyword index (`porter unicode61 remove_diacritics 2`) in `atoms_fts` with in-place rebuild migration. |
| `1.3.0` | `memory-core-1.3.0` | `2.0.3` | `2.0.0`..`2.0.2` → `2.0.3` | Secret detector (`Error::Refused`) on all write paths; `access_code` sensitive field type in `sensitive_identity`. |
| `1.4.0` | `memory-core-1.4.0` | `2.0.4` | `2.0.0`..`2.0.3` → `2.0.4` | `pending_proposals` confirmation queue table, `ConfirmAll`, duplicate detection (`TurnOutcome.skipped`), `change_passphrase_with_recovery`, typed vocabulary errors. |
| `1.5.0` | `memory-core-1.5.0` | `2.1.0` | `2.0.0`..`2.0.4` → `2.1.0` | L6 Conversations (`conversations`, `turns`, `turns_fts`), `prompt_safe_turns_{local,cloud}`, `raphs:conversation_retention_days` facet. |
| `1.5.1` | `memory-core-1.5.1` | `2.1.0` | `2.0.0`..`2.0.4` → `2.1.0` | Refined cross-kind duplicate rules (conformance I2). |
| `1.6.0` | `memory-core-1.6.0` | `2.2.0` | `2.0.0`..`2.1.0` → `2.2.0` | Reminders on `memory_atoms`, `atom_revisions`, and `pending_proposals` (`remind_at`, `completed_at`, `idx_atoms_remind_at`), reminder query/lifecycle API, `REMIND:` signalling, grounding annotations. |
| `1.7.0` | `memory-core-1.7.0` | `2.3.0` | `2.0.0`..`2.2.0` → `2.3.0` | Additive L7 Course & Curriculum Vaults (`course_*` tables, `course_questions_fts`, `course_topic_cards_fts`, `course_embeddings`, `course.read`/`course.write` scopes, 3-way blob reference counting, `course_catalog_snapshot`, RRF `search_course`, comments + attachments, and `export_clean_course_bundle`). |
