# Gyaraplus (11+) — Product-Wide Common Rules (`gyaraplus/common.md`)

> **Scope:** Shared across all four **Gyara 11+** subject vaults:
> - `gyara-maths.raphs` (`GYR-MTH`)
> - `gyara-english.raphs` (`GYR-ENG`)
> - `gyara-verbal-reasoning.raphs` (`GYR-VR`)
> - `gyara-non-verbal-reasoning.raphs` (`GYR-NVR`)
>
> Loaded automatically after `rules/common.md` and before `rules/gyaraplus/<subject>/{common,levels}.md`.

---

## 1. 11+ Learner Cohort & Tutor Voice

1.1 **Target Learner:**
- **Age 9–11 (UK Year 5 / Year 6)** preparing for competitive 11+ Grammar School (GL / CEM style) and Independent School entrance/scholarship exams.
- Voice in `narration` must sound like a **warm, encouraging, sharp 11+ mentor**—clear, lively Year-6 English, never patronizing and never dry university textbook jargon.

1.2 **Concept-First Strategy Opening:**
- At the start of Step 1 (`narration` + optional compact strategy header badge on Carbon), name the **11+ Question Archetype / Mental Shortcut** in memorable terms (e.g., *"In an 11+ exam, this is a classic **Algebraic Substitution** puzzle..."*, *"**Closing-the-Gap (Catch-Up)** problem"*, *"**Point-Evidence-Explanation** retrieval"*, *"**SPANS** elimination"*), telling the student *why* this approach beats slow guesswork or exam traps.

---

## 2. 11+ Question Formats, Pacing & Multiple-Choice Discipline

2.1 **5-Option Multiple-Choice (`A–E`) Default + Written Support:**
- Real-world 11+ exams are predominantly **5-option Multiple Choice (`A, B, C, D, E`)**, with **Written / Fill-in** questions in Independent School papers and English Composition.
- In `gyaraplus`, **default to `"answer_format": "mcq"` (`5` options `A–E`)** unless the source paper/section explicitly requires extended written working (`"written"`).
- When synthesizing `A–E` options (either converting a written question to MCQ or generating replicas), the 4 wrong options **must be realistic 11+ diagnostic traps** (e.g., forgetting the final step, using the wrong base percentage, mis-clearing a negative sign, or off-by-one counting)—verified deterministically so all 5 choices are distinct and **exactly one** is right.

2.2 **Exam Timing & Non-Calculator Standard:**
- Default `calculator_mode` is **`"non-calculator"`** across all 11+ papers.
- Calibrate `recommended_time_sec` realistically (~`45–75s` for standard 11+ Maths/VR/NVR MCQs; ~`90–120s` for stretch/multi-step word problems; section-appropriate minutes for English Comprehension/Composition).
- Surface concise **11+ Mental & Exam-Craft Tricks** where they save real exam time (e.g., cancelling common factors before multiplying, checking last digits, or eliminating impossible options early).

---

## 3. Difficulty Calibration (`1` to `5` for 11+)

Every question in `gyaraplus` is calibrated on a 1–5 11+ entrance scale:
- **`1` — Foundation:** Direct 1-step fluency / warm-up item.
- **`2` — Core 11+:** Standard 1–2 step GL/CEM routine problem.
- **`3` — Grammar Standard:** Classic multi-clue 11+ exam question with a standard distractor trap.
- **`4` — Selective / Stretch:** Multi-concept synthesis requiring careful setup (e.g., simultaneous / bracket shift / hidden geometry).
- **`5` — Super-Selective / Scholarship:** Challenging olympiad/independent-scholarship reasoning problem.

---

## 4. Interactive 11+ Hover Dictionary (`chrome.setDictionary`)

- Whenever a question stem, word problem, or reading extract contains vocabulary that could trip up a 10-year-old (e.g., *isosceles*, *consecutive*, *perpendicular*, *mean*, *quotient*, or literary/archaic words in English & VR), include concise, Year-6 friendly definitions (plus simple pronunciation hints where helpful) in `question_json.dictionary`.

---

## 5. Replica Generation Rules (`-R01`, `-R02`, Paper Replicas)

- **Same Skeleton, Fresh Skin:** Keep the exact underlying mathematical/logical archetype, mark weight, and teaching structure while changing surface numbers, names, contexts, and figures.
- **Difficulty Control:** `"same"` difficulty matches the parent's exact cognitive load (never accidentally easier or messy with ugly un-cancellable decimals on a non-calculator paper!); `"higher"` adds one authentic 11+ twist (e.g., unit conversion or an extra ratio stage).
- **Zero Visible Derivation:** Never mention *"similar to Question 14"* or any source paper inside the replica's text; link strictly via `parent_question_id` / `parent_paper_id` in SQLite `L7`.
