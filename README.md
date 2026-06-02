# Mobie Studio

AI-powered desktop application for automated mobile testing, built with **Rust** and **GPUI**.

## Vision

Mobie Studio empowers mobile-first QA and Engineers to run automated mobile tests with zero-effort setup. Instead of writing brittle interaction scripts, users **converse** with an autonomous agent that navigates the UI to achieve high-level goals. Conversations are **multi-turn and stateful** — the LLM retains full context across every prompt, so follow-up instructions build naturally on prior actions.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Frontend (GPUI)                                                │
│  Chat · Device status · LLM config · Session controls           │
│  (New / End / Resume)                                           │
└────────────────────────────┬────────────────────────────────────┘
                             │ mpsc channels
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  Agent Engine (Async Task Manager, dedicated Tokio thread)      │
│                                                                 │
│  ActiveSession { id, rig_history, step_history, yaml_path }     │
│                                                                 │
│  per turn:                                                      │
│  1. Resolve session  (reuse or mint sess_<timestamp>)           │
│  2. Auto-observe     (device.observe_ui()  →  XML)              │
│  3. LLM call         agent.prompt(combined)                     │
│                      .with_history(&mut history)                │
│                      .max_turns(50)                             │
│  4. Persist          (rig_history_json + chat_messages)         │
│  5. UI sync          (status bar, chat, YAML event)             │
└──────────┬───────────────────────────┬──────────────────────────┘
           │                           │
           ▼                           ▼
┌──────────────────────┐    ┌────────────────────────────────────┐
│  Device Bridge (ADB) │    │  LLM Client (rig-core + reqwest)   │
│  tap · swipe · input │    │  BYOK · OpenRouter headers         │
│  observe · screenshot│    │  native tool calling · with_history│
└──────────────────────┘    └────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────┐
│  Persistence (SQLite via rusqlite)                              │
│  sessions(id, goal, status, summary, yaml_path, rig_history_json)│
│  chat_messages(id, session_id, role, content, timestamp)       │
│                                                                 │
│  ~/mobie-results/<session_id>.yaml   (overwritten per turn)     │
│  ~/mobie-results/screenshots/<session_id>/                      │
└─────────────────────────────────────────────────────────────────┘
```

### The Agent Loop (per turn)

1. **Session Resolution** — Reuse the active session if one is open, otherwise mint `sess_<unix_timestamp>` and insert a row in `sessions`.
2. **Auto-Observe** — Call `device.observe_ui()` synchronously and prepend the compressed XML to the prompt as `[Auto-observed current screen state]\n<xml>`. Handles the "user manually changed apps between prompts" case.
3. **LLM Call** — `agent.prompt(combined).with_history(&mut history).max_turns(50)`. The agent mutates the `Vec<rig::completion::Message>` in place, appending the new turn + all tool calls + the final assistant reply.
4. **Persist** — Serialize the post-turn `history` to `sessions.rig_history_json`. Overwrite `~/mobie-results/<session_id>.yaml` with the latest `TestStep` list.
5. **UI Sync** — Push `AgentUpdate::SessionStateUpdate { turn_count, step_count, ... }` to the status bar.

The LLM is in full control of the loop via its tool calls. There is no implicit state machine — `max_turns(50)` gives the agent enough headroom for complex navigation.

### Multi-Turn Sessions

The engine is **not single-shot**. It holds a single `ActiveSession` in memory between `StartGoal` calls and persists it on every turn.

- **Explicit lifecycle:** the `+ New Session` button in the chat input is the only way to start fresh. `■ End Session` finalizes the YAML and closes the window. The session detail page has a `↩ Resume` button to switch back to a prior session.
- **Memory:** the full LLM conversation (`Vec<rig::completion::Message>`) is persisted to `sessions.rig_history_json` and replayed on every turn. Tool calls and tool results are included.
- **Stale-context safety:** every new prompt is preceded by a fresh `observe_ui()`, so the LLM always sees the current screen even if the user manually closed the app.
- **Stable YAML:** `tests/<session_id>.yaml` is rewritten after each successful turn, so the file always reflects the latest step list. No data loss on app crash mid-session.

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust 1.75+ |
| UI | [GPUI](https://gpui.rs) 0.2.2 |
| Async | Tokio (dedicated current-thread runtime for the agent engine) |
| HTTP | reqwest |
| Serialization | serde / serde_json / serde_yaml |
| Database | SQLite via `rusqlite` (bundled) |
| Device | ADB (via `std::process::Command`, mocked via `CommandRunner` trait) |
| LLM | Bring-Your-Own-Key (any OpenAI-compatible API) via `rig-core` 0.32 |

## Getting Started

### Prerequisites

- Rust toolchain (1.75+)
- Android device or emulator with ADB accessible
- An LLM API key (OpenAI, Anthropic, OpenRouter, etc.)

### Build & Run

```bash
cargo build
cargo run
```

### Tests

```bash
cargo test
```

53 unit + integration tests cover the session lifecycle, rig history round-trip, YAML export, and device bridge.

## Development

This project follows **Git Flow**:

```bash
# New feature
git checkout -b feature/my-feature develop

# Commits use Conventional Commits
git commit -m "feat: add device selector dropdown"
```

The agent architecture and design decisions are documented in [`AGENTS.md`](./AGENTS.md).

## Project Structure

```
src/
├── main.rs              # Entry point — GPUI Application setup, keybindings
├── lib.rs               # Module declarations
├── config.rs            # AppConfig (LLM BYOK) load/save
├── ui/mod.rs            # Frontend — MobieWorkspace view, chat, sidebar,
│                        #           session controls, status bar
├── agent/
│   ├── mod.rs           # Agent Engine — ActiveSession lifecycle,
│   │                    #              auto-observe, dispatcher, run_loop
│   ├── rig_agent.rs     # RigAgent — multi-turn think() using with_history
│   ├── tools.rs         # LLM-callable tools: Tap, Input, Swipe, KeyEvent,
│   │                    #                   Observe, Screenshot
│   └── action.rs        # Action enum (legacy/manual action model)
├── device/
│   ├── mod.rs           # DeviceBridge — async ADB interactor
│   └── xml_parser.rs    # UI dump XML compression
├── db/mod.rs            # SessionManager — SQLite schema + migrations,
│                        #                 rig_history_json persistence
├── llm/mod.rs           # LlmConfig + model list fetch
└── yaml_exporter.rs     # YAML TestCase serialization, export_to_path()
```

## License

MIT
