---
product: gcse
course_id: GCSE-MTH
subject: maths
stage_label: "GCSE"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: true
  preset: uk_gcse
  angle_mode: deg
  allow_mixed_fractions: false
  anchor_tokens: true
defaults:
  answer_format: written
  calculator_mode: allowed
  max_sublevel: 9
  target_min_sublevel: 1
level_9_policy:
  forbidden_at_level_9:
    - calculus_integration
    - radian_measure
    - complex_numbers
---

# GCSE Maths — Subject Rules (`gcse/maths/common.md`)

- **Theta Grounding:** `preset: "uk_gcse"`, `angle_mode: "deg"`. Use `calc.transform` (`factor`, `expand`, `complete_square`, `rationalize`), `calc.solve` (`trace: true`), `calc.stats` (`grouped_table` midpoints, quartiles), and `calc.analyze_graph` (exact roots, turning points, intersections).
- **Geometry & Graphs:** Couple `coordinatePlane` curves to exact symbolic landmarks from `calc.analyze_graph` and 3D mensuration solids (`cylinder`, `cone`, `frustum`, `sphere`, `prism`) to live equations (`lockViewScale: true`).
- **Tier Overrides:** Load `foundation.md` or `higher.md` when compiling tier-specific papers.
