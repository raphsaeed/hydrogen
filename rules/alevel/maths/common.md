---
product: alevel
course_id: ALV-MTH
subject: maths
stage_label: "A-Level"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: true
  preset: uk_alevel_maths
  angle_mode: rad
  allow_mixed_fractions: false
  anchor_tokens: true
defaults:
  answer_format: written
  calculator_mode: allowed
  max_sublevel: 9
  target_min_sublevel: 1
---

# A-Level Maths — Subject Rules (`alevel/maths/common.md`)

- **Theta Grounding:** `"preset": "uk_alevel_maths"` (**$g = 9.8\text{ m/s}^2$** in Mechanics; `2 s.f.` or `3 s.f.` when $g = 9.8$ is used). Default `"angle_mode": "rad"` for all calculus and radian trig (`"deg"` only when degrees are explicitly specified).
- **Calculus, Algebra & Stats:** Verify derivatives (`diff`), integrals (`integrate`), partial fractions, bounded trig solutions, and the 5-part A-Level `hypothesis_test` structure ($H_0/H_1 \to$ model $\to$ statistic/critical region $\to$ comparison $\to$ contextual conclusion) via `theta_calc`.
