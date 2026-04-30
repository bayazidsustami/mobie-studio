# Implementation Plan: Clear All Chats

## Objective
Implement a "Clear All" button in the chat header that deletes all session history (DB + disk artifacts) and updates the UI to an empty state.

## Key Files & Context
- `src/ui/mod.rs`: Update header to include "Clear All" button and implement empty state view.
- `src/db/mod.rs`: Add a function to delete all sessions.
- `src/yaml_exporter.rs`: Add logic to clear all exported artifacts.
- `src/main.rs`: Handle the message/action for clearing all chats.

## Phases

### Phase 1: Backend Deletion Logic [checkpoint: 455eb3a]
- [x] Task: Implement `clear_all_sessions` in `src/db/mod.rs`
    - [x] Create unit test in `tests/chat_db_persistence.rs` for mass deletion.
    - [x] Implement SQL `DELETE FROM sessions` logic.
- [x] Task: Implement artifact cleanup in `src/yaml_exporter.rs`
    - [x] Create unit test for clearing the export directory.
    - [x] Implement logic to delete all `.yaml` files and `screenshots/` subdirectories.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Backend Deletion Logic' (Protocol in workflow.md)

### Phase 2: UI Implementation
- [x] Task: Add "Clear All" button to the Chat Header in `src/ui/mod.rs`
    - [x] Style the button according to GPUI conventions.
    - [x] Connect button to an asynchronous action.
- [x] Task: Implement Empty State View in `src/ui/mod.rs`
    - [x] Create a reusable `render_empty_state` function.
    - [x] Add "Start New Session" button to the empty state.
- [x] Task: Update Sidebar and View Refresh Logic
    - [x] Ensure the sidebar list clears immediately after the action.
    - [x] Handle transition from active session to empty state.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: UI Implementation' (Protocol in workflow.md)

### Phase 3: Integration & Final Polish
- [x] Task: End-to-end Integration Test
    - [x] Create a test that populates multiple sessions, triggers "Clear All", and verifies disk/DB state.
- [x] Task: Final UI/UX Polish
    - [x] Verify alignment and responsive behavior.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Integration & Final Polish' (Protocol in workflow.md)