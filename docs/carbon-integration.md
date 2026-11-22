# Carbon Integration & Solution-Authoring Specification (`carbon-integration.md`)

> **Audience:** External applications, AI orchestrators (`Visage`), curriculum & vault services (`tutor.raphs`, `Gyara`, `Theta`), calculator/CAS engines, classroom hubs, and lesson compilers integrating with **Carbon**.
>
> **Protocol Version:** `0.1` (`PROTOCOL_VERSION = "0.1"`)

---

## 1. Overview & Capabilities

**Carbon** is a standalone, curriculum-agnostic, 120Hz GPU interactive whiteboard engine. External services never render pixels or manage layout math directly—instead, they connect to Carbon over a clean bidirectional **UTF-8 JSON WebSocket** (`Carbon Protocol v0.1`), send high-level declarative commands (or atomic batches), and subscribe to sequenced real-time board & student events.

### What Carbon Renders & Executes Natively

| Domain | Built-in Capabilities |
| :--- | :--- |
| **Layouts, Surfaces & Themes** | `single` (full-canvas) and `split` (`0.22–0.55` draggable Question \| Working Whiteboard divider, collapsible left pane); `plain`, `grid`, `dotted`, `ruled`, `lined-margin`, `isometric` surfaces; `cream-daylight`, `pure-white`, `slate-focus`, `chalkboard` themes |
| **Header Chrome, Book Index & Rich Feedback** | Subject/paper badges, part tabs (`(a)(b)(c)`), marks chip, `CalculatorMode` (`allowed` / `non-calculator`), **Collapsible Slide-Over Left-Band Book Index** (`Subjects` switcher + `Papers` \| `Topics/Chapters` \| `Stats` tabs + live search/filters), **Multi-Comment Drawer with 1-Click `"Snap Board"` PNG & Image/PDF Attachments** (gated `authorMode` for authoring tools), Dual Smart Timers (`Session` + `Item`), 3-state `ProgressStatus` (`default` / `in-progress` / `completed`), `Important ★` star, spaced-repetition `ReminderInterval` (`0d..15d`), hover vocabulary dictionary, pinned reference strip, and `StepActionChip`s |
| **Dual Typography & Speech-Synced Writing** | `handwritten` (`Caveat` / `PatrickHand` + natural baseline jitter) and `typeset` (`Lato` prose + embedded `KaTeX` math); `\htmlId{tok}{...}` sub-token hit-boxes; `SpeechBeat`-synced progressive stroke reveal; callout cards (`solid`, `dashed`, `double-underline`, collapsible `auto-margin` cards); seamless partial `edit.rub` with inline replacement |
| **2D Geometry, Calculus, 22 Macros & Physics** | Composable primitives (`point`, `segment`, `ray`, `arrow`/`vector`, `rect`, `polygon`, `circle`, `arc`, `angle`, `table`, `svgPath`, `label`), `coordinatePlane` calculus (`plots` with auto roots/intersections/stationary points, `tangentAt`, `riemannSum`, `signedIntegral`), 22 Infant-to-GCSE macros (`tenFrame`, `barModel`, `fractionBar`, `columnCalc`, `clock`, `venn`, `pieChart`, `smartTable`, …), and 9 `PhysicsRig` components (`spring`, `pulley`, `inclinedPlane`, `pendulum`, `opticsLensRay`, `forceBody`, …) |
| **Inline 3D Mathematical Stage** | Right-handed `(x, y, z)` (`+z` up) orbit/zoom/turntable stage: `cuboid`, `cylinder`, `cone`, `sphere`, `pyramid`, `triangular-prism`, `trapezoidal-prism`, regular/custom polygonal prisms, `slicePolygons`, `internalPaths`, 3D `rightAngles` & `angleArcs`, multivariate `surface3D` $z = f(x, y)$, `solidOfRevolution` + Riemann discs, `planeSlice3D` conic sections (circle/ellipse/parabola/hyperbola), voxel `cubeAssembly`, all 11 cube `netFold`s + pyramid/prism/cylinder nets with NVR `faceMarkings`, simultaneous `planes` intersections, and 3D `molecule`s |
| **Reactive `@param` & 120Hz CAS Sync** | Live sliders (`bottom-bar`, `top-bar`, `floating`, `hidden`), 2D/3D constrained drag handles, inline editable tokens & table cells, `@param` / `@eval(...)` / `@{...}` / `repeat` bindings, and zero-lag two-way `param.changed` $\leftrightarrow$ `reactive.setParams` calculator loop |
| **Stylus Ink, Rubber, Loupe & Socratic Flow** | 240Hz pressure/velocity/tilt Apple Pencil ink, 450ms palm rejection, two-mode rubber (`student-only` vs `include-teacher`; `pixel` partial split vs `object`), ruler/protractor/compass tools, `optics.loupe` (`1.5×–6.0×` crisp vector re-render), Socratic `interact.ask` prompts, `flow.detourPush`/`Pop` side-boards, and reversible `flow.timeline` (`restore` or `dim-future`) |
| **Native Zero-Copy Media & PNG Capture** | Inline, floating, or `docked-pip` video & audio player (`media.open`/`control`/`close`) streamed offline via `HTTP 206` byte-ranges, procedural DSP micro-audio cues, and `captureBoard` base64 PNG snapshots (full board, single pane, or auto-cropped to student ink for Vision/VLM grading) |

---

## 2. Transport, Endpoints & Startup Flags

### 2.1 Network Endpoints (Default Port `7420`)

| Endpoint | Protocol | Purpose |
| :--- | :--- | :--- |
| `ws://<host>:7420/carbon` | WebSocket (UTF-8 JSON) | **Primary Driver & Integration Channel.** Send `ClientMessage` (`command`, `batch`, `getState`, `captureBoard`, `interact`, `ping`); receive `ServerMessage` (`ack`, `state`, `capture`, `event`, `pong`, `error`). Upon connection, Carbon immediately emits `board.ready`. |
| `ws://<host>:7420/carbon/lan-sync` | WebSocket (UTF-8 JSON) | **Classroom Peer Mirror Channel.** Replays the current board's `lan_journal` on late-join and mirrors mutating messages across connected classroom iPads without duplicating `ServerMessage` replies on `/carbon`. |
| `http://<host>:7420/?studentId=<id>` | HTTP `GET` | Zero-install classroom iPad stage (serves embedded HTML + WebGL2 host; tags interactions with `studentId`). |
| `http://<host>:7420/carbon/engine.wasm` | HTTP `GET` | Embedded `wasm32-unknown-unknown` `StageRuntime` binary (`application/wasm`). |
| `http://<host>:7420/carbon/media/<assetId>` | HTTP `GET` (`206`) | Offline local media byte-range stream (`Accept-Ranges: bytes`, `HTTP 206 Partial Content`) automatically registered when `media.open` specifies `localPath`. |
| `http://<host>:7420/health` | HTTP `GET` | Liveness probe returning `200 OK` (`{"ok":true,"version":"0.1"}`). |

### 2.2 Launching Carbon for Integration

```bash
# Local 1-on-1 desktop integration (ws://127.0.0.1:7420/carbon)
carbon-shell --split

# Multi-device classroom LAN integration (ws://<host-ip>:7420/carbon + http://<host-ip>:7420/)
carbon-shell --lan --split

# Dedicated external TV / projector fullscreen
carbon-shell --lan --fullscreen --display 1

# Automated CI / headless integration service (no OS window required)
carbon-shell --headless --port 7420
```

---

## 3. Top-Level Wire Envelopes

Every WebSocket text frame is a single UTF-8 JSON object.

### 3.1 Inbound `ClientMessage` (Service $\to$ Carbon)

Tagged by `"type"` (`camelCase`):

#### A. Single Command (`"type": "command"`)
Note that `CarbonCommand` (`"op"` + `"params"`) flattens directly onto the envelope:
```json
{
  "version": "0.1",
  "type": "command",
  "id": "req-001",
  "actor": { "studentId": null, "deviceId": "visage-conductor", "role": "driver" },
  "timestampMs": 1732180000120,
  "op": "config.setLayout",
  "params": {
    "mode": "split",
    "splitRatio": 0.34
  }
}
```

#### B. Atomic Frame Batch (`"type": "batch"`) — *Recommended for Multi-Command Steps*
Executes all commands in order on the **exact same render frame** so layout, surfaces, Chrome, and diagrams appear with zero intermediate flicker:
```json
{
  "version": "0.1",
  "type": "batch",
  "id": "batch-setup-q1",
  "commands": [
    { "op": "config.setLayout", "params": { "mode": "split", "splitRatio": 0.34 } },
    { "op": "config.setTheme", "params": { "theme": "cream-daylight" } },
    { "op": "config.setSurface", "params": { "pane": "left", "surface": { "kind": "plain" } } },
    { "op": "config.setSurface", "params": { "pane": "right", "surface": { "kind": "grid", "pitch": "medium" } } }
  ]
}
```

#### C. Query Board Snapshot (`"type": "getState"`)
```json
{
  "version": "0.1",
  "type": "getState",
  "id": "state-001",
  "includeTokens": true,
  "includeAttachmentData": false
}
```
*(By default `includeAttachmentData` is `false`, stripping large base64 attachment bytes from `snapshot.chrome.feedback` while keeping attachment metadata fast and compact; pass `true` only when full base64 blobs are needed in `getState`.)*

#### D. Capture PNG Board / Student Ink (`"type": "captureBoard"`)
Used by vision/VLM grading services (`Visage`) or archival exporters:
```json
{
  "version": "0.1",
  "type": "captureBoard",
  "id": "cap-001",
  "pane": "right",
  "cropToUserInk": true,
  "includeChrome": false
}
```

#### E. Simulated or Bridged Student Interaction (`"type": "interact"`)
```json
{
  "version": "0.1",
  "type": "interact",
  "id": "int-001",
  "actor": { "studentId": "student-maya", "deviceId": "ipad-04", "role": "student" },
  "interaction": {
    "kind": "changeParam",
    "pane": "right",
    "paramId": "r",
    "value": 3.5,
    "phase": "commit"
  }
}
```

#### F. Liveness Probe (`"type": "ping"`)
```json
{ "type": "ping", "id": "ping-1", "timestampMs": 1732180005000 }
```

---

### 3.2 Outbound `ServerMessage` (Carbon $\to$ Service)

Tagged by `"type"` (`camelCase`):

1. **`"ack"`** — Returned immediately when a `command`, `batch`, or `interact` is accepted or rejected:
   ```json
   {
     "type": "ack",
     "requestId": "batch-setup-q1",
     "ok": true,
     "actionIds": ["act-1", "act-2", "act-3", "act-4"]
   }
   ```
   *(Note: Animated commands such as `scene.write` with `speed: "natural"` acknowledge immediately in `"ack"` with an `actionId`, and later emit `"event": "action.completed"` when the stroke animation finishes!)*

2. **`"event"`** — Asynchronous live event envelope (`seq` is strictly monotonic):
   ```json
   {
     "type": "event",
     "seq": 14,
     "timestampMs": 1732180002450,
     "actor": { "studentId": "student-maya", "deviceId": "ipad-04", "role": "student" },
     "event": "param.changed",
     "payload": {
       "pane": "right",
       "paramId": "r",
       "value": 3.5,
       "phase": "commit",
       "source": "slider",
       "params": { "r": 3.5, "h": 5.0 }
     }
   }
   ```

3. **`"state"`** — Response to `getState`:
   ```json
   { "type": "state", "requestId": "state-001", "snapshot": { "...": "see §6.4" } }
   ```

