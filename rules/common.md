# Global Common Rules — Clarity, Simplicity, Elegance & Carbon Discipline

> **Scope:** Applies to **every** product (`infant`, `primary`, `gyaraplus`, `gcse`, `alevel`) and **every** subject (`maths`, `physics`, `english`, `verbal-reasoning`, `non-verbal-reasoning`).

---

## 1. The Three North-Star Qualities: Clarity, Simplicity, Elegance

1.1 **Clarity (WHY Before HOW):**
- Every step must focus on **one clear idea** so the learner immediately sees *why* a move is made before *how* it is calculated.
- Never hide two or three unspoken leaps inside one cryptic line at lower sub-levels.

1.2 **Simplicity (Visual-First, Zero Prose Walls):**
- A great human tutor with a marker and whiteboard **never writes paragraphs of prose on the board**.
- **On the Carbon Board (`scene.write`, `focus.annotate`):** Use crisp equations, clean diagrams/tables/macros, and short pinpoint micro-callouts anchored directly to specific tokens or diagram features.
- **In Spoken Voice (`narration`):** Put warm, natural, full-sentence tutor explanations inside the `narration` field for **SIGMA** (TTS), keeping the visual board spacious and uncluttered.

1.3 **Elegance (Clean Layout, Zero Hacks):**
- Never add empty spacer strings, fake padding spaces, or manual pixel tweaks to force layout. Rely on Carbon's relative `AnchorSpec` (`below` with `align: "equals"`, `right-of`, `auto-margin` with obstacle-routed `elbow-arrow` connectors).

---

## 2. Two-Pane Carbon Board Discipline (`Carbon Protocol v0.1`)

2.1 **Clean, Zero-Scroll Question Pane (`pane: "left"`, `zone: "question-stem"`):**
- Must remain clean, legible, and faithful to the original question's intent, fitting comfortably without vertical overflow.
- Multiple-choice options (`A–E`) or horizontal figure sequences must stay compact and cleanly grouped—never stacked into tall overflowing columns.
- Parts mirror printed structure (`(a)`, `(b)`, `(i)`, `(ii)`), with accurate `marks` per part and per question.

2.2 **Glyph-Hugging, Transient Annotations (`focus.annotate`):**
- Wrap only the specific clue words, numbers, or sub-expressions that the solution points to inside `\htmlId{tok_<name>}{...}`. Do **not** pollute every word with unused IDs.
- Highlights, underlines, circles, and boxes must **hug only the concise 2–5 word clue or symbol**—never underline multi-line sentences.
- Mark step-specific clue highlights `"transient": true` (or clear superseded callouts) so annotations never pile up into visual clutter across steps.
- **Consistent Semantic Colors:** Keep clue colors consistent across steps—if Clue 1 is highlighted in blue (`#2563EB`) in Step 1, use that same blue when substituting or checking Clue 1 in Step 3.

2.3 **KaTeX & Typography Hygiene (`scene.write`):**
- **Mandatory Math Delimiters:** Every mathematical symbol, number-with-unit expression, fraction, or equation MUST be wrapped in `$...$` or `\(...\)`. Never emit raw ASCII math (`x^2`, `3/4`, `180 deg`) outside `$...$`.
- **Clean LaTeX Commands:** Prefer `^{\circ}` over Unicode `°` (never put `\,` directly before `^{\circ}`—write `45^{\circ}`, not `45\,^{\circ}`), `-` over Unicode `−`, and `\tfrac{a}{b}` for inline fractions.
- **No Duplicate Tick Glyphs in Check Annotations:** Never type literal `✓`, `✅`, or `\checkmark` inside a `focus.annotate` `"kind": "check"` label, since Carbon's `"check"` annotation draws a crisp tick mark natively.
- **Canonical Carbon Enums:** Use `chrome.update`, surface `"plain"` / `"squared"` / `"ruled"` / `"dotted"` / `"isometric"`, and themes `"warm-paper"` / `"classroom-white"` / `"dark-glass"`.

2.4 **Diagram Cleanliness & Regeneration:**
- Place external dimension labels and vertex letters clearly outside shape perimeters with generous breathing room so text never collides with borders or shading.
- Never invent extra labels inside a question stimulus figure that weren't part of the problem; put deduced angles/lengths on the Solution Board (or draw them progressively during solution steps).
- Graphs, charts, tables, clocks, nets, and geometry are **always** regenerated as crisp native Carbon `scene.place2D` / `scene.place3D` specs (or clean vector SVG); real photographs are recreated cleanly as fresh generated illustrations stored in `course_media_assets`.

---

## 3. Compliance, Anonymization & Verification (Non-Negotiable)

3.1 **100% Source Anonymity Inside `.raphs`:**
- **Never** include school names, exam board names, publisher titles, sitting dates, or source PDF filenames anywhere in `question_json`, `solution_json`, titles, badges, or IDs inside `<product>-<subject>.raphs`.
- Raw PDF crops and source provenance live **exclusively** in Hydrogen's private, git-ignored `private-xref.json` and `archive/`.

3.2 **Deterministic Verification Before Commit:**
- Every mathematical/scientific calculation, unit conversion, graph landmark, `Check` step, and multiple-choice distractor must pass **`theta_calc`** verification with zero intermediate rounding drift before entering `.raphs`.
- Every solution closes with a clear **Exact Verification / Sense Check** (or self-marking rubric for open writing) so the learner sees how to confirm their answer with 100% confidence.
