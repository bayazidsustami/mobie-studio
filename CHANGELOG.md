# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-03

### Added
- **Core Agent Engine**: Implemented the asynchronous Observe-Think-Act loop for autonomous mobile testing.
- **UI Framework**: Integrated [GPUI](https://gpui.rs) 0.2.2 for a high-performance desktop interface.
- **Chat Interface**: New chat-based workspace for interacting with the AI agent.
- **Device Bridge**: ADB integration for interacting with Android devices and emulators.
- **LLM Client**: Bring-Your-Own-Key (BYOK) support for OpenAI-compatible APIs via `reqwest`.
- **Project Infrastructure**: Initial setup with `Cargo.toml`, `README.md`, and MIT license.

### Changed
- Migrated to GPUI 0.2.2 API for improved stability and features.
- Wired the agent loop with the UI for real-time interaction.