4. **`"capture"`** — Response to `captureBoard`:
   ```json
   {
     "type": "capture",
     "requestId": "cap-001",
     "mimeType": "image/png",
     "widthPx": 920,
     "heightPx": 480,
     "bounds": { "x": 0.08, "y": 0.12, "w": 0.64, "h": 0.42 },
     "dataBase64": "iVBORw0KGgoAAAANSUhEUgAA..."
   }
   ```

5. **`"error"`** — Top-level decode/version failure (`code` is one of `"invalid_json"`, `"unsupported_version"`, `"invalid_parameter"`, `"target_not_found"`, `"capability_not_ready"`, `"internal_error"`):
   ```json
   {
     "type": "error",
     "requestId": "req-bad",
     "error": { "code": "invalid_parameter", "message": "splitRatio must be within 0.22..=0.55", "op": "config.setLayout" }
   }
   ```

---

## 4. Spatial Coordinates, Panes, Anchors & Pacing

### 4.1 Panes (`PaneId`) & Spatial Zones (`SpatialZoneId`)

- **`pane`:** `"single"` (when `mode == "single"`), `"left"` (Question / Stimulus Pane in `split` mode), or `"right"` (Working Solution Whiteboard in `split` mode).
- **`zone`** (optional semantic region inside a pane):
  - `"question-stem"` — Top-to-bottom flow inside the Left Question Pane
  - `"given-strip"` — Top summary strip on the Working Board (`right` or `single`)
  - `"main-work"` *(default)* — Primary step-by-step vertical derivation column
  - `"rough-margin"` — Right-hand side column for arithmetic, factor trees, or side notes
  - `"summary-box"` — Bottom conclusion / final answer zone

### 4.2 Collision-Free Relative Anchoring (`AnchorSpec`)

Avoid hardcoding pixel coordinates when authoring multi-step solutions! Carbon's `carbon-scene` layout solver automatically places items relative to existing object or sub-token IDs with zero overlap:

| `anchor.kind` | JSON Example | Behavior |
| :--- | :--- | :--- |
| *(omitted)* | *(omit `anchor`)* | Appends cleanly below the previous item in the target `pane` + `zone` |
| `"below"` | `{"kind": "below", "targetId": "eq-1", "align": "equals", "gapPt": 14}` | Stacks directly below `eq-1` (optional `"align": "equals"` aligns `=` signs vertically!) |
| `"right-of"` | `{"kind": "right-of", "targetId": "eq-2", "offsetPt": [18, 0]}` | Places inline to the right of `eq-2` (ideal for `(\div 2)` step justifications) |
| `"left-of"` | `{"kind": "left-of", "targetId": "diag-1", "offsetPt": [-16, 0]}` | Places to the left of `diag-1` |
| `"above"` | `{"kind": "above", "targetId": "eq-1"}` | Places above `eq-1` |
| `"above-right"` | `{"kind": "above-right", "targetId": "tok-num"}` | Superscript/carry/cancel placement above-right of a sub-token |
| `"auto-margin"` | `{"kind": "auto-margin", "targetId": "eq-2:rhs", "prefer": "e", "autoConnector": true, "connectorStyle": "elbow-arrow"}` | Automatically finds open margin space (`prefer`: `"n"`, `"ne"`, `"e"`, `"se"`, `"s"`, `"sw"`, `"w"`, `"nw"`) and draws an obstacle-routed leader arrow to `targetId` |
| `"normalized"` | `{"kind": "normalized", "x": 0.12, "y": 0.18}` | Explicit `[0.0..1.0]` pane-normalized coordinates |

### 4.3 Pacing (`SpeedSpec`) & Voice Synchronization (`SpeechBeat`)

- **`speed`:** `"instant"` (`0ms`), `"fast"`, `"natural"` (default human board-writing pace), `"slow"`, `"dramatic"`, or `{"durationMs": 850}`.
- **`speechBeats`** (on `scene.write`): Synchronizes progressive stroke writing with TTS audio timestamps:
  ```json
  "speechBeats": [
    { "tokenId": "lhs", "startMs": 0, "endMs": 420 },
    { "tokenId": "rhs", "startMs": 450, "endMs": 1100 }
  ]
  ```

---

## 5. Complete Reference of All `CarbonCommand` Operations

### 5.1 Board Configuration (`config.*`)

| `op` | `params` Schema & Options |
| :--- | :--- |
| **`config.setLayout`** | `{"mode": "single" \| "split", "splitRatio": 0.34, "leftCollapsed": false, "animate": true}` (`splitRatio` clamped to `0.20..=0.60`). |
| **`config.setSurface`** | `{"pane": "right", "surface": {"kind": "plain" \| "squared" \| "dotted" \| "ruled" \| "isometric", "pitch": "x-large" \| "large" \| "medium" \| "fine" \| "x-fine"}}` |
| **`config.setTheme`** | `{"theme": "warm-paper" \| "classroom-white" \| "dark-glass"}` |

---

### 5.2 Header Chrome, Timers, Rich Comments & Full-Page Course Book Index (`chrome.*`)

