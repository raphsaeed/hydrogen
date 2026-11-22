---
product: gcse
course_id: GCSE-PHY
subject: physics
stage_label: "GCSE"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: true
  preset: uk_gcse
  angle_mode: deg
  anchor_tokens: true
defaults:
  answer_format: written
  calculator_mode: allowed
  max_sublevel: 9
  target_min_sublevel: 1
---

# GCSE Physics — Subject Rules (`gcse/physics/common.md`)

- **Theta Preset (`g = 9.8 m/s²`):** Always pass `"preset": "uk_gcse"` (`g = 9.8 m/s^2`, `angle_mode: "deg"`).
- **Mandatory FIFA Method at `Level 9`:**
  1. **Convert to SI First (`C1` mark):** Convert `kJ -> J`, `cm -> m`, `g -> kg`, `minutes -> s`, `km/h -> m/s` via `calc.eval`.
  2. **Formula:** State governing equation via `calc.formula`.
  3. **Insert & Fine-Tune (Rearrange):** Show substitution and symbolic rearrangement (`rearranged.katex`), justifying any rejected negative root (`discarded_branches`).
  4. **Answer + Sig-Figs + SI Unit:** Round to `2` or `3 s.f.` matching input precision and attach explicit SI unit (`J`, `W`, `N`, `Pa`, `V`, `A`, `\Omega`, `Hz`).
- **Carbon `PhysicsRig` Visuals:** Pair mechanics/optics/electricity problems with Carbon `PhysicsRig` components (`spring`, `cart`, `inclinedPlane`, `pulley`, `pendulum`, `opticsLensRay`, `forceBody`).
