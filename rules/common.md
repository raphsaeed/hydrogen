# Global Common Rules — Clarity, Simplicity, Elegance & Carbon Warm-Paper Editorial Discipline

> **Scope:** Applies to **every** product (`infant`, `primary`, `gyaraplus`, `gcse`, `alevel`) and **every** subject (`maths`, `physics`, `english`, `verbal-reasoning`, `non-verbal-reasoning`).  
> **Companion Visual Reference:** [`docs/carbon-visual-theme.md`](../docs/carbon-visual-theme.md)

---

## 1. The Three North-Star Qualities: Clarity, Simplicity, Elegance

1.1 **Clarity (WHY Before HOW):**
- Every step must focus on **one clear idea** so the learner immediately sees *why* a move is made before *how* it is calculated.
- Never hide two or three unspoken leaps inside one cryptic line at lower sub-levels.

1.2 **Simplicity (Visual-First, Zero Prose Walls):**
- A great human tutor with a marker and whiteboard **never writes walls of unanchored prose on the board**.
- **On the Carbon Board (`scene.write`, `focus.annotate`):** Use crisp equations, clean diagrams/tables/macros, and short pinpoint micro-callouts anchored directly to specific tokens or diagram features.
- **In Spoken Voice (`narration`):** Put warm, natural, full-sentence tutor explanations inside the `narration` field for **SIGMA** (TTS), keeping the visual board spacious and uncluttered.

1.3 **Elegance (Clean Layout, Zero Hacks):**
- Never add empty spacer strings, fake padding spaces, or manual pixel tweaks to force layout. Rely on Carbon's relative `AnchorSpec` (`below` with `align: "equals"`, `right-of`, `auto-margin` with obstacle-routed `elbow-arrow` connectors) and canonical `zone`s (`"spine"`, `"scratch"`, `"diagram"`).

---

## 2. Unified Warm-Paper Editorial Theme & Color Rules (Zero Visual Conflict)

2.1 **Do Not Fight Carbon's Light Editorial Canvas:**
- Carbon runs a calm light editorial theme (`"warm-paper"` / `"classroom-white"`): Warm Ivory Paper `#FDFBF7` (Index & Left Question Pane), Clean Light Ivory `#FDFCF9` with sky-blue grid `#DAE4F0`/`#CCDBEC` (Right Whiteboard), Warm Stone `#F2ECE1` chrome, and Deep Charcoal `#1F2421` primary ink.
- **FORBIDDEN:** **NEVER** emit dark-mode canvas fills, dark card backgrounds, or light/pastel/yellow/white text ink (light text is invisible on paper surfaces).
- **STRICTLY PNG ONLY for Brand/Subject Logos:** `brandLogo` and `logo` in `chrome.setCatalog` must **strictly** use PNG (`data:image/png;base64,...` or `.png` path)—**never** SVG (font-based SVGs alter the brand calligraphy).

