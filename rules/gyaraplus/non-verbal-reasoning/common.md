---
product: gyaraplus
course_id: GYR-NVR
subject: non-verbal-reasoning
stage_label: "11+"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: false
defaults:
  answer_format: mcq
  mcq_option_count: 5
  calculator_mode: non-calculator
  max_sublevel: 9
  target_min_sublevel: 1
---

# Gyaraplus 11+ Non-Verbal Reasoning — Subject Rules (`gyaraplus/non-verbal-reasoning/common.md`)

> **Purpose:** Visual geometry, 3D spatial nets/cubes, and **SPANS** elimination rules for **`gyara-non-verbal-reasoning.raphs`**.

---

## 1. Visual Stimulus & Single-Row `A–E` Choices

1.1 **Zero-Scroll Horizontal Row Layout:**
- Figure series, analogies, matrices, and `A–E` visual answer choices MUST sit cleanly in compact horizontal rows (`scene.place2D` / `scene.place3D`) so the entire puzzle and all 5 options are visible simultaneously with zero scrolling.
- Every polygon side count, fill/hatching/shading, line thickness, arrow orientation, and rotation angle must match the source puzzle with 100% geometric precision.

1.2 **3D Spatial Nets & Block Views:**
- Use Carbon's native `scene.place3D` **`"netFold"`** (supporting all 11 cube hexomino nets + NVR `"faceMarkings"` like arrows, diagonals, dots, stripes, and `@fold` animation `0.0 → 1.0`) and **`"cubeAssembly"`** (`front`, `side`, `plan` elevation highlights).

---

## 2. Mandatory SPANS Elimination Architecture

Teach students never to guess by *"what looks right"*—solve systematically using **SPANS**:
- **S**hape · **P**osition · **A**ngle / Rotation · **N**umber · **S**hading

1. **Step 1 — Easiest SPANS Feature First + Immediate Cross-Out:** Isolate the clearest visual rule (e.g., *Number of inner dots* or *Shading swap*) and immediately cross out (`focus.annotate` `"kind": "cross"`) the 2–3 `A–E` options that fail it!
2. **Step 2 — Second SPANS Feature on Survivors Only:** Check the next feature (e.g., *$90^{\circ}$ clockwise rotation*) only on the surviving options until 1 unique winner remains.
3. **Final Verification Table:** Compact `smartTable` (`SPANS Feature | Rule | Winning Option`) confirming 100% match.
