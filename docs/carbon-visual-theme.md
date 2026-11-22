# Carbon Visual Theme, Palette & Solution Authoring Specification (`carbon-visual-theme.md`)

> **Authority:** Shared visual design & authoring contract between **Carbon** (`Carbon Protocol v0.1`), **Hydrogen** (Curriculum & Solution Compiler), and **SIGMA** (Runtime Tutor).  
> Every catalog payload, question stem, diagram, table, callout card, and step/substep solution compiled by **Hydrogen** MUST harmonize with Carbon's unified **Warm-Paper / Clean White Editorial Theme**.

---

## 1. Unified Surface & Chrome Theme (Do Not Fight the Canvas)

Carbon runs a calm, high-contrast light editorial theme throughout both the **Full-Page Course Index** and the **Split Working Whiteboard**:
- **Default Theme:** `"warm-paper"` (or `"classroom-white"`). **NEVER** emit dark-mode canvas fills, dark card backgrounds, or light/pastel/yellow/white text ink—light text is invisible on Carbon's paper surfaces.
- **Canvas Surfaces (Reference Only — Carbon renders these automatically; Hydrogen's own local Web Console mirrors this palette):**
  - **Course Index & Left Question Pane:** Warm Ivory Paper `#FDFBF7`
  - **Right Solution Whiteboard:** Clean Light Ivory `#FDFCF9` with subtle pastel sky-blue grid lines (`#DAE4F0` minor, `#CCDBEC` 5×5 major)
  - **Top Header & 3-Group Bottom Bar Chrome:** Warm Stone `#F2ECE1`, border `#DBD2C2`, stadium pills `#FCF9F2`
  - **Primary Body Ink:** Deep Charcoal `#1F2421` | **Muted Label Ink:** `#68645B`

---

## 2. Course Index, Subject, Paper & Question Cards (`chrome.setCatalog`)

### 2.1 Title-Bar `brandLogo` vs. Individual Per-Subject `logo` (Strictly PNG Only)
- **Format Rule — PNG ONLY:** Every logo (`brandLogo` and `availableSubjects[].logo`) **MUST** be a transparent `2×` Retina **PNG** (`"data:image/png;base64,..."` or a local `.png` path, e.g., `GYR-LOGO-BLUE` from `gyara-blue.png`). **NEVER** send SVG logos (font-based SVGs alter the brand wordmark).
- **Independent Title-Bar & Subject Card Logos (`CATALOG_LOGO_TEX_BASE`):**
  - **`brandLogo` (top-level on `chrome.setCatalog`):** Renders centered in the **Index Title Bar**.
  - **`availableSubjects[i].logo` (per subject):** Cached independently in Carbon's GPU texture table so each subject can optionally display its own distinct PNG emblem on the left side of its Subject card while the Title Bar keeps `brandLogo`:
    - `"logo": "data:image/png;base64,..."` $\to$ Displays **that subject's individual PNG logo** on its card.
    - `"logo": null` (or omitted) $\to$ Falls back to displaying `brandLogo` on that Subject card.
    - `"logo": ""` (empty string) $\to$ Suppresses the `brandLogo` fallback so that Subject card renders **with no logo** (title text shifts cleanly left).
- **Topics & Papers Scope:**
  - PNG logos apply to the **Title Bar (`brandLogo`)** and **Subject Cards (`availableSubjects[].logo`)** only.
  - `topics[]` (`CatalogTopicNode`), `papers[]` (`CatalogPaper`), and `cards[]` (`CatalogConceptCard`) do **not** take a `logo` field—style them using clean titles, `badge` / `tagBadges`, and muted `"color"` accents.

### 2.2 Subject Grid, Blending Math & 12-Subject Muted Mineral Palette (`availableSubjects[].color`)
- **Fixed Non-Stretching Grid:** Carbon renders fixed-size cards (`Subject`: `268×104 pt` in up to `4` columns per row—so `12` subjects render as a centered `4 × 3` grid with vertical scrolling; `Paper`: `282×74 pt`; `Topic`: `376×78 pt`; `Question`: `184×70 pt`).
- **Hydrogen Controls the Accent Colour:**
  - Hydrogen can set `"color": "#RRGGBB"` on `availableSubjects[]`, `topics[]`, `papers[]`, `cards[]`, and `stats` bars/metrics.
  - **Built-in Fallback Limit (`6` colours):** When `"color"` is `null`/omitted, Carbon cycles `idx % 6` (`#5B7DB1`, `#4D8B7A`, `#7A6BA8`, `#A8647A`, `#A37E58`, `#568A9E`). Whenever Hydrogen serves **more than 6 subjects (e.g. 12 GCSE/Primary subjects)**, Hydrogen **MUST** assign an explicit `"color"` to every subject so subjects `7–12` never repeat `1–6`.
- **Automatic Soft-Mineral Blending (How Carbon Renders Your Hex):**
  - Carbon softens `"color"` automatically: `13%` blended into `#FFFFFF` for card fill, `42%` into `#D8E0EA` for card border, `68%` for the left accent bar, and darkened into rich editorial ink for the `Lato-Bold` title.
- **Canonical 12-Subject Distinct Muted Mineral Palette (Never Neon Primaries):**
  1. `#5B7DB1` — Slate Blue *(Maths / Quantitative)*
  2. `#4D8B7A` — Eucalyptus Sage *(English / Literature)*
  3. `#7A6BA8` — Wisteria Lavender *(Verbal Reasoning / Languages)*
  4. `#A8647A` — Dusty Rose *(Non-Verbal Reasoning / Biology / Art)*
  5. `#A37E58` — Warm Ochre *(History / Humanities)*
  6. `#568A9E` — Mist Cyan *(Physics / Computing)*
  7. `#6B8E67` — Moss Olive *(Geography / Environmental Science)*
  8. `#8F6B9E` — Muted Plum *(Chemistry / Further Maths)*
  9. `#B07462` — Soft Terracotta *(Design & Technology / Economics)*
  10. `#5C768D` — Steel Indigo *(Statistics / Mechanics)*
  11. `#8C7D53` — Antique Brass *(Religious Studies / Classics)*
  12. `#4F8A8B` — Deep Seafoam *(Music / Combined Science)*
- Keep `tagBadges` soft and understated (`#6D5CAE` or `#475569`).

### 2.3 4-State Card Semantics (Driven Automatically by Metadata)
- Set `status` strictly to `"default"` (Not Attempted), `"in-progress"`, or `"completed"`:
  - `"default"` $\to$ Pure White `#FFFFFF` (border `#DEE4EC`)
  - `"in-progress"` $\to$ Soft Ice-Slate `#F1F6FD` (border `#BAD4F7`)
  - `"completed"` $\to$ Soft Mint-Sage `#F0FBF4` (border `#B2E8C6`)
- Set `dueReminder: true` when spaced repetition (`reminder`: `"0d" | "1d" | "3d" | "5d" | "7d" | "15d"`) is due $\to$ Carbon renders Soft Rose-Blush `#FFF1F2` (border `#FBABB8`, deep rose title `#881337`).
- Set `importantIcon: "triangle"` (`△` coral `#D92D44` is the standard Important marker across both Index cards and the Board Header).

---

## 3. Solution Ink, Card & Diagram Color Rules (Zero Visual Conflict)

On the `#FDFCF9` squared/plain whiteboard, restrict all equations, diagrams, tables, and callout cards to Carbon's high-contrast editorial ink tokens (`"c1"`..`"c8"` or exact hex equivalents):

| Pedagogical Role | Token / Ink Hex | Soft Card Fill (when boxed) | Card Border |
| :--- | :--- | :--- | :--- |
| **Standard Question Stem & Working Steps** | *(omit `color` $\to$ `#1F2421`)* | *(unboxed clean paper)* | — |
| **Primary Algebraic / Geometric Focus (`c1`)** | `#2563EB` (Royal Blue) | `#EFF6FF` | `#3B82F6` |
| **Final Answer & Correct Checkmarks (`c2`)** | `#15803D` / `#16A34A` (Emerald) | `#F0FDF4` | `#16A34A` |
| **Key Warning / Mistake / Constraint (`c3`)** | `#DC2626` / `#B91C1C` (Crimson) | `#FFF1F2` | `#F87171` |
| **Hint / Method Note / Highlight (`c4`)** | `#B45309` / `#D97706` (Warm Amber) | `#FFFBEB` | `#F59E0B` |
| **Sub-Step Breakdown / Substitution (`c5`)** | `#6D28D9` / `#7C3AED` (Deep Violet) | `#F5F3FF` | `#8B5CF6` |
| **Secondary Diagram / Quantity B (`c6`)** | `#0D9488` (Teal) | `#F0FDFA` | `#14B8A6` |
| **Auxiliary Construction / Neutral (`c8`)** | `#475569` (Slate Graphite) | `#F8FAFC` | `#CBD5E1` |

### Mandatory Anti-Clash Rules for Diagrams, Tables & Callouts
1. **Transparent Labels Only:** Never place opaque white background boxes behind coordinate, vertex, or diagram labels (opaque boxes clip ugly holes over Carbon's `#DAE4F0` grid).
2. **Fine Strokes Only:** Keep diagram/axis/table strokes crisp (`1.0–1.75 pt`); never use thick heavy borders (`> 2.2 pt`) that overpower `Lato` text or KaTeX math.
3. **Clean Table Cell & Sub-Token Highlights:**
   - When highlighting a table cell (`"tbl:r0c2"`) or equation sub-token (`\htmlId{tok_1}{...}`), pair a soft pastel `cellStyles` fill (`#DCFCE7`, `#DBEAFE`, or `#FEF3C7`) with concise `focus.annotate` (`"kind": "check"` or `"label"`).
   - **NEVER** include a literal `✓` character inside `label` when `"kind": "check"` (Carbon already draws a crisp corner tick).
   - Mark intermediate step callouts `"transient": true` (or `edit.erase` them on later steps) so leader lines never pile up.

---

## 4. Typography & LaTeX/KaTeX Discipline

1. **Smooth Sans-Serif Prose (`Lato` / `Lato-Bold`):**
   - Carbon renders all UI chrome in `Lato-Bold` and all `"style": "typeset"` prose in `Lato` (`16.8 pt` base in the Left Question Pane).
   - Always set `"style": "typeset"` for question stems, multi-sentence explanations, breakdown cards, and `"NEVER EVER..."` misconception cards so Carbon wraps paragraphs cleanly.
2. **Strict `$...$` LaTeX Math Everywhere:**
   - Wrap **every** formula, variable, fraction, exponent, angle, ratio, and currency/unit expression in `$...$` or `\(...\)` (e.g. `$\frac{3}{5}$`, `$x^2 + 4x - 12 = 0$`, `$3 : 5$`, `$\pounds 72$`, `$18\text{ cm}^2$`).
   - **NEVER** emit raw ASCII math (`x^2`, `3/5`, `sqrt(2)`, `!=`, `<=`) outside `$...$`.
   - Wrap key target terms in `\htmlId{step2_rhs}{...}` so steps, breakdowns, and `edit.rub` substitutions anchor with pixel precision.

---

## 5. Solution Structure for Carbon's 3-Group Bottom Bar & Keyboard Controls

Carbon's bottom bar has **3 groups** plus hardware keyboard shortcuts:
- **Group 1:** `SESSION` & `ITEM` timers
- **Group 2:** `(a)` `(b)` Part Tabs + `|◀` `◀` `Step i / N` `▶` (`Left` / `Right` Arrow keys) + `Break step down` + `Reveal Answer`
- **Group 3:** `◀ Back` & `Next ▶` (`Shift + Left` / `Shift + Right` Arrow keys — switches **directly between questions**, while `(a)`/`(b)` pills switch between parts)

Every question & solution compiled by Hydrogen structures each part (`a`, `b`, ...) with:
1. **`questionStem`** (`pane: "left"`, `zone: "spine"`, plus any stimulus diagram/image in `zone: "diagram"`).
2. **`steps[]` (`totalSteps: N`, `0`-indexed `activeStep`):** Concise, aligned vertical whiteboard steps (`anchor: { "mode": "below", "targetId": "...", "align": "equals" }`) streamed when Carbon emits `"step.navigated"` (`action`: `"first" | "back" | "next"`), with each step also storing **Question-Relative `sublevels` (`9 → 1`)** for adaptive SIGMA tutoring.
3. **`substeps[]` per step (`breakdownAvailable: true`):** Finer pedagogical explanation cards (`zone: "scratch"`, soft `#EFF6FF` fill + `#3B82F6` border) streamed when Carbon emits `"step.navigated"` with `"action": "breakdown"` (`activeSubstep: 0, 1, ...`).
4. **`finalAnswer`:** Crisp summary line or `#F0FDF4` green-bordered (`#16A34A`) answer card + `"kind": "check"` annotation revealed on the final step or immediately when Carbon emits `"answer.toggled"` (`revealed: true`).
5. **Question & Part Sync (`chrome.update`):** Whenever loading or jumping to any question/part (`"catalog.itemSelected"`, `"part.selected"`, `"item.navigated"`, or direct tutor jump), always include `activeItemId`, `itemLabel` (`"Q4"`), `partTabs` (`[{"id":"a","label":"(a)"},{"id":"b","label":"(b)"}]` or `[]`), `activePartId`, `totalSteps`, `activeStep`, `activeSubstep`, `breakdownAvailable`, and `answerRevealed`.
