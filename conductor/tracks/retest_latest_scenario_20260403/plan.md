# Implementation Plan: Retest Latest Scenario (YAML Replay)

## Objective
Enable users to "retest the latest scenario" by leveraging the auto-generated YAML test cases, bypassing the LLM agent loop for significantly faster and token-efficient execution.

## Tasks

- [x] Task 1: Track Agent Actions (Test Steps) b0d6f4f
  - Modify `src/agent/rig_agent.rs` to maintain an `Arc<Mutex<Vec<TestStep>>>` per session.
  - Pass this shared state to each tool implementation in `src/agent/tools.rs`.
  - Inside each tool's `call` method, append a new `TestStep` containing the action name, parameters, and the agent's reasoning to the shared history.
- [x] Task 2: Generate YAML on Successful Run 11c49fa
  - In `src/agent/mod.rs` (inside `run_loop`), after a successful `rig_agent.think(&goal)` invocation:
    - Retrieve the recorded `TestStep`s.
    - Construct a `TestCase` using the original goal and the steps.
    - Call `yaml_exporter::export(&test_case)`.
  - Introduce a new `AgentUpdate::TestGenerated(PathBuf)` and send it back to the UI upon successful export.
- [x] Task 3: Update UI to Store Latest Test 11c49fa
  - In `src/ui/mod.rs`, add a `latest_test: Option<PathBuf>` field to the main application state.
  - When `AgentUpdate::TestGenerated(path)` is received, update `latest_test` and optionally show a visual indicator in the chat.
- [ ] Task 4: Implement the "Retest" Command
  - Add `AgentMessage::RetestScenario(PathBuf)` to the engine messages.
  - In `src/ui/mod.rs`'s chat input handler, parse intents like `/retest`. If detected, ensure `latest_test` is `Some`, and dispatch `AgentMessage::RetestScenario`.
  - In `src/agent/mod.rs`'s `run_loop`, handle `RetestScenario(path)`:
    - Parse the YAML file using `serde_yaml` back into a `TestCase`.
    - Provide status updates (`AgentStatus::Acting`) back to the UI.
    - Sequentially execute the mapped device actions (Tap, Input, Swipe) directly via `device` methods.