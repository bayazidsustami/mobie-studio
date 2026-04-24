# Implementation Plan: Test Case Explorer

## Objective
Enable users to visualize the exact sequence of actions and screenshots from past exploratory runs by reading the generated YAML test cases.

## Key Files & Context
- `src/ui/mod.rs`: Update `MobieWorkspace` and rendering logic.
- `src/yaml_exporter.rs`: Use existing data structures (`TestCase`, `TestStep`).

## Implementation Steps

### Phase 1: Data Loading
- [x] Task: Update `MobieWorkspace` to hold an optional `TestCase` for the currently selected session.
- [x] Task: Implement logic to load and parse the YAML file when `selected_session` changes.

### Phase 2: UI Rendering (Steps)
- [x] Task: Create `render_execution_timeline` to iterate over `TestCase.steps`.
- [x] Task: Design and implement a "Step Card" component that displays action, reasoning, and parameters.

### Phase 3: UI Rendering (Screenshots)
- [x] Task: Implement logic to resolve the local filesystem path for screenshots based on the YAML filename and step index.
- [x] Task: Use GPUI's image rendering capabilities to display screenshots inline with each step.

## Verification & Testing
- **Manual Verification:** Open a past session with screenshots and verify that all steps and images are correctly displayed.
- **Error Handling:** Ensure the UI doesn't crash if a YAML file or screenshot directory is missing/moved.