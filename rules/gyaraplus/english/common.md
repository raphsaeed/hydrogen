---
product: gyaraplus
course_id: GYR-ENG
subject: english
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

# Gyaraplus 11+ English — Subject Rules (`gyaraplus/english/common.md`)

> **Purpose:** Comprehension, Grammar/Cloze, Vocabulary Dictionary, and Composition rules for **`gyara-english.raphs`**.

---

## 1. Shared Passages (`course_stimuli`) & Paper Sections

1.1 **Reading Passages & Timed Sections:**
- Store shared reading extracts once in `course_stimuli` (`stimulus_kind: "passage"`) with paragraph numbers (`[1]`, `[2]`, ...) and link all questions using that passage via `stimulus_id`.
- Preserve paper sections (`course_paper_sections`: e.g., *Section A — Comprehension*, *Section B — Grammar / Punctuation / Spelling*, *Section C — Creative Composition*).
- Keep the relevant paragraph excerpt (with `\htmlId{tok_quote_...}{...}` spans on target evidence phrases) visible in the Left Question Pane so the student never hunts across screens.

1.2 **Mandatory 11+ Vocabulary Dictionary (`chrome.setDictionary`):**
- Every English passage and question MUST populate `question_json.dictionary` covering unfamiliar, literary, or archaic words for a Year-6 student (with part of speech, clear 1–2 sentence child-friendly definition, pronunciation note for tricky words, and short example sentence).

---

## 2. Comprehension & Grammar Solution Rules

2.1 **MCQ Comprehension & Grammar (Default):**
- Use **P.E.E. (Point, Evidence, Explanation)** anchored directly to highlighted passage spans (`focus.annotate` with consistent colors).
- Show **why** the correct option is proven by the text and **why** tempting distractor options twist or over-stretch the text.

2.2 **Written Comprehension (`answer_format: "written"`):**
- Follow the 4-part structure: **(1) Prompt Breakdown**, **(2) Highlighted Text Evidence → Technique/Meaning**, **(3) Realistic Year-6 Model Paragraph** (written at authentic top-scoring 10–11-year-old length, never university literary prose; include a `Text Detail → Own Words` Carbon `smartTable` for synthesis questions), and **(4) Self-Marking Rubric Table** (`marks` breakdown).

---

## 3. Composition / Creative Writing Rules (`answer_format: "written"`)

- Teach the timed split (**5m Plan** $\to$ **22m Writing** $\to$ **3m SPaG Check**):
  1. **5-Minute Plan (`smartTable`):** 4–5 paragraph structure with sensory & figurative targets.
  2. **Technique Showcase:** Strong verbs, imagery, and varied openers/punctuation.
  3. **Realistic Year-6 Model Piece (~260–310 words)** + **Self-Marking Rubric Table**.
