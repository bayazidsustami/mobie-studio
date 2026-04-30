# Implementation Plan: Increase Agent Max Turn Limit

## Objective
Increase the Rig agent's `max_turns` limit from 20 to 50 to prevent `MaxTurnError` during complex, multi-step exploratory runs in the mobile testing environment.

## Key Files & Context
- `src/agent/rig_agent.rs`: Contains the agent initialization and the hardcoded `max_turns(20)` limit.

## Implementation Steps
- [x] Change the hardcoded `max_turns(20)` to `max_turns(50)` in the `RigAgent::think` method within `src/agent/rig_agent.rs`.

## Verification & Testing
- [x] Compile the project to ensure there are no syntax errors (`cargo build`).
- [ ] Verify manually that the agent can now perform more than 20 actions in a single session without erroring out.
