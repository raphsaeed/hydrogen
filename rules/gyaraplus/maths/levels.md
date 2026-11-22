# Gyaraplus 11+ Maths — Explanation Sub-Levels (`9 → 1`) & Difficulty Rules (`gyaraplus/maths/levels.md`)

> **Purpose:** Defines how **every solving step** in `gyara-maths.raphs` scales from **`Level 9`** (expected 11+ exam working) down to **`Level 1`** (concrete infant-school intuition), so **SIGMA** can adapt seamlessly to any student without ever improvising math or Carbon geometry.

---

## 1. Core `9 → 1` Rules for 11+ Maths

1. **`Level 9` = Expected Full-Marks 11+ Exam Step:**
   - Uses our **Algebraic Method First** (for any linear-solvable problem) or crisp 11+ unitary/cancellation working—clean, fast, and exam-authentic.
2. **Completeness Bias (Populate Deeply Down to `Level 1`):**
   - Sub-level numbers are invisible to the learner. Always aim to populate **all 9 levels (`9, 8, 7, 6, 5, 4, 3, 2, 1`)** whenever distinct visual scaffolds exist; only stop at `min_sublevel > 1` on trivial steps where lower levels would violate the Anti-Repetition Rule.
3. **Method Pivot Across the Bridge (`Algebra at L9..L6` $\to$ `Visual Bar/Box at L4..L3` $\to$ `Concrete at L2..L1`):**
   - When a student steps down from `L6` into `L4–L3` on an algebraic step, **bridge the algebra into a tangible spatial model** (Carbon `barModel`, `balanceScale`, or `functionMachine`) so a visual learner sees *why* the algebraic equation is true!

---

## 2. Exact Definition of All 9 Sub-Levels in 11+ Maths

| Sub-Level | 11+ Maths Role | Carbon Primitives & Macros Used | Board Callouts & `narration` |
| :---: | :--- | :--- | :--- |
| **`9`** | **Expected 11+ Exam Working** | Crisp `scene.write` (`align: "equals"`, `\htmlId` key tokens) + required stimulus/solution figure. | Minimal/clean board working; clear concise `narration`. |
| **`8`** | **Guided 11+ Step + Mental/Check Cue** | Same 11+ algebraic/unitary step + `right-of` inverse tags (`(\div 3)`) + compact 1-line `Check` or mental-math tip badge. | Short `1–6` word operation tags on arrows. |
| **`7`** | **Token-Pointed Working + Scratchpad** | `focus.annotate` (`auto-margin` `elbow-arrow`) pointing to individual `\htmlId` terms (why `$5(D-18)$` has brackets, why LCM is `6`) + `rough-margin` scratchpad. | Pinpoint token callouts explaining each symbol. |
| **`6`** | **Zero-Leap Unpacked Balancing / Cancellation** | Multi-substep `flow.timeline` (`substep`), `edit.rub` inline value substitution, `fracCancel` red slashes, or explicit symmetric operations on both sides of `=`. | Short action label per micro-move. |
| **`5`** | **11+ Trap & MCQ Distractor Contrast (`✗` vs `✓`)** | Side-by-side Red `✗` card (showing the exact mistake behind a tempting `A–E` distractor, e.g., $5D - 18$ forgetting to multiply $18$ by $5$, or $20\%$ of $£48$) vs. Green `✓` truth. | Crisp trap warning callout. |
| **`4`** | **Visual Chunk Strips / Easy-Numbers Mirror** | `fractionBar` ($10\%$ or unit-fraction strips), `columnCalc` in margin, prime-factor strips, or a side-by-side **"Easy Numbers Mirror"** (e.g., showing $10 - 2 = 8$ alongside $H - 18 = 30$). | Brief chunk labels. |
| **`3`** | **Spatial Model (`barModel` / `balanceScale` / `functionMachine`)** | Native Carbon spatial macros: **"Parts" `barModel`** (`Tina = 2 boxes, Sarah = 3 boxes`), **`balanceScale`** (for equations), **`functionMachine`** (with reversed inverse arrows), or 3D **`netFold` / `cubeAssembly`**. | Minimal box/scale labels. |
| **`2`** | **Number-Line Hop Arcs & Pictorial Grouping** | `numberLine` macro with curved `jumps` (`+12`, `-18`), `tenFrame`, `clock`, or `jug`/`dial` visual scales. | Simple jump & count cues. |
| **`1`** | **Concrete Real-World Objects (Infant Intuition)** | `counters` macro (colored groups with `crossedOut` items) or clean illustrated objects (sweets, apples, coins, blocks) with count badges (`1, 2, 3...`). | Single-word / number labels; ultra-gentle `narration`. |

---

## 3. Concrete Walkthrough of the `9 → 1` Pivot on an Algebraic Step

**Example Step:** Solving $2D - 18 = 5D - 90$ (from *"Hareni has twice as many sweets as Dora; after each eats 18, Hareni has 5 times as many"*):
- **`L9`:** Aligned 3-line derivation: $2D - 18 = 5D - 90 \implies 72 = 3D \implies D = 24$.
- **`L8`:** Adds margin step arrows `(+90 to both sides)`, `(-2D from both sides)`, `(÷3)` + quick check `$48 - 18 = 30 = 5 \times 6$`.
- **`L7`:** Token pointers on `-90` (*"$5 \times (-18) = -90$: multiply BOTH terms inside the bracket!"*) and `3D` (*"Collect $D$'s on the right so it stays positive"*).
- **`L6`:** Unpacks `edit.rub` expansion $5(D - 18) \to 5D - 90$ and shows explicit $+90$ and $-2D$ balancing lines one sub-step at a time.
- **`L5` (`✗ vs ✓`):** Shows `✗ 5(D - 18) = 5D - 18` (classic bracket trap leading to wrong MCQ option!) vs `✓ 5(D - 18) = 5D - 90`.
- **`L4` (Chunk / Difference Strip):** Compares `"Before eating"` ($H$ is $1D$ ahead of $D$) vs `"After eating 18 each"` (their *difference never changes* when both eat 18! Difference $= 4 \text{ new parts} = 1D$).
- **`L3` (`balanceScale` + `barModel`):** Renders a visual `balanceScale` with `[2D boxes]` vs `[5D boxes - 72]` removing `2D` boxes from both pans so `3 boxes = 72` $\to$ `1 box = 24`.
- **`L2` (`numberLine`):** Number line showing $72$ split into $3$ equal jumps of $+24$ (`0 → 24 → 48 → 72`).
- **`L1` (`counters`):** Concrete sweet groups showing sharing $72$ sweets equally into $3$ bags ($24$ per bag).
