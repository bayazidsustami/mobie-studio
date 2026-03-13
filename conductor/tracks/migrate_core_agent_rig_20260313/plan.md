# Implementation Plan: Core Agent Migration to Rig

## Objective
Migrate the existing custom agent loop to the **Rig** framework, using a Rig-native architecture for autonomous testing with OpenRouter as the primary LLM provider.

## Key Files & Context
- **Current Agent:** `src/agent/mod.rs`, `src/agent/action.rs`
- **Current LLM:** `src/llm/mod.rs`, `src/llm/prompt.rs`
- **Tech Stack:** `conductor/tech-stack.md` (to be updated)
- **Workflow:** `conductor/workflow.md` (TDD required)

## Phase 1: Environment Setup & Rig Foundation
- [ ] Task: Add Rig dependencies to `Cargo.toml`
    - [ ] Add `rig-core`, `rig-openai` (or relevant) and any needed async traits.
- [ ] Task: Update `tech-stack.md` with Rig details
    - [ ] Document the adoption of Rig and the removal of custom LLM wrappers.
- [ ] Task: Create Rig Integration Test Skeleton
    - [ ] Write a failing test in `tests/rig_init.rs` that attempts to initialize a Rig agent.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Environment Setup & Rig Foundation' (Protocol in workflow.md)

## Phase 2: Core Agent Re-architecture
- [ ] Task: Implement `RigAgent` Structure
    - [ ] Define the new agent structure in `src/agent/rig_agent.rs`.
    - [ ] Write unit tests for agent initialization.
- [ ] Task: Implement Rig-Native Loop
    - [ ] Use Rig's `Agent` abstraction to handle the "Think" phase.
    - [ ] Write tests to verify the agent can process a simple string prompt via Rig.
- [ ] Task: Refactor `src/agent/mod.rs` to use `RigAgent`
    - [ ] Replace the old loop with the new implementation.
    - [ ] Ensure existing `Observe` and `Act` calls can be triggered.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Core Agent Re-architecture' (Protocol in workflow.md)

## Phase 3: Tooling & Action Mapping
- [ ] Task: Map ADB Actions to Rig Tools
    - [ ] Implement Rig's `Tool` trait for existing actions (click, type, etc.).
    - [ ] Write failing tests for tool execution via Rig agent.
- [ ] Task: Map XML Observation to Rig Tools
    - [ ] Wrap the `xml_parser` logic into a Rig tool for the agent to "Observe".
    - [ ] Verify the agent can use the "Observe" tool to see the screen state.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Tooling & Action Mapping' (Protocol in workflow.md)

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
