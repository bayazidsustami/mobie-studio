# Implementation Plan: Model Selection Dropdown

## Phase 1: API Integration for Models
- [ ] Task: Define structs for the provider's `/models` API response.
- [ ] Task: Implement an HTTP request to fetch models from the configured provider (e.g., OpenRouter) and cache them in the application state.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: API Integration for Models' (Protocol in workflow.md)

## Phase 2: Dropdown UI Implementation
- [ ] Task: Replace the text input field for the model name in the Settings Panel (`src/ui/mod.rs` or similar) with a GPUI dropdown/select component.
- [ ] Task: Bind the dropdown options to the cached model list fetched in Phase 1.
- [ ] Task: Handle selection changes to update the application configuration.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Dropdown UI Implementation' (Protocol in workflow.md)

## Phase 3: Code Review & Finalization
- [ ] Task: Request `@rust-reviewer` to review the changes as requested.
- [ ] Task: Apply feedback from the reviewer.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Code Review & Finalization' (Protocol in workflow.md)