2.2 **Semantic Ink, Card Fill, Stamp & Border Tokens (`c1`..`c8` & Curated Hexes):**
Restrict all equations, diagrams, tables, stamps, and callout cards to these exact high-contrast editorial tokens:
- **Question Stem & Working Steps:** `#0F172A` / `#1F2421` (or omit `color` $\to$ Deep Charcoal on unboxed paper); underlined key stem phrases bold `#991B1B`
- **`c1` Primary Algebraic / Geometric Focus:** Ink `#2563EB` / `#1D4ED8` | Box Fill `#EFF6FF` | Border `#3B82F6`
- **Worked `E.g.` Example Lead:** Ink `#0284C7` (Sky Blue)
- **`c2` Final Answer & Correct Checkmarks:** Ink `#15803D` / `#16A34A` | Box Fill `#F0FDF4` | Border `#16A34A`
- **`c3` Key Warning / Mistake / `"NEVER EVER"` Rule Card:** Ink `#DC2626` / `#B91C1C` / `#991B1B` | Box Fill `#FDF2F8` (border `#9F1239`) or `#FFF1F2` (border `#F87171`)
- **`c4` Sticky `RE-CHECK` / Hint / Method Note:** Ink `#B45309` / `#D97706` / `#1F2421` | Box Fill `#FEF9C3` (dashed border `#CA8A04`, banner `#A16207`) or `#FFFBEB` (border `#F59E0B`)
- **`c5` Sub-Step Breakdown / Substitution:** Ink `#6D28D9` / `#7C3AED` | Box Fill `#F5F3FF` | Border `#8B5CF6`
- **`c6` Secondary Diagram / Quantity B:** Ink `#0D9488` | Box Fill `#F0FDFA` | Border `#14B8A6`
- **`c8` Auxiliary Construction / Neutral:** Ink `#475569` | Box Fill `#F8FAFC` | Border `#CBD5E1`
- **Solid Inverted `typewriter` Stamps (`EXAMPLE`, `NON-CALC`, `KEY TRAP`):** `#FFFFFF` text strictly on `#1565C0` (border `#0D47A1`) or `#991B1B` solid stamp cards (or `#1F2421` ink on `#FFFBEB` clue slips)

