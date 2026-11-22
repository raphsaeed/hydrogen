---
product: primary
course_id: PRI-MTH
subject: maths
stage_label: "Primary (KS2)"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: true
  preset: uk_gcse
  angle_mode: deg
  allow_mixed_fractions: true
defaults:
  answer_format: written
  calculator_mode: non-calculator
  max_sublevel: 9
  target_min_sublevel: 1
level_9_policy:
  forbidden_at_level_9:
    - simultaneous_equations
    - quadratic_equations
    - trigonometry
---

# Primary Maths — Subject Rules (`primary/maths/common.md`)

- **Primary Methods at `Level 9`:** Use Carbon `barModel`, `columnCalc` (with carries/borrows and bus-stop division), `fractionBar`, `functionMachine` (working backwards), and mixed numbers (`allow_mixed_fractions: true`).
- *(Optional class-year overlays `year-3.md`..`year-6.md` may be added in this folder as needed.)*
