# Mobie Studio

AI-powered desktop application for automated mobile testing, built with **Rust** and **GPUI**.

## Vision

Mobie Studio empowers mobile-first QA and Engineers to run automated mobile tests with zero-effort setup. Instead of writing brittle interaction scripts, users converse with an autonomous agent that navigates the UI to achieve high-level goals.

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Frontend (GPUI)                                │
│  Chat interface · Device status · LLM config    │
├─────────────────────────────────────────────────┤
│  Agent Engine (Async Task Manager)              │
│  Observe → Think → Act loop via mpsc channels   │
├──────────────────┬──────────────────────────────┤
│  Device Bridge   │  LLM Client (BYOK)           │
│  ADB interactor  │  HTTP → user's LLM provider  │
└──────────────────┴──────────────────────────────┘
```

### The Agent Loop

1. **Observe** — Pull XML UI dump via ADB
2. **Think** — Send compressed XML + user goal to LLM
3. **Act** — Execute LLM's decided action via ADB input commands
4. **Verify** — Repeat; the agent handles timing by observing subsequent dumps

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust 1.75+ |
| UI | [GPUI](https://gpui.rs) 0.2.2 |
| Async | Tokio + GPUI background executor |
| HTTP | reqwest |
| Serialization | serde / serde_json / serde_yaml |
| Device | ADB (via `std::process::Command`) |
| LLM | Bring-Your-Own-Key (any OpenAI-compatible API) |

## Getting Started

### Prerequisites

- Rust toolchain (1.75+)
- Android device or emulator with ADB accessible
- An LLM API key (OpenAI, Anthropic, etc.)

### Build & Run

```bash
cargo build
cargo run
```

## Development

This project follows **Git Flow**:

```bash
# New feature
git checkout -b feature/my-feature develop

# Commits use Conventional Commits
git commit -m "feat: add device selector dropdown"
```

## Project Structure

```
src/
├── main.rs          # Entry point — GPUI Application setup
├── ui/mod.rs        # Frontend — MobieWorkspace view, chat, sidebar
├── agent/mod.rs     # Agent Engine — async Observe→Think→Act loop
├── device/mod.rs    # Device Bridge — ADB device interaction
└── llm/mod.rs       # LLM Client — BYOK HTTP client
```

## License

MIT