1. **`chrome.update`** *(also accepts alias **`"chrome.setHeader"`**)* — Partial patch (`null`/omitted fields stay unchanged):
   ```json
   {
     "op": "chrome.update",
     "params": {
       "mode": "full",
       "title": "11+ Maths · Practice Paper 08-ai",
       "activeItemId": "GYR-MTH-P008AI-Q04",
       "paperBadge": "PP 08-ai",
       "itemLabel": "Q4",
       "partTabs": [
         { "id": "a", "label": "(a)" },
         { "id": "b", "label": "(b)" }
       ],
       "activePartId": "a",
       "marksChip": "3 marks",
       "calculatorMode": "non-calculator",
       "status": "in-progress",
       "important": true,
       "importantIcon": "triangle",
       "reminder": "15d",
       "dictionaryEnabled": true,
       "catalogButtonVisible": true,
       "authorMode": false,
       "activeStepId": "step-2",
       "authorMeta": null,
       "stepActions": [
         { "id": "why-divide-3", "label": "Why divide by 3?", "active": false }
       ],
       "pinnedReferences": [
         { "id": "ref-cyl", "label": "Part (a)", "content": "$V = \\pi r^2 h$" }
       ]
     }
   }
   ```
   - **`status` (`ProgressStatus` — strictly 3 user states):**
     - `"default"` *(Not Attempted — also accepts alias `"not-attempted"` / `"not-started"`)*
     - `"in-progress"` *(In Progress)*
     - `"completed"` *(Completed — also accepts alias `"solved"`)*
   - **`importantIcon` (`ImportantIconStyle`, default `"triangle"`):**
     - `"triangle"` *(default — crisp coral-red `△` marker on both Carbon's Header bar and all Index cards)*
     - `"star"` | `"flag"` *(optional `SIGMA` override)*
   - **`reminder` (`ReminderInterval`):** `"0d"` *(Off — also accepts alias `"none"`)* | `"1d"` | `"3d"` | `"5d"` | `"7d"` | `"15d"`
   - **`calculatorMode`:** `"allowed"` | `"non-calculator"`
   - **`authorMode` (`bool`, default `false`):**
     - **`false` (Public / Student Mode):** End users only see a clean Feedback box (plain text + **1-Click `"Snap Board"`** + **Image/PDF upload or paste**) and standard learner `⋮` card options (`Not Attempted` / `In Progress` / `Completed`, `△ Important`, `0d..15d` Reminder). Zero internal authoring categories, open/resolved triage filters, or replica actions are visible or accepted.
     - **`true` (Authorised Owner / Admin Session):** Unlocks author comment `kind` pills (`"fix"`, `"replica"`, `"topic"`, `"note"`, `"engine"`), `All | Pending | Done` filter, Resolve/Reopen toggle, automatic `activeStepId` / `authorMeta` stamping, and card `⋮` author actions (`"create-replica"`, emitting `catalog.actionRequested`).

2. **`chrome.timer`** — Controls the **Session** or **Item** Dual Smart Timer:
   ```json
   {
     "op": "chrome.timer",
     "params": {
       "target": "item",
       "action": "configure",
       "mode": "countdown",
       "durationSec": 180,
       "autoStart": true
     }
   }
   ```
   - `target`: `"session"` | `"item"`
   - `action`: `"configure"` | `"start"` | `"pause"` | `"reset"` | `"adjust"` (`deltaSec: 60`)
   - `mode`: `"countdown"` | `"stopwatch"`

3. **`chrome.setDictionary`** — Hover/tap key-term definitions in the Question Pane:
   ```json
   {
     "op": "chrome.setDictionary",
     "params": {
       "enabled": true,
       "entries": [
         { "term": "frustum", "definition": "The portion of a cone or pyramid remaining after its top is cut off by a plane parallel to its base.", "pronunciation": "frus-tum", "example": "Volume = V_large - V_small" }
       ]
     }
   }
   ```

4. **Rich Comments + 1-Click `"Snap Board"` & Image/PDF Attachments (`chrome.setFeedback`, `chrome.upsertComment`, `chrome.deleteComment`):**
   - Clicking the Header `💬` button opens the Comment Drawer for `activeItemId` (showing an open-comment count badge).
   - Clicking **`Snap Board`** inside the drawer captures a clean PNG of the board (`right` pane, `left` pane, or `full` stage—including both scene objects and freehand stylus ink, automatically hiding the comment drawer itself during capture) and attaches it as a removable preview thumbnail. Users can also attach/paste `image/png`, `image/jpeg`, `image/svg+xml`, or `application/pdf`.
   - **`FeedbackEntry` Schema (100% backward-compatible with `v0.1`):**
     ```json
     {
       "commentId": "cmt-019968b0-4a12",
       "itemId": "GYR-MTH-P001-Q014",
       "partId": "a",
       "stepId": "step-2",
       "kind": "note",
       "status": "pending",
       "text": "Use a 4-box bar model here like my red marker sketch.",
       "attachments": [
         {
           "id": "att-01",
           "source": "board-capture",
           "pane": "right",
           "mimeType": "image/png",
           "filename": "board-snap-1.png",
           "widthPx": 1280,
           "heightPx": 800,
           "dataBase64": "iVBORw0KGgoAAAANSUhEUgAA..."
         }
       ],
       "authorMeta": { "sublevel": 5 },
       "resolutionNote": null,
       "createdAtMs": 1732180000000,
       "updatedAtMs": 1732180005000
     }
     ```
     - `text` also accepts inbound alias `"comment"`.
     - `status`: `"pending"` *(alias `"open"`)* | `"done"` *(alias `"resolved"`)*.
     - `kind`: `"note"` *(default; forced in public `authorMode: false`)* | `"fix"` | `"replica"` | `"topic"` | `"engine"` *(available in `authorMode: true`)*.
     - `attachments[].id` also accepts alias `"attachmentId"`; `attachments[].source`: `"board-capture"` | `"upload"` | `"clipboard"`.
   - **Commands:**
     - **`chrome.setFeedback`**: Accepts `{ "entries": [FeedbackEntry, ...] }` (or a single `v0.1` `FeedbackEntry`).
     - **`chrome.upsertComment`**: `{ "entry": FeedbackEntry }` (inserts or updates a single comment in `O(1)`).
     - **`chrome.deleteComment`**: `{ "commentId": "cmt-019968b0-4a12", "itemId": "GYR-MTH-P001-Q014", "partId": "a" }`.
   - **Emitted Events:** `"comment.saved"` (`{ "entry": FeedbackEntry }` including full `dataBase64` / `dataRef` attachments so `Hydrogen` / `SIGMA` can persist blobs immediately) and `"comment.deleted"` (`{ "commentId", "itemId", "partId" }`).

5. **Full-Page Course & Book Index Gallery (`chrome.setCatalog`, `chrome.patchCatalogItem`, `chrome.setCatalogOpen`):**
   - **100% Full-Stage Landing & Navigation Overlay (`displayMode: "full-stage"` default):**
     - Covers **100% of the Carbon window** across both panes on a crisp dark-slate gallery backdrop (`#151A24`), giving generous multi-column breathing room for **Hero Subject Cards**, a **3–4 column Practice Papers / Chapters & Topics Grid**, and a **4–6 column Question & Concept Cards Grid**.
     - **Direct Blank / Scratchpad Board Access:** If `SIGMA` boots Carbon without calling `chrome.setCatalog` (or passes `"open": false`), Carbon starts directly on the clean whiteboard. Whenever the Full-Page Index *is* open, a persistent top-right **`Open Whiteboard` / `Return to Board ✕`** button lets the user dismiss the Index onto an empty or active whiteboard without selecting a subject or question.
   - **3-Level Breadcrumb Flow + `SIGMA` Logos (`brandLogo` & per-Subject `logo`):**
     - **Top Bar:** `[🏠]` `>` `[Subject Pill]` `>` `[Practice Papers | Chapters & Topics | Stats]` `>` `[Active Paper / Chapter]` on the left, **`SIGMA` `brandLogo`** (inline `<svg>`, `data:image/...`, or `local://` PNG rasterized offline onto GPU textures) centered, and **`[Open Whiteboard ✕]`** on the right.
     - **Level 1 (`"subjects"`):** Renders `1..N` dynamic hero cards from `SIGMA`'s `availableSubjects` (never a fixed count—auto-centers `1–4` cards or wraps into a scrollable grid for `5+` subjects), each showing its custom jewel-toned fill, `logo` (or `brandLogo`), title, and optional paper/due summary. Clicking a Subject emits `catalog.itemSelected` (`kind: "subject"`).
     - **Level 2 (`"subject-root"`):** Full-width multi-column grid of `SIGMA`'s `papers` (`Practice Paper 01`..`N`), `topics` (`Chapters → Topics`), or `stats`, with dynamic `SIGMA` `filterTags` (e.g. `All`, `Real papers`, `AI practice`) alongside `Important △`, `Reminders`, `In Progress`, `Completed`, and instant search.
     - **Level 3 (`"paper-detail"` / `"topic-detail"`):** Multi-column Question & Concept Card grid (`Q1`..`QN`) + clickable bottom **`📊 Progress`** summary bar. Clicking a Question card emits `catalog.itemSelected` (`kind: "question"`) and lands cleanly on the working whiteboard.
   - **4-State Visual Card Colour Semantics & `⋮` Three-Dot Card Menu:**
     - **Not Attempted (`"default"`):** Crisp White card (`#FFFFFF`) with dark ink.
     - **In Progress (`"in-progress"`):** Soft Slate-Blue card (`#CBD5E1` / `#DBEAFE`).
     - **Completed (`"completed"`):** Clean Sage-Emerald card (`#DCFCE7` + `✓`).
     - **Repeat Due Today / Overdue (`dueReminder: true` on question, or `dueCount > 0` on paper):** **Deep Crimson (`#6B1414`)** with crisp white text (`repeat due (15d)` / `2 due`), while future scheduled reminders (`reminder: "15d"`, `dueReminder: false`) keep their normal card colour with a subtle pink `15d` chip.
     - **`⋮` Three-Dot Menu on Cards:** Clicking **`⋮`** opens an anchored menu directly on the card for `Not Attempted / In Progress / Completed` (`status.changed`), `△ Important` (`important.toggled`), `0d..15d` Reminder (`reminder.changed`), plus **`+ Create Replica`** (`catalog.actionRequested`) strictly when `authorMode == true`.

   - **`chrome.setCatalog` — Load Dynamic Subjects, Papers, Chapters/Topics, Logos & Stats:**
     ```json
     {
       "op": "chrome.setCatalog",
       "params": {
         "open": true,
         "displayMode": "full-stage",
         "viewLevel": "paper-detail",
         "activeTab": "papers",
         "activeSubjectId": "maths",
         "activePaperId": "GYR-MTH-P008AI",
         "activeTopicId": null,
         "activeItemId": "GYR-MTH-P008AI-Q04",
         "autoCollapseOnSelect": true,
         "importantIcon": "triangle",
         "brandLogo": "<svg viewBox='0 0 180 48' xmlns='http://www.w3.org/2000/svg'>...</svg>",
         "vaultTitle": "11+ Maths",
         "vaultSubtitle": "Practice Papers, Chapters & Topics",
         "filterTags": [
           { "id": "real", "label": "Real papers" },
           { "id": "ai", "label": "AI practice" }
         ],
         "availableSubjects": [
           { "id": "maths", "title": "Maths", "badge": "11+", "color": "#2563EB", "logo": null, "progressText": "23 Papers", "dueCount": 9 },
           { "id": "english", "title": "English", "badge": "11+", "color": "#0D9488", "progressText": "18 Papers", "dueCount": 2 },
           { "id": "vr", "title": "Verbal Reasoning", "badge": "11+", "color": "#7C3AED", "progressText": "16 Papers", "dueCount": 0 },
           { "id": "nvr", "title": "Non-Verbal Reasoning", "badge": "11+", "color": "#DB2777", "progressText": "16 Papers", "dueCount": 1 }
         ],
         "papers": [
           {
             "id": "GYR-MTH-P008AI",
             "title": "Practice Paper 08-ai",
             "badge": "6 questions · 30 min · 1 repeat due",
             "tagBadges": [{ "label": "AI", "color": "#8B5CF6" }],
             "tags": ["ai"],
             "color": "#2563EB",
             "durationSec": 1800,
             "totalMarks": 18,
             "calculatorMode": "non-calculator",
             "questionCount": 6,
             "attemptedCount": 0,
             "completedCount": 0,
             "dueCount": 1,
             "hasImportant": true,
             "progressLabel": "0/6 attempted · 0 completed",
             "openCommentCount": 0,
             "items": [
               {
                 "id": "GYR-MTH-P008AI-Q01",
                 "label": "Q1",
                 "sublabel": "p1",
                 "title": "Ratio Share & Difference",
                 "badge": "3 marks",
                 "marks": 3,
                 "difficulty": 2,
                 "color": "#2563EB",
                 "tags": ["ai"],
                 "topicIds": ["GYR-MTH-T_RAT01"],
                 "topicLabels": ["Ratio"],
                 "status": "default",
                 "important": true,
                 "reminder": "15d",
                 "dueReminder": false,
                 "openCommentCount": 0
               },
               {
                 "id": "GYR-MTH-P008AI-Q04",
                 "label": "Q4",
                 "sublabel": "p4",
                 "title": "Unitary Bar Model Challenge",
                 "badge": "3 marks",
                 "marks": 3,
                 "difficulty": 3,
                 "color": "#2563EB",
                 "tags": ["ai"],
                 "topicIds": ["GYR-MTH-T_RAT01"],
                 "topicLabels": ["Ratio"],
                 "status": "default",
                 "important": true,
                 "reminder": "15d",
                 "dueReminder": true,
                 "openCommentCount": 0
               }
             ]
           }
         ],
         "topics": [
           {
             "id": "GYR-MTH-CH02",
             "parentId": null,
             "title": "Chapter 2: Ratio & Proportion",
             "badge": "Chapter",
             "color": "#0D9488",
             "questionCount": 28,
             "completedCount": 19,
             "dueCount": 1,
             "masteryPct": 82.0,
             "cards": [],
             "itemIds": []
           },
           {
             "id": "GYR-MTH-T_RAT01",
             "parentId": "GYR-MTH-CH02",
             "title": "Sharing in a Given Ratio",
             "badge": "Topic",
             "color": "#0D9488",
             "questionCount": 14,
             "completedCount": 11,
             "dueCount": 1,
             "masteryPct": 85.0,
             "cards": [
               {
                 "id": "GYR-MTH-T_RAT01-C01",
                 "kind": "concept",
                 "title": "Bar Model Strategy for Ratio Problems",
                 "badge": "Concept",
                 "color": "#0D9488",
                 "completed": true,
                 "openCommentCount": 0
               }
             ],
             "itemIds": ["GYR-MTH-P008AI-Q01", "GYR-MTH-P008AI-Q04"],
             "items": []
           }
         ],
         "stats": {
           "headline": "84 of 250 Questions Completed",
           "metrics": [
             { "id": "completed", "label": "Completed", "value": "84 / 250", "subtext": "33.6% of Maths", "color": "#16A34A" },
             { "id": "important", "label": "Important △", "value": "5", "subtext": "Flagged by learner", "color": "#EF4444" },
             { "id": "due", "label": "Repeat Due", "value": "6", "subtext": "Due for review today", "color": "#DC2626" }
           ],
           "breakdowns": [
             { "id": "ratio", "label": "Ratio & Proportion", "valueText": "82%", "ratio": 0.82, "color": "#0D9488" },
             { "id": "fractions", "label": "Fractions & Decimals", "valueText": "76%", "ratio": 0.76, "color": "#2563EB" },
             { "id": "algebra", "label": "Algebra", "valueText": "58%", "ratio": 0.58, "color": "#D97706" }
           ]
         },
         "authorActions": ["create-replica"]
       }
     }
     ```
   - **`chrome.patchCatalogItem` — Fast `O(1)` Incremental Update After Answering/Commenting:**
     ```json
     {
       "op": "chrome.patchCatalogItem",
       "params": {
         "activeItemId": "GYR-MTH-P008AI-Q05",
         "itemPatch": {
           "id": "GYR-MTH-P008AI-Q04",
           "status": "completed",
           "important": true,
           "reminder": "15d",
           "dueReminder": false,
           "openCommentCount": 0
         },
         "paperPatch": {
           "id": "GYR-MTH-P008AI",
           "attemptedCount": 1,
           "completedCount": 1,
           "dueCount": 0,
           "progressLabel": "1/6 attempted · 1 completed"
         },
         "topicPatch": {
           "id": "GYR-MTH-T_RAT01",
           "completedCount": 12,
           "dueCount": 0,
           "masteryPct": 88.0
         },
         "stats": { "...": "optional updated CatalogStatsView" }
       }
     }
     ```
   - **`chrome.setCatalogOpen` — Open / Close / Jump Level or Tab:**
     ```json
     {
       "op": "chrome.setCatalogOpen",
       "params": {
         "open": true,
         "displayMode": "full-stage",
         "viewLevel": "subjects",
         "activeTab": "papers"
       }
     }
     ```

---

### 5.3 Scene Writing & Rich Cards (`scene.write`)

Renders KaTeX equations (`$...$` / `\(...\)`), mixed prose + math, warning/rule cards, inline editable fill-in-the-blank tokens, and addressable `\htmlId{token}{...}` sub-expressions:

```json
{
  "op": "scene.write",
  "params": {
    "id": "eq-volume",
    "pane": "right",
    "zone": "main-work",
    "anchor": { "kind": "below", "targetId": "cyl-3d", "gapPt": 16 },
    "text": "Volume $V = \\pi \\htmlId{r_tok}{r^2} \\htmlId{h_tok}{h} = \\pi (@r)^2 (@h) = @eval(pi * r^2 * h)\\text{ cm}^3$",
    "typography": "typeset",
    "mathStyle": "display",
    "color": "#0F172A",
    "sizePt": 18.5,
    "speed": "natural",
    "spanStyles": {
      "r_tok": { "color": "#2563EB", "bold": true },
      "h_tok": { "color": "#0D9488", "bold": true }
    },
    "card": {
      "title": "Live Formula",
      "fillColor": "#F8FAFC",
      "borderColor": "#CBD5E1",
      "borderStyle": "solid",
      "paddingPt": 10.0,
      "collapsible": false
    }
  }
}
```

- **`typography`:** `"typeset"` (`Lato` prose + `KaTeX` math) or `"handwritten"` (`Caveat` marker prose + handwritten-styled math).
- **`mathStyle`:** `"inline"` or `"display"` (tall $\sum, \int, \frac{}{}$ limits).
- **`editableTokens`:** Make any `\htmlId{ans}{?}` token interactive on the board:
  ```json
  "editableTokens": [
    { "tokenId": "ans", "paramId": "student_k", "initialValue": "", "placeholder": "?", "widthPt": 48.0, "numericOnly": true }
  ]
  ```
- **Addressing Sub-Tokens Later:** Any `\htmlId{r_tok}{...}` inside `id: "eq-volume"` is addressable in `focus.annotate`, `edit.rub`, `scene.write` anchors, and `token.tapped` events as `"eq-volume:r_tok"` (or `"r_tok"`).

---

### 5.4 2D Diagrams, Calculus Plots, Educational Macros & Physics (`scene.place2D`)

```json
{
  "op": "scene.place2D",
  "params": {
    "id": "graph-quad",
    "pane": "right",
    "zone": "main-work",
    "spec": { "...": "see 5.4.1 - 5.4.4" },
    "speed": "natural"
  }
}
```

#### Top-Level `scene.place2D` `spec` Options
- `lockViewScale`: `true` *(mandatory whenever `@param` or drag handles resize geometry so shapes visibly grow/shrink against fixed axes!)*
- `ghostParams`: `{"r": 2.0, "h": 4.0}` *(renders a faint dashed baseline silhouette of the initial geometry behind live changes)*
- `displayWidth`, `displayHeight`: Explicit logical size in points (e.g. `360`, `260`)
- `labelStyle`: `"typeset"` *(default)* | `"handwritten"`
- `interactive`: `true` | `false`

#### 5.4.1 Composable 2D Geometry (`"elements": [...]`)
Supported element `"kind"` values:
- `"point"`: `{"kind": "point", "id": "A", "at": [0, 0], "label": "$A$", "draggable": true, "bindParamX": "ax", "bindParamY": "ay"}`
- `"segment"` / `"line"`: `{"kind": "segment", "from": [0, 120], "to": [160, 120], "tickMarks": 2, "parallelMarks": 1, "label": "$8\\text{ cm}$", "alignToSegment": true, "dash": "solid" | "dashed" | "dotted"}`
- `"ray"`: `{"kind": "ray", "origin": [0, 0], "angleDeg": 35, "length": 120}`
- `"arrow"` / `"vector"`: `{"kind": "vector", "from": [0, 0], "to": ["@eval(vx*20)", "@eval(-vy*20)"], "bothEnds": false, "draggableTip": true, "label": "$\\vec{v}$"}`
- `"rect"` / `"box"`: `{"kind": "rect", "x": 0, "y": 0, "width": 120, "height": 70, "radius": 6, "fill": "#2563EB", "fillOpacity": 0.14, "label": "Area"}`
- `"polygon"`: `{"kind": "polygon", "points": [[0, 120], [160, 120], [0, 0]], "vertexLabels": ["$B$", "$C$", "$A$"], "fill": "#2563EB", "fillOpacity": 0.14}`
- `"circle"`: `{"kind": "circle", "center": [100, 100], "radius": "@eval(r * 18)", "fill": "#0D9488", "fillOpacity": 0.12}`
- `"arc"`: `{"kind": "arc", "center": [100, 100], "radius": 60, "startDeg": 0, "endDeg": 120, "sector": true}`
- `"angle"`: `{"kind": "angle", "vertex": [0, 120], "ray1": [160, 120], "ray2": [0, 0], "rightAngle": true}` *(or `"label": "$\\theta$"` for curved angle arc)*
- `"table"`: `{"kind": "table", "id": "tbl", "headers": ["$x$", "$y$"], "rows": [["-1", "0"], ["0", "-3"], ["2", "?"]], "editableCells": [{"row": 2, "col": 1}], "cellStyles": {"r0c1": {"fill": "#DCFCE7"}}}` *(cells addressable as `"tbl:r0c1"`)*
- `"svgPath"`: `{"kind": "svgPath", "d": "M 0 0 C 40 -50, 80 -50, 120 0 Z", "fill": "#F59E0B", "fillOpacity": 0.2}`
- `"label"`: `{"kind": "label", "at": [80, -14], "text": "$y = f(x)$", "fontSize": 13.5}`

#### 5.4.2 Calculus & Coordinate Planes (`"kind": "coordinatePlane"`)
```json
{
  "lockViewScale": true,
  "displayWidth": 420,
  "displayHeight": 300,
  "elements": [
    {
      "kind": "coordinatePlane",
      "id": "calc-plane",
      "xRange": [-4, 4],
      "yRange": [-3, 6],
      "xLabel": "$x$",
      "yLabel": "$y$",
      "grid": true,
      "plots": [
        {
          "id": "f",
          "expr": "a * x^2 - 2",
          "color": "#2563EB",
          "label": "$y = ax^2 - 2$",
          "markRoots": true,
          "markYIntercept": true,
          "markStationary": true,
          "tangentAt": 1.5
        },
        {
          "id": "g",
          "expr": "x + 1",
          "color": "#0D9488",
          "dash": "dashed",
          "intersectionsWith": "f"
        }
      ],
      "signedIntegral": {
        "expr": "a * x^2 - 2",
        "range": [-1.5, 2.0],
        "positiveColor": "#16A34A",
        "negativeColor": "#E11D48"
      },
      "handles": [
        { "id": "pt-p", "x": "@px", "expr": "a * x^2 - 2", "bindParamX": "px", "label": "$P$" }
      ]
    }
  ]
}
```
Also supports parametric curves (`"xExpr": "cos(t)", "yExpr": "sin(2*t)", "tRange": [0, 6.283]`), piecewise domains (`"domain": [0, 4]`), inequality half-planes (`"shade": "above" | "below"`), and `"riemannSum": {"expr": "...", "range": [0, 3], "n": 6, "method": "left" | "right" | "midpoint" | "trapezoid"}`.

#### 5.4.3 Curriculum-Spanning Educational 2D Macros (`"macro": "<name>"`)
Pass `"macro"` at the root of `spec` (you can also overlay extra composable `"elements": [...]` on top of any macro!):

| Stage | Macro Name | Key JSON Parameters |
| :--- | :--- | :--- |
| **Infant / KS1** | `"tenFrame"` | `{"macro": "tenFrame", "frames": 2, "filled": 13, "altFilled": 3, "color": "#2563EB", "altColor": "#0D9488"}` |
| **Infant / KS1** | `"counters"` | `{"macro": "counters", "groups": [{"count": 4, "label": "4", "color": "#2563EB"}, {"count": 3, "label": "3", "color": "#F59E0B"}], "crossedOut": 1}` |
| **Primary / 11+** | `"numberLine"` | `{"macro": "numberLine", "min": 0, "max": 20, "step": 2, "markers": [{"at": 6, "label": "Start"}], "jumps": [{"from": 6, "to": 14, "label": "$+8$"}]}` |
| **Primary / 11+** | `"fractionBar"` | `{"macro": "fractionBar", "bars": [{"parts": 4, "shaded": 3, "label": "$\\frac{3}{4}$"}, {"parts": 8, "shaded": 6, "label": "$\\frac{6}{8}$"}]}` |
| **Primary / 11+** | `"barModel"` | `{"macro": "barModel", "totalLabel": "$72$", "rows": [[{"value": 3, "label": "Ali"}, {"value": 5, "label": "Ben"}], [{"value": 8, "label": "Total = 8 parts"}]]}` |
| **Primary / GCSE** | `"columnCalc"` | `{"macro": "columnCalc", "op": "+" \| "-" \| "×" \| "÷", "top": "482", "bottom": "159", "result": "641", "carries": {"1": "1"}}` *(use `"op": "÷"` for bus-stop short/long division)* |
| **Primary / GCSE** | `"fracCancel"` | `{"macro": "fracCancel", "num": [{"text": "12", "crossed": true, "superscript": "3"}], "den": [{"text": "16", "crossed": true, "subscript": "4"}]}` |
| **Primary / 11+** | `"functionMachine"` | `{"macro": "functionMachine", "input": "$x$", "steps": ["$\\times 3$", "$- 5$"], "output": "$22$"}` |
| **Primary / 11+** | `"clock"` / `"mirrorClock"` | `{"macro": "clock", "hour": 3, "minute": 45, "roman": false, "showMinuteTicks": true}` (`"mirrorClock"` renders horizontal mirror-reflection 11+ puzzles) |
| **Primary / GCSE** | `"dial"` / `"jug"` / `"balanceScale"` | `{"macro": "jug", "capacity": 1000, "level": 650, "unit": "ml", "majorStep": 200}`; `{"macro": "balanceScale", "left": ["$2x$", "$3$"], "right": ["$11$"], "tiltDeg": 0}` |
| **Statistics** | `"pieChart"` / `"barChart"` / `"lineChart"` / `"dataStrip"` | `{"macro": "pieChart", "slices": [{"label": "Walk", "value": 15}, {"label": "Bus", "value": 9}], "showAngles": true}` |
| **Sets & Discrete** | `"venn"` / `"network"` | `{"macro": "venn", "sets": ["$A$", "$B$"], "regions": {"A": "7", "B": "5", "AB": "4", "outside": "2"}, "universal": "$\\xi = 18$"}` |
| **2D Geometry / Nets** | `"net2D"` / `"areaGrid"` / `"smartTable"` | `{"macro": "net2D", "pattern": "t-cross", "labels": ["1", "2", "3", "4", "5", "6"]}`; `{"macro": "smartTable", "headers": [...], "rows": [...]}` |

#### 5.4.4 Mechanics, Kinematics & Optics (`PhysicsRig` Elements)
Every component below works **both** as a top-level `"macro"` (e.g. `{"macro": "inclinedPlane", ...}`) **and** inside `"elements": [...]` (or grouped inside `{"kind": "physicsRig", "showGround": true, "rigs": [...]}`):
- `"spring"`: `{"kind": "spring", "anchor": [80, 20], "orientation": "vertical" | "horizontal", "naturalLength": 90, "extension": "@dx", "massLabel": "$m$", "showRuler": true}`
- `"massBlock"` / `"cart"`: `{"kind": "cart", "x": "@x_pos", "y": 140, "width": 64, "height": 36, "label": "$2\\text{ kg}$"}`
- `"inclinedPlane"`: `{"kind": "inclinedPlane", "angleDeg": "@theta", "baseLength": 180, "blockPos": 0.55, "massLabel": "$m$", "showComponents": true, "showFriction": true}` (auto-draws $mg\sin\theta$, $mg\cos\theta$, $R$, and $F$)
- `"pulley"`: `{"kind": "pulley", "center": [140, 40], "radius": 22, "leftDrop": "@h1", "rightDrop": "@h2", "leftMass": "$m_1$", "rightMass": "$m_2$", "showTension": true}`
- `"pendulum"`: `{"kind": "pendulum", "pivot": [140, 20], "length": 120, "angleDeg": "@theta", "bobLabel": "$m$", "showEquilibrium": true}`
- `"projectileArc"`: `{"kind": "projectileArc", "speed": "@u", "launchAngleDeg": "@alpha", "g": 9.8, "showMaxHeight": true, "showRange": true}`
- `"opticsLensRay"`: `{"kind": "opticsLensRay", "lensKind": "converging" | "diverging", "focalLength": "@f", "objectDist": "@u_obj", "objectHeight": 36}` (auto-traces principal rays & real/virtual image)
- `"forceBody"` / `"freeBody"`: `{"kind": "forceBody", "label": "$P$", "forces": [{"id": "T", "angleDeg": 35, "magnitude": 60, "label": "$T$", "draggable": true}, {"id": "W", "angleDeg": -90, "magnitude": 50, "label": "$mg$"}]}`

---

### 5.5 Inline Images & SVG Diagrams (`scene.placeImage`)

```json
{
  "op": "scene.placeImage",
  "params": {
    "id": "stim-fig-1",
    "pane": "left",
    "zone": "question-stem",
    "src": "data:image/svg+xml;base64,...",
    "alt": "Circuit diagram",
    "widthPt": 280,
    "border": true,
    "caption": "Figure 1 (not to scale)",
    "draggable": true,
    "resizable": true,
    "hotspots": [
      { "id": "bulb-1", "bounds": { "x": 0.42, "y": 0.25, "w": 0.16, "h": 0.22 }, "label": "Lamp $L_1$" }
    ]
  }
}
```
Supports offline `data:image/png;base64,...`, `data:image/jpeg;base64,...`, `data:image/svg+xml;base64,...`, inline `<svg>...</svg>`, and local workspace file paths.

---

### 5.6 Inline 3D Mathematical Stage (`scene.place3D`)

Right-handed 3D world space `(x, y, z)` where **`+z` points vertically up** (rotating camera to `yawDeg: -90, pitchDeg: 89` smoothly aligns `+x` right and `+y` up like a 2D Cartesian graph!).

```json
{
  "op": "scene.place3D",
  "params": {
    "id": "cyl-3d",
    "pane": "right",
    "zone": "main-work",
    "spec": {
      "solid": "cylinder",
      "radius": "@r",
      "height": "@h",
      "color": "#2563EB",
      "opacity": 0.28,
      "lockViewScale": true,
      "ghostSolid": { "solid": "cylinder", "radius": 2.0, "height": 4.0 },
      "axes": { "xRange": [-5, 5], "yRange": [-5, 5], "zRange": [-1, 7], "tickStep": 1, "showGrid": true },
      "camera": { "yawDeg": -56, "pitchDeg": 24, "fovDeg": 28, "zoom": 1.0 },
      "handles": [
        { "id": "h-radius", "kind": "radial", "min": 0.5, "max": 4.5, "value": "@r", "bindParam": "r", "label": "$r$" },
        { "id": "h-height", "kind": "axis", "axis": "z", "origin": [0, 0, -2], "min": 1.0, "max": 6.5, "pos": [0, 0, "@eval(h * 0.5)"], "bindParam": "h", "label": "$h$" }
      ],
      "caption": "Drag $r$ or $h$, or orbit freely"
    }
  }
}
```

#### Complete Catalog of `scene.place3D` Builders & 3D Handles
1. **Parametric Solids (`"solid"` or `"solids": [...]`):**
   - `"cuboid"` / `"box"` / `"cube"` (`dimensions: [dx, dy, dz]` or `width3D`, `depth3D`, `height`, `side`)
   - `"cylinder"` (`radius`, `height`)
   - `"cone"` (`radius`, `height`)
   - `"sphere"` (`radius`)
   - `"pyramid"` / `"square-pyramid"` / `"tetrahedron"` (`base`, `height`, `sides`)
   - `"triangular-prism"` (`triangleBase`, `triangleHeight`, `length`, `rightTriangle`: `true|false`, `axis`: `"x"|"y"|"z"`)
   - `"trapezoidal-prism"` (`bottomWidth`, `topWidth`, `trapHeight`, `length`, `axis`)
   - `"pentagonal-prism"`, `"hexagonal-prism"`, `"octagonal-prism"`, or `"prism"` with arbitrary 2D `"profile": [[u0, v0], ...]`
2. **Symbolic 3D Anchor Expressions & Internal Pythagoras/Trig (`slicePolygons`, `internalPaths`, `rightAngles`, `angleArcs`, `points`, `segments`):**
   - Solids automatically register named 3D world anchors: `"origin"`, `"center"`, `"base-center"`, `"top-center"`, `"apex"`, `"rim+x"`, vertex letters `"A".."H"` (or `"v0".."v7"`), plus functional anchor expressions **`"mid(A, B)"`** and **`"lerp(v0, apex, 0.25)"`**!
   - Use them anywhere a 3D point is accepted:
     - `slicePolygons: [{"vertices": ["A", "C", "G", "E"], "fill": "#F59E0B", "opacity": 0.28}]`
     - `internalPaths: [{"points": ["A", "C", "G"], "color": "#E11D48", "dash": "dashed"}]`
     - `rightAngles: [{"at": "C", "from": "A", "to": "G"}]`
     - `angleArcs: [{"at": "A", "from": "C", "to": "G", "label": "$\\theta$"}]`
3. **Multivariate & Parametric Surfaces (`"surface3D"` or `"surfaces": [...]`):**
   ```json
   "surface3D": {
     "id": "saddle",
     "expr": "0.22 * a * (x^2 - y^2)",
     "ghostExpr": "0.22 * (x^2 - y^2)",
     "xRange": [-3, 3], "yRange": [-3, 3],
     "stepsX": 24, "stepsY": 24,
     "wireStride": 2,
     "zClip": [-4, 4],
     "contours": [-1.5, -0.5, 0.5, 1.5],
     "contourColor": "#E11D48",
     "floorContours": true
   }
   ```
   *(Also supports parametric 3D surfaces via `"xExpr"`, `"yExpr"`, `"zExpr"` over `"uRange"` and `"vRange"`!)*
4. **Volumes of Revolution — Disc & Washer Methods (`"solidOfRevolution"` or `"revolutions": [...]`):**
   ```json
   "solidOfRevolution": {
     "id": "rev-parabola",
     "expr": "0.35 * x^2 + 0.6",
     "innerExpr": "0.2 * x",
     "axis": "x",
     "domain": [0.0, 3.0],
     "sweepDeg": "@sweep",
     "sliceAt": "@x0",
     "discThickness": 0.22,
     "showDiscLabel": true,
     "color": "#0D9488"
   }
   ```
5. **Analytical Conic Sections (`"planeSlice3D"` / `"conicSlice"`):**
   ```json
   "planeSlice3D": {
     "id": "conic-demo",
     "target": "double-cone",
     "doubleCone": true,
     "coneRadius": 2.6,
     "halfHeight": 3.2,
     "tiltDeg": "@tilt",
     "planeYawDeg": 0.0,
     "zOffset": 1.35,
     "fillSection": true,
     "showConicLabel": true
   }
   ```
   Automatically classifies and labels **Circle**, **Ellipse**, **Parabola**, **Hyperbola**, or **Intersecting Generators** in exact `f64` 3D coordinates!
6. **11+ Spatial Cubes & Articulated `2D <-> 3D` Folding Nets (`"cubeAssembly"` & `"netFold"`):**
   - `"cubeAssembly"`: `{"cubes": [[0,0,0], [1,0,0], [0,1,0], [0,0,1]], "highlightElevation": "front" | "side" | "plan"}`
   - `"netFold"`: Supports `"shape": "cube"` (**all 11 valid hexomino cube nets** via `"variant": 1..=11`, plus `"invalid-row"`, `"invalid-u"`, `"invalid-t-collide"` with `"highlightCollisions": true` for spatial-reasoning misconception exercises!), `"cuboid"`, `"square-pyramid"`, `"tetrahedron"`, `"triangular-prism"`, `"octahedron"`, and curved `"cylinder"` unrolling (`"fold": "@fold"` in `0.0..=1.0`, `"showGhostFlat": true`).
   - Rigidly-rotating 3D NVR `"faceMarkings"` (object keyed by face ID/label or array): `"arrow-up"`, `"arrow-down"`, `"arrow-left"`, `"arrow-right"`, `"diagonal"`, `"diagonal-backslash"`, `"circle"`, `"dot"`, `"cross"`, `"triangle"`, `"stripe-h"`, `"stripe-v"`, `"dots-1"`..`"dots-6"`, or any text symbol.
7. **Simultaneous 3D Planes & Ball-and-Stick Molecules (`"planes"` & `"molecule"`):**
   - `"planes"`: `[{"equation": [1, 1, 1, 2], "color": "#2563EB", "label": "$\\Pi_1$", "showNormal": true}, {"normal": [1, -1, 0], "d": 0, "color": "#0D9488", "label": "$\\Pi_2$"}]` with top-level `"showIntersections": true` (exact BSP-split painter order + pairwise intersection lines & unique 3-plane intersection point).
   - `"molecule"`: Presets (`"preset": "water" | "methane" | "ammonia" | "co2" | "ethene" | "benzene"`, `"showSymbols": true`, `"showBondAngles": true`) or custom CPK `"atoms"` + single/double/triple `"bonds"`.
8. **Ray-Constrained 3D Manipulation Handles (`"handles": [...]`):**
   - `"kind": "axis"` (`axis: "x"|"y"|"z"` or `dir: [dx,dy,dz]`, `origin`, `min`, `max`, `bindParam`)
   - `"kind": "radial"` (`angleDeg`, `z`, `min`, `max`, `bindParam` — ideal for radius `r`)
   - `"kind": "arc-angle"` / `"net-fold"` / `"plane-tilt"` (`axis`, `minDeg`, `maxDeg`, `bindParam` — sweeps revolution angle, `[0..1]` net fold, or conic plane tilt)
   - `"kind": "plane-xy"` / `"free"` (`limit`, `bindX`, `bindY`, `bindZ` — drags 3D points/vector tips)

---

### 5.7 Editing, Partial Rubbing, Focus, Optics, Socratic Flow, Reactive & Media

| `op` | Purpose & Key `params` |
| :--- | :--- |
| **`scene.scribble`** | Teacher freehand gesture stroke (`id`, `pane`, `points: [{"x":..,"y":..,"p":0.7,"t":0}]`, `color`, `widthPt`, `speed`) |
| **`edit.modify`** | In-place update (`targetId`, `text`, `color`, `patch`: RFC 7396 JSON Merge Patch on 2D/3D `spec`, e.g. `{"cellStyles": {"r0c2": {"fill": "#DCFCE7"}}}`) |
| **`edit.rub`** | Authentic partial rubber! (`targetIds: ["eq-1:c_tok"]`, `region`, `replaceWith: {"id": "eq-1-sub", "latex": "5", "color": "#16A34A"}`, `speed`) |
| **`edit.erase`** | Cleanly remove objects by `targetIds: [...]` |
| **`edit.clear`** | Clear `pane` (`"right"` / `"left"` / omit for all), optional `zone`, `keepQuestion: true` |
| **`focus.annotate`** | `kind`: `"highlight"` \| `"underline"` \| `"double-underline"` \| `"circle"` \| `"box"` \| `"brace"` \| `"connector"` \| `"label"` \| `"check"` \| `"cross"`; supports `targetId` + `targetIds` (`1-to-N` fan-out connectors!), `label`, `color`, `transient: true` |
| **`focus.scale`** | Temporary pedagogical zoom (`targetId`, `scale: 1.35`, `holdMs: 1200`) |
| **`focus.spotlight`** | Dim background around `targetIds: ["eq-2"]` (`backdropOpacity: 0.45`, `active: true`) |
| **`optics.camera`** | Smooth viewport pan/zoom (`pane`, `zoom: 1.5`, `center: [0.5, 0.4]`, `focusTargetId`, `reset: false`, `turntable3D: {"targetId": "cyl-3d", "rpm": 6.0}`) |
| **`optics.loupe`** | Vector-crisp magnifying glass (`active: true`, `pane`, `position: [0.45, 0.38]`, `magnification: 2.5`, `radiusPt: 85`) |
| **`interact.ask`** | Socratic prompt (`id`, `pane`, `kind`: `"tap-token"` \| `"checkpoint"` \| `"numeric"` \| `"free-ink"`, `prompt`, `choices: [{"id":"a","label":"$x=3$"}]`) |
| **`flow.detourPush`** | Push clean side-board onto stack (`id`, `title`: `"Quick Foundation Check"`, `scope`: `"solution"` *(keeps Question visible)* \| `"full"`) |
| **`flow.detourPop`** | Pop active detour and restore exact parent board state (`animate: true`) |
| **`flow.timeline`** | Reversible step playback: `{"action": "step"}`, `{"action": "substep"}`, `{"action": "back", "rewindStyle": "restore" \| "dim-future"}`, `{"action": "reset"}`, `{"action": "goto", "stepId": "..."}`, `{"action": "snapshot", "snapshotId": "s1"}`, `{"action": "restore", "snapshotId": "s1"}` |
| **`reactive.defineParams`** | Define live parameters & sliders (`pane`, `sliderPlacement`: `"bottom-bar"` \| `"top-bar"` \| `"floating"` \| `"hidden"`, `params: [{"id": "r", "label": "$r$", "value": 2.0, "min": 0.5, "max": 5.0, "step": 0.1, "unit": "cm", "showSlider": true}]`) |
| **`reactive.setParams`** | High-frequency (`120Hz`) batch value push from external CAS/solver (`pane`, `values: {"area": 12.56, "status_txt": "Valid"}`, `emitEvent: false`) |
| **`reactive.sweepParam`** | Smooth parameter animation (`paramId`, `from`, `to`, `durationMs`, `easing`: `"linear"` \| `"ease-in-out"` \| `"bounce"`) |
| **`reactive.clearParams`** | Clear reactive parameters (`pane`, `paramIds: [...]`) |
| **`card.setCollapsed`** | Expand/collapse collapsible margin/explanation card (`targetId`, `collapsed: true`) |
| **`ui.penPill`** | Configure student Pen Pill (`visible`, `tool`: `"select"` \| `"marker"` \| `"highlighter"` \| `"rubber"` \| `"laser"` \| `"loupe"`, `thickness`: `"fine"` \| `"medium"` \| `"bold"`, `rubberMode`: `"student-only"` \| `"include-teacher"`, `rubberGranularity`: `"pixel"` \| `"object"`, `rulerVisible`, `protractorVisible`, `compassVisible`) |
| **`audio.control`** | Procedural DSP micro-audio (`muted`, `volume`, `triggerCue`: `"marker-stroke"` \| `"rubber-swish"` \| `"attention-pop"` \| `"check-positive"` \| `"check-nudge"` \| `"session-expired"` \| `"item-expired"`) |
| **`media.open`** | Open native offline video/audio (`id`, `kind`: `"video"` \| `"audio"`, `localPath`: `"/abs/path/clip.mp4"`, `title`, `mode`: `"docked-pip"` \| `"floating"` \| `"inline"`, `pane`, `dockCorner`: `"bottom-right"`, `windowSize`: `"medium"`, `autoplay`: `true`, `captions`: `[{"startMs":0,"endMs":2500,"text":"Observe the oscillation."}]`) |
| **`media.control`** | Control playback (`id`, `action`: `"play"` \| `"pause"` \| `"toggle"` \| `"seek"` (`timeMs`) \| `"set-rate"` (`rate`) \| `"set-volume"` (`volume`) \| `"set-muted"` \| `"minimize"` \| `"restore"` \| `"set-mode"`) |
| **`media.close`** | Close media player (`id` or omit to close all) |

---

## 6. Reactive Expressions, Live Events & `state()` Snapshot

### 6.1 Reactive Expression Syntax (`@param`, `@eval(...)`, `@{...}`, `repeat`)

Once parameters are defined via `reactive.defineParams`, you can reference them anywhere inside `scene.write`, `scene.place2D`, or `scene.place3D`:

1. **Direct Scalar/String Injection (`"@r"`):** Replaced with a numeric JSON value in `spec` fields (e.g. `"radius": "@r"`) or formatted cleanly in text (`"Radius = @r cm"`).
2. **Inline Mathematical Evaluation (`"@eval(...)"` or `"@{...}"`):**
   - Evaluates standard algebra & functions (`+`, `-`, `*`, `/`, `^`, `pi`, `e`, `sqrt`, `abs`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `ln`, `log10`, `exp`, `min`, `max`, `floor`, `ceil`, `round`).
   - Example: `"Area $A = \\pi r^2 = @eval(round(pi * r^2 * 100) / 100)\\text{ cm}^2$"`
3. **Count-Driven Template Repetition (`repeat`):**
   ```json
   {
     "repeat": "@n",
     "indexVar": "i",
     "template": {
       "kind": "circle",
       "center": ["@eval(30 + i * 28)", 40],
       "radius": 10,
       "fill": "#2563EB"
     }
   }
   ```
4. **Two-Way 120Hz External Calculator / CAS Loop:**
   - Student drags slider `r` or 3D handle `h-radius` $\to$ Carbon immediately re-renders local `@eval(...)` geometry AND emits `param.changed` (`phase: "scrub"` during drag, `"commit"` on pointer release).
   - If an external CAS/Python/Rust service computes non-elementary quantities (ODE trajectories, matrix eigenvalues, exact symbolic LaTeX), it listens for `param.changed` and pushes derived parameters back via `reactive.setParams` (`"emitEvent": false` prevents echo loops!).

---

### 6.2 Live `CarbonEvent`s (Carbon $\to$ External Service)

| Event Name (`event`) | Emitted When | Key `payload` Fields |
| :--- | :--- | :--- |
| `board.ready` | WebSocket client connects | `protocolVersion`, `gpuBackend`, `viewportPx`, `scaleFactor` |
| `action.completed` | Timed write/rub/camera/sweep finishes | `actionId`, `op`, `targetId`, `elapsedMs` |
| `layout.changed` | Split divider dragged or layout toggled | `mode`, `splitRatio`, `leftCollapsed` |
| `token.tapped` | Student taps a word, `\htmlId` token, or 2D sub-element | `objectId`, `tokenId`, `pane`, `position`, `pointerType` |
| `prompt.answered` | Student submits `interact.ask` answer | `promptId`, `value` |
| `checkpoint.selected` | Student taps a Socratic checkpoint chip | `promptId`, `choiceId` |
| `ink.strokeAdded` | Student finishes a freehand ink/highlighter stroke | `strokeId`, `pane`, `tool`, `color`, `thickness`, `pointCount`, `bounds`, `points`, `overlappedTokenIds` |
| `ink.erased` | Student erases ink or teacher tokens (`include-teacher`) | `erasedStrokeIds`, `erasedTeacherTargetIds`, `pane` |
| `object3d.interacted` | Student orbits a 3D viewport | `objectId`, `elementId`, `yawDeg`, `pitchDeg` |
| `param.changed` | Slider, handle, or token mutates a `@param` | `pane`, `paramId`, `value`, `phase` (`scrub`\|`commit`), `source`, `params` |
| `table.cellChanged` | Student edits an interactive 2D table cell | `objectId`, `cellId`, `row`, `col`, `rawInput`, `numericValue`, `phase`, `pane` |
| `token.valueChanged` | Student types into an inline `editableTokens` box | `objectId`, `tokenId`, `paramId`, `rawInput`, `numericValue`, `phase`, `pane` |
| `shape2d.changed` | Student drags a 2D geometry/graph handle | `objectId`, `handleId`, `coords`, `boundParams`, `phase`, `pane` |
| `shape3d.changed` | Student drags a 3D ray-constrained handle | `objectId`, `handleId`, `coords`, `boundParams`, `phase`, `pane` |
| `marginCard.toggled` | Student collapses/expands a margin card | `objectId`, `collapsed`, `source`, `pane` |
| `stepAction.requested` | Student clicks a contextual `StepActionChip` | `actionId`, `targetTokenId`, `pane` |
| `timer.sessionExpired` | Session countdown hits `00:00` | `configuredSec`, `overtimeSec` |
| `timer.itemExpired` | Item countdown hits `00:00` | `configuredSec`, `overtimeSec` |
| `status.changed` | Header or card `⋮` menu status changed | `itemId`, `status` (`default`\|`in-progress`\|`completed`) |
| `important.toggled` | Header or card `⋮` menu `Important △` toggled | `itemId`, `important` |
| `reminder.changed` | Header or card `⋮` menu reminder (`0d..15d`) changed | `itemId`, `interval` |
| `comment.saved` | User/author saves a comment (+ screenshot/image/PDF attachments) | `entry` (`FeedbackEntry` with full `attachments[]`) |
| `comment.deleted` | Comment deleted | `commentId`, `itemId`, `partId` |
| `catalog.itemSelected` | User clicks a Subject, Paper, Topic, Concept Card, or Question in Full-Page Index | `kind` (`subject`\|`paper`\|`topic`\|`topic-card`\|`question`), `itemId`, `subjectId`, `paperId`, `topicId` |
| `catalog.toggled` | Full-Page Index opened, closed (`Open Whiteboard`), or level/tab switched | `open`, `displayMode` (`full-stage`\|`overlay`\|`docked`), `activeTab` (`papers`\|`topics`\|`stats`), `viewLevel` |
| `catalog.actionRequested` | Authorised author triggers a card `⋮` action (`authorMode: true` only) | `action`, `targetKind`, `targetId` |
| `media.stateChanged` | Video/audio play/pause/seek/PiP state changes | `mediaId`, `kind`, `status`, `playing`, `currentTimeMs`, `durationMs`, `minimized` |
| `media.ended` | Video/audio finishes playback | `mediaId`, `kind`, `durationMs` |

### 6.3 Inbound `ClientInteraction` Payloads (`"type": "interact"`)

Sent by classroom LAN iPads, custom native shells (`visage-ios`), or automated test harnesses inside `ClientMessage::Interact`:

| `interaction.kind` | Required / Optional Fields | Emitted `CarbonEvent` |
| :--- | :--- | :--- |
| `tapToken` | `objectId`, `tokenId`, `pane`, `position: [x, y]`, `pointerType` (`pen`\|`touch`\|`mouse`) | `token.tapped` |
| `answerPrompt` | `promptId`, `value` (number, string, or JSON object) | `prompt.answered` |
| `selectCheckpoint` | `promptId`, `choiceId` | `checkpoint.selected` |
| `addInkStroke` | `strokeId`, `pane`, `tool`, `color`, `thickness`, `points: [{"x","y","p","t","tiltX","tiltY","twist"}]` | `ink.strokeAdded` |
| `eraseInk` | `strokeIds: [...]`, `pane` | `ink.erased` |
| `interact3D` | `objectId`, `elementId` *(opt)*, `yawDeg`, `pitchDeg` | `object3d.interacted` |
| `changeParam` | `pane`, `paramId`, `value` (scalar or string), `phase` (`scrub`\|`commit`) | `param.changed` |
| `editTableCell` | `objectId`, `row`, `col`, `rawInput`, `phase`, `pane` | `table.cellChanged` |
| `editTokenValue` | `objectId`, `tokenId`, `rawInput`, `phase`, `pane` | `token.valueChanged` |
| `dragShape2DHandle` | `objectId`, `handleId`, `coords: [x, y]`, `phase`, `pane` | `shape2d.changed` (+ `param.changed` if bound) |
| `dragShape3DHandle` | `objectId`, `handleId`, `coords: [x, y, z]`, `phase`, `pane` | `shape3d.changed` (+ `param.changed` if bound) |
| `toggleMarginCard` | `objectId`, `collapsed: bool`, `pane` | `marginCard.toggled` |
| `requestStepAction` | `actionId`, `targetTokenId` *(opt)*, `pane` | `stepAction.requested` |
| `saveComment` | `entry: FeedbackEntry` | `comment.saved` |
| `deleteComment` | `commentId` *(opt)*, `itemId`, `partId` *(opt)* | `comment.deleted` |
| `selectCatalogItem` | `targetKind`, `itemId`, `subjectId` *(opt)*, `paperId` *(opt)*, `topicId` *(opt)* | `catalog.itemSelected` |
| `toggleCatalog` | `open: bool`, `displayMode` *(opt)*, `activeTab` *(opt)* | `catalog.toggled` |
| `requestCatalogAction` | `action`, `targetKind`, `targetId` *(honoured only when `authorMode == true`)* | `catalog.actionRequested` |
| `controlMedia` | `mediaId` *(opt)*, `op: { "action": "play" \| "pause" \| "seek" \| ... }` | `media.stateChanged` |
| `closeMedia` | `mediaId` *(opt)* | `media.stateChanged` |

---

### 6.4 `state()` Snapshot Schema (`BoardStateSnapshot`)

Querying `{"type": "getState"}` returns:
- `protocolVersion`: `"0.1"`
- `layout`: `{ "mode": "split", "splitRatio": 0.34, "leftCollapsed": false, "referenceView": "question", "effectiveOrientation": "horizontal", "leftPaneBounds": {...}, "rightPaneBounds": {...} }`
- `theme`: `"warm-paper"` | `"classroom-white"` | `"dark-glass"`
- `surfaces`: `{ "left": {...}, "right": {...}, "detour": null }`
- `regionSpace`: Array of `RegionSpaceSummary` (`pane`, `zone`, `totalBounds`, `freeBounds`, `freeRatio`)
- `objects`: Every live `SceneObjectSummary` on the board (`id`, `kind`: `"writing"` | `"diagram2-d"` | `"image"` | `"scene3-d"` | `"scribble"` | `"annotation"` | `"prompt"`, `pane`, `zone`, `bounds`, and `tokens: [{"tokenId": "...", "bounds": {...}}]`)
- `optics`: `{ "cameraScale", "cameraCenter", "loupe" }`
- `chrome`: Complete header & feedback snapshot (`authorMode`, `activeItemId`, `status`, `important`, `reminder`, `dictionaryEnabled`, `partTabs`, `pinnedReferences`, `stepActions`, `feedback` — attachment `dataBase64` stripped unless `includeAttachmentData: true`)
- `catalog`: `{ "open": false, "displayMode": "full-stage", "viewLevel": "paper-detail", "activeTab": "papers", "activeSubjectId": "maths", "activePaperId": "GYR-MTH-P008AI", "activeItemId": "GYR-MTH-P008AI-Q04", "subjectCount": 4, "paperCount": 23, "topicCount": 14, "questionCount": 250 }`
- `timers`: `{ "session": {...}, "item": {...} }`
- `detourDepth`, `detourStack`
- `userInk`: `{ "strokeCount", "strokeIds", "activeStudentIds" }`
- `reactive`: `{ "params", "clockSec", "clockPlaying", "inputPolicy", "collapsedCardIds", "rewindStyle" }`
- `media`: Active `MediaStageSnapshot` (if open)
- `telemetry`: `{ "gpuBackend", "fps", "frameTimeMs", "viewportPx", "scaleFactor" }`

---

## 7. Standalone Lesson JSON Formats (`--load-lesson`)

For batch/file-driven playback without a live WebSocket AI conductor, `carbon-studio --load-lesson <file.json>` (via `LessonAdapterRegistry`) natively accepts two open JSON formats:

### 7.1 Format A — Native Carbon Script (`"format": "carbon-script/v1"`)
```json
{
  "format": "carbon-script/v1",
  "id": "gcse-quad-01",
  "stage": "GCSE",
  "title": "Solving Quadratics by Factorising",
  "commands": [
    { "op": "config.setLayout", "params": { "mode": "split", "splitRatio": 0.34 } }
  ],
  "steps": [
    [
      { "op": "scene.write", "params": { "id": "s1", "pane": "right", "text": "Step 1: Factorise $(x - 3)(x + 2) = 0$" } }
    ],
    [
      { "op": "scene.write", "params": { "id": "s2", "pane": "right", "text": "Roots: $x = 3$ or $x = -2$" } }
    ]
  ]
}
```
Each entry in `"steps"` automatically inserts a `flow.timeline` `"step"` checkpoint so the student or teacher can step backward (`dim-future` / `restore`) and forward!

### 7.2 Format B — Schema-Tolerant Open Question Record (`open-question-record`)
```json
{
  "id": "pilot-q01",
  "stage": "11+ / Year 6",
  "topic": "Ratio & Bar Models",
  "paperBadge": "NON-CALC",
  "itemLabel": "Q4",
  "marks": 3,
  "itemTimerSec": 120,
  "question": "Ali and Ben share $\\pounds 72$ in the ratio $3 : 5$. How much more money does Ben receive than Ali?",
  "vocabulary": [
    { "term": "ratio", "definition": "Compares the relative sizes of two or more quantities." }
  ],
  "diagram2D": {
    "macro": "barModel",
    "totalLabel": "$\\pounds 72$",
    "rows": [[{ "value": 3, "label": "Ali (3)" }, { "value": 5, "label": "Ben (5)" }]]
  },
  "solutionSteps": [
    "Total parts $= 3 + 5 = 8$ parts",
    "1 part $= \\pounds 72 \\div 8 = \\pounds 9$",
    { "latex": "\\text{Difference} = (5 - 3) \\times \\pounds 9 = \\pounds 18", "cardTitle": "Final Answer" }
  ]
}
```

---

## 8. Mandatory Solution-Authoring Design Rules (`§5.5` & `§5.6`)

Any external LLM prompt (`Visage`) or curriculum compiler authoring solutions for Carbon **MUST** follow these rules so every lesson looks like a crisp textbook + master-teacher board:

1. **Always Wrap Math in `$...$` or `\(...\)`:**
   - **DO:** `"Gradient $m = \\frac{y_2 - y_1}{x_2 - x_1} = \\frac{1}{2}$"`
   - **NEVER:** Raw ASCII math (`"m = (y2 - y1)/(x2 - x1) = 1/2"` or `"x^2 + 3x"` outside `$...$`).
2. **Keep Prose in Clean Sans-Serif (`typeset` `Lato`):**
   - Put question stems, multi-sentence explanations, and `"NEVER EVER..."` warning cards in `"typography": "typeset"` so Carbon wraps them cleanly at responsive reading sizes (`16.8 pt` base in the Left Question Pane) rather than shrinking a paragraph onto one tiny line.
3. **Interactive 2D/3D Geometry Coupled to Live Equations:**
   - Whenever placing a geometric solid (`cylinder`, `cone`, `cuboid`, `prism`), graph, or physics rig, define `@param`s via `reactive.defineParams`, set `"lockViewScale": true` (plus optional `"ghostParams"` / `"ghostSolid"`), show coordinate axes through `(0,0)` / `(0,0,0)`, and reference `@r` / `@eval(...)` inside `scene.write` so dragging a slider or handle updates **both** the shape and the worked formula live!
4. **Zero-Overlap Table-Cell & Sub-Token Callouts:**
   - When highlighting an interior table cell (`"tbl:r0c2"`) or mid-equation token (`"eq1:mid"`):
     - Pair `edit.modify` `cellStyles` (`{"r0c2": {"fill": "#DCFCE7"}}`) with concise `focus.annotate` (`"kind": "check"`, `"label": "$y = 0$"`, **without** typing a literal `✓` inside `label` since `"kind": "check"` already draws a fine corner tick!).
     - Mark temporary intermediate callouts `"transient": true` (or `edit.erase` them once superseded) so callouts never pile up across steps.
5. **Use Partial `edit.rub` + Detours Pedagogically:**
   - Instead of clearing the whole board to substitute a value, wrap tokens in `\htmlId{tok}{...}` and use `edit.rub` with `replaceWith` for authentic whiteboard substitution/simplification.
   - When a student misses a prerequisite during `interact.ask`, push a `flow.detourPush` (`"scope": "solution"`) so the original Question in the Left Pane stays visible while teaching the mini-concept, then `flow.detourPop` cleanly back to the main derivation.

---

## 9. Complete End-to-End Worked Solution Session (Copy-Paste JSON)

Send the following messages sequentially over `ws://127.0.0.1:7420/carbon` to see a complete GCSE/A-Level interactive 3D + reactive KaTeX + step-by-step annotated walkthrough in action:

### Message 1 — Setup Split Board, Question Stem, Reactive Sliders & Interactive 3D Cylinder
```json
{
  "version": "0.1",
  "type": "batch",
  "id": "demo-step-1",
  "commands": [
    { "op": "edit.clear", "params": {} },
    { "op": "config.setLayout", "params": { "mode": "split", "splitRatio": 0.34 } },
    { "op": "config.setTheme", "params": { "theme": "cream-daylight" } },
    { "op": "config.setSurface", "params": { "pane": "left", "surface": { "kind": "plain" } } },
    { "op": "config.setSurface", "params": { "pane": "right", "surface": { "kind": "grid", "pitch": "medium" } } },
    {
      "op": "chrome.setHeader",
      "params": {
        "breadcrumbs": ["GCSE Maths", "Higher Paper 2", "3D Mensuration"],
        "paperBadge": "CALC",
        "itemLabel": "Q12",
        "marksChip": "4 marks",
        "calculatorMode": "allowed",
        "status": "in-progress"
      }
    },
    {
      "op": "chrome.timer",
      "params": { "target": "item", "action": "configure", "mode": "countdown", "durationSec": 240, "countUpOnExpiry": true }
    },
    {
      "op": "chrome.timer",
      "params": { "target": "item", "action": "start" }
    },
    {
      "op": "scene.write",
      "params": {
        "id": "q-stem",
        "pane": "left",
        "zone": "question-stem",
        "typography": "typeset",
        "text": "A solid metal cylinder has radius $r\\text{ cm}$ and height $h\\text{ cm}$.\n\nInitially $r = 3\\text{ cm}$ and $h = 5\\text{ cm}$. Calculate the exact volume of the cylinder in terms of $\\pi$, and explore how doubling $r$ compares with doubling $h$."
      }
    },
    {
      "op": "reactive.defineParams",
      "params": {
        "pane": "right",
        "sliderPlacement": "bottom-bar",
        "params": [
          { "id": "r", "label": "$r$", "value": 3.0, "min": 1.0, "max": 6.0, "step": 0.5, "unit": "cm", "showSlider": true },
          { "id": "h", "label": "$h$", "value": 5.0, "min": 1.0, "max": 8.0, "step": 0.5, "unit": "cm", "showSlider": true }
        ]
      }
    },
    {
      "op": "scene.place3D",
      "params": {
        "id": "cyl-stage",
        "pane": "right",
        "zone": "main-work",
        "spec": {
          "solid": "cylinder",
          "radius": "@r",
          "height": "@h",
          "color": "#2563EB",
          "opacity": 0.28,
          "lockViewScale": true,
          "ghostSolid": { "solid": "cylinder", "radius": 3.0, "height": 5.0 },
          "axes": { "xRange": [-6, 6], "yRange": [-6, 6], "zRange": [-1, 9], "tickStep": 2, "showGrid": true },
          "handles": [
            { "id": "drag-r", "kind": "radial", "min": 1.0, "max": 6.0, "value": "@r", "bindParam": "r", "label": "$r$" },
            { "id": "drag-h", "kind": "axis", "axis": "z", "min": 1.0, "max": 8.0, "pos": [0, 0, "@eval(h * 0.5)"], "bindParam": "h", "label": "$h$" }
          ]
        }
      }
    },
    {
      "op": "scene.write",
      "params": {
        "id": "eq-step1",
        "pane": "right",
        "zone": "main-work",
        "anchor": { "kind": "below", "targetId": "cyl-stage", "gapPt": 14 },
        "typography": "typeset",
        "mathStyle": "display",
        "text": "$V = \\pi \\htmlId{r_sq}{r^2} h = \\pi (@r)^2 (@h) = \\htmlId{exact_coeff}{@eval(round(r^2 * h * 100) / 100)}\\pi\\text{ cm}^3 \\approx @eval(round(pi * r^2 * h * 10) / 10)\\text{ cm}^3$"
      }
    }
  ]
}
```

### Message 2 — Advance Timeline Step, Annotate Key Quadratic Term & Add Margin Insight Card
```json
{
  "version": "0.1",
  "type": "batch",
  "id": "demo-step-2",
  "commands": [
    { "op": "flow.timeline", "params": { "action": "step" } },
    {
      "op": "focus.annotate",
      "params": {
        "id": "ann-rsq",
        "pane": "right",
        "targetId": "eq-step1:r_sq",
        "kind": "box",
        "color": "#D97706",
        "label": "Squared! Doubling $r$ multiplies $V$ by $2^2 = 4$"
      }
    },
    {
      "op": "scene.write",
      "params": {
        "id": "ans-card",
        "pane": "right",
        "zone": "summary-box",
        "anchor": { "kind": "below", "targetId": "eq-step1", "gapPt": 22 },
        "typography": "typeset",
        "text": "For $r = 3\\text{ cm},\\; h = 5\\text{ cm}$: **$V = 45\\pi\\text{ cm}^3$** ($141.4\\text{ cm}^3$ to 1 d.p.)",
        "card": {
          "title": "Final Answer (4 marks)",
          "fillColor": "#F0FDF4",
          "borderColor": "#16A34A",
          "borderStyle": "solid",
          "paddingPt": 10.0
        }
      }
    }
  ]
}
```

---

## 10. External Service Integration Recipes (`Theta`, `Visage`, `tutor.raphs` & Classroom LAN)

Carbon is strictly **Board ONLY**—it never hardcodes external calculator, LLM, or vault dependencies. Below is the exact blueprint for connecting sibling services cleanly over `Carbon Protocol v0.1` (or in-process Rust/WASM):

### 10.1 Integrating `Theta` (High-Speed Deterministic Calculator & CAS Engine)

`Theta` computes exact symbolic (`exact`), `f64` numerical (`approx`), and strict `KaTeX` (`katex`) results across `calc.eval`, `calc.solve`, `calc.formula`, `calc.transform`, `calc.stats`, and `calc.analyze_graph`. Here is how every `Theta` output maps onto Carbon:

1. **Triple Output `{ exact, approx, katex }` + `\htmlId` Sub-Tokens $\to$ `scene.write`:**
   - `Theta` returns inner KaTeX without outer dollar delimiters (e.g. `"katex": "\\frac{5\\sqrt{2}}{2}"`, with optional `anchor_tokens: true` emitting `\htmlId{tok_num}{5\sqrt{2}}`).
   - Wrap `Theta`'s `katex` string in `$...$` inside `scene.write`:
     - When Carbon's header `calculatorMode == "non-calculator"`, display `Theta`'s exact rational/surd/$\pi$ `katex` (`"$r = 5\\sqrt{\\frac{2}{\\pi}}\\text{ cm}$"`).
     - When `calculatorMode == "allowed"`, show the exact step followed by `approx` rounded via `round_dp` / `round_sf` (`"$r = 5\\sqrt{\\frac{2}{\\pi}} \\approx 3.99\\text{ cm}$"`).
2. **White-Box Step Traces (`trace: true`) $\to$ `flow.timeline` + `align: "equals"` + `edit.rub`:**
   - Each step in `Theta`'s `steps: [{ rule, exact, katex }]` maps to a `flow.timeline` `{"action": "step"}` followed by `scene.write` anchored `{"kind": "below", "targetId": prevId, "align": "equals"}` (with `rule` placed as a concise `right-of` or `auto-margin` justification), OR an inline `edit.rub` (`replaceWith`) on a `\htmlId{...}` sub-token.
3. **Domain & Physical Branch Filtering (`calc.formula` `discarded_branches`) $\to$ `auto-margin` Warning Cards:**
   - When `calc.formula` discards an unphysical root (e.g. `"-5 * sqrt(2 / pi)"`, `"violates constraint r >= 0"`), render a collapsible `auto-margin` warning card (`"Since radius $r \\ge 0$, reject the negative root."`) linked to the solved step.
4. **Graph Landmarks (`calc.analyze_graph`) & Statistics (`calc.stats`) $\to$ `scene.place2D`:**
   - Feed `calc.analyze_graph` (`x_intercepts`, `y_intercept`, `stationary_points`, `asymptotes`, `intersections`) into `coordinatePlane` (`plots`, dashed asymptote lines, and exact KaTeX coordinate labels such as `$(-\\frac{1}{2}, 0)$` instead of float approximations).
   - Feed `calc.stats` frequency-table columns ($f \cdot x$, cumulative frequency, quartiles, regression lines, hypothesis test critical regions) into `smartTable` / `table` (`cellStyles` + `focus.annotate`) and `barChart` / `lineChart` / `coordinatePlane`.
5. **3D Calculus, Conics & Linear Systems $\to$ `scene.place3D`:**
   - `calc.transform` (`diff` partial derivatives $\partial f/\partial x, \partial f/\partial y$) $\to$ `surface3D` (`contours`, `floorContours`) + live tangent-plane equations.
   - `calc.transform` (`integrate` $\pi \int_a^b ([f(x)]^2 - [g(x)]^2)\,dx$) $\to$ `solidOfRevolution` (`expr`, `innerExpr`, `sliceAt`, `sweepDeg`).
   - `calc.solve` ($3 \times 3$ linear systems) $\to$ `planes` (`showIntersections: true`) highlighting unique point, line of intersection, or inconsistent parallel planes.
6. **Two-Way 120Hz Reactive & RK45 Physics Loop (2 Deployment Topologies):**
   - **Topology A — Out-of-Process / WebSocket Co-Processor:** Listen on `ws://<host>:7420/carbon` for `param.changed` (`phase: "scrub" | "commit"`), `shape2d.changed`, `shape3d.changed`, `table.cellChanged`, and `token.valueChanged`. Evaluate in `Theta` (`< 25 µs`) and push derived scalars or dynamic KaTeX strings back via:
     ```json
     {
       "version": "0.1",
       "type": "command",
       "op": "reactive.setParams",
       "params": {
         "pane": "right",
         "values": { "exact_vol_katex": "\\frac{45\\pi}{2}", "rk45_y": 3.412 },
         "emitEvent": false
       }
     }
     ```
     **Always set `"emitEvent": false`** on solver-driven `reactive.setParams` calls so Carbon updates all bound `@param` equations, 2D rigs (`spring`, `pendulum`, `projectileArc`), and 3D stages on the next 120Hz frame without re-emitting `param.changed`!
   - **Topology B — In-Process / Shared `wasm32` Runtime:** Because both `carbon-shell` (`StageRuntime`) and `libtheta_calc` compile to Pure Rust native + `wasm32-unknown-unknown`, a host shell can link both in-process and pass `StageRuntime` parameter updates directly with zero socket serialization.

---

### 10.2 Integrating `Visage` (AI Conductor, STT/TTS Voice & Vision Grading)

1. **Lockstep Voice-to-Marker Writing (`speechBeats`):** Pass TTS word/phrase timing spans in `scene.write.params.speechBeats` so Carbon unveils each `\htmlId{...}` math token at the exact millisecond the tutor speaks it, then await `"event": "action.completed"` (`actionId`) before advancing to the next pedagogical beat.
2. **Handwritten Student Work Inspection (`ink.strokeAdded` + `captureBoard`):**
   - Subscribe to `ink.strokeAdded` (which automatically reports `overlappedTokenIds` if the student circled, underlined, or crossed out a specific equation term!).
   - Request `{"type": "captureBoard", "pane": "right", "cropToUserInk": true, "includeChrome": false}` to obtain a clean, tight base64 PNG of the student's handwritten working for VLM/OCR verification (or pass extracted LaTeX to `Theta` to verify algebraic equivalence).
3. **Socratic Misconception Recovery (`interact.ask` $\to$ `flow.detourPush` / `Pop`):**
   - Ask a targeted check via `interact.ask` (`checkpoint`, `tap-token`, `numeric`, or `free-ink`).
   - On `prompt.answered` / `checkpoint.selected`, if a prerequisite gap is detected, call `flow.detourPush` (`"scope": "solution"`) to open a clean scratchpad while keeping the Left Question Pane visible, teach the micro-skill, and call `flow.detourPop` to return seamlessly to the main solution.

---

### 10.3 Integrating `Hydrogen` / `SIGMA`, `tutor.raphs` Vaults & Multi-Student Classrooms

1. **Book Index Lifecycle (`chrome.setCatalog` + `catalog.itemSelected`):**
   - On startup (or when switching subjects), `Hydrogen` / `SIGMA` opens **one subject `.raphs` cartridge** (e.g. `gyara-maths.raphs`) + `student.raphs`, and sends `chrome.setCatalog` containing `availableSubjects` (e.g. `Maths`, `English`, `VR`, `NVR`), the active subject's `papers` and `topics` (`Chapters → Topics → Cards/Questions`), and pre-computed `stats` (`metrics` + `breakdowns`).
   - When the learner clicks a Question (`kind: "question"`), Topic Card (`kind: "topic-card"`), or another Subject (`kind: "subject"`), Carbon emits `catalog.itemSelected` $\to$ `Hydrogen` / `SIGMA` loads that question onto Carbon (or mounts the newly selected subject `.raphs` and calls `chrome.setCatalog`).
   - After each completed question, star toggle, or reminder update, call `chrome.patchCatalogItem` for an `O(1)` badge & stats update without resending the full catalog.
2. **Lifelong Student Vault Signals (`student.raphs`):**
   - Subscribe to `status.changed` (`default` | `in-progress` | `completed`), `important.toggled` (`★`), `reminder.changed` (`0d..15d`), `timer.itemExpired`, and `timer.sessionExpired`—all stamped with `itemId` and `actor.studentId`—so `SIGMA` persists learner progress and computes spaced-repetition queues and mastery stats externally.
3. **Public Feedback vs. Authorised Authoring (`authorMode`):**
   - Leave `"authorMode": false` for public/student sessions (exposing only plain comment text + 1-click `"Snap Board"` PNG + image/PDF attachments).
   - Set `"authorMode": true` via `chrome.update` only inside authenticated author/admin sessions to unlock `fix`/`replica`/`topic`/`engine` tags, `Pending`/`Done` triage, `authorMeta` context stamps, and `catalog.actionRequested` triggers.
4. **Open File / Cartridge Playback (`LessonSourceAdapter`) & Multi-iPad LAN (`/carbon/lan-sync`):**
   - Static JSON lessons load via `carbon-script/v1` (§7.1) or `open-question-record` (§7.2).
   - In classroom `--lan` mode, late-joining Safari iPads (`http://<host-ip>:7420/?studentId=<id>`) automatically replay `lan_journal` via `/carbon/lan-sync` and tag every interaction with `actor.studentId`.
