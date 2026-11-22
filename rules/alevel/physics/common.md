---
product: alevel
course_id: ALV-PHY
subject: physics
stage_label: "A-Level"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: true
  preset: uk_alevel_physics
  angle_mode: rad
  anchor_tokens: true
defaults:
  answer_format: written
  calculator_mode: allowed
  max_sublevel: 9
  target_min_sublevel: 1
---

# A-Level Physics — Subject Rules (`alevel/physics/common.md`)

- **Critical Gravity Convention:** Always pass `"preset": "uk_alevel_physics"` (**$g = 9.81\text{ m/s}^2$** — distinct from A-Level Maths Mechanics $9.8$!) and auto-inject NIST CODATA constants (`h_planck`, `c_light`, `e_charge`, `k_B`, `G_grav`, `epsilon_0`).
- **Units, Prefixes & Sig-Figs:** Explicitly show `eV <-> J`, `nm -> m`, `u_amu -> kg`, `degC -> K` conversions via `calc.eval`, match final significant figures to the least precise input datum (`2–3 s.f.`), and use `vm.ode_rk45` / `calc.stats` log-linear regression for SHM, capacitor decay, and data-analysis questions.
