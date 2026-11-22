# Hydrogen Pedagogical & Visual Rules System (`rules/`)

This directory holds the **human-authored, machine-enforced rules** governing how **Hydrogen** extracts questions, solves problems with **Theta**, authors **`Level 9 → Level 1`** explanations on **Carbon**, and writes narration cues for **SIGMA**.

## 1. Strict 4-Tier Cascade (Zero Cross-Product Bleed)

Every product (`gyaraplus`, `gcse`, `alevel`, `primary`, `infant`) is **strictly isolated** in its own directory so GCSE rules never bleed into A-Level or 11+. When Hydrogen compiles a course vault (`<product>/<subject>`), `src/rules/` loads **only** this deterministic 4-file chain:

1. **`rules/common.md`** — **Global Common Rules** across *all* products: clarity, simplicity, elegance, visual-first Carbon layout, KaTeX hygiene, glyph-hugging transient clues, and strict source-paper anonymization.
2. **`rules/<product>/common.md`** — **Product Common Rules** shared across all subjects within that exam/stage (e.g., `gyaraplus` 11+ timed MCQ defaults & Year-6 tone; `gcse` command words & `M1/A1` mark style; `alevel` proof rigor & formula booklets).
3. **`rules/<product>/<subject>/common.md`** — **Subject Rules** for that specific product + subject (includes machine-read YAML frontmatter configuring `Theta` presets, default answer formats, allowed/forbidden methods, diagram rules, verification checks, and gold-standard examples).
4. **`rules/<product>/<subject>/levels.md`** *(+ optional tier/class override such as `foundation.md` / `higher.md` or `year-3.md`..`year-6.md`)* — **Subject Level Rules** defining exact behavior across **Explanation Sub-Levels `9 → 1`** and difficulty/tier calibration within that subject.

## 2. Directory Map

```text
rules/
├── README.md
├── common.md                                    # Tier 1: Global Common (all products)
├── gyaraplus/                                   # Product: 11+ Entrance (Gyara)
│   ├── common.md                                # Tier 2: 11+ Common
│   ├── maths/{common.md, levels.md}             # Tier 3 & 4: 11+ Maths (Algebra-First)
│   ├── english/{common.md, levels.md}           # Tier 3 & 4: 11+ English
│   ├── verbal-reasoning/{common.md, levels.md}  # Tier 3 & 4: 11+ VR
│   └── non-verbal-reasoning/{common.md, levels.md} # Tier 3 & 4: 11+ NVR
├── gcse/                                        # Product: UK GCSE (Ages 14–16)
│   ├── common.md
│   ├── maths/{common.md, levels.md, foundation.md, higher.md}
│   └── physics/{common.md, levels.md}
├── alevel/                                      # Product: UK A-Level (Ages 16–18)
│   ├── common.md
│   ├── maths/{common.md, levels.md}
│   └── physics/{common.md, levels.md}
├── primary/                                     # Product: Primary / KS2 (Ages 7–11)
│   ├── common.md
│   └── maths/{common.md, levels.md}
└── infant/                                      # Product: Infant / EYFS–KS1 (Ages 4–7)
    ├── common.md
    └── maths/{common.md, levels.md}
```

## 3. Editing Rules Without Recompiling Rust
All `.md` files in `rules/` are loaded dynamically from disk by Hydrogen's `RuleLoader` (both in CLI mode and in the `axum` Web UI). Whenever you refine a rule after teaching on Carbon, simply edit the relevant `.md` file and re-run the question or comment batch.
