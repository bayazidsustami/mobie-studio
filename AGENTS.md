# Mobie Studio: Agent Vision & Architecture

## Vision
Mobie Studio is a lightweight, AI-powered desktop application built with Rust and GPUI. It empowers mobile-first QA and Engineers to run automated mobile tests with zero-effort setup. Instead of writing brittle interaction scripts, users converse with an autonomous agent that navigates the UI to achieve high-level goals. Conversations are **multi-turn and stateful** — the LLM retains full context across every prompt in a session, so follow-up instructions build naturally on prior actions.

## Key Terms
- **Agent:** The autonomous, LLM-driven entity that interprets user goals, observes the mobile UI, and decides the next sequence of actions using the `rig-core` framework.
- **UI Dump:** The XML representation of the current screen hierarchy extracted from the device (via Android's `uiautomator`). This serves as the agent's "eyes".
- **Action / Tool:** A specific, atomic interaction sent to the device (e.g., `Tap`, `Swipe`, `Input`, `KeyEvent`, `Observe`, `Screenshot`) executed as a native tool by the LLM.
- **Auto-Observe:** Before each user turn, the engine calls `device.observe_ui()` synchronously and prepends the compressed XML to the LLM prompt. This guarantees the model always has fresh context, even if the user manually changed apps between prompts.
- **Active Session:** The open multi-turn conversation window. Lives in `AgentEngine` between `StartGoal` calls. Contains the `session_id`, accumulated rig history, per-session `TestStep` history, and a stable YAML export path. Reused across prompts; finalized on `EndSession` or `NewSession`.
- **Rig History:** The full `Vec<rig::completion::Message>` representing the LLM conversation (user prompts, assistant replies, tool calls, tool results). Persisted to SQLite as JSON in `sessions.rig_history_json`. Replayed into `PromptRequest::with_history(&mut history)` on every turn so the LLM sees prior context.
- **Session Lifecycle:** `StartGoal` → reuse or mint active session → auto-observe → call LLM with accumulated history → persist updated history + step list → overwrite stable YAML. The user explicitly clicks "New Session" or "End Session" to close the window.
- **Implicit Wait & Retry:** The execution loop where the agent acts, observes the resulting state, and autonomously decides whether to retry an action or proceed.
- **Session Memory:** The agent's contextual awareness of recent actions to prevent repetitive failure loops.
- **Exploratory Run:** A conversational session where the user provides a goal and the agent figures out the steps dynamically via multi-step planning (Plan -> Execute).
- **Stable YAML Path:** `~/mobie-results/<session_id>.yaml`. Rewritten after every successful turn so the file always reflects the latest step list. Screenshots live in `screenshots/<session_id>/`.
- **YAML Test Case:** A declarative file generated after a successful exploratory run, documenting the exact steps and assertions for future, repeatable CI/CD execution.

## Architecture & Implementation Standards
The application operates within a single process, utilizing asynchronous Rust tasks to ensure the UI remains responsive during agent operations. Implementation will strictly adhere to the following skill guidelines:

- **Rust Architecture & Quality:** `@systems-programming-rust-project` and `@rust-pro` for robust project structure, modern Rust (1.75+) features, and production-ready systems programming.
- **Concurrency:** `@rust-async-patterns` for handling the asynchronous Agent Engine, Tokio task management, and safe state sharing across threads.
- **Frontend Design:** `@ui-skills` for building a clean, responsive, and opinionated GPUI interface.

### Core Modules
1. **Frontend (GPUI):** Manages the chat interface, LLM provider settings (BYOK model), device selection, session controls (New / End / Resume), and displays the generated YAML outputs.
2. **Agent Engine (Async Task Manager):** Owns the **active session state** in its `run_loop`, handles message dispatch, performs auto-observe, drives the multi-turn `rig-core` agent loop via `with_history(&mut Vec<Message>)`, and persists history + YAML on every turn. Communicates progress back to the UI via asynchronous channels (`mpsc`).
3. **Device Bridge (ADB Interactor):** A dedicated module executing `std::process::Command` calls to local `adb` binaries for device discovery, UI dumping (`observe_ui`), and action execution. Mocked via the `CommandRunner` trait for tests.
4. **LLM Client (`rig-core`):** Integrates the `rig-core` AI framework (native tool calling, structured reasoning). It uses `reqwest` to securely interact with LLM providers like OpenRouter, injecting mandatory headers (e.g., `HTTP-Referer`, `X-Title`) required for BYOK. Multi-turn is achieved with `agent.prompt(goal).with_history(&mut history).max_turns(50)`, which mutates the `Vec<rig::completion::Message>` in place to accumulate the full conversation including tool calls.
5. **Persistence (SQLite via `rusqlite`):** Stores sessions, chat messages, and the serialized rig history. Schema migration is additive (`ALTER TABLE ... ADD COLUMN`) so existing user data is never rewritten. See **Data Model** below.

### Data Model
| Table | Purpose | New in this revision |
|---|---|---|
| `sessions` | One row per multi-turn session | + `rig_history_json TEXT` (nullable, serialized `Vec<Message>`) |
| `chat_messages` | Per-turn user/assistant chat log | unchanged |
| `chat_messages.role` | `'user'` or `'assistant'` | unchanged |

Migrations are forward-only and idempotent (`let _ = self.conn.execute("ALTER TABLE ...")` pattern).

## The Agent Loop
The engine is no longer single-shot. It maintains a single `ActiveSession` in memory and persists it on every turn. When a user sends a prompt:

1. **Session Resolution:** Reuse the active session if one is open, otherwise mint a new `sess_<unix_timestamp>` id and insert a row in `sessions`.
2. **Auto-Observe:** Call `device.observe_ui()` synchronously (no LLM round-trip) and prepend the compressed XML to the user prompt as `[Auto-observed current screen state]\n<xml>\n\nUser request: <goal>`. This handles the "user manually closed the app between prompts" case.
3. **LLM Call:** Pass the accumulated `Vec<rig::completion::Message>` into `agent.prompt(combined_prompt).with_history(&mut history).max_turns(50)`. The agent mutates `history` in place, appending the new prompt + all tool calls + the final assistant reply.
4. **Persist:** Serialize the post-turn `history` to `sessions.rig_history_json` and append a row to `chat_messages`. Overwrite `~/mobie-results/<session_id>.yaml` with the latest `TestStep` list.
5. **UI Sync:** Push `AgentUpdate::SessionStateUpdate { turn_count, step_count, ... }` so the status bar updates.

The `with_history` API replaces the previous "clear and re-prompt" pattern. There is no implicit state machine — the LLM is in full control of the loop via its tool calls, and `max_turns(50)` gives it enough headroom for complex navigation.

## Decision Log
- **Architecture:** Monolithic GPUI + Rust desktop app.
- **AI Framework:** Migrated from manual JSON parsing to `rig-core` for LLM provider abstraction, native tool calling, and structured reasoning.
- **Agent Loop:** Transitioned to an autonomous, tool-driven loop with multi-step planning, session memory tracking, and implicit verification.
- **LLM Strategy:** Bring-Your-Own-Key (BYOK) with OpenRouter support via mandatory custom HTTP headers.
- **Test Output:** Auto-generated declarative YAML test cases after successful exploratory runs.
- **Multi-turn Memory:** Use `rig-core`'s `PromptRequest::with_history(&mut Vec<Message>)` to carry the full LLM conversation across prompts. Persist the vec as JSON in `sessions.rig_history_json`. No compaction yet — the JSON is opaque and only used for resume.
- **Auto-Observe:** Every new user prompt is preceded by a synchronous `observe_ui()` so the LLM always sees the current screen, even if the user manually changed apps between prompts. Costs ~1-3k extra tokens per turn but eliminates the stale-context failure mode.
- **Session Lifecycle:** Explicit `NewSession` / `EndSession` / `SwitchSession` actions. The engine does not auto-create a session on every send — once a session is open, every `StartGoal` continues it. The `+ New Session` button in the chat input is the only way to start fresh.
- **YAML Export Semantics:** Stable per-session path `~/mobie-results/<session_id>.yaml`. Overwritten after every successful turn. Screenshots live in `~/mobie-results/screenshots/<session_id>/`. No data loss on app crash mid-session because the file is rewritten after each turn, not on session close.
- **DB Migrations:** Additive-only. Existing data is never rewritten. New columns are nullable and backfilled with `NULL` on upgrade.

## Development Workflow
We implement and strictly follow **Git Flow** for managing changes to the project. 
- **New Features & Additions:** Any new feature, enhancement, or change must start with a new branch created from the main development branch (typically `develop`).
  - Example: `git checkout -b feature/my-new-feature develop`
- **Commits:** Follow the Conventional Commits specification for all commit messages.
- **Merging:** Features are developed in isolation and merged back into the development branch via pull requests once complete and validated.
