# AquaTick — Phase 8B: Reminder Presentation Layer Implementation Plan

## 1. Objectives
This phase establishes the foundational Presentation Layer for the Reminder Experience, adhering strictly to the architecture defined in Phase 8A. The objective is to implement the reactive UI components (Reminder Window, Overlay Window, and Character System) that map directly to the conceptual Presentation State Machine. This phase ensures the windows are correctly positioned, managed in z-order, focused properly, and synchronized with the Event Bridge without containing any business logic.

## 2. Scope

### In Scope
* Reminder Window lifecycle (presentation starts, presentation becomes visible, presentation hides).
* Overlay Window behavior (transparency, spanning, click-through).
* Character synchronization (connecting character visual states to presentation events).
* Visibility management (showing, hiding).
* Window positioning and z-order management (via existing architecture).
* Window focus rules (preventing stealing focus inappropriately).
* Presentation state synchronization (mapping backend events to UI states).
* Presentation event handling (listening for Engine events, emitting ready/closed acknowledgments).

### Out of Scope
* Reminder outcomes (e.g., Drink, Snooze, Ignore, Timeout).
* The Reminder business state machine (Reminder Engine).
* Audio timeline and Sound System implementations.
* Animation sequencing (reward animation, success animation, dismissal animation, outcome feedback).
* Edge-case handling (e.g., unexpected failures or recovery).
* Scheduler logic modifications.
* Statistics service updates.

## 3. Existing Components
* **Reminder Window**: A Tauri window responsible for rendering the primary call-to-action UI. It must react to events but hold no internal reminder state.
* **Overlay Window**: A Tauri window responsible for rendering screen overlays (e.g., dimming or border highlights) to draw attention.
* **Character System**: An existing React/Rust component responsible for character visuals. It will receive new triggers from the presentation layer but will not dictate reminder logic.
* **Event Bridge**: The existing messaging layer that facilitates decoupled communication between the Rust backend and the React frontend.
* **Reminder Engine**: The architectural source of truth that emits state changes.

## 4. Components Requiring Changes

### `ReminderWindow` (Frontend/React & Backend window setup)
* **Why it changes**: Needs to react to `ReminderPresented` and `ReminderDismissed` events.
* **Responsibility gained**: Rendering the conceptual presentation states (hidden, preparing, showing).
* **Responsibility NOT gained**: Managing timeouts, outcome resolution, animation sequencing, or capturing business outcomes.

### `OverlayWindow` (Frontend/React & Backend window setup)
* **Why it changes**: Must coordinate with the Event Bridge to appear simultaneously with the reminder.
* **Responsibility gained**: Rendering screen-wide subtle overlays based on the presentation state.
* **Responsibility NOT gained**: Capturing interaction, business logic, or animation sequencing.

### `CharacterSystem` (Frontend/React)
* **Why it changes**: Needs to visually react when a presentation starts or hides.
* **Responsibility gained**: Tying character visual state to the presentation lifecycle.
* **Responsibility NOT gained**: Acting as the initiator or controller of the reminder session, or defining outcome animations.

### `Existing Window Architecture` (Backend/Rust)
* **Why it changes**: Must handle the physical showing, focusing, z-ordering, and hiding of the Reminder and Overlay windows on the OS level without introducing new services.
* **Responsibility gained**: Applying positioning, z-order, and focus rules to existing window types during the presentation lifecycle.
* **Responsibility NOT gained**: Business state tracking, resolving reminder sessions, or acting as a new "Window Manager" subsystem.

## 5. Presentation Lifecycle
The Presentation Layer follows a strict, conceptual lifecycle corresponding to visibility states:
1. **Hidden**: The windows are unrendered or completely invisible off-screen.
2. **Preparing**: Upon receiving `ReminderPresented`, components prepare for display. The presentation layer requests positioning according to the approved window architecture.
3. **Showing**: The windows transition into view. Z-order and focus rules are applied by the backend.
4. **Waiting**: The UI settles into an idle state while visible. The `PresentationReady` event is fired.
5. **Closing**: Upon receiving a `ReminderDismissed` event, the windows transition out of view.
6. **Hidden**: The windows become completely invisible again, and `PresentationClosed` is fired.

## 6. Event Flow
* **`ReminderPresented`**
  * **Source**: Reminder Engine (via Event Bridge)
  * **Destination**: Reminder Window, Overlay Window, Character System
  * **Payload**: Session ID
  * **Purpose**: Instruct the presentation layer that presentation starts.
* **`PresentationReady`**
  * **Source**: Reminder Window / Overlay Window
  * **Destination**: Reminder Engine (via Event Bridge)
  * **Payload**: Component ID
  * **Purpose**: Optional diagnostic acknowledgment that the window is successfully visible.
* **`ReminderDismissed`**
  * **Source**: Reminder Engine (via Event Bridge)
  * **Destination**: Reminder Window, Overlay Window, Character System
  * **Payload**: None
  * **Purpose**: Instruct the presentation layer that presentation hides.
* **`PresentationClosed`**
  * **Source**: Reminder Window / Overlay Window
  * **Destination**: Reminder Engine (via Event Bridge)
  * **Payload**: Component ID
  * **Purpose**: Acknowledge that the window is successfully `Hidden`.

## 7. Component Interaction
1. The **Reminder Engine** (backend) broadcasts `ReminderPresented` via the **Event Bridge**.
2. The **Event Bridge** delivers this payload simultaneously to the **Reminder Window**, **Overlay Window**, and **Character System** (frontend).
3. The **Overlay Window** and **Reminder Window** begin their showing sequence. The presentation layer requests positioning according to the approved window architecture.
4. The **Character System** synchronizes its visual state to match the presentation start.
5. Once visible, the frontend components independently emit `PresentationReady` back through the **Event Bridge**. This event is strictly optional; the Reminder Engine never blocks waiting for it, and the Reminder Engine always remains authoritative. It exists only for diagnostics or optional synchronization.
6. When the presentation must end, the **Reminder Engine** broadcasts `ReminderDismissed`.
7. The presentation components independently transition to hidden and emit `PresentationClosed`, reverting to the `Hidden` passive state.

## 8. Character Synchronization
The Character System operates strictly as a passive presentation component mirroring the reminder's visibility lifecycle.
* **Visibility Ownership**: The character is only shown or updated based on `ReminderPresented` and `ReminderDismissed` events broadcasted over the Event Bridge.
* **Presentation Ownership**: The Character System owns its rendering but relies entirely on the Reminder Engine for state. It holds no internal timers or reminder logic.
* **Lifecycle Relationship**: The character's visual state is intrinsically tied to the presentation lifecycle (starts showing when presented, returns to idle when hidden).
* **Synchronization Rules**: The Character System reacts simultaneously with the Reminder Window and Overlay Window to Event Bridge broadcasts, maintaining decoupled independence.

## 9. Implementation Order

### Step 1: Event Bridge Wiring
* **Purpose**: Define the types and listeners for `ReminderPresented`, `PresentationReady`, `ReminderDismissed`, and `PresentationClosed`.
* **Dependencies**: None.
* **Expected outcome**: Frontend components can successfully listen to and emit these specific events over Tauri IPC.

### Step 2: Window Visibility Logic (Backend)
* **Purpose**: Implement the OS-level positioning, z-ordering, and focus rules for the Reminder and Overlay windows using existing window architecture.
* **Dependencies**: Step 1.
* **Expected outcome**: Existing windowing systems reliably show, position, and hide the required windows.

### Step 3: Overlay Window Lifecycle (Frontend)
* **Purpose**: Implement the `Hidden` -> `Showing` -> `Waiting` -> `Closing` visual states for the Overlay.
* **Dependencies**: Step 1, Step 2.
* **Expected outcome**: Overlay transitions from hidden to showing upon receiving the respective events, emitting acknowledgments.

### Step 4: Reminder Window Lifecycle (Frontend)
* **Purpose**: Implement the conceptual state machine for the main Reminder Window.
* **Dependencies**: Step 1, Step 2.
* **Expected outcome**: Reminder window appears, enters the waiting state, optionally emits `PresentationReady`, and later hides itself upon command, emitting `PresentationClosed`.

### Step 5: Character Synchronization
* **Purpose**: Tie the character's visual state to the presentation events.
* **Dependencies**: Step 1.
* **Expected outcome**: Character transitions visually when the presentation starts and hides.

## 10. Risks

* **Circular dependencies**: The UI emitting `PresentationReady` might accidentally trigger another `ReminderPresented` if the Event Bridge isn't strictly unidirectional in its handlers.
  * *Mitigation*: Ensure event payloads and topics are distinct. UI only emits acknowledgment events.
* **Presentation owning business logic**: Developers might be tempted to manage timeouts in the React component.
  * *Mitigation*: Enforce through code review that the UI *only* hides when it receives the `ReminderDismissed` event from the Engine.
* **Window synchronization issues**: Overlay and Reminder window appearing out of sync due to React rendering differences.
  * *Mitigation*: Both components rely on the single `ReminderPresented` broadcast. Minor millisecond desyncs are acceptable as long as both react to the same master event.
* **Event duplication**: React strict mode or multiple listeners causing duplicate `PresentationReady` events.
  * *Mitigation*: Use careful `useEffect` cleanup and strict deduplication logic on the backend Event Bridge listeners.

## 11. Verification Checklist
* [ ] Phase 8A architecture is strictly preserved.
* [ ] Reminder Engine remains the single authoritative source of truth.
* [ ] Presentation layer remains entirely passive, reacting only to incoming events.
* [ ] No business logic, timers, or timeouts leak into the UI components.
* [ ] Components remain decoupled (Overlay doesn't explicitly wait for Reminder Window).
* [ ] Event flow (`Presented` -> `Ready`, `Dismissed` -> `Closed`) remains deterministic and verifiable.
* [ ] Presentation lifecycle independent from reminder outcomes.
* [ ] Character owns no reminder state.
* [ ] `PresentationReady` never blocks Reminder Engine.
* [ ] Overlay owns no business logic.
* [ ] No Phase 8C responsibilities exist inside Phase 8B.
