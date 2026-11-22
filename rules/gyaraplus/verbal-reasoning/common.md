---
product: gyaraplus
course_id: GYR-VR
subject: verbal-reasoning
stage_label: "11+"
inherits:
  - ../../common.md
  - ../common.md
levels_file: ./levels.md
theta:
  enabled: true
  preset: uk_gcse
defaults:
  answer_format: mcq
  mcq_option_count: 5
  calculator_mode: non-calculator
  max_sublevel: 9
  target_min_sublevel: 1
---

# Gyaraplus 11+ Verbal Reasoning — Subject Rules (`gyaraplus/verbal-reasoning/common.md`)

> **Purpose:** Rules for all 21+ classic 11+ Verbal Reasoning (VR) question types in **`gyara-verbal-reasoning.raphs`**.

---

## 1. Block Instructions & Worked Examples (`course_stimuli`)
- VR papers group 5–10 questions under shared rule instructions and a printed example (*Hidden 4-Letter Words*, *Letter Series*, *Word-Number Codes*, *Compound Words*, *Number Sequences*, *Logic Deductions*). Store shared block rules in `course_stimuli` (`stimulus_kind: "code_key"`) and display clearly above the stem.

## 2. Visual VR Solving Rules on Carbon
1. **Alphabet & Cipher Questions:** Always render the relevant `A..Z` alphabet strip with position indices (`1..26`) inside a crisp Carbon `smartTable` or `numberLine` with hop arrows (`+2`, `-1`)—never ask a child to count letters in their head blindly. *(Verify arithmetic shifts in Theta!)*
2. **Word Construction (Hidden / Missing / Compound Words):** Visually bracket exact letter boundaries using `\htmlId` highlights (`ba[r]n` + `ar[m]s`, `CA[MP]US`).
3. **Vocabulary (Synonyms / Antonyms / Odd-One-Out / Analogies):** Give crisp Year-6 definitions of every candidate word (`chrome.setDictionary` + step table) so vocabulary grows on every question.
4. **Logic & Deduction Puzzles:** Build a step-by-step elimination grid (`smartTable` with `✓` and `✗` cells) clue by clue.
