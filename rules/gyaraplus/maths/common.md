---
product: gyaraplus
course_id: GYR-MTH
subject: maths
stage_label: "11+"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: true
  preset: uk_gcse
  angle_mode: deg
  allow_mixed_fractions: true
  anchor_tokens: true
defaults:
  answer_format: mcq
  mcq_option_count: 5
  calculator_mode: non-calculator
  max_sublevel: 9
  target_min_sublevel: 1
level_9_policy:
  primary_philosophy: algebraic_method_first
  allowed_algebra:
    - single_letter_initials
    - linear_equations_one_unknown
    - linear_substitution_and_brackets
    - simultaneous_linear_elimination
    - clearing_fraction_denominators
  forbidden_at_level_9:
    - quadratic_equations
    - non_linear_systems
    - trigonometry_sin_cos_tan
    - calculus
    - radian_measure
---

# Gyaraplus 11+ Maths — Subject Rules (`gyaraplus/maths/common.md`)

> **Purpose:** Authoritative solving, Theta verification, and Carbon diagram playbook for **`gyara-maths.raphs`**.

---

## 1. Core Solving Philosophy: Algebraic Method First (`Levels 9 → 7`)

Whenever an 11+ problem (word problem, age/sweet-sharing shift, cost/bundle puzzle, fraction-of-an-unknown, sum/difference, or simultaneous relationship) can be modelled cleanly with **simple linear algebra**, you **MUST solve it algebraically at `Levels 9 → 7`** to build rock-solid fluency for GCSE and beyond:

1. **Define Intuitive Single-Letter Initials in Step 1:**
   - Choose meaningful capital initials from the story (e.g., **Hareni $= H$**, **Dora $= D$**, **Adult ticket $= A$**, **Child ticket $= C$**, or unknown number $= N$ / $x$). Never repeat full words inside equations.
2. **Use the 4 Core 11+ Linear Algebra Techniques Cleanly:**
   - **Substitution & Bracket Expansion:** Write one clean equation per clue and substitute step by step:
     $$H = 2D \quad\text{and}\quad H - 18 = 5(D - 18) \implies 2D - 18 = 5D - 90 \implies 72 = 3D \implies D = 24$$
   - **Adding or Subtracting Stacked Equations (Simultaneous Elimination):** For two-item bundle/puzzle clues (e.g., $2A + 3C = 20$ and $5A + 3C = 35$), align `=` signs vertically on Carbon (`"align": "equals"`) and subtract/add directly to eliminate a letter in one clean move.
   - **Clearing Denominators Immediately:** Whenever an equation contains fractions (e.g., $\frac{N}{2} = \frac{N}{3} + 8$ or $\frac{N + 4}{3} = 9$), multiply across by the denominator or LCM (`× 6`) first so all working uses clean whole numbers ($3N = 2N + 48 \implies N = 48$).
   - **Positive-Coefficient Balancing:** Keep one clear operation per line and collect unknowns on whichever side keeps the coefficient positive!
3. **Strict 11+ Ceiling (What NOT to Use):**
   - **Never** introduce quadratic equations ($ax^2 + bx + c = 0$), non-linear equations, or `sin`/`cos`/`tan` in 11+ Maths. If a problem would lead to a quadratic algebraically (e.g., *"two numbers multiply to 48 and differ by 2"*), solve it via **structured factor pairs** (`rough-margin` scratchpad) instead!

---

## 2. Non-Algebraic 11+ Archetypes & Mental-Math Craft

