# Update AGENTS.md

## Objective
Update the `AGENTS.md` documentation to accurately reflect recent architectural changes, particularly the integration of the `rig-core` AI framework, the new agent execution loop, advanced reasoning capabilities, and updated LLM client configuration.

## Scope & Impact
- Target File: `AGENTS.md`
- This is a documentation-only update to ensure the project vision and architecture guide aligns with the current `develop` codebase.

## Implementation Steps

### 1. Update Core Modules Section [x] (efd7194)
- Modify the `LLM Client` description to state that it utilizes the `rig-core` framework for native tool calling instead of manual JSON parsing.
- Mention the inclusion of custom HTTP headers (e.g., `HTTP-Referer`, `X-Title`) via `reqwest` to support OpenRouter BYOK functionality.

### 2. Rewrite "The Agent Loop" [x] (f22d22b)
- Replace the rigid 4-step manual loop (Observe -> Think -> Act -> Verify) with a description of the agent-driven loop powered by `rig-core`.
- Detail how the agent autonomously invokes tools (`Tap`, `Input`, `Swipe`, `Observe`, `KeyEvent`).
- Explain the implementation of advanced reasoning patterns: Multi-step Planning (Plan -> Execute) and Session Memory tracking for loop prevention.
- Note the configuration allowing for extended iterations (e.g., up to 20 turns) to reach the goal.

### 3. Update Decision Log [x] (ad80763)
- Add an entry reflecting the decision to use the `rig-core` framework for managing LLM interactions and native tool support.

## Verification
- Review the updated `AGENTS.md` to ensure all sections are coherent and accurately represent the current state of `src/agent/rig_agent.rs` and the broader project architecture.