# Implementation Plan: Session History

## Objective
Implement a "Session History" feature that uses an SQLite database to store and display past exploratory test runs, including their chat logs and generated YAML test cases, providing better traceability and reusability for users.

## Key Files & Context
- `Cargo.toml`: Needs dependency addition (`rusqlite`).
- `src/db/mod.rs` (New): Will house the `SessionManager` and SQLite database logic.
- `src/agent/mod.rs`: The `AgentEngine` loop needs to be updated to record the start, state, and end of a session.
- `src/ui/mod.rs` (and submodules): GPUI components for displaying the session list and detail views.

## Implementation Steps

### Phase 1: Storage Layer (SQLite via `rusqlite`)
- [x] Task: Add `rusqlite` dependency to `Cargo.toml`.
- [x] Task: Create a new module `src/db/mod.rs` with a `SessionManager` struct.
- [x] Task: Implement schema creation for a `sessions` table (e.g., `id`, `timestamp`, `goal`, `status`, `chat_log_path`, `yaml_path`).
- [x] Task: Implement functions to `insert_session` and `get_all_sessions`.
- [x] Task: Write basic unit tests for the database module.

### Phase 2: Agent Integration
- [x] Task: Modify `AgentEngine` in `src/agent/mod.rs` to generate a unique session ID upon starting an exploratory run.
- [x] Task: Capture the full chat interaction sequence (prompts, tool calls, results).
- [x] Task: On session completion (success/failure), save the chat log to disk and write the session metadata to the SQLite database via `SessionManager`.

### Phase 3: GPUI Frontend Integration
- [x] Task: Add a "Sessions" sidebar/list view to the main GPUI interface.
- [x] Task: Wire the sidebar to query and display the list of past sessions from the `SessionManager`.
- [x] Task: Create a "Session Detail" view that renders when a user clicks a session from the list, displaying the chat history and the generated YAML script side-by-side or stacked.
- [x] Task: Ensure the UI remains responsive while querying the database.

## Verification & Testing
- **Unit Tests:** The `SessionManager` should be thoroughly tested for CRUD operations.
- **Integration Test:** A simulated agent run should successfully write a record to the database, which is then retrievable.
- **UI Test:** The GPUI view should correctly populate the sessions list when launched, and selecting a session should visually render the correct chat log and YAML data.