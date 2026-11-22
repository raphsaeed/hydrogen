# Question-Relative 9-Level Visual-First Solution Specification (`solution-levels.md`)

> **Purpose of this Document:** Authoritative pedagogical and visual-authoring specification for **Hydrogen** (Heavy AI Solution Compiler powered by **Gemini 4** + **Theta** `theta_calc` verification + **Carbon** `Carbon Protocol v0.1` pre-compilation) and **SIGMA** (On-Device Voice/Board Tutor).
>
> **Key Design Principles:**
> 1. **Question-Relative `Level 9 → Level 1` Scale:** **`Level 9`** is **always** what is expected at exam standard for *that specific question's step* (whether 11+, Primary, GCSE, or A-Level), progressively unpacking down to **`Level 1`** (basic infant-school / concrete intuition).
> 2. **Completeness Bias ("More Levels Rather Than Fewer"):** Sub-levels are invisible to the learner and exist as SIGMA's safety net. Hydrogen errs toward generating **more** distinct sub-levels rather than fewer so SIGMA is never forced to improvise.
> 3. **Heavy Hydrogen, Thin SIGMA:** Hydrogen pre-compiles exact verified KaTeX math (`$...$`), `\htmlId` token anchors, 2D/3D Carbon scene/macro coordinates, and plain-English narration scripts into `course_questions.solution_json` (`memory.raphs` Schema `2.3.0`).
> 4. **Visual-First + Anchored Callouts Across Every Level:** Zero unanchored walls of prose.

---

## 1. Core Mandate: Visual-First + Anchored Callouts (Never Prose Walls)

