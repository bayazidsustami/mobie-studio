# Mobie Studio: Tech Stack

## Architecture
- **Type:** Desktop Application / Monolith
- **Pattern:** Agent Loop (Observe -> Think -> Act)

## Core Technologies
- **Language:** Rust 1.75+
- **UI Framework:** GPUI 0.2.2

## Backend & Concurrency
- **Async Runtime:** Tokio
- **Concurrency Management:** Asynchronous mpsc channels for UI-Agent communication

## Integrations & APIs
- **HTTP Client:** reqwest
- **Device Interaction:** Local ADB (`std::process::Command` calls)
- **LLM Framework:** Rig Rust (`rig-core`)
- **LLM Provider:** OpenRouter via OpenAI-compatible HTTP API

## Data & Serialization
- **Serialization Formats:** serde (JSON, YAML, TOML)
- **Database:** SQLite (via `rusqlite`) for local persistence of session history

## Observability & Error Handling
- **Logging:** tracing, tracing-subscriber
- **Error Handling:** anyhow