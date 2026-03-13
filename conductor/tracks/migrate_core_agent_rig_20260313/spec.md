# Track Specification: Core Agent Migration to Rig (Rust)

## Overview
Migrate the existing custom agent implementation to use the **Rig** Rust agent framework. This change focuses on adopting a more standard, robust, and extensible architecture for autonomous testing while maintaining compatibility with the current Mobie Studio vision.

## Goal
Replace the custom "Observe -> Think -> Act" loop and LLM integration logic with a **Rig-native** implementation using its `Agent` and `Completion` abstractions.

## Functional Requirements
- **Rig Integration:** Incorporate the `rig-core` and relevant provider crates (e.g., `rig-openai` or custom for OpenRouter) into the project.
- **Agent Re-architecture:** Re-implement the core agent loop using Rig's native patterns, moving away from the existing manual state management.
- **Tooling Abstraction:** Map existing device interactions (ADB, XML parsing) to Rig's `Tool` trait for seamless LLM execution.
- **LLM Provider:** Configure and validate **OpenRouter** as the primary LLM interface via Rig's OpenAI-compatible client.
- **Direct Replacement:** Refactor `src/agent/` and `src/llm/` to use Rig, removing redundant custom logic.

## Non-Functional Requirements
- **Rust Standards:** Follow idiomatic Rust patterns (using `@rust-pro` guidance where applicable) and maintain compatibility with Rust 1.75+.
- **Performance:** Ensure the new agent loop is at least as responsive as the current implementation.
- **Maintainability:** Use Rig's abstractions to simplify the addition of future tools and capabilities.

## Acceptance Criteria
1.  The core agent successfully initializes and executes a test goal using Rig's framework.
2.  The agent can call tools (e.g., ADB commands, UI observation) via Rig's tool-calling mechanism.
3.  Successful communication with OpenRouter is established and verified.
4.  Existing agent tests in `tests/` are updated or pass with the new implementation.

## Out of Scope
- Implementing Vector Stores or RAG capabilities in this phase.
- Multi-agent orchestration or complex workflows.
- Significant changes to the GPUI-based desktop interface.