2.3 **Classy 12-Subject Soft-Mineral Palette & Independent Logos (`chrome.setCatalog`):**
- **12-Subject Muted Mineral Palette** (always explicitly set `"color"` on every subject, especially when `> 6` subjects so Carbon's 6-colour fallback never repeats):
  `#5B7DB1` (Slate Blue) · `#4D8B7A` (Eucalyptus Sage) · `#7A6BA8` (Wisteria Lavender) · `#A8647A` (Dusty Rose) · `#A37E58` (Warm Ochre) · `#568A9E` (Mist Cyan) · `#6B8E67` (Moss Olive) · `#8F6B9E` (Muted Plum) · `#B07462` (Soft Terracotta) · `#5C768D` (Steel Indigo) · `#8C7D53` (Antique Brass) · `#4F8A8B` (Deep Seafoam).
- **3-Way Per-Subject `logo` Rule (`availableSubjects[i].logo`, PNG Only):** `"data:image/png;base64,..."` renders a distinct subject PNG emblem; `null`/omitted falls back to top-bar `brandLogo`; `""` suppresses the logo cleanly (text-only subject card). `topics[]`, `papers[]`, and `cards[]` never take `logo`—use `color` + `badge`/`tagBadges` (`#6D5CAE` for `AI` replicas, `#475569` neutral) and `importantIcon: "triangle"`.

---

## 3. Curated Font Voices, Natural `PennstanderMath`, Size, Angle & Diagram Discipline

3.1 **Golden Rule for `$...$` Equations & Zero Stiff Serifs (`PennstanderMath`):**
- Typography across Hydrogen + Theta + SIGMA + Carbon must always feel smooth, neat, and naturally human—**NEVER** stiff academic Times-Roman print unless a user explicitly requests `"classic-serif"`.
- Wrap **every** formula, variable, fraction, exponent, angle, ratio, and currency/unit expression in `$...$` or `\(...\)` (e.g. `$\frac{3}{5}$`, `$x^2 + 4x - 12 = 0$`, `$3 : 5$`, `$\pounds 72$`, `$18\text{ cm}^2$`, `$45^{\circ}$` without `\,` before `^{\circ}`).
- **NEVER** emit raw ASCII math (`x^2`, `3/5`, `sqrt(2)`, `!=`, `<=`) outside `$...$`.
- **Natural Math Everywhere:** Carbon automatically renders `$...$` equations in **`PennstanderMath`** (a full OpenType `MATH` natural teacher-handwriting font supporting `\frac`, `\sqrt`, `\sum`, `\int`, matrices, Greek, and stretchy brackets) across **all** 5 standard voices—even inside clean `Lato` question stems!
- **FORBIDDEN BY DEFAULT:** **NEVER** request `"classic-serif"` (`"classic-tex"` / `"serif"`).
- Wrap key target terms in `\htmlId{step2_rhs}{...}` (only on referenced tokens—do not pollute unreferenced words) so steps, breakdowns, and `edit.rub` substitutions anchor cleanly.

3.2 **The 5 Curated Font Voices, Sizes, Angles & Roles (`scene.write.style` & `spans[].form`):**

| Pedagogical Situation | `style` / `font` Value (Aliases) | Bundled Prose Font (+ `$...$` Math) | Size (`scale` / `sizePt`) | Direction & Angle (`rotationDeg`, `textAlign`) | Preferred Ink & Card Colours |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **1. Worked Solution Steps, `Theta` Derivations & `E.g.` Examples** | **`"handwritten"`** *(default; `"natural"`, `"pennstander"`)* | `Pennstander` + `PennstanderMath` | Right Board `21.0 pt` (`scale: 1.0`); Left `E.g.` `scale: 1.05` | LTR, `0°` upright, `textAlign: "left"` | Primary `#1F2421`, Focus `#2563EB` (`c1`), `E.g.` lead `#0284C7`, Final `#15803D` (`c2`) |
| **2. Question Stems, Numbered Instructions & Golden-Rule Cards** | **`"typeset"`** *(`"clean"`, `"lato"`, `"sans"`)* | Smooth `Lato` (`Regular`/`Bold`) + `PennstanderMath` | Left Question Pane `16.8 pt` (`scale: 1.0`) | LTR, `0°` upright, `textAlign: "left"` (auto-wrap) | `#0F172A` / `#1F2421`; underlined key phrase bold `#991B1B`; `"NEVER EVER"` card `#FDF2F8` + `#9F1239` border |
| **3. Friendly Sticky Notes, Breakdown Cards & Primary/11+ Hints** | **`"casual"`** *(`"single-day"`, `"note"`)* | `Single Day` + `PennstanderMath` | `scale: 0.88–0.95` | LTR, gentle human tilt `rotationDeg: -2.0` to `+1.5`, `left` | Yellow check card `#FEF9C3` + dashed `#CA8A04` border (banner `#A16207`); or `#EFF6FF` + `#3B82F6` border |
| **4. Bold Classroom Marker Headlines & Key Takeaways** | **`"marker"`** *(`"patrick-hand"`, `"chalk"`, `"board-marker"`)* | `Patrick Hand` + `PennstanderMath` | `scale: 0.95–1.10` | LTR, `rotationDeg: -1.5` to `+1.5`, `left` or `center` | `#1D4ED8` (Blue), `#0D9488` (Teal), `#15803D` (Green), `#B45309` (Amber) |
| **5. Corner Stamps, Exam Badges & Word-Problem Clues** | **`"typewriter"`** *(`"special-elite"`, `"stamp"`)* | `Special Elite` + `PennstanderMath` | `sizePt: 14.0–16.0` (`scale: 0.80–0.90`) | Stamp tilt `rotationDeg: -3.5` (or `0°` badge) | `#FFFFFF` on `#1565C0` blue (`#0D47A1` border) or `#991B1B` crimson solid card; or `#1F2421` on `#FFFBEB` clue slip |

- **Inline Voice & Weight Mixing (`spans[].form` & `tokenStyles[].form`):** Set `"form"` to `"regular"`, `"bold"`, `"italic"`, `"bold-italic"`, `"monospace"`, `"math"`, `"handwritten"`, `"typeset"`, `"casual"`, `"marker"`, or `"typewriter"` to mix voices cleanly on a single line.

3.3 **Mandatory Anti-Clash Rules for Diagrams, Tables & Callouts:**
1. **Transparent Labels Only:** Never place opaque white background boxes behind coordinate, vertex, or diagram labels (opaque boxes clip ugly holes over Carbon's `#DAE4F0` grid). Place external dimension/vertex labels cleanly outside shape perimeters.
2. **Fine Strokes Only (`1.0–1.75 pt`):** Never use thick heavy borders (`> 2.2 pt`) that overpower `Lato` prose or `PennstanderMath` equations.
3. **Clean Table Cell & Sub-Token Highlights:** Pair soft pastel `cellStyles` fills (`#DCFCE7`, `#DBEAFE`, or `#FEF3C7`) with concise `focus.annotate` (`"kind": "check"` or `"label"`).
4. **No Duplicate `✓` Inside `"kind": "check"`:** **NEVER** include a literal `✓`, `✔`, `✅`, or `\checkmark` inside `label` when `focus.annotate` uses `"kind": "check"` (Carbon already draws a crisp corner tick).
5. **Transient Intermediate Callouts:** Mark intermediate step callouts `"transient": true` (or `edit.erase` them on later steps) so leader lines never pile up. Keep semantic clue colors consistent across steps.
6. **Clean Regeneration:** Never invent extra deduced labels inside a question stimulus figure; regenerate all graphs/geometry/tables/clocks/nets natively in Carbon (`scene.place2D` / `scene.place3D`) and recreate real photographs as clean generated assets in `course_media_assets`.

---

## 4. Structure for Carbon's 3-Group Bottom Bar & Navigation

Structure every question & solution part (`a`, `b`, ...) to drive Carbon's 3-Group Bottom Bar (`Timers` | `(a)(b) + Step i/N + Break step down + Reveal Answer` | `Back / Next Question`):
1. **`questionStem`:** `pane: "left"`, `zone: "spine"` (`"style": "typeset"` in `#0F172A`/`#1F2421`, optional `"typewriter"` corner stamp & `"handwritten"` `E.g.` example), plus any stimulus figure in `zone: "diagram"` and compact `A–E` choices.
2. **`steps[]` (`totalSteps: N`, `0`-indexed `activeStep`):** Natural `"style": "handwritten"` (`Pennstander` + `PennstanderMath`, `21.0 pt` base, `0°` upright) vertical working steps (`pane: "right"`, `zone: "spine"`, `anchor: { "mode": "below", "targetId": "..." }`), paired with Question-Relative `sublevels` (`9 → 1`) for SIGMA.
3. **`substeps[]` per step (`breakdownAvailable: true`):** Friendly `"style": "casual"` (`Single Day`, `scale: 0.88–0.95`, `rotationDeg: -2.0..+1.5`) or `"typeset"` explanation cards (`zone: "scratch"`, `#EFF6FF` + `#3B82F6` border or `#FEF9C3` + dashed `#CA8A04` border) revealed on `"step.navigated"` (`action: "breakdown"`).
4. **`finalAnswer`:** Crisp `"handwritten"` or `"marker"` / `"typeset"` `#15803D` summary line or `c2` `#F0FDF4` fill + `#16A34A` border answer card + `"kind": "check"` annotation, revealed on the final step or on `"answer.toggled"` (`revealed: true`).
5. **`chrome.update` Sync:** Always keep `activeItemId`, `itemLabel`, `partTabs`, `activePartId`, `totalSteps`, `activeStep`, `activeSubstep`, `breakdownAvailable`, and `answerRevealed` synchronized.

---

## 5. Compliance, Anonymization & Verification (Non-Negotiable)

5.1 **100% Source Anonymity Inside `.raphs`:**
- **Never** include school names, exam board names, publisher titles, sitting dates, or source PDF filenames anywhere inside `<product>-<subject>.raphs`. Raw crops live **only** in Git-ignored `private-xref.json` and `archive/`.

5.2 **Deterministic Verification Before Commit:**
- Every calculation, unit conversion, graph landmark, `Check` step, and `A–E` MCQ distractor must pass **`theta_calc`** verification with zero intermediate rounding drift before entering `.raphs`.