A great human Maths or Physics tutor with a pen and whiteboard **never writes unanchored paragraphs of prose**—whether at `Level 9` (exam working) or `Level 1` (concrete basics). Instead, they pair **visual diagrams, graphs, macros, and whiteboard scribbles** with **short, pinpoint callouts anchored directly to the math or diagram** (while spoken conversational warmth lives in SIGMA's `narration` field).

### Rule 0: The Visual-First & Anchored-Text Law (Applies to Levels `9` down to `1`)
1. **Every Populated Level (`9` down to `1`) Has a Visual / Whiteboard Layer on Carbon:**
   - Higher sub-levels (`8–9`) use clean exam-grade equations (`align: "equals"`), crisp graphs/diagrams, color-matched `\htmlId` highlights, and compact `Check ✔` footers.
   - Middle sub-levels (`5–7`) unpack intermediate moves with token pointer arrows (`focus.annotate`), exploratory `rough-margin` scratchpads, partial inline substitutions (`edit.rub`), `fracCancel` slashes, and `✗ vs ✓` trap contrasts.
   - Lower sub-levels (`1–4`) shift into concrete spatial & pictorial Carbon macros: `fractionBar` / chunk strips (`L4`), `barModel` / `areaGrid` / `functionMachine` / `balanceScale` / `netFold` (`L3`), `numberLine` hop arcs / `tenFrame` (`L2`), and `counters` / concrete real-world object groups (`L1`).
2. **"Board Text Only When Needed" — Anchored vs. Spoken Text:**
   - **FORBIDDEN on the Carbon Board (Unanchored Prose Walls):** Never write multi-sentence paragraphs explaining maths in abstract prose on the whiteboard canvas. Put full spoken sentences in `narration` (for SIGMA's TTS), not on the board.
   - **ENCOURAGED on the Carbon Board (Visually Anchored Micro-Callouts):**
     - **Token Pointer Callouts:** Carbon `focus.annotate` (`auto-margin` + `elbow-arrow` or `connector`) pointing directly to `\htmlId{tok_id}{...}` spans (e.g., pointing to `2` $\to$ *"Biggest factor of 8 and 2"*; split pointers to both `-` signs $\to$ *"Want $-19x$ and $+18$, so both signs are $-$."*).
     - **Color-Linked Graph / Geometry Deductions:** Short badges whose colors match plotted landmarks or geometric segments on `scene.place2D` / `scene.place3D`.
     - **Exam-Craft Tip Cards:** Compact `scene.write` `card`s for exam technique (e.g., *"Non-calculator: cancel common factors before multiplying!"*, *"2 marks — show the substitution line"*).

---

## 2. How the Question-Relative `9 → 1` Scale Works

### 2.1 Question-Relative Anchoring
Unlike fixed age scales, **`Level 9` is always anchored to the target exam expectation of that course vault**:
- In **`gyara-maths.raphs` (11+ Maths):** `Level 9` = crisp, full-marks expected 11+ exam step (using 11+-appropriate arithmetic, ratios, fractions, or introductory algebra—never unnecessary A-Level notation).
- In **`gcse-maths.raphs` / `gcse-physics.raphs`:** `Level 9` = crisp, full-marks GCSE exam step (e.g., FIFA formula working in Physics, standard algebraic/surd working in Maths).
- In **`alevel-maths.raphs`:** `Level 9` = crisp A-Level exam derivation.

From `Level 9`, each drop (`8 → 7 → 6 → 5 → 4 → 3 → 2 → 1`) progressively lowers abstraction and increases visual/concrete scaffolding **for that exact step** all the way down toward infant/reception-level intuition at `Level 1`.

### 2.2 Sub-Level Visibility, Per-Step Zoom & Completeness Bias
1. **Invisible to the Learner:** Raw level numbers (`9..1`) are **never** shown as grades or numbers to the student on Carbon; they are internal indices driven by **SIGMA** (and inspectable by the author in Hydrogen's Console).
2. **Independent Per-Step (and Sub-Token) Zoom:** If a student understands Step 1 at `Level 9` (`GYR-MTH-P001-Q014:a:S01:L09`) but gets stuck on Step 2, SIGMA steps down **only Step 2** (`L09 → L08 → L07 → ...`), and resumes Step 3 back at `Level 9`.
3. **"9 or Fewer as Needed" — Bias Toward Completeness:**
   - Every step starts at `max_sublevel = 9` and populates contiguous levels downward (`9, 8, 7, ..., min_sublevel`).
   - While a trivial step may stop at `min_sublevel > 1` if further decomposition would violate the Anti-Repetition Rule, **Hydrogen must err toward generating MORE sub-levels (all 9 whenever distinct visual steps exist) rather than fewer**, ensuring SIGMA never hits an explanation dead-end with a confused learner.
   - If a learner reaches `min_sublevel` and still needs foundation help, SIGMA can also push a linked **Topic / Prerequisite Card** (`course_concept_links` $\to$ `course_topic_cards`) via Carbon `flow.detourPush` (`scope: "solution"`).

---

## 3. Calibrated Archetypes Across Levels `9` Down to `1`

> **Anti-Repetition Law:** Stepping down from Level $N$ to Level $N-1$ must **never** simply repeat Level $N$ with longer sentences. Every drop **must change the visual granularity, method scaffold, trap contrast, or spatial Carbon model**.

| Level | Pedagogical Role (Relative to Question) | Carbon Visual / Macro Strategy | Board Callout Density |
| :---: | :--- | :--- | :--- |
| **9** | **Expected Exam-Standard Step** | Clean exam-expected equation/working (`align: "equals"`) + clean required sketch/diagram. | **Minimal / None** (exam working speaks for itself) |
| **8** | **Guided Exam Step + Quick Verification** | Same exam method with explicit flow cues (`\div 4` margin tag) + compact `Check ✔` line or MCQ sanity check. | **Short arrow/margin tags** (`1–6` words) |
| **7** | **Pinpoint Token-Annotated Working + Scratchpad** | `\htmlId` sub-token highlights + `focus.annotate` leader pointers + `rough-margin` candidate scratchpad + Exam Tip card. | **Pinpoint token callouts** |
| **6** | **Zero-Leap Unpacked Micro-Steps** | Every hidden arithmetic/algebraic sub-move revealed via `flow.timeline` `substep`, `edit.rub` inline substitution, or `fracCancel` slashes. | **Action label on every move** |
| **5** | **Visual "Why & Common Trap" Contrast (`✗` vs `✓`)** | Side-by-side Red `✗` (why the tempting mistake/distractor fails) vs. Green `✓` visual truth; or `netFold` collision highlight. | **Crisp contrast callouts** |
| **4** | **Visual Chunk Strips / Easy-Numbers Mirror** | Tangible deconstruction: `fractionBar` ($10\%$ tiles), `columnCalc` arithmetic, expanded factor strips, or side-by-side Easy-Numbers Mirror. | **Brief chunk labels** |
| **3** | **Structural Spatial Model (Bar / Box / Machine)** | Pure spatial model: `barModel`, `areaGrid` (`2×2` box), `functionMachine` (reversible arrows), `balanceScale`, or `cubeAssembly` / `netFold`. | **Minimal box/arrow labels** |
| **2** | **Pictorial Grouping & Number-Line Hop Arcs** | `numberLine` with curved `jumps` (`+1`, `+10`), `tenFrame`, `clock`, `jug` / `dial`, or shaded fraction circles. | **Simple count/jump cues** |
| **1** | **Concrete Real-World Objects (Infant Intuition)** | `counters` macro or clean SVG/asset illustrations (apples, coins, sweets, blocks) with grouping circles and count badges (`1, 2, 3...`). | **Single-word/number labels** |

---

## 4. Seven Mandatory Pedagogical Flexibility Patterns

Hydrogen's compiler must apply all **7 flexible patterns** whenever relevant to a question:

### Pattern A: Token-Level Pointer Callouts & Sub-Token Zooming
A single expression often packs multiple decisions into adjacent symbols (e.g., `2xy(4x + y)` or unit conversion + ratio share).
- Wrap distinct sub-expressions in `\htmlId{tok_<name>}{...}` (generated deterministically via Theta's `"anchor_tokens": true` or Hydrogen's KaTeX emitter).
- Attach pinpoint `focus.annotate` pointers to individual tokens (`"eq1:tok_hcf"`, `"eq1:tok_vars"`), recording `sub_drill_target` if a specific token links to a lower sub-level or Prerequisite Topic Card.

### Pattern B: Exploratory Scratchpads & Partial Skeletons (`zone: "rough-margin"`)
For non-mechanical steps (factor pairs, prime trees, trial-and-improvement, 11+ logic elimination, bus-stop division):
- Render an authentic **Scratchpad** inside Carbon's `"zone": "rough-margin"` (e.g., candidate pairs, `columnCalc` working, or sign-less bracket skeletons) linked to `"zone": "main-work"`.

### Pattern C: Method Pivoting Across Levels
Different levels can switch to a more concrete or deterministic method:
- **11+ Ratio / Reverse Percentage Example:** `Level 9–8` uses multiplier / unitary fraction working $\to$ `Level 6` unpacks $100\% - 20\% = 80\% \to 1\%$ unitary steps $\to$ `Level 4` uses a 10-tile `fractionBar` $\to$ `Level 3` uses a 5-box `barModel`.
- **Algebra Quadratic Example:** `Level 9–7` uses fast inspection + skeleton $\to$ `Level 6` pivots to deterministic `ac` middle-term split $\to$ `Level 3` pivots to a 2D `areaGrid` box.

### Pattern D: Built-In Reverse `"Check ✔"` & MCQ Trap Elimination
- Include a compact `Check ✔` verification line where steps are reversible (e.g., plugging the answer back into the word problem).
- For **11+ Multiple-Choice (`A–E`)** questions (Gyara default), `Level 8` / `Level 5` can also highlight **why tempting MCQ distractors are traps** (verified by Theta!).

### Pattern E: Progressive Feature-to-Diagram / Graph Sync
- Synchronize algebraic/numerical deductions step-by-step with visual updates on `scene.place2D` (`coordinatePlane`, geometry angles/ticks) or `scene.place3D` (`netFold` fold parameter `@fold`, `cubeAssembly` elevation highlights).

### Pattern F: Multi-Part State & Structural Substitution (`Part a → Part b`)
- Export exact symbolic/rational results (`exported_exact`) from Part `(a)` into Part `(b)` with **zero intermediate rounding drift** (via Theta).
- Visually highlight structural chunk substitution using matching `spanStyles` colors and `focus.annotate` connectors.

### Pattern G: Adaptive Step Count & Exam-Craft Badges
- **Zero Artificial Step Bloat:** If a question is naturally a 1-step insight at `Level 9`, keep `step_count = 1` at `Level 9` (while still offering rich `Levels 8 → 1` if the student needs unpacking).
- **Exam-Craft Badges:** Surface `calculatorMode` (`NON-CALC` vs `CALC`), marks (`[2 marks]`), recommended item timer (`recommended_time_sec`), and concise exam-technique callouts.

---

## 5. Worked 11+ Example (`gyara-maths.raphs`): Question-Relative `Level 9 → Level 1`

> **Question (`GYR-MTH-P001-Q014`, MCQ Default, Non-Calculator, 2 Marks):**  
> *"In a sale, all prices are reduced by $20\%$. A coat now costs $£48$. What was the original price of the coat?"*  
> **Options:** `A) £38.40` · `B) £57.60` · `C) £58.00` · `D) £60.00` ✔ · `E) £62.40` *(All verified via Theta; `B` is the classic $+20\%$ of $£48$ trap!)*

### Step 1 of 2: Relate $£48$ to the Remaining Percentage ($80\%$)
- **Level 9 (Expected 11+ Exam Step):** `$100\% - 20\% = 80\%$, so $80\% \text{ of Original} = £48$.`
- **Level 8 (Guided + Tag):** `$100\% \xrightarrow{-20\%} \htmlId{pct80}{80\%} = £48$` *(pointer: "Sale price is what's LEFT")*.
- **Level 7 (Annotated Token Callouts):** Separate pointers on `100%` (*"Original full price"*) and `- 20%` (*"Discount taken off original"*), leaving `80% = £48`.
- **Level 6 (Unpacked Fraction Equivalence):** `$100\% - 20\% = 80\% = \frac{80}{100} = \frac{4}{5}$ of the original price equals $£48$.`
- **Level 5 (`✗ vs ✓` Classic Distractor `B` Trap):**
  - `✗ TRAP (Option B = £57.60):` Taking $20\%$ of $£48$ ($£9.60$) and adding it—wrong because $20\%$ came off the *bigger original price*, not off $£48$!
  - `✓ TRUTH:` $£48$ **is** the $80\%$ remaining bar.
- **Level 4 ($10\%$ Chunk Strip — `fractionBar`):** Draws 10 equal $10\%$ tiles; crosses out the last 2 tiles ($20\%$ off) and brackets the remaining **8 tiles $= £48$**.
- **Level 3 (5-Box `barModel`):** Draws 5 equal fifths ($20\%$ each); shades 4 boxes $= £48$ and marks 1 box as `"20% off"`.
- **Level 2 (`numberLine` Percentage Jumps):** Number line from $0\%$ to $100\%$ in $20\%$ hops, showing hop back from $100\%$ to $80\%$ landing at $£48$.
- **Level 1 (Concrete `counters` / $£10$ Note Groups):** Concrete visual showing out of every 5 coins/notes, 1 is taken away in the sale, leaving 4 coins for every 5 you started with.

### Step 2 of 2: Scale Up from $80\%$ ($\frac{4}{5}$) Back to $100\%$ ($£60$)
- **Level 9 (Expected 11+ Unitary Working):** `$10\% = £48 \div 8 = £6 \implies 100\% = £6 \times 10 = \mathbf{£60}$ (Option D).`
- **Level 8 (With Reverse `Check ✔`):** `$£48 \div 4 \times 5 = £12 \times 5 = \mathbf{£60}$. Check: $20\% \text{ of } £60 = £12;\; £60 - £12 = £48\;\checkmark$.`
- **Level 7–6 (Unpacked Two-Stage Arrows + `columnCalc` in Margin):** Explicit `÷ 8` arrow on both sides ($80\% \to 10\%$, $£48 \to £6$), then `× 10` arrow ($10\% \to 100\%$, $£6 \to £60$).
- **Level 5 (`✗ vs ✓` Division Check):** Contrast dividing by `2` vs dividing `£48` into `8` equal $10\%$ chunks.
- **Level 4 ($10\%$ Tile Fill-In):** Writes `£6` inside every one of the 8 remaining tiles ($8 \times £6 = £48$), then writes `£6` into the 2 discount tiles $\to$ all 10 tiles $= £60$.
- **Level 3 (`barModel` + Reversible `functionMachine`):** Fills `£12` into each of the 4 shaded boxes ($£48 \div 4 = £12$), copies `£12` into the 5th box $\to$ $5 \times £12 = £60$.
- **Level 2 (`numberLine` Skip-Counting by $£12$):** Jumps `0 → £12 → £24 → £36 → £48 (4th jump) → £60 (5th jump)`.
- **Level 1 (Sharing `48` Counters Equally into `4` Plates):** Visualizes sharing 48 into 4 equal groups of 12, then adding the 5th group of 12 to make 60.

---

## 6. Canonical `question_json` & `solution_json` Blueprint Schema (`course_questions`)

Inside `memory.raphs` Schema `2.3.0`, `course_questions` stores `question_json` (Left-Pane stimulus & answer definition) and `solution_json` (Right-Pane `Steps × Sub-Levels 9..1` pre-compiled Carbon commands, Theta groundings & SIGMA narration scripts):

### 6.1 `question_json` Structure
```json
{
  "schemaVersion": "1.0",
  "questionId": "GYR-MTH-P001-Q014",
  "answerFormat": "mcq",
  "stemMarkdownKatex": "In a sale, all prices are reduced by $20\\%$. A coat now costs $\\pounds 48$.\n\nWhat was the original price of the coat?",
  "dictionary": [
    { "term": "reduced", "definition": "Made smaller in price or amount (subtracted from the original)." }
  ],
  "pinnedReferences": [],
  "leftPaneCarbonCommands": [
    {
      "op": "scene.write",
      "params": {
        "id": "q-stem",
        "pane": "left",
        "zone": "spine",
        "style": "typeset",
        "content": "In a sale, all prices are reduced by $20\\%$. A coat now costs $\\pounds 48$.\n\nWhat was the original price of the coat?"
      }
    },
    {
      "op": "interact.ask",
      "params": {
        "id": "q-mcq",
        "pane": "left",
        "kind": "checkpoint",
        "prompt": "Select one answer:",
        "choices": [
          { "id": "A", "label": "$\\pounds 38.40$" },
          { "id": "B", "label": "$\\pounds 57.60$" },
          { "id": "C", "label": "$\\pounds 58.00$" },
          { "id": "D", "label": "$\\pounds 60.00$" },
          { "id": "E", "label": "$\\pounds 62.40$" }
        ]
      }
    }
  ],
  "parts": [
    {
      "partId": "main",
      "label": null,
      "marks": 2.0,
      "answerKey": {
        "correctChoiceId": "D",
        "exactValue": "60",
        "katexValue": "\\pounds 60.00",
        "unit": "GBP",
        "distractorDiagnostics": {
          "A": "Subtracted 20% of £48 (£48 × 0.8 = £38.40) instead of reversing.",
          "B": "Added 20% of £48 (£48 × 1.2 = £57.60) — classic reverse-percentage trap.",
          "C": "Added £10 instead of scaling.",
          "E": "Added 30% of £48."
        }
      }
    }
  ]
}
```

### 6.2 `solution_json` Structure (`Steps × Sub-Levels 9 → 1`)
```json
{
  "schemaVersion": "1.0",
  "questionId": "GYR-MTH-P001-Q014",
  "thetaPreset": "uk_gcse",
  "parts": [
    {
      "partId": "main",
      "usesPriorParts": [],
      "exportedExact": { "original_price": "60" },
      "finalAnswer": {
        "katex": "$\\pounds 60.00\\text{ (Option D)}$",
        "carbonCommands": [
          {
            "op": "scene.write",
            "params": {
              "id": "ans_final",
              "pane": "right",
              "zone": "spine",
              "style": "typeset",
              "color": "#15803D",
              "content": "Original price $= \\mathbf{\\pounds 60.00}$ **(Option D)**",
              "card": { "fillColor": "#F0FDF4", "borderColor": "#16A34A", "borderStyle": "solid" }
            }
          },
          {
            "op": "focus.annotate",
            "params": { "id": "ans_chk", "pane": "right", "targetId": "ans_final", "kind": "check", "color": "#16A34A" }
          }
        ]
      },
      "steps": [
        {
          "stepId": "S01",
          "stepIndex": 0,
          "stepNumber": 1,
          "stepGoal": "Identify that £48 represents 80% (4/5) of the original price",
          "breakdownAvailable": true,
          "substeps": [
            {
              "substepIndex": 0,
              "narration": "The original price before the sale is always 100 percent. Taking 20 percent off leaves 80 percent.",
              "carbonCommands": [
                {
                  "op": "scene.write",
                  "params": {
                    "id": "s1_sub0",
                    "pane": "right",
                    "zone": "scratch",
                    "style": "typeset",
                    "content": "Original $= 100\\%$. Sale takes off $20\\%$, leaving $100\\% - 20\\% = 80\\%$.",
                    "card": { "headerBanner": "WHY 80%?", "fillColor": "#EFF6FF", "borderColor": "#3B82F6", "borderStyle": "solid" }
                  }
                }
              ]
            }
          ],
          "prerequisiteCardIds": ["GYR-MTH-T_PCT01-C01"],
          "thetaVerification": [
            { "tool": "calc.eval", "args": { "expr": "100 - 20" }, "expectedExact": "80" }
          ],
          "sublevels": {
            "9": {
              "archetype": "expected_exam_step",
              "narration": "Since 20 percent was taken off the original 100 percent price, the £48 sale price is 80 percent of the original price.",
              "speechTokens": ["s1_l9_eq:pct80", "s1_l9_eq:val48"],
              "carbonCommands": [
                {
                  "op": "scene.write",
                  "params": {
                    "id": "s1_l9_eq",
                    "pane": "right",
                    "zone": "spine",
                    "style": "typeset",
                    "content": "$100\\% - 20\\% = \\htmlId{pct80}{80\\%} \\implies 80\\%\\text{ of Original} = \\htmlId{val48}{\\pounds 48}$"
                  }
                }
              ]
            },
            "5": {
              "archetype": "trap_contrast",
              "narration": "Careful not to fall for Option B! 20 percent came off the bigger original price, not off £48. Adding 20 percent of £48 only gives £57.60.",
              "carbonCommands": [
                {
                  "op": "scene.write",
                  "params": {
                    "id": "s1_l5_trap",
                    "pane": "right",
                    "zone": "spine",
                    "style": "typeset",
                    "content": "**NEVER EVER** take $20\\%$ of $\\pounds 48$ ($\\pounds 48 + \\pounds 9.60 = \\pounds 57.60$) — wrong whole!\n\n**Truth:** $\\pounds 48$ **is** the remaining $80\\%$ of the Original.",
                    "card": { "headerBanner": "WATCH OUT FOR OPTION B", "fillColor": "#FFF1F2", "borderColor": "#F87171", "borderStyle": "solid" }
                  }
                }
              ]
            },
            "3": {
              "archetype": "spatial_model",
              "narration": "Let's draw the original price as 5 equal 20-percent boxes. One box was taken off in the sale, so the 4 boxes left must equal £48.",
              "carbonCommands": [
                {
                  "op": "scene.place2D",
                  "params": {
                    "id": "s1_l3_bar",
                    "pane": "right",
                    "zone": "diagram",
                    "spec": {
                      "macro": "barModel",
                      "totalLabel": "Original ($100\\%$)",
                      "rows": [
                        [
                          { "value": 4, "label": "$\\pounds 48$ ($80\\%$ left)" },
                          { "value": 1, "label": "$20\\%$ off" }
                        ]
                      ]
                    }
                  }
                }
              ]
            }
          }
        }
      ]
    }
  ]
}
```
*(In full compiled output, Hydrogen populates `9, 8, 7, 6, 5, 4, 3, 2, 1` contiguously down to `min_sublevel` for each step.)*
