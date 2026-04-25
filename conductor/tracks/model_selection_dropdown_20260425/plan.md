# Implementation Plan: Model Selection Dropdown

## Phase 1: API Integration for Models [checkpoint: ee2fc5e]
- [x] Task: Define structs for the provider's `/models` API response. f6a439e
- [x] Task: Implement an HTTP request to fetch models from the configured provider (e.g., OpenRouter) and cache them in the application state. 3a1a47a
- [x] Task: Conductor - User Manual Verification 'Phase 1: API Integration for Models' (Protocol in workflow.md)

## Phase 2: Dropdown UI Implementation
- [x] Task: Replace the text input field for the model name in the Settings Panel (`src/ui/mod.rs` or similar) with a GPUI dropdown/select component. ba6e5a5
- [x] Task: Bind the dropdown options to the cached model list fetched in Phase 1. ba6e5a5
- [x] Task: Handle selection changes to update the application configuration. ba6e5a5
- [~] Task: Conductor - User Manual Verification 'Phase 2: Dropdown UI Implementation' (Protocol in workflow.md)

## Phase 3: Code Review & Finalization
- [ ] Task: Request `@rust-reviewer` to review the changes as requested.
- [ ] Task: Apply feedback from the reviewer.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Code Review & Finalization' (Protocol in workflow.md)