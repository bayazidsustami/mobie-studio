# Mobie Studio: Agent Vision & Architecture

## Vision
Mobie Studio is a lightweight, AI-powered desktop application built with Rust and GPUI. It empowers mobile-first QA and Engineers to run automated mobile tests with zero-effort setup. Instead of writing brittle interaction scripts, users converse with an autonomous agent that navigates the UI to achieve high-level goals. 

## Key Terms
- **Agent:** The autonomous, LLM-driven entity that interprets user goals, observes the mobile UI, and decides on the next sequence of actions using the `rig-core` framework.
- **UI Dump:** The XML representation of the current screen hierarchy extracted from the device (via Android's `uiautomator`). This serves as the agent's "eyes".
- **Action / Tool:** A specific, atomic interaction sent to the device (e.g., `Tap`, `Swipe`, `Input`, `KeyEvent`, `Observe`) executed as a native tool by the LLM.
- **Implicit Wait & Retry:** The execution loop where the agent acts, observes the resulting state, and autonomously decides whether to retry an action or proceed.
- **Session Memory:** The agent's contextual awareness of recent actions to prevent repetitive failure loops.
- **Exploratory Run:** A conversational session where the user provides a goal and the agent figures out the steps dynamically via multi-step planning (Plan -> Execute).
- **YAML Test Case:** A declarative file generated after a successful exploratory run, documenting the exact steps and assertions for future, repeatable CI/CD execution.

## Architecture & Implementation Standards
The application operates within a single process, utilizing asynchronous Rust tasks to ensure the UI remains responsive during agent operations. Implementation will strictly adhere to the following skill guidelines:

- **Rust Architecture & Quality:** `@systems-programming-rust-project` and `@rust-pro` for robust project structure, modern Rust (1.75+) features, and production-ready systems programming.
- **Concurrency:** `@rust-async-patterns` for handling the asynchronous Agent Engine, Tokio task management, and safe state sharing across threads.
- **Frontend Design:** `@ui-skills` for building a clean, responsive, and opinionated GPUI interface.

### Core Modules
1. **Frontend (GPUI):** Manages the chat interface, LLM provider settings (BYOK model), device selection, and displays the generated YAML outputs.
2. **Agent Engine (Async Task Manager):** Handles the state machine for the active session and multi-step planning, communicating progress back to the UI via asynchronous channels (`mpsc`).
3. **Device Bridge (ADB Interactor):** A dedicated module executing `std::process::Command` calls to local `adb` binaries for device discovery, UI dumping, and action execution.
4. **LLM Client (`rig-core`):** Utilizes `reqwest` to interact with LLMs, injecting custom headers (e.g., `HTTP-Referer`, `X-Title`) for OpenRouter BYOK compatibility. It utilizes the `rig-core` AI framework to bind Rust structs as native tools that the agent can autonomously invoke, replacing manual JSON action parsing.

## The Agent Loop
Instead of a rigid hardcoded loop, the Agent Engine uses an autonomous, tool-driven loop managed natively by the `rig-core` framework. When a user submits a goal:
1. **Multi-step Planning:** The agent can decompose the high-level goal into logical sub-goals.
2. **Tool Execution:** The agent natively decides to use tools like `Observe`, `Tap`, `Input`, or `Swipe` to fulfill the active sub-goal.
3. **Iterative Verification:** The agent is configured to allow up to 20 maximum iterations (`max_turns(20)`) to act, evaluate the changed UI state via the `Observe` tool, and self-correct using session memory if it encounters an error or a repetitive loop.

## Decision Log
- **Architecture:** Monolithic GPUI + Rust desktop app.
- **AI Framework:** `rig-core` for LLM provider abstraction and native tool calling.
- **Agent Loop:** Agent-driven tool invocation (`max_turns` iteration) with implicit wait, multi-step planning, and session memory.
- **LLM Strategy:** Bring-Your-Own-Key (BYOK) with OpenRouter support via custom HTTP headers.
- **Test Output:** Auto-generated declarative YAML test cases after successful exploratory runs.

## Development Workflow
We implement and strictly follow **Git Flow** for managing changes to the project. 
- **New Features & Additions:** Any new feature, enhancement, or change must start with a new branch created from the main development branch (typically `develop`).
  - Example: `git checkout -b feature/my-new-feature develop`
- **Commits:** Follow the Conventional Commits specification for all commit messages.
- **Merging:** Features are developed in isolation and merged back into the development branch via pull requests once complete and validated.