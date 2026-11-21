# Theta (`mini-Symbolica`) — Solution-Design & Application Integration Guide

## 1. Purpose of This Document & Role of Theta

**Theta** (`theta_calc` / `theta-calc-mcp`) is a first-party, headless, ultra-fast **deterministic Symbolic + Numerical Calculator Engine, Bidirectional Formula Solver, and 60–120 FPS Interactive Graph VM** written in **100% Pure Rust** (`2.82 MB` release binary, `< 1 ms` startup, `0` C/C++/Fortran/SQLite dependencies, verified **`150,000 / 150,000` (`100.0000%`)** against 50-digit `SymPy 1.14.0 + mpmath 1.3.0` in `21.61 s`).

Downstream applications (**"Solution Drivers"** — such as AI Tutors, Worked-Solution Architects, Exam Mark-Scheme Generators, Homework Graders, Engineering Co-Pilots, and Interactive Electronic Whiteboards) use Theta in **two complementary ways**:
1. **Direct On-Demand Calculation & Verification:** Computing exact symbolic, rational, surd, unit-aware, vector-calculus, integral-transform, special-function, statistical, or numerical answers in microseconds whenever needed.
2. **Designing Complete, Pedagogically Faithful Question Solutions (Infant $\to$ Primary $\to$ 11+ $\to$ GCSE $\to$ A-Level Maths, Further/Advanced Maths, Physics & University STEM):** Grounding every intermediate step, algebraic rearrangement, unit conversion, rejected extraneous/unphysical branch, statistical working column, curve landmark, and live whiteboard diagram so **solutions are 100% complete, self-contained, and exam-accurate—never half-baked, hand-waved, or corrupted by premature rounding**.

### Architectural Contract: Solution Driver vs. Theta

| Responsibility | Owned by the **Solution Driver** (Application / Tutor / Planner) | Provided On-Demand by **Theta** (`theta_calc` / `theta-calc-mcp`) |
| :--- | :--- | :--- |
| **Pedagogy & Narrative** | Chooses age-appropriate method & tone (e.g., bar models/inverse ops for Primary vs. SUVAT/calculus/vectors/Laplace for A-Level & University); writes full, clear explanatory prose for every step. | Supplies optional white-box algebraic/calculus transitions (`"trace": true` $\to$ `steps[]`) so explanations match mathematically valid steps. |
| **Multi-Step Chaining** | Decomposes multi-part word/physics/engineering problems into logical sub-questions and decides *when* to call Theta (or solve trivial conceptual steps directly). | Returns exact canonical strings (`result.exact`, e.g., `"(5 * sqrt(2)) / 2"`, `"49/5"`) that feed directly into subsequent calls with **zero intermediate rounding drift**. |
| **Formulas & Physics Laws** | Selects the physical/financial/geometric principle (conservation of energy, Snell's law, parallel-axis MOI, annuity PV) and explains *why* it applies. | Looks up **313 canonical equations** (`calc.formula`), validates 7-base SI dimensions, rearranges symbolically (`rearranged`), injects curriculum `g` (`9.8` vs `9.81`) & CODATA constants, and reports `assumptions`. |
| **Edge Cases & Exam Marks** | Explains *why* invalid roots are rejected in plain English (often worth explicit `R1`/`A1` exam marks). | Automatically filters extraneous radical/log roots, removable poles, and physical domain violations, returning them explicitly in `discarded_branches[]` with exact reasons. |
| **Final Presentation & Visuals** | Formats final exam answers (mixed numbers for KS2, exact surds/$\pi$ for Pure Maths, `3 s.f.` + SI unit for Physics) and orchestrates whiteboard animations. | Renders strict `KaTeX` (`katex` + optional `\htmlId{tok_<id>}{...}` `anchor_tokens`), extracts exact curve landmarks (`calc.analyze_graph`), and sweeps 120 FPS curves/regions/ODEs (`WhiteboardEngine` / `vm.*`). |

> **Core Principle for Solution Drivers:** Solutions presented to learners must always be **complete from start to finish** (problem setup $\to$ governing law/model $\to$ step-by-step working $\to$ branch/unit check $\to$ final formatted conclusion). Theta is a deterministic co-processor available at any step where exact symbolic algebra, numerical precision, unit verification, or graph geometry strengthens or verifies the solution.

---

## 2. Designing Complete Solutions Across Curriculum Stages (Infant $\to$ University STEM)

Different educational levels use **different mathematical conventions** (mixed fractions vs. improper fractions, degrees vs. radians, $g = 9.8$ vs. $g = 9.81\text{ m/s}^2$, exact surds vs. significant figures). Pass the matching flags per stage so Theta's outputs match classroom and exam-board expectations:

### Stage-by-Stage Configuration & Pedagogy Matrix

| Stage | Target Preset & Flags | Expected Output Style | Key Theta Capabilities Used When Designing Solutions |
| :--- | :--- | :--- | :--- |
| **1. Infant & Primary (KS1–KS2, Ages 5–11)** | `"allow_mixed_fractions": true`, `"space": "real"` | Whole numbers, remainders (`div`/`mod`), simplified proper fractions, mixed numbers (`2 1/3`), exact decimals (`0.3`), metric units (`cm`, `m`, `kg`, `ml`). **Avoid introducing algebraic $x$** where inverse arithmetic or bar-model steps belong. | `calc.eval` (`"35% of 240"`, `"mixed(2, 1, 3) + 3/4"`, `"1.5 km in m"`), `calc.transform` (`"op": "mixed_fraction"`, `"prime_factors"`, `"hcf_lcm"`), `calc.stats` (`"kind": "raw"` for mean/median/mode/range). |
| **2. 11+ & Lower Secondary (KS3, Ages 11–14)** | `"preset": "uk_gcse"`, `"angle_mode": "deg"`, `"allow_mixed_fractions": true` | Exact fractions or rounded decimals (`round_dp(x, 2)`), compound percentage multipliers, linear equations, area/volume in terms of $\pi$ and rounded, 3-figure bearings. | `calc.eval` (`"bearing(0, 0, -3, 4)"`), `calc.solve` (1-variable linear equations & formula rearrangement with `"trace": true`), `calc.formula` (`geometry.circle.*`, `geometry.trapezium.area`, `physics.mechanics.density`). |
| **3. UK GCSE Maths (Ages 14–16)** | `"preset": "uk_gcse"`, `"angle_mode": "deg"`, `"space": "real"` | Exact simplified surds (`6*sqrt(2)`), rationalized denominators, exact $\pi$ multiples, interval inequalities, `3 s.f.` on non-exact trig/quadratics. | `calc.transform` (`factor`, `expand`, `complete_square`, `rationalize`, `simplify`), `calc.solve` (quadratics, `2x2` linear & line+circle systems, inequalities), `calc.stats` (`frequency_table`, `grouped_table` linear interpolation, box-plot quartiles/outliers), `calc.analyze_graph`. |
| **4. UK GCSE Physics (Ages 14–16)** | `"preset": "uk_gcse"` (**$g = 9.8\text{ m/s}^2$**), `"angle_mode": "deg"` | Standard form or decimal rounded to `2` or `3 s.f.` **with explicit SI unit** (`J`, `W`, `N`, `Pa`, `V`, `A`, `Omega`, `Hz`). Supports GCSE symbols like `e` for spring extension. | **FIFA Workflow** (Section 3.1): `calc.eval` for prefix/unit conversion (`kJ -> J`, `cm -> m`, `km/h -> m/s`) $\to$ `calc.formula` for equation, symbolic rearrangement, substitution & `discarded_branches` $\to$ `round_sf(exact, 3)`. |
| **5. UK A-Level Mathematics (Pure, Stats & Mechanics)** | `"preset": "uk_alevel_maths"` (**$g = 9.8\text{ m/s}^2$**), `"angle_mode": "rad"` (calculus) or `"deg"`/`"rad"` (trig) | Exact surds, logarithms (`ln`), $\pi$ multiples in Pure; `3 s.f.` (or `2 s.f.` when $g = 9.8$ is used in Mechanics); `4 d.p.` for probabilities & p-values. | `calc.transform` (`poly_divmod`, `partial_fractions`, `diff`, `implicit_diff`, `parametric_diff`, `integrate` indefinite & definite, `series`, `limit`), `calc.solve` (bounded periodic trig, radical/log equations), `calc.stats` (Binomial, Normal, PMCC, regression, hypothesis tests), `calc.analyze_graph`. |
| **6. UK A-Level Further ("Advanced") Maths** | `"preset": "uk_alevel_maths"`, `"angle_mode": "rad"`, `"space": "complex"` (when complex roots/argand needed) | Exact Cartesian complex `a + b*i`, exact rational matrices/inverses, hyperbolic/inverse-trig antiderivatives (`asinh`, `asin`, `atan`, `asec`), Maclaurin polynomials. | `calc.solve` (`"space": "complex"`, degree 5–6+ Aberth polynomials, `3x3`–`5x5` systems), `calc.transform` (`det`, `inverse`, `rref`, `transpose`, `char_poly`, `eigenvalues`, improper `integrate` to `"inf"`), 3D `dot`/`cross`/`angle_between`, `chi_square` GOF & contingency tests, `student_t` & `f` distributions, `vm.ode_rk45`. |
| **7. UK A-Level Physics (Ages 16–18)** | `"preset": "uk_alevel_physics"` (**$g = 9.81\text{ m/s}^2$**), `"angle_mode": "rad"` (SHM/waves) or `"deg"` (inclines/optics) | Scientific notation matching input sig-figs (`2`–`3 s.f.`) + SI unit; seamless `eV <-> J`, `nm -> m`, `u_amu -> kg`, `degC -> K`, and CODATA constants (`h_planck`, `c_light`, `e_charge`, `k_B`, `G_grav`, `epsilon_0`). | `calc.formula` across SHM, circular/rotational motion, fields, capacitors (`RC` exponential decay), quantum/photoelectric, nuclear half-life, ideal gases/thermal, astro & relativity (`|v| < c` guard), plus `calc.stats` (`exponential`/`power` log-linear graph gradients) & `vm.ode_rk45`. |
| **8. University STEM, Finance & Engineering (*Schaum's 2,400* Scope)** | `"preset": "codata_exact"` or `"uk_alevel_physics"`, `"angle_mode": "rad"` | Exact symbolic orthogonal polynomials, Laplace/vector-calculus identities, Faulhaber/Bernoulli rationals, exact Lagrange polynomials, and 15-digit special-function values. | `calc.transform` (`grad`, `div`, `curl`, `laplacian`, `jacobian`, `hessian`, `laplace`, `inv_laplace`, `interpolate`, Ch. 17 radical/Weierstrass/Ostrogradsky `integrate`), Bessel (`bessel_j/y/i/k` + zeros via `calc.solve`), Legendre/Hermite/Laguerre/Chebyshev, `hypergeom_2f1`, elliptic `K`/`E`/`F`, `digamma`, `beta`, `zeta`, `exp_ei`/`sin_si`/`fresnel_c`, annuities, and `hypergeometric` stats. |

> **Critical UK Curriculum Trap Handled by `preset`:** UK **A-Level Mathematics (Mechanics)** mandates **$g = 9.8\text{ m/s}^2$** (`"preset": "uk_alevel_maths"`), whereas UK **A-Level Physics** mandates **$g = 9.81\text{ m/s}^2$** (`"preset": "uk_alevel_physics"`), and international metrological physics uses exact **`9.80665 m/s^2`** (`"preset": "codata_exact"`). Always set `preset` to match the subject!

---

## 3. Solution-Design Patterns & End-to-End Worked Blueprints

### 3.1 Five High-Value Solution-Building Patterns

1. **Zero-Drift Chaining (`exact` $\to$ `exact` $\to$ `round_sf` at the Very End):**
   In multi-part questions (Part (a) feeds Part (b)), **never** feed a 3-significant-figure rounded number into Part (b)—that causes "premature rounding" loss of accuracy marks. Keep Theta's `result.exact` (or 16-digit `approx`) for every internal step, while displaying both the exact expression and the rounded value in prose, and call `round_sf(<exact>, 3)` only when formatting a final numerical answer.
2. **FIFA Physics & Engineering Pattern (Formula $\to$ Insert/Convert $\to$ Fine-Tune/Rearrange $\to$ Answer + Unit):**
   - **Step 1 (Normalize Units):** Convert every non-SI value given in the question (`"72 km/h in m/s"`, `"450 nm in m"`, `"2.4 eV in J"`, `"250 psi in Pa"`, `"15 hp in W"`) via `calc.eval` so unit conversion marks (`C1`) are shown explicitly.
   - **Step 2 (Formula & Rearrangement):** Call `calc.formula` with `formula_id`, `known` SI values, `solve_for`, and `preset`. Display `formula_used.katex` (stating the law), then `rearranged.katex` (isolating the target symbol before or after substitution).
   - **Step 3 (Branch Justification):** Inspect `discarded_branches`. If non-empty (e.g., `v = -14` rejected by `v >= 0` or removable pole `R_2 = R_1`), add a sentence explaining *why* only the physical root is kept.
   - **Step 4 (Final Sig-Figs & Unit):** Round via `calc.eval` (`"round_sf(3.989422804014327, 3)"`) and attach `result.unit`.
3. **White-Box Algebraic Ladder (`"trace": true` + Verification):**
   When designing worked algebra or calculus solutions (`calc.solve` or `calc.transform`), pass `"trace": true` to obtain verified milestones (`steps[]`). For tricky indefinite integrals or factorizations, the driver can optionally show a 1-line check (`diff(antiderivative, x)` $\to$ original integrand) to teach self-checking habits.
4. **Interactive Whiteboard Token Anchoring (`"anchor_tokens": true`):**
   When rendering equations onto an Electronic Whiteboard where the tutor highlights, circles, or slash-cancels terms across steps, set `"anchor_tokens": true`. Theta wraps every AST sub-expression in `\htmlId{tok_<id>}{...}` (render in `KaTeX` with `{ trust: true }`), giving every numerator, denominator, exponent, and term an addressable DOM element ID.
5. **Visual + Symbolic Co-Design (`calc.analyze_graph` + `WhiteboardEngine`):**
   Whenever a GCSE, A-Level, or University question says *"Sketch the curve..."*, *"Find the coordinates of the turning points and asymptotes..."*, or *"Shade the region satisfying..."*, call:
   - `calc.analyze_graph` to get exact symbolic coordinates (`(x, y)` in `exact` and `katex`) for every intercept, local minimum/maximum, inflection point, asymptote, tangent/normal, and curve intersection to label on the diagram, **and**
   - `WhiteboardEngine::sample_1d` / `sample_implicit_2d` (or `vm.sample_1d` / `vm.sample_2d`) to plot the exact curve and asymptote breaks at 60–120 FPS.

---

### 3.2 End-to-End Solution Blueprints Across Stages

#### Blueprint A — Primary / 11+ Multi-Step Fraction & Percentage Word Problem
> *Question:* *"A baker has $3\frac{1}{4}\text{ kg}$ of flour. She uses $\frac{2}{5}$ of it for bread, and then uses $750\text{ g}$ for scones. How much flour is left, as a mixed number in $\text{kg}$?"*

1. **Convert `750 g` to exact `kg`** (`calc.eval`):
   `{"expr": "750 g in kg"}` $\to$ `exact: "3/4"`, `katex: "\\frac{3}{4}"`, `unit: "kg"`.
2. **Flour left after using $2/5$ for bread** (`calc.eval` with `allow_mixed_fractions: true`):
   `{"expr": "mixed(3, 1, 4) * (1 - 2/5)"}` $\to$ `exact: "39/20"`, `katex: "\\frac{39}{20}"` ($\text{kg}$).
3. **Subtract `3/4 kg` used for scones** (`calc.eval`):
   `{"expr": "39/20 - 3/4"}` $\to$ `exact: "6/5"` ($\text{kg}$).
4. **Convert improper fraction `6/5` to Primary mixed number** (`calc.transform`):
   `{"op": "mixed_fraction", "expr": "6/5"}` $\to$ `exact: "1 1/5"`, `katex: "1\\frac{1}{5}"`.
*The Solution Driver now writes a crystal-clear 4-step primary solution ($3\frac{1}{4} = \frac{13}{4}\text{ kg}$ $\to$ $\frac{3}{5}$ remaining $= \frac{39}{20}\text{ kg}$ $\to$ minus $\frac{3}{4}\text{ kg} = \frac{24}{20} = \frac{6}{5}\text{ kg} = 1\frac{1}{5}\text{ kg}$) with zero decimal rounding errors.*

---

#### Blueprint B — GCSE / A-Level Pure Maths: Curve Sketching, Intersections & Enclosed Area
> *Question:* *"Find the coordinates of the stationary points of $y = x^3 - 6x^2 + 9x + 1$, find where the curve meets the line $y = x + 1$, and calculate the exact finite area enclosed between the curve and the line."*

1. **Extract Exact Landmarks & Intersections in One Call** (`calc.analyze_graph`):
   ```json
   { "expr": "x^3 - 6*x^2 + 9*x + 1", "second_expr": "x + 1", "x_min": -2.0, "x_max": 6.0 }
   ```
   * Returns `stationary_points`: `(1, 5)` (`local_max`) and `(3, 1)` (`local_min`), `inflection_points`: `(2, 3)`, `y_intercept`: `(0, 1)`, and `intersections`: `(0, 1)`, `(2, 3)`, `(4, 5)`!
2. **Show Differentiated Working Steps** (`calc.transform` + `calc.solve` with `trace: true`):
   * `{"op": "diff", "expr": "x^3 - 6*x^2 + 9*x + 1", "var": "x"}` $\to$ `"3 * x^2 - 12 * x + 9"`
   * `{"op": "factor", "expr": "3*x^2 - 12*x + 9"}` $\to$ `"3 * (x - 3) * (x - 1)"`
3. **Compute Exact Enclosed Areas Over `[0, 2]` and `[2, 4]`** (`calc.transform`):
   * `{"op": "integrate", "expr": "(x^3 - 6*x^2 + 9*x + 1) - (x + 1)", "var": "x", "bounds": [0, 2]}` $\to$ `exact: "4"`
   * `{"op": "integrate", "expr": "(x + 1) - (x^3 - 6*x^2 + 9*x + 1)", "var": "x", "bounds": [2, 4]}` $\to$ `exact: "4"`
   * Total exact enclosed area: `"4 + 4"` $\to$ **`8`** square units (avoiding the classic student mistake of integrating straight from `0` to `4` which cancels to `0`!).
4. **Plot Both Curves on the Whiteboard** (`WhiteboardEngine::sample_1d` / `vm.sample_1d`).

---

#### Blueprint C — A-Level Physics: Photoelectric Effect & Stopping Potential (`uk_alevel_physics`)
> *Question:* *"Monochromatic UV light of wavelength $220\text{ nm}$ is incident on a zinc plate with work function $4.30\text{ eV}$. Find (a) the maximum kinetic energy of emitted photoelectrons in $\text{J}$, and (b) their maximum speed."*

1. **Convert Wavelength `220 nm` $\to$ `m` and Work Function `4.30 eV` $\to$ `J`** (`calc.eval`):
   * `{"expr": "220 nm in m"}` $\to$ `exact: "11/50000000"`, `approx: 2.2e-7`, `unit: "m"`
   * `{"expr": "4.30 eV in J"}` $\to$ `approx: 6.8893595262e-19`, `unit: "J"`
2. **Find Frequency $f = c / \lambda$** (`calc.formula`):
   * `{"formula_id": "physics.waves.wave_speed", "known": {"v": "299792458", "lambda": "2.2e-7"}, "solve_for": "f"}` $\to$ `exact` & `approx: 1.362692990909091e15` $\text{Hz}$.
3. **Apply Einstein's Photoelectric Equation $h f = \phi + E_{k,\max}$** (`calc.formula`, auto-injecting CODATA `h_planck`):
   * `{"formula_id": "physics.quantum.photoelectric", "known": {"f": "1.362692990909091e15", "phi_wf": "6.8893595262e-19"}, "solve_for": "E_k_max", "preset": "uk_alevel_physics"}`
   * Returns `rearranged`: `"E_k_max = f * h_planck - phi_wf"` and `approx: 2.13994...e-19`, `unit: "J"`.
4. **Solve $E_k = \frac{1}{2} m_e v^2$ for Maximum Electron Speed $v$** (`calc.formula`):
   * `{"formula_id": "physics.mechanics.kinetic_energy", "known": {"E_k": "2.13994e-19", "m": "9.1093837139e-31"}, "solve_for": "v"}`
   * Returns positive root `approx: 6.854...e5` (`m/s`), **explicitly recording the negative root in `discarded_branches`** (`violates constraint v >= 0`) and confirming $v \ll c$ (non-relativistic assumption valid!).
5. **Round Final Answers to 3 Significant Figures** (`calc.eval`):
   * `{"expr": "round_sf(685432.1, 3)"}` $\to$ `685000` ($6.85 \times 10^5\text{ m/s}$).

---

#### Blueprint D — GCSE / A-Level Statistics: Grouped Continuous Data, Higher Moments & Hypothesis Testing
* **Grouped Table & Higher Moments:** Calling `calc.stats` with `{"op": "summary", "kind": "grouped_table", "classes": [[0,10],[10,25],[25,50]], "frequencies": [8, 19, 13]}` returns **all intermediate exam-mark working arrays** in one call: `midpoints` ($x_i$), `fx_products` ($f_i x_i$), `cumulative_frequencies`, `sum`, `sum_sq`, `mean`, `midrange`, `rms`, `mean_deviation`, linear-interpolated `q1`/`median`/`q3`/`iqr`, outlier fences `fence_low`/`fence_high`, `modal_class`, `pop_std_dev` ($\div N$), `sample_std_dev` ($\div (N-1)$), `skewness`, `kurtosis`, and `excess_kurtosis`.
* **Hypothesis Test Question:** Calling `calc.stats` with `"op": "hypothesis_test"` returns `test_statistic`, `degrees_of_freedom`, `p_value`, `critical_values`, `reject_h0`, and `conclusion`—giving the Solution Driver everything needed to write the full 5-part A-Level structure ($H_0/H_1$ definition $\to$ distribution under $H_0$ $\to$ test statistic / critical region $\to$ comparison against $\alpha$ $\to$ contextual conclusion).

---

#### Blueprint E — Advanced / University STEM: Laplace ODE Transform, Vector Fields & Drumhead Modes
1. **Damped Driven Oscillator via Laplace Transform (`calc.transform`):**
   * Transform forcing term $f(t) = t e^{-2t}$: `{"op": "laplace", "expr": "t * exp(-2*t)", "var": "t"}` $\to$ `"1 / (s + 2)^2"`.
   * Decompose transfer response $Y(s)$ into partial fractions (`"op": "partial_fractions"`) and invert back to time domain: `{"op": "inv_laplace", "expr": "(2*s + 5) / ((s + 2)^2 + 9)", "var": "s"}` $\to$ `"2 * exp(-2 * t) * cos(3 * t) + (1 / 3) * exp(-2 * t) * sin(3 * t)"`.
2. **Conservative Vector Field Verification (`calc.transform`):**
   * Check irrotational condition $\nabla \times \mathbf{F} = \mathbf{0}$: `{"op": "curl", "expr": "[2*x*y, x^2 + cos(z), -y*sin(z)]", "vars": ["x", "y", "z"]}` $\to$ `"[0, 0, 0]"`.
3. **Circular Membrane Nodal Radius via Bessel Zero (`calc.solve`):**
   * Find first positive zero $j_{0,1}$ of $J_0(k r) = 0$: `{"equations": ["bessel_j(0, x) = 0"], "solve_for": ["x"], "domain": {"space": "real", "min": 2.0, "max": 3.0}}` $\to$ `2.404825557695773`.

---

## 4. Mathematical Scope, Units, Constants & Formula Registry Catalog

### 4.1 Nine Core Mathematical Domains
1. **Arithmetic, Fractions, Surds, Number Theory & 7-Base SI Units:** Exact `i64`/`BigInt`/`Rational`, `mixed(w,n,d)`, `%` / `% of`, `round_dp`, `round_sf`, `nCr`, `nPr`, `multinomial`, `bernoulli`, `euler_num`, Faulhaber `sum_powers(n, p)`, `gcd`, `lcm`, `prime_factors`, `euler_gamma`, `catalan`, `phi_golden`, quadratic surd reduction & denominator rationalization, exact $15^\circ$/$18^\circ$ special-angle trig, and SI + engineering/imperial/CGS dimensional conversions.
2. **Symbolic Algebra, Polynomials & Interpolation:** `simplify`, `expand`, `factor`, `complete_square`, `partial_fractions`, `rationalize`, `poly_gcd`, `poly_divmod`, exact Lagrange polynomial `interpolate`, and full reciprocal/inverse trig & hyperbolic laws (`coth`, `sech`, `csch`, `acot`, `asec`, `acsc`, `acoth`, `asech`, `acsch`).
3. **Equations, Systems & Inequalities:** Symbolic `rearrange`, linear/quadratic/cubic/quartic, degree 5–6+ **Aberth–Ehrlich** real/complex roots, `2x2`–`5x5+` linear systems, `2x2` non-linear conic systems, radical/log equations with extraneous-root & removable-pole rejection, bounded periodic trig (`[min, max]`), homogeneous `a*sinh(u) + b*cosh(u) = 0` & special-function root-finding, linear/quadratic/modulus/rational interval inequalities, and **Brent + Halley** transcendental `nsolve`.
4. **Calculus, Vector Analysis, Laplace Transforms, Series & ODEs:** Arbitrary-order `diff`, partial derivatives, `implicit_diff`, `parametric_diff`, 3D vector calculus (`grad`, `div`, `curl`, `laplacian`, `jacobian`, `hessian`), symbolic `laplace` & `inv_laplace`, symbolic `integrate` (reverse chain, LIATE IBP, partial fractions, radical linear $t=(ax+b)^{1/q}$, completing-the-square quadratic radicals, Ostrogradsky repeated quadratics, degree `<= 8` trig/hyp powers, Weierstrass rational trig & `37` compiled integral rules), exact FTC & **15/31-pt Gauss–Kronrod** definite/improper integrals, L'Hôpital `limit`, Taylor/Maclaurin `series`, and adaptive **Dormand–Prince `RK45`** ODE rigs.
5. **University Special Functions & Orthogonal Polynomials:** 15-digit Pure-Rust `digamma`, `beta`, `erfc`, `bessel_j`, `bessel_y`, `bessel_i`, `bessel_k` (Temme series + Steed CF1/CF2), `hypergeom_2f1`, complete & incomplete elliptic integrals (`elliptic_k`, `elliptic_e`, `elliptic_f`, `elliptic_e_inc`), `exp_e1`, `exp_ei`, `sin_si`, `cos_ci`, `fresnel_s`, `fresnel_c`, Riemann `zeta` (symbolic at even positive & non-positive integers + Borwein Chebyshev acceleration), and exact symbolic + numeric classical orthogonal polynomials (`legendre_p`, `assoc_legendre_p`, `hermite_h`, `laguerre_l`, `assoc_laguerre_l`, `chebyshev_t`, `chebyshev_u`).
6. **Matrices, Vectors, Coordinate Geometry & Bearings:** Matrix `+`, `-`, `*`, `A^k`, `det`, `inverse`, `rref`, `transpose`, `char_poly`, `eigenvalues`; 2D/3D `dot`, `cross`, `norm`, `angle_between`; `heron`, planar & spherical Sine/Cosine/Tangent rules, circle tangents/chords, 2D/3D point-line/plane distances, quadric surfaces, and clockwise `bearing(x1, y1, x2, y2)`.
7. **Statistics, Regression, Distributions & Hypothesis Tests:** Raw/frequency/grouped summary stats (including `midrange`, `rms`, `mean_deviation`, `skewness`, `kurtosis`, `excess_kurtosis`), `linear`/`exponential`/`power`/`polynomial` regression (`r`, `r_squared`, Spearman $\rho$), exact PDF/PMF/CDF/SF/PPF (`normal`, `student_t`, `chi_square`, `f`, `binomial`, `poisson`, `geometric`, `hypergeometric`), and 7 A-Level hypothesis tests.
8. **120 FPS Graph VM & Curve Landmarks:** `calc.analyze_graph` symbolic/numeric landmarks + `calc-vm` SIMD/GPU 1D curve sweeps (with exact pole & `NaN` domain breaks) and 2D Marching Squares contours/inequality masks.
9. **Bidirectional Formula Bank (`313` Compiled Formulas Across `9` TOML Packs in `registry/formulas/*.toml`):**

| TOML Pack | Count | Formula ID Namespaces Covered |
| :--- | ---: | :--- |
| `geometry.toml` | 32 | `geometry.circle.*` (area, circumference, arc, sector), `geometry.triangle.*` (Pythagoras, Sine/Cosine rule, area $\frac{1}{2}ab\sin C$, `heron`), `geometry.trapezium.*`, `geometry.parallelogram.*`, `geometry.prism.*`, `geometry.cuboid.*`, `geometry.cylinder.*`, `geometry.cone.*`, `geometry.sphere.*`, `geometry.pyramid.*`, `geometry.coord.*`, `geometry.conics.*` (ellipse, parabola, hyperbola, eccentricity), `geometry.vectors.*` |
| `schaum_mensuration_curves.toml` | 39 | `geometry.rhombus.*`, `geometry.polygon.regular_*`, `geometry.circle.{chord_length,segment_area,annulus_area}`, `geometry.{cube,tetrahedron,octahedron,frustum,torus,ellipsoid,paraboloid}.*`, `geometry.sphere.{cap_*,sector_volume,girard_triangle_area}`, `curves.{cycloid,astroid,cardioid,lemniscate,folium_descartes,catenary,witch_agnesi}.*` |
| `schaum_analytic_geometry.toml` | 22 | `geometry.coord.{slope_two_points,angle_between_slopes,triangle_area_vertices}`, `geometry.conics.{polar_focus,ellipse_latus_rectum,*_focal_distance}`, `geometry.coord3d.*` (distance, direction cosines, cylindrical/spherical), `geometry.quadrics.*`, `trig.plane.law_of_tangents`, `trig.spherical.*` |
| `mechanics.toml` | 56 | `physics.suvat.*` (all 5 constant-acceleration equations), `physics.mechanics.*` (weight, Hooke, work, $E_k$, $E_p$, $E_e$, power, efficiency, density, pressure, momentum, impulse, moments), `physics.materials.*` (stress, strain, Young modulus), `physics.circular.*`, `physics.shm.*`, `physics.rotational.*`, `physics.com.*`, `physics.collisions.*`, `physics.fluids.*` |
| `schaum_moments_of_inertia.toml` | 17 | `physics.rotational.{radius_of_gyration,parallel_axis,moi_rod_*,moi_rect_plate_*,moi_solid_cylinder_*,moi_hollow_cylinder_axial,moi_thin_hoop_axial,moi_cylindrical_shell_transverse,moi_solid_sphere_tangent,moi_hollow_sphere_thick,moi_solid_cone_*,moi_ellipsoid_axial,moi_torus_axial}` |
| `electricity.toml` | 31 | `physics.electricity.*` ($Q=It$, Ohm's law, $P=VI=I^2R=V^2/R$, resistivity, EMF & internal resistance, series/parallel resistors & capacitors, capacitor energy & $RC$ charge/discharge, transformers, AC rms), `physics.electronics.*` (potential divider, inverting/non-inverting op-amps) |
| `waves_optics.toml` | 27 | `physics.waves.*` ($T=1/f$, $v=f\lambda$, standing waves on strings, double-slit fringe spacing, diffraction grating $d\sin\theta = n\lambda$, intensity), `physics.optics.*` (refractive index, Snell's law, critical angle, thin lens & magnification), `physics.quantum.*` ($E=hf$, $E=hc/\lambda$, photoelectric, de Broglie), `physics.medical.*`, `physics.signalling.*` |
| `fields_nuclear_thermal.toml` | 47 | `physics.fields.*` (Newtonian gravity, orbital speed, escape velocity, Coulomb's law, electric field/potential, capacitance, magnetic force $BIL$ & $BQv$, flux linkage, Faraday), `physics.thermal.*` (SHC, latent heat, ideal gas $pV=nRT$ & $pV=Nk_BT$, kinetic theory $c_{\text{rms}}$, Boltzmann, Carnot), `physics.nuclear.*` ($E=mc^2$, exponential decay, half-life), `physics.astro.*` (Kepler, Wien, Stefan, Hubble, Schwarzschild), `physics.relativity.*` (Lorentz $\gamma$, time dilation, length contraction) |
| `finance_stats.toml` | 42 | `finance.*` (`compound_interest`, `present_value_compound`, `continuous_compounding`, `effective_annual_rate`, `ordinary_annuity_fv`, `ordinary_annuity_pv`, `perpetuity_pv`), `algebra.series.*`, `algebra.summation.*`, `stats.{probability,binomial,normal,geometric,neg_binomial,uniform,exponential,sampling,wilcoxon}.*`, `discrete.sets.*` |

### 4.2 Complete Reference of Units & CODATA Physical Constants

* **Compound Unit Syntax (`calc.eval`):** Supports products (`*` or space), quotients (`/`), and integer powers (`^`), e.g. `"72 km/h in m/s"`, `"250 cm^2 in m^2"`, `"14.7 psi in kPa"`, `"100 degC in K"`, `"68 degF in degC"`.
* **Atomic Unit Tokens by Dimension:**
  * **Length:** `m`, `km`, `cm`, `mm`, `um` (`μm`), `nm`, `pm`, `fm`, `angstrom` (`Å`), `mil`, `in` (`inch`), `ft`, `yd`, `fathom`, `mi` (`mile`), `nmi`, `AU`, `ly`.
  * **Area & Volume:** `m^2`, `cm^2`, `mm^2`, `km^2`, `ha` (`hectare`), `acre`, `barn`; `m^3`, `cm^3`, `L` (`l`, `litre`), `mL` (`ml`), `gal_us` (`gal`), `gal_uk` (`imp_gal`), `qt_us` (`qt`), `pt_us` (`pint`), `fl_oz`.
  * **Mass:** `kg`, `g`, `mg`, `ug` (`μg`), `carat`, `tonne`, `lb` (`lbs`), `oz`, `ton_us` (`short_ton`), `ton_uk` (`long_ton`), `slug`, `u` (`amu`).
  * **Time & Frequency:** `s` (`sec`), `ms`, `us` (`μs`), `ns`, `min`, `h` (`hr`), `day`, `yr`; `Hz`, `kHz`, `MHz`, `GHz`, `Bq`.
  * **Speed, Force & Pressure:** `mph`, `kn` (`knot`); `N`, `kN`, `MN`, `dyne` (`dyn`), `lbf`, `kgf`; `Pa`, `hPa` (`mbar`), `kPa`, `MPa`, `GPa`, `bar`, `atm`, `torr` (`mmHg`), `psi`.
  * **Energy & Power:** `J`, `mJ`, `kJ`, `MJ`, `GJ`, `erg`, `cal`, `kcal`, `btu` (`BTU`), `ft_lbf`, `Wh`, `kWh`, `eV`, `keV`, `MeV`, `GeV`; `W`, `mW`, `kW`, `MW`, `GW`, `hp`.
  * **Electricity, Magnetism, Temperature & Amount:** `A`, `mA`, `uA`; `C`, `mC`, `uC`, `nC`; `V`, `mV`, `kV`; `ohm` (`Ω`), `kohm`, `Mohm`; `F`, `mF`, `uF`, `nF`, `pF`; `T`, `mT`, `gauss` (`Gs`), `Wb`, `H`, `mH`; `K`, `degC` (`°C`), `degF` (`°F`); `mol`, `mmol`, `cd`, `rad`, `deg` (`°`).
* **Auto-Injectable NIST CODATA 2022 Constants (`calc.formula` & `calc.eval`):**
  `c_light` (`299792458 m/s`), `h_planck` (`6.62607015e-34 J*s`), `hbar` (`1.0545718176461565e-34 J*s`), `e_charge` (`1.602176634e-19 C`), `k_B` (`1.380649e-23 J/K`), `N_A` (`6.02214076e23 mol^-1`), `R_gas` (`8.31446261815324 J/(mol*K)`), `G_grav` (`6.67430e-11 m^3/(kg*s^2)`), `epsilon_0` (`8.8541878188e-12 F/m`), `mu_0` (`1.25663706127e-6 N/A^2`), `k_e` (`8.9875517862e9 N*m^2/C^2`), `m_e` (`9.1093837139e-31 kg`), `m_p` (`1.67262192595e-27 kg`), `m_n` (`1.67492750056e-27 kg`), `u_amu` (`1.66053906892e-27 kg`), `sigma_sb` (`5.670374419184429e-8 W/(m^2*K^4)`), `F_faraday` (`96485.33212331002 C/mol`), `g_0` (`9.80665 m/s^2`).

---

## 5. Architecture & Semantic JSON Endpoint Contracts

```text
┌───────────────────────────────────────────────────────────────────────────────┐
│  Layer 7: calc-api (lib name: theta_calc | bin: theta-calc-mcp)               │
│  MCP/HTTP/NDJSON Server • Rust SDK • C-FFI (theta_calc.h) • WASM Bindings     │
├───────────────────────────────────────────────────────────────────────────────┤
│  Layer 6: calc-vm                                                             │
│  Register Bytecode Compiler • wide::f64x4 CPU SIMD • Optional wgpu (WGSL)     │
├───────────────────────────────────────┬───────────────────────────────────────┤
│  Layer 4: calc-cas                    │  Layer 5: calc-numeric                │
│  Simplify • Factor • Solve/Isolate    │  Jet2 AD • Brent/Halley • Aberth      │
│  Diff • Integrate • VecCalc • Laplace │  Gauss-Kronrod • RK45 ODE • Stats     │
├───────────────────────────────────────┴───────────────────────────────────────┤
│  Layer 3: calc-registry                                                       │
│  build.rs TOML Compiler • PHF O(1) 313-Formula Lookup • 37 Integral Rules     │
├───────────────────────────────────────────────────────────────────────────────┤
│  Layer 2: calc-ir                                                             │
│  Clean Math ASCII + LaTeX Parsers • Hash-Consed Vec<Node> Arena • KaTeX       │
├───────────────────────────────────────┬───────────────────────────────────────┤
│  Layer 1A: calc-num                   │  Layer 1B: calc-tables                │
│  ExactInt • Rational • Surds • 7D SI  │  Primes • Bessel/Elliptic/2F1/Zeta    │
└───────────────────────────────────────┴───────────────────────────────────────┘
```

All endpoints accept both **Clean Math ASCII** (`"x^2 + 1"`) and **LaTeX** (`"\\frac{1}{\\sqrt{x^2 + 1}}"`) transparently, returning `"ok": true` on success or `"ok": false` with `"error": { "code": "<snake_case>", "message": "..." }` (`syntax_error`, `division_by_zero`, `domain_violation`, `dimension_mismatch`, `no_real_solution`, `singular_matrix`, `unknown_formula`, `budget_exceeded`, `unsupported`).

### 5.1 `calc.eval`
```json
{
  "expr": "bessel_j(0, 2.5) + euler_gamma",
  "vars": {},
  "preset": "uk_alevel_maths",
  "angle_mode": "rad",
  "space": "real",
  "allow_mixed_fractions": false,
  "anchor_tokens": false
}
```
*Built-in constants & expression functions:*
* **Constants:** `pi`, `e`, `i`, `inf`, `euler_gamma`, `catalan`, `phi_golden`.
* **Arithmetic, Combinatorics & Number Theory:** `sqrt`, `cbrt`, `root(n, x)`, `abs`, `floor`, `ceil`, `sign`, `round_dp(x, d)`, `round_sf(x, s)`, `mixed(w, n, d)`, `gcd(a, b)`, `lcm(a, b)`, `mod(a, b)`, `nCr(n, r)`, `nPr(n, r)`, `multinomial(k1, k2, ...)`, `bernoulli(n)`, `euler_num(n)`, `sum_powers(n, p)`.
* **Elementary Transcendentals:** `exp`, `ln`, `log10`, `log2`, `log(b, x)`, `sin`, `cos`, `tan`, `cot`, `sec`, `csc`, `asin`, `acos`, `atan`, `acot`, `asec`, `acsc`, `atan2(y, x)`, `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch`, `asinh`, `acosh`, `atanh`, `acoth`, `asech`, `acsch`.
* **Special Functions & Orthogonal Polynomials:** `erf`, `erfc`, `gamma`, `digamma`, `beta(a, b)`, `zeta(s)`, `bessel_j(nu, x)`, `bessel_y(nu, x)`, `bessel_i(nu, x)`, `bessel_k(nu, x)`, `legendre_p(n, x)`, `assoc_legendre_p(n, m, x)`, `hermite_h(n, x)`, `laguerre_l(n, x)`, `assoc_laguerre_l(n, alpha, x)`, `chebyshev_t(n, x)`, `chebyshev_u(n, x)`, `hypergeom_2f1(a, b, c, z)`, `elliptic_k(m)` ($m = k^2$), `elliptic_e(m)`, `elliptic_f(phi, m)`, `elliptic_e_inc(phi, m)`, `exp_e1(x)`, `exp_ei(x)`, `sin_si(x)`, `cos_ci(x)`, `fresnel_s(x)`, `fresnel_c(x)`.
* **Vectors & Geometry:** `dot(u, v)`, `cross(u, v)`, `norm(u)`, `angle_between(u, v)`, `transpose(M)`, `heron(a, b, c)`, `bearing(x1, y1, x2, y2)`.

### 5.2 `calc.solve`
```json
{
  "equations": ["sqrt(2*x + 3) = x - 3"],
  "solve_for": ["x"],
  "domain": { "space": "real", "min": null, "max": null },
  "angle_mode": "deg",
  "trace": true
}
```
Returns `method`, `solutions[]` (array of variable-to-`{exact, approx, katex}` maps), `interval_set` (for inequalities like `"2*x^2 - 5*x - 3 <= 0"` $\to$ `"[-1/2, 3]"`), `discarded_branches[]`, and optional `steps[]` (`{rule, description, exact, katex}`). Also solves simultaneous systems (`equations: ["x^2 + y^2 = 25", "y = x + 1"], solve_for: ["x", "y"]`) and special-function roots on bounded intervals (`min`/`max`).

### 5.3 `calc.formula`
```json
{
  "formula_id": "finance.ordinary_annuity_pv",
  "known": { "PMT": "500", "i_rate": "0.05", "n": "10" },
  "solve_for": "PV",
  "preset": "uk_alevel_maths"
}
```
*(Omit `known` or pass `{}` to get a pure symbolic `rearranged` formula; omit `solve_for` when `N - 1` variables are bound to infer the unique unknown automatically.)*

### 5.4 `calc.transform` (`32` Operations)
* **Algebra, Interpolation & Number Theory:** `simplify`, `expand`, `factor`, `complete_square`, `partial_fractions`, `rationalize`, `poly_gcd` (`second_expr`), `poly_divmod` (`second_expr`), `interpolate` (`points: [[x0, y0], ...]`), `prime_factors`, `hcf_lcm` (`integers` or `second_expr`), `mixed_fraction`.
* **Calculus & Integral Transforms:** `diff` (`var`, `order`), `implicit_diff` (`expr`, `var`, `second_expr`), `parametric_diff` (`expr`: $y(t)$, `second_expr`: $x(t)$, `var`: `"t"`, `order`), `integrate` (omit `bounds` for indefinite; `"bounds": [a, b]` for definite/improper), `limit` (`var`, `at`), `series` (`var`, `at`, `order`), `laplace` (`var`: `"t"` $\to s$), `inv_laplace` (`var`: `"s"` $\to t$).
* **Vector & Multivariable Calculus:** `grad` (`expr`, `vars`), `div` (`expr`: `"[Fx, Fy, Fz]"`, `vars`), `curl` (`expr`: `"[Fx, Fy, Fz]"`, `vars`), `laplacian` (`expr`, `vars`), `jacobian` (`expr`: `"[f1, f2, ...]"`, `vars`), `hessian` (`expr`, `vars`).
* **Matrices (`"matrix": [[...], ...]`):** `det`, `inverse`, `rref`, `transpose`, `char_poly`, `eigenvalues`.

*Representative `calc.transform` requests:*
```json
{ "op": "integrate", "expr": "1 / (3 + 2*sin(x) + cos(x))", "var": "x" }
{ "op": "grad", "expr": "x^2 * y + sin(z)", "vars": ["x", "y", "z"] }
{ "op": "jacobian", "expr": "[r * cos(theta), r * sin(theta)]", "vars": ["r", "theta"] }
{ "op": "laplace", "expr": "t^2 * exp(-3*t) + sin(4*t)", "var": "t" }
{ "op": "inv_laplace", "expr": "(3*s + 7) / ((s + 1)*(s - 2))", "var": "s" }
{ "op": "interpolate", "points": [[0, 1], [1, 3], [2, 7], [3, 13]] }
```

### 5.5 `calc.stats`
* `"op": "summary"` with `"kind": "raw"` (`values`), `"frequency_table"` (`values`, `frequencies`), or `"grouped_table"` (`classes`, `frequencies`) — returns `count`, `sum`, `sum_sq`, `mean`, `geometric_mean`, `harmonic_mean`, `min`, `max`, `range`, `midrange`, `rms`, `mean_deviation`, `median`, `q1`, `q3`, `iqr`, `fence_low`, `fence_high`, `outliers`, `modes`, `modal_class`, `pop_variance`, `pop_std_dev`, `sample_variance`, `sample_std_dev`, `skewness`, `kurtosis`, and `excess_kurtosis`.
* `"op": "regression"` with `"model": "linear" | "exponential" | "power" | "polynomial"` (`x`, `y`, optional `degree`).
* `"op": "distribution"` with `"dist": "normal" | "student_t" | "chi_square" | "f" | "binomial" | "poisson" | "geometric" | "hypergeometric"` (`x` for `pdf_or_pmf`/`cdf`/`survival_upper`, and/or `prob` for `quantile_ppf`).
  ```json
  { "op": "distribution", "dist": "hypergeometric", "pop_n": 52, "success_r": 13, "draws_n": 5, "x": 2 }
  ```
* `"op": "hypothesis_test"` with `"test_kind": "one_sample_z" | "one_sample_t" | "binomial_exact" | "poisson_exact" | "pmcc_correlation" | "chi_square_goodness_of_fit" | "chi_square_independence"`.

### 5.6 `calc.analyze_graph`
```json
{ "expr": "x^3 - 3*x", "second_expr": "x", "x_min": -10.0, "x_max": 10.0, "tangent_at": 2 }
```
Returns `y_intercept`, `x_intercepts`, `stationary_points`, `inflection_points`, `asymptotes`, `intersections`, `tangent_line`, and `normal_line`.

### 5.7 `vm.*` Real-Time Graph & ODE Stream Operations (`--ndjson` / `WhiteboardEngine`)
When running `theta-calc-mcp --ndjson` (or calling `WhiteboardEngine` directly in Rust / C-FFI / WASM):
* **`vm.sample_1d`:** `{"expr": "a * tan(w * x)", "var": "x", "x_min": -5.0, "x_max": 5.0, "num_points": 2000, "params": {"a": 1.5, "w": 2.0}}` $\to$ returns interleaved `xy` (`null` at `NaN` domain gaps) + `segment_breaks[]` (exact pole indices so whiteboards never draw false vertical asymptote lines!).
* **`vm.sample_2d`:** `{"expr": "x^2 + y^2 <= r^2", "x_min": -4.0, "x_max": 4.0, "y_min": -4.0, "y_max": 4.0, "nx": 128, "ny": 128, "params": {"r": 2.5}}` $\to$ Marching Squares contour segments + boolean `region_mask`.
* **`vm.ode_rk45`:** `{"rhs": ["v", "-omega^2 * x - 2*zeta_d*omega*v"], "state_vars": ["x", "v"], "time_var": "t", "t0": 0.0, "t_end": 5.0, "y0": [1.0, 0.0], "params": {"omega": 3.0, "zeta_d": 0.2}}` $\to$ adaptive Dormand–Prince `RK45` trajectory state.

---

## 6. Five Application Integration Modes

### Mode 1: MCP Tool Server (`stdio` or `--http`), REST & CLI for AI Agents
Speaks **Model Context Protocol (`2024-11-05`, JSON-RPC 2.0)** natively:
```json
{
  "mcpServers": {
    "theta-calc": {
      "command": "/path/to/theta-calc-mcp",
      "args": ["--mcp"]
    }
  }
}
```
* **HTTP JSON-RPC & Direct REST Server:** `./target/release/theta-calc-mcp --http 127.0.0.1:8080`
  * `GET /health` $\to$ service readiness check
  * `GET /tools` $\to$ full MCP JSON Schema tool definitions
  * `POST /mcp` $\to$ MCP JSON-RPC 2.0 endpoint
  * `POST /call/<tool>` (e.g. `POST /call/calc.eval`, `POST /call/calc.transform`) $\to$ direct REST JSON call!
* **Direct CLI:** `./target/release/theta-calc-mcp eval "30% of 40"`, `solve`, `formula`, `transform`, `stats`, `analyze-graph`, `tools`.

### Mode 2: Persistent `--ndjson` Subprocess Stream (Python / Node.js / Go Solution Drivers)
Keep a persistent worker process open and exchange one JSON line per call (`~6,940–8,700+` verified calls/sec on 16 cores, zero OS process-spawn overhead):

```python
import json, subprocess

proc = subprocess.Popen(
    ["./target/release/theta-calc-mcp", "--ndjson"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    text=True,
    bufsize=1,
)

def call_theta(tool: str, args: dict) -> dict:
    proc.stdin.write(json.dumps({"id": 1, "tool": tool, "args": args}) + "\n")
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())["response"]
```

### Mode 3: Direct In-Process Rust Crate (`theta_calc`)
```toml
[dependencies]
theta_calc = { package = "calc-api", path = "crates/calc-api" }
```
```rust
use theta_calc::{EvalRequest, WhiteboardEngine, calc_eval};

let resp = calc_eval(&EvalRequest { expr: "72 km/h in m/s".into(), ..Default::default() });
assert_eq!(resp.result.unwrap().exact, "20");

let mut wb = WhiteboardEngine::new();
let fn_id = wb.compile_expr("A * sin(w * x)", &["x"], &["A", "w"]).unwrap();
let mut xy = vec![0.0_f64; 2 * 2000];
let mut breaks = Vec::new();
wb.sample_1d(fn_id, -10.0, 10.0, &[2.5, 3.0], &mut xy, &mut breaks).unwrap();
```

### Mode 4: Native C / C++ / Swift (iOS & macOS) / Kotlin JNI (Android) via C-FFI
`cargo build --release` builds `libtheta_calc.so` / `.dylib` / `.dll` (`~2.7 MB`) and `libtheta_calc.a` / `.lib`, paired with [`include/theta_calc.h`](include/theta_calc.h):
```c
#include "include/theta_calc.h"

char *json = theta_calc_call_json("calc.eval", "{\"expr\":\"sin(15)\",\"angle_mode\":\"deg\"}");
/* parse JSON ... */
theta_calc_free_str(json); /* Always free via theta_calc_free_str! */
```
* **Thread Safety:** `theta_calc_call_json` is stateless and thread-safe across all threads; use one `WhiteboardEngine*` (`theta_wb_new` / `theta_wb_free`) per render thread.

### Mode 5: WebAssembly (`wasm32-unknown-unknown`) for Browser & Web Whiteboards
```bash
wasm-pack build crates/calc-api --target web --release
```
```ts
import init, { theta_calc_dispatch, WasmWhiteboardEngine } from "./pkg/calc_api.js";
await init();

const out = JSON.parse(theta_calc_dispatch("calc.transform", JSON.stringify({
  op: "complete_square", expr: "2*x^2 - 12*x + 11"
})));

const wb = new WasmWhiteboardEngine();
const id = wb.compile_expr("a * tan(w * x)", "x", "a,w");
const xy = new Float64Array(2 * 1025);
const breaks = wb.sample_1d(id, -4.0, 4.0, new Float64Array([2.0, 1.0]), xy);
```

---

## 7. OS, Platform, Hardware & Operational Information

### 7.1 Cross-Platform Target Matrix

| OS / Environment | Rust Target Triple(s) | Produced Artifacts |
| :--- | :--- | :--- |
| **Linux (`x86_64` & `ARM64` / DGX)** | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (`musl` static supported) | `theta-calc-mcp` (`2.82 MB`), `libtheta_calc.so`, `libtheta_calc.a` |
| **macOS (Apple Silicon & Intel)** | `aarch64-apple-darwin`, `x86_64-apple-darwin` | `theta-calc-mcp`, `libtheta_calc.dylib`, `libtheta_calc.a` |
| **iOS / iPadOS** | `aarch64-apple-ios`, `aarch64-apple-ios-sim` | `libtheta_calc.a` (`.xcframework` ready) |
| **Android (NDK)** | `aarch64-linux-android`, `x86_64-linux-android` | `libtheta_calc.so` |
| **Windows** | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | `theta-calc-mcp.exe`, `theta_calc.dll`, `theta_calc.lib` |
| **Web / Browser / Workers** | `wasm32-unknown-unknown` | `theta_calc.wasm` + JS/TS `wasm-bindgen` bindings |

### 7.2 Toolchain, SIMD & Optional `wgpu` Compute
* **Build Requirement:** Rust toolchain supporting **Edition 2024** (`rustc >= 1.85`). Zero C/C++/Fortran system packages required.
* **Runtime Requirement:** Zero external files, databases, or environment variables—all formulas, rules, primes, quadrature weights, and CODATA constants reside in `.rodata`.
* **Portable CPU SIMD (Default):** Uses `wide::f64x4` (`AVX2` on x86_64, `NEON` on ARM64 / Apple Silicon, `SIMD128` on WASM). Recommended for 99% of workloads.
* **Optional Cross-Platform GPU (`--features gpu`):** Enables `wgpu` (`WGSL` $\to$ Vulkan / Metal / DX12 / WebGPU) for dense `>= 50,000`-point 2D implicit/inequality grids with automatic headless CPU SIMD fallback.

### 7.3 Measured Latency Profile (`aarch64 --release`, 150,000-Question Audit)

| Endpoint / Operation | Calls Audited | `p50` | `p95` | `p99` |
| :--- | ---: | ---: | ---: | ---: |
| `calc.stats` (summary moments, regression, 8 distributions, hypothesis tests) | `13,243` | `6.85 µs` | `12.13 µs` | `14.06 µs` |
| `calc.eval` (rationals, `%`, surds, 29+ SI/imperial units, Bessel/Elliptic/`2F1`/`zeta`) | `37,218` | `15.18 µs` | `37.70 µs` | `72.18 µs` |
| `vm.ode_rk45` (coupled adaptive `RK45` trajectory integration) | `1,800` | `20.80 µs` | `34.00 µs` | `39.89 µs` |
| `vm.sample_1d` (`compile_fn` `13.8 µs` + 2k-pt SIMD sweep `18.8 µs`) | `3,200` | `33.42 µs` | `74.53 µs` | `88.59 µs` |
| `calc.transform` (algebra, Ch. 17 `integrate`, `grad`/`curl`, `laplace`, `interpolate`, `rref`) | `51,723` | `58.16 µs` | `159.36 µs` | `259.18 µs` |
| `calc.solve` (equations, `2x2`–`5x5` systems, Aberth roots, Bessel zeros, `nsolve`) | `24,030` | `61.98 µs` | `205.76 µs` | `723.91 µs` |
| `calc.formula` (313-formula bidirectional rearrange + SI/branch-checked solve) | `14,986` | `76.94 µs` | `268.62 µs` | `956.53 µs` |
| `vm.sample_2d` (Marching Squares implicit contour + inequality region mask) | `1,200` | `97.73 µs` | `302.99 µs` | `312.23 µs` |
| `calc.analyze_graph` (exact intercepts, turning/inflection pts, asymptotes, intersections) | `2,600` | `113.74 µs` | `310.33 µs` | `324.43 µs` |
| **Overall Across All 150,000 Verified Questions** | **`150,000`** | **`43.14 µs`** | **`164.98 µs`** | **`319.83 µs`** |

---

## 8. Extending Theta & Running the 150k Verification Suite

For step-by-step maintainer workflows (4-bucket PDF ingestion rule, 7-step special-function crate checklist, numerical/solver tricks, and unattended Git/SSH setup), see [`sop.md`](sop.md).

1. **Authoring New Formulas / Rules:** Add entries to `registry/formulas/*.toml` or `registry/rules/*.toml` in Canonical Math ASCII. `crates/calc-registry/build.rs` validates syntax at compile time, and `cargo test --workspace` runs the 5-point SI dimensional & bidirectional round-trip audit across every variable of every formula (`313 / 313`).
2. **Running the 150,000-Question `SymPy + mpmath` (50-Digit) Audit:**
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   cargo build --release
   qa-oracle/.venv/bin/python qa-oracle/generators/gen_all.py
   qa-oracle/.venv/bin/python qa-oracle/run_audit.py
   ```
   `cat8_registry_formulas.py` dynamically loads all `.toml` files in `registry/formulas/`, and `cat9_schaum_advanced.py` audits `50,000` randomized questions across Ch. 1–47 & Tables 32–35 of *Schaum's Mathematical Handbook*.