When a question is pure arithmetic, ratio-parts without shifts, geometry, or rates/catch-up where chunking is cleaner:
- **Direct Column Arithmetic (`columnCalc` macro):** When pure arithmetic *is* the entire question (*"Work out $3201 - 456$"*), render Carbon's `columnCalc` macro and step column-by-column with carries/borrows. When arithmetic is merely one small step inside a word problem, **do not** bloat the board with a full column grid—write one crisp line plus a compact 11+ mental split tip if helpful (e.g., *$198 \div 11$: split $198 = 110 + 88 \to 10 + 8 = 18$*).
- **Cancel Before You Multiply (`fracCancel` macro):** For fraction-of-an-amount or compound multiplication/division ($\frac{3}{8} \times 56$ or $\frac{14 \times 15}{21}$), **always** cancel common factors via Carbon's `fracCancel` macro rather than multiplying out huge numbers first.
- **Powers of 10:** Show decimal-point hop arcs rather than column multiplication for $\times / \div 10, 100, 1000$.
- **Mixed Numbers & Exact Values:** Set `allow_mixed_fractions: true` in Theta. Express improper fractions as simplified mixed numbers ($3\tfrac{1}{4}$) when natural for 11+ final answers, unless the question asks for a decimal or improper fraction.

---

## 3. Mandatory Exact 11+ Verification Step (Never Rough Estimation)

- Every multi-step word, algebra, ratio, or puzzle solution **must** verify the answer against the **original question clues** (either as a compact `Check:` footer on the final step at `L9–L8` or an explicit `Quick 11+ Sense / Table Check` card using `smartTable` / bullet checks):
  - *Example:* Check both clues: *"Start: $48 = 2 \times 24$. After eating $18$: $30 = 5 \times 6$."*
- **Exact Check Only:** Never use rough rounding/estimation (`≈`) after finding an exact answer.

---

## 4. Carbon 2D & 3D Diagram Rules for 11+ Maths

- **Placement & Scale:** Place wide/detailed geometry diagrams, charts, or tables cleanly in `"question-stem"` (Left Pane) and reference/copy or annotate them on `"main-work"` (Right Pane) with `"lockViewScale": true` if interactive.
- **Label Clearance:** Keep exterior side lengths (`"$8\text{ cm}$"`), vertex letters (`$A, B, C$`), and angle labels well clear of lines and shaded interiors.
- **Construction Overlays on Solution Board:** For composite shapes, cutouts, or hidden triangles, draw dashed construction lines/circles (`"dash": "dashed"`) in a contrasting semantic color during the solution steps so the student sees the hidden shapes emerge.
- **Angle Arcs:** Verify from the high-DPI crop in `archive/` whether an angle arc is interior or exterior—never guess.
- **3D Spatial, Cubes & Nets:** Use Carbon's native `scene.place3D` `"cubeAssembly"` (for 3D block counting & plan/front/side elevations) and `"netFold"` (variants `1..11` + `@fold` sweep) so students can see 2D nets fold into 3D solids.

---

## 5. Canonical 11+ Maths Topic Codes (`gyara-maths.raphs`)

| Topic ID | Category | Scope |
| :--- | :--- | :--- |
| `GYR-MTH-T_NUM` | Number | Place value, BODMAS, factors/HCF/LCM, primes, negative numbers, powers/roots |
| `GYR-MTH-T_FDP` | Fractions, Decimals & % | Mixed fractions, `fracCancel`, percentage changes, reverse percentages |
| `GYR-MTH-T_RAT` | Ratio & Proportion | Ratio sharing, unitary scaling, recipes, direct/inverse proportion |
| `GYR-MTH-T_ALG` | Algebra & Sequences | Linear equations, substitution, simultaneous puzzles, clearing fractions, $n$th term |
| `GYR-MTH-T_MEA` | Measurement & Rates | Metric units, time/timetables, speed-distance-time, perimeter/area/volume |
| `GYR-MTH-T_GEO` | 2D & 3D Geometry | Angle laws, polygons, coordinates/transformations, bearings, 3D solids & cube nets |
| `GYR-MTH-T_STA` | Statistics & Probability | Mean/median/mode/range, `pieChart`, `barChart`, `lineChart`, `venn`, probability |
| `GYR-MTH-T_LOG` | Word & Logic Puzzles | Catch-up/rate chunks, working backwards, combinatorics, calendar/clock puzzles |
