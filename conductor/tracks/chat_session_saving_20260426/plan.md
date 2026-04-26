# Implementation Plan: Chat Session Saving and Reloading

## Phase 1: Database Schema and Persistence [checkpoint: ]
- [x] Task: Update SQLite schema to include `chat_messages` table and migration in `src/db/mod.rs`. [8469d7e]
    - [x] Write failing test for `chat_messages` schema creation and insertion.
    - [x] Implement `create_chat_messages_table` and `insert_chat_message` to pass the test.
- [x] Task: Implement logic to save chat messages during an active session in `src/agent/mod.rs`. [8469d7e]
    - [x] Write failing test for intercepting and saving messages.
    - [x] Modify `AgentEngine` to call `insert_chat_message` on each user prompt and agent response.
- [x] Task: Update session deletion logic in `src/db/mod.rs` to cascade and delete associated `chat_messages`. [8469d7e]
    - [x] Write failing test for cascading deletion.
    - [x] Implement the cascading delete logic.
- [~] Task: Conductor - User Manual Verification 'Phase 1: Database Schema and Persistence' (Protocol in workflow.md)

## Phase 2: Session Metadata and AI Summary [checkpoint: ]
- [ ] Task: Implement AI summary generation for sessions in `src/agent/mod.rs` or a new module.
    - [ ] Write failing test for the summary prompt and logic.
    - [ ] Add an LLM call at the end of a session to generate a concise summary.
- [ ] Task: Update the session title logic in `src/ui/mod.rs` to display Date & Time, Initial Goal, and the AI Summary.
    - [ ] Write failing test/mock test for UI title generation formatting.
    - [ ] Modify the UI components to construct and render this composite title.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Session Metadata and AI Summary' (Protocol in workflow.md)

## Phase 3: Reload Session UI and Logic [checkpoint: ]
- [ ] Task: Add "Reload Session" button to the session detail view in `src/ui/mod.rs`.
    - [ ] Write a test to verify the button appears when a session is selected.
    - [ ] Implement the UI button and tie its click handler to a new `ReloadSession` event.
- [ ] Task: Implement the session reloading state transition in `src/ui/mod.rs` and `src/agent/mod.rs`.
    - [ ] Write a test verifying that `ReloadSession` fetches messages and replaces the active state.
    - [ ] Implement the logic to fetch `chat_messages` for the `session_id`, clear current chat state, and populate it with the fetched messages.
    - [ ] Ensure the application redirects the user to the main chat interface upon successful reload.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Reload Session UI and Logic' (Protocol in workflow.md)