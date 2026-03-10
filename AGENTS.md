# Mobie Studio: Agent Vision & Architecture

## Vision
Mobie Studio is a lightweight, AI-powered desktop application built with Rust and GPUI. It empowers mobile-first QA and Engineers to run automated mobile tests with zero-effort setup. Instead of writing brittle interaction scripts, users converse with an autonomous agent that navigates the UI to achieve high-level goals. 

## Key Terms
- **Agent:** The autonomous, LLM-driven entity that interprets user goals, observes the mobile UI, and decides on the next sequence of actions.
- **UI Dump:** The XML representation of the current screen hierarchy extracted from the device (via Android's `uiautomator`). This serves as the agent's "eyes".
- **Action:** A specific, atomic interaction sent to the device (e.g., `tap`, `swipe`, `input text`) based on the agent's decision.
- **Implicit Wait & Retry:** The core execution loop where the agent acts, observes the resulting state, and autonomously decides whether to retry an action or proceed, mimicking human interaction.
- **Exploratory Run:** A conversational session where the user provides a goal and the agent figures out the steps dynamically.
- **YAML Test Case:** A declarative file generated after a successful exploratory run, documenting the exact steps and assertions for future, repeatable CI/CD execution.

## Architecture & Implementation Standards
The application operates within a single process, utilizing asynchronous Rust tasks to ensure the UI remains responsive during agent operations. Implementation will strictly adhere to the following skill guidelines:

- **Rust Architecture & Quality:** `@systems-programming-rust-project` and `@rust-pro` for robust project structure, modern Rust (1.75+) features, and production-ready systems programming.
- **Concurrency:** `@rust-async-patterns` for handling the asynchronous Agent Engine, Tokio task management, and safe state sharing across threads.
- **Frontend Design:** `@ui-skills` for building a clean, responsive, and opinionated GPUI interface.

### Core Modules
1. **Frontend (GPUI):** Manages the chat interface, LLM provider settings (BYOK model), device selection, and displays the generated YAML outputs.
2. **Agent Engine (Async Task Manager):** Handles the state machine for the active session, communicating progress back to the UI via asynchronous channels (`mpsc`).
3. **Device Bridge (ADB Interactor):** A dedicated module executing `std::process::Command` calls to local `adb` binaries for device discovery, UI dumping, and action execution.
4. **LLM Client:** Formats prompts, compresses the XML UI dump, and parses JSON responses from the user's configured LLM provider.

## The Agent Loop
When a user submits a goal, the Agent Engine executes the following loop until success or failure:
1. **Observe:** Pull the XML UI dump via ADB.
2. **Think:** Send the compressed XML and the user's goal to the LLM to determine the next logical action.
3. **Act:** Execute the LLM's decided action via ADB input commands.
4. **Verify:** Repeat the loop. The agent natively handles timing by observing if the expected state was reached in subsequent dumps.

## Decision Log
- **Architecture:** Monolithic GPUI + Rust desktop app.
- **Agent Loop:** Implicit Wait & Retry.
- **LLM Strategy:** Bring-Your-Own-Key (BYOK).
- **Test Output:** Auto-generated declarative YAML test cases after successful exploratory runs.

## Development Workflow
We implement and strictly follow **Git Flow** for managing changes to the project. 
- **New Features & Additions:** Any new feature, enhancement, or change must start with a new branch created from the main development branch (typically `develop`).
  - Example: `git checkout -b feature/my-new-feature develop`
- **Commits:** Follow the Conventional Commits specification for all commit messages.
- **Merging:** Features are developed in isolation and merged back into the development branch via pull requests once complete and validated.