# AquaTick — Phase 5A: Character System Implementation Roadmap

This roadmap breaks down the Character System implementation into small, independent, and verifiable phases. This ensures adherence to the Architecture-First workflow, minimizing risk and allowing every phase to compile and visually test successfully before moving to the next.

---

## Phase 5A.1: Domain Models and Interfaces

- **Objective**: Define the foundational types, enums, and API signatures for the Character System.
- **Files Created / Modified**:
  - `apps/desktop/src/features/character/types.ts`
  - `apps/desktop/src/features/character/constants.ts`
- **Dependencies**: None.
- **Architectural Boundaries**: Pure TypeScript definitions. No React, no rendering logic.
- **Verification Checklist**:
  - [ ] TypeScript compiles successfully without errors.
  - [ ] `CharacterSystemAPI` matches the architecture document perfectly.
  - [ ] Asset metadata JSON interfaces are strictly typed.
  - [ ] State enumerations and transition definitions are typed.
- **Completion Criteria**: All interfaces and types are exported and successfully compile in the Vite/TS environment.

---

## Phase 5A.2: React Host Integration

- **Objective**: Provide the React layer to mount the Character System in the DOM early, allowing subsequent phases to visually render immediately.
- **Files Created / Modified**:
  - `apps/desktop/src/features/character/components/CharacterCanvas.tsx`
  - `apps/desktop/src/features/character/index.ts`
- **Dependencies**: Phase 5A.1.
- **Architectural Boundaries**: Purely a React host. Creates the canvas element to host the upcoming Render Engine. No business logic.
- **Verification Checklist**:
  - [ ] Component renders a transparent, absolute/fixed positioned `<canvas>`.
  - [ ] Transparent window overlays correctly.
  - [ ] Canvas respects DPI scaling.
  - [ ] Canvas resizes correctly with the window.
  - [ ] Pointer events behavior is verified (this is important for a desktop companion).
- **Completion Criteria**: The React component renders a blank, scalable, transparent canvas successfully in the application dashboard.

---

## Phase 5A.3: Asset Manager & Render Engine

- **Objective**: Implement the generic Render Engine and the Asset Manager to load and draw sprites onto the React Host.
- **Files Created / Modified**:
  - `apps/desktop/src/features/character/core/AssetManager.ts`
  - `apps/desktop/src/features/character/core/RenderEngine.ts`
- **Dependencies**: Phase 5A.1, Phase 5A.2.
- **Architectural Boundaries**: Framework-agnostic rendering logic hooked into the host canvas. Only DOM Canvas API and browser Image API.
- **Verification Checklist**:
  - [ ] `AssetManager` can fetch and parse `character.json`.
  - [ ] `AssetManager` can preload `spritesheet.png` and cache it.
  - [ ] `RenderEngine` successfully clears and draws a static frame on the React Host Canvas.
- **Completion Criteria**: A static character sprite renders on the screen, proving asset loading and canvas context binding work.

---

## Phase 5A.4: Core Controllers

- **Objective**: Implement deterministic time-based frame calculations and spatial interpolation to animate the static sprite.
- **Files Created / Modified**:
  - `apps/desktop/src/features/character/core/AnimationController.ts`
  - `apps/desktop/src/features/character/core/MovementController.ts`
- **Dependencies**: Phase 5A.1, Phase 5A.3.
- **Architectural Boundaries**: Pure math and time calculations. Agnostic of rendering, React, and the main state machine.
- **Verification Checklist**:
  - [ ] `AnimationController` calculates the correct frame index based on FPS and delta-time.
  - [ ] `AnimationController` correctly handles looping vs. one-shot animations.
  - [ ] `MovementController` computes intermediate $(x, y)$ coordinates over time using movement interpolation.
  - [ ] A test loop animates and moves the character on the canvas visually.
- **Completion Criteria**: The sprite animates frames accurately and glides across the screen, verifying math and delta-time loops.

---

## Phase 5A.5: Character State Machine (CSM)

- **Objective**: Implement the deterministic state machine to govern character visual states and prevent invalid transitions.
- **Files Created / Modified**:
  - `apps/desktop/src/features/character/core/CharacterStateMachine.ts`
- **Dependencies**: Phase 5A.1, Phase 5A.4.
- **Architectural Boundaries**: State transition logic only. Orchestrates Core Controllers without depending on Tauri or React.
- **Verification Checklist**:
  - [ ] CSM rejects invalid transitions.
  - [ ] Start transition logic (e.g., `Hidden` -> `Entering` triggers Movement and Animation).
  - [ ] Action transition logic (e.g., `Walking` -> `PickBottle` -> `DrinkLoop`).
  - [ ] State properties cleanly expose current Animation/Movement data for the Render Engine.
- **Completion Criteria**: Visual state transitions work seamlessly (e.g., character switches from walking to picking up the bottle).

---

## Phase 5A.6: Character Controller (Public API)

- **Objective**: Wire all core components together and expose the unified Public API for the Event Coordinator.
- **Files Created / Modified**:
  - `apps/desktop/src/features/character/core/CharacterController.ts`
- **Dependencies**: Phases 5A.1 - 5A.5.
- **Architectural Boundaries**: The orchestrator of the Character System. Still completely framework-agnostic. Implements the `CharacterSystemAPI`.
- **Verification Checklist**:
  - [ ] Instantiates CSM, AssetManager, RenderEngine, AnimationController, and MovementController.
  - [ ] The public methods (`play()`, `walkTo()`, `show()`, `hide()`, `leave()`) map cleanly to internal state updates.
  - [ ] Methods returning Promises resolve correctly based on Animation/Movement completion.
  - [ ] React Host properly exposes the Public API via Ref/Context.
- **Completion Criteria**: The public API fully orchestrates the character visually from external triggers, passing all integration checks.
