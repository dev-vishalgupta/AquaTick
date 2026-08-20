# AquaTick 💧

> A lightweight Windows desktop companion that helps you build a consistent water-drinking habit through timely reminders and a friendly animated character.

AquaTick is designed around **habit formation**, not water tracking. Instead of asking users to constantly log water intake, AquaTick gently reminds them to drink at appropriate intervals and turns each reminder into a small, pleasant interaction.

## Features

- ⏰ Configurable water-drinking reminders
- 🎯 Automatic or manual reminder targets
- 💤 Idle detection to avoid interrupting users when they are away
- 🧍 Animated desktop character companion
- 🪟 Dedicated reminder experience
- ✅ **Yes, I Drank** action
- 💤 **Snooze** action
- 📊 Weekly activity/statistics view
- 🔔 Windows system/tray integration
- 🚀 Launch on Windows startup
- 💾 Local persistence using SQLite
- 🎨 Character selection and character-system support
- 🌙 UI designed to work across light and dark themes

## How AquaTick Works

AquaTick runs in the background and schedules reminder sessions according to the user's settings.

When a reminder is triggered:

1. The reminder engine creates and manages the reminder session.
2. The presentation layer displays the reminder experience.
3. The character can enter and perform its interaction sequence.
4. The user can confirm that they drank, snooze the reminder, or allow the session to resolve according to the configured workflow.
5. The result is persisted locally for statistics and future scheduling.

When the user is idle for the configured period, reminder activity can be paused so AquaTick does not interrupt an unattended desktop.

## Architecture

AquaTick follows a strict layered architecture:

```text
React
  ↓
IPC / Event Bridge
  ↓
Services
  ↓
Repositories
  ↓
SQLite
```

### Responsibility Boundaries

| Component | Responsibility |
|---|---|
| **React / Frontend** | UI, user interaction, and presentation |
| **IPC / Event Bridge** | Communication between frontend and backend |
| **Services** | Business logic and application workflows |
| **Repositories** | Database access only |
| **Scheduler** | Reminder timing and scheduling |
| **Reminder Engine** | Reminder session lifecycle and resolution |
| **Activity Monitor** | Desktop idle/activity detection |
| **Character System** | Character state, animation, movement, and rendering |
| **Sound System** | Audio playback |
| **SQLite** | Local persistent application data |

The architecture intentionally keeps timing, business logic, persistence, UI, animation, and audio concerns separate.

## Tech Stack

- **Frontend:** React + TypeScript
- **Build Tool:** Vite
- **Desktop Runtime:** Tauri v2
- **Backend:** Rust
- **Database:** SQLite
- **State Management:** Zustand
- **Styling:** Tailwind CSS
- **Validation:** Zod
- **Package Manager:** pnpm
- **Platform:** Windows

## Project Structure

The repository is organized as a pnpm monorepo.

```text
AquaTick/
├── apps/
│   └── desktop/
│       ├── src/
│       │   └── ...
│       └── src-tauri/
│           └── ...
├── package.json
├── pnpm-workspace.yaml
└── README.md
```

The desktop application contains the React frontend and the Tauri/Rust backend.

Backend responsibilities are separated into application services, repositories, scheduling, reminder processing, activity monitoring, and related infrastructure.

## Development

### Prerequisites

Install:

- Node.js
- pnpm
- Rust
- Tauri v2 prerequisites for Windows development

### Install Dependencies

From the repository root:

```bash
pnpm install
```

### Run in Development

```bash
pnpm run dev
```

The development environment starts the AquaTick desktop application through the configured Tauri/Vite development workflow.

> Development autostart behavior is intentionally separated from production startup registration so that development builds do not accidentally register debug executables with Windows startup.

## Building

Use the repository's configured Tauri build command to create a production Windows application.

```bash
pnpm run tauri build
```

The generated application artifacts are produced by Tauri according to the project's current configuration.

## Character System

The AquaTick character is driven by a state-based character system rather than embedding animation behavior directly into the UI.

The character system separates:

- Character state
- Animation control
- Movement
- Rendering
- Asset management
- Character orchestration

The reminder interaction is designed around states such as:

```text
Hidden
  ↓
Entering
  ↓
Walking
  ↓
PickBottle
  ↓
DrinkLoop
  ↓
PutBottleDown
  ↓
Leaving
```

The character remains oriented toward the user during the bottle interaction. The bottle is retrieved from behind the character through the character's hand rather than turning the character around.

## Reminder Architecture

Reminder timing and reminder workflow are intentionally separate.

```text
Scheduler
    │
    ▼
Reminder Engine
    │
    ├── Reminder Session
    ├── Presentation Events
    └── Session Resolution
             │
             ▼
        Persistence
```

The Scheduler determines **when** a reminder should occur.

The Reminder Engine determines **what happens during the reminder session**.

The frontend presents the resulting experience without owning the underlying reminder business logic.

## Data & Persistence

AquaTick uses SQLite for local persistence.

Repositories are responsible for database operations, while services own the business rules that determine how stored data is used.

This keeps persistence concerns isolated from the rest of the application.

## Current Status

The core AquaTick application architecture and major application systems are implemented.

### Completed

- Desktop application foundation
- React/Tauri integration
- SQLite persistence
- Settings and configuration
- Scheduler
- Reminder Engine
- Activity Monitor
- Reminder presentation/event bridge
- Character system architecture
- Character state machine
- Tray/lifecycle functionality
- Statistics-related functionality
- Settings workflows
- Reliability and lifecycle fixes

### Remaining

The primary remaining product work is the **final character animation production and integration**.

The character production workflow is intentionally treated separately from the application architecture. Final production-quality animation assets should maintain consistent character proportions, appearance, and frame-to-frame continuity.

## Character Production

The planned production pipeline is:

```text
Character Production Specification
          ↓
Turnaround Sheet
          ↓
Expression Sheet
          ↓
Pose Sheet
          ↓
Animation Key Poses
          ↓
Clean Sprite Production
          ↓
Sprite Sheets
          ↓
AquaTick Integration
```

AI-generated sprite sheets are not intended to be used directly as final production assets. They can be used as visual references, but final animation requires consistent frame-to-frame character construction.

## Design Philosophy

AquaTick is intentionally designed to feel like a small desktop companion rather than another productivity dashboard.

The product focuses on:

- Gentle interruption
- Consistent habit formation
- Minimal friction
- Pleasant character interaction
- Local-first data
- Clear separation between application systems

## Project Status

AquaTick is currently in the final production stage.

The application architecture is substantially complete. The remaining major work is bringing the character's final production-quality animation assets into the existing Character System.

## License

No license is declared here unless one has been explicitly added to the repository.
