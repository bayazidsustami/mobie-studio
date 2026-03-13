# Implementation Plan: Core Agent Migration to Rig

## Objective
Migrate the existing custom agent loop to the **Rig** framework, using a Rig-native architecture for autonomous testing with OpenRouter as the primary LLM provider.

## Key Files & Context
- **Current Agent:** `src/agent/mod.rs`, `src/agent/action.rs`
- **Current LLM:** `src/llm/mod.rs`, `src/llm/prompt.rs`
- **Tech Stack:** `conductor/tech-stack.md` (to be updated)
- **Workflow:** `conductor/workflow.md` (TDD required)

## Phase 1: Environment Setup & Rig Foundation [checkpoint: 74e082e]
- [x] Task: Add Rig dependencies to `Cargo.toml` (1a3cf6a)
    - [x] Add `rig-core`, `rig-openai` (or relevant) and any needed async traits.
- [x] Task: Update `tech-stack.md` with Rig details (db24278)
    - [x] Document the adoption of Rig and the removal of custom LLM wrappers.
- [x] Task: Create Rig Integration Test Skeleton (9dd3a97)
    - [x] Write a failing test in `tests/rig_init.rs` that attempts to initialize a Rig agent.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Environment Setup & Rig Foundation' (Protocol in workflow.md) (74e082e)

## Phase 2: Core Agent Re-architecture [checkpoint: c85798c]
- [x] Task: Implement `RigAgent` Structure (319c4e9)
    - [x] Define the new agent structure in `src/agent/rig_agent.rs`.
    - [x] Write unit tests for agent initialization.
- [x] Task: Implement Rig-Native Loop (3d7c11d)
    - [x] Use Rig's `Agent` abstraction to handle the "Think" phase.
    - [x] Write tests to verify the agent can process a simple string prompt via Rig.
- [x] Task: Refactor `src/agent/mod.rs` to use `RigAgent` (0559d5e)
    - [x] Replace the old loop with the new implementation.
    - [x] Ensure existing `Observe` and `Act` calls can be triggered.
- [x] Task: Conductor - User Manual Verification 'Phase 2: Core Agent Re-architecture' (Protocol in workflow.md) (c85798c)

## Phase 3: Tooling & Action Mapping [checkpoint: b8e384c]
- [x] Task: Map ADB Actions to Rig Tools (99ef8d1)
    - [x] Implement Rig's `Tool` trait for existing actions (click, type, etc.).
    - [x] Write failing tests for tool execution via Rig agent.
- [x] Task: Map XML Observation to Rig Tools (5cf1c94)
    - [x] Wrap the `xml_parser` logic into a Rig tool for the agent to "Observe".
    - [x] Verify the agent can use the "Observe" tool to see the screen state.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Tooling & Action Mapping' (Protocol in workflow.md) (b8e384c)

## Phase 4: OpenRouter Integration & Final Validation
- [ ] Task: Configure OpenRouter Provider via Rig
    - [ ] Implement or configure an OpenAI-compatible client for OpenRouter.
    - [ ] Write tests verifying connectivity and completion.
- [ ] Task: Update and Run Full Integration Suite
    - [ ] Run `tests/agent_history.rs` and other existing tests.
    - [ ] Fix any regressions caused by the migration.
- [ ] Task: Cleanup Legacy Code
    - [ ] Remove redundant logic in `src/llm/` and old agent files.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: OpenRouter Integration & Final Validation' (Protocol in workflow.md)
