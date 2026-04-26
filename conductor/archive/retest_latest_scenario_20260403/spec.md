# Retest Latest Scenario

## Objective
The application needs to support a "retest latest scenario" feature where the agent skips the LLM and replays the last generated YAML test case. This is crucial for fast, token-efficient testing of a deterministic sequence.

## Requirements
- The Agent Engine should maintain a list of actions executed in the current successful exploratory session.
- Once a session completes successfully, the history of actions must be exported as a YAML file via `yaml_exporter::export`.
- The GPUI application state must store the path to the most recent generated YAML test case.
- A new command/intent (e.g. `/retest` or "retest the latest scenario") must trigger the playback of the stored YAML file.
- During playback, the LLM reasoning is bypassed, and actions are routed directly to the Device Bridge.