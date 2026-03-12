# Implementation Plan: Improve Testing Agent

## Phase 1: Observation & History Enhancement
- [ ] Task: Refine XML compression and element filtering logic in `src/device/xml_parser.rs`.
    - [ ] Write Tests for enhanced compression
    - [ ] Implement Feature
- [ ] Task: Implement `SessionHistory` tracker in `src/agent/mod.rs` to store recent actions.
    - [ ] Write Tests for history tracking and loop detection
    - [ ] Implement Feature
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Observation & History Enhancement' (Protocol in workflow.md)

## Phase 2: Planning & Reasoning Logic
- [ ] Task: Update system prompt in `src/llm/prompt.rs` to support planning and history context.
    - [ ] Write Tests for prompt template with history and plan context
    - [ ] Implement Feature
- [ ] Task: Refactor `AgentEngine` loop in `src/agent/mod.rs` to handle multi-step planning and explicit verification cycles.
    - [ ] Write Tests for planning and verification state machine
    - [ ] Implement Feature
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Planning & Reasoning Logic' (Protocol in workflow.md)

## Phase 3: Robust Interaction Patterns
- [ ] Task: Implement logic to detect and wait for dynamic loading states (spinners, etc.) in `AgentEngine`.
    - [ ] Write Tests for dynamic wait logic
    - [ ] Implement Feature
- [ ] Task: Improve element targeting and scroll logic for long lists in `src/device/mod.rs`.
    - [ ] Write Tests for list navigation
    - [ ] Implement Feature
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Robust Interaction patterns' (Protocol in workflow.md)