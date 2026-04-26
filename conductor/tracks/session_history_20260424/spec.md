# Specification: Session History

## 1. Overview
The "Session History" feature will allow users to revisit past exploratory runs executed by the AI testing agent. This will provide an easy way to view the conversation history, the agent's reasoning, and the final generated YAML test script, improving the reusability and traceability of test cases.

## 2. Requirements

### 2.1 Backend (Data Storage)
- Introduce `rusqlite` as an embedded SQLite database to persist session metadata.
- A "Session" entity should include at minimum: `id`, `timestamp`, `user_goal`, `status` (success, failure, cancelled), and the paths/content for the associated `chat_log` and `yaml_test_case`.
- A database manager (`SessionManager`) should handle schema creation, insertion, and retrieval of past sessions.

### 2.2 Agent Integration
- The `AgentEngine` must hook into the start and end of a test run.
- Upon completion (or failure), the agent must persist the full chat history and generated YAML test script, passing the relevant metadata to the `SessionManager` for saving.

### 2.3 Frontend (UI/GPUI)
- Add a new "History" or "Sessions" tab/sidebar to the main GPUI window.
- The UI should display a list of past sessions sorted by date/time (most recent first).
- Clicking a session in the list should display its details in the main view, specifically showing the full chat log of that run and the final generated YAML test case.

## 3. Scope & Constraints
- **Scope**: The scope is strictly limited to viewing past runs and their outputs. We are not implementing a "resume session" feature at this stage. 
- **Dependencies**: The primary new dependency will be `rusqlite`. The rest of the implementation will rely on the existing GPUI frontend and Tokio asynchronous runtime.