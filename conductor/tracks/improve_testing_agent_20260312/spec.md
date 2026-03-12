# Specification: Improve Testing Agent

## Overview
Enhance the Mobie Studio agent's core capabilities to increase its task success rate on complex mobile applications. This track focuses on making the Observe-Think-Act loop more intelligent, resilient, and aware of its own execution history.

## Functional Requirements

### 1. Enhanced Observation & Filtering
- Refine `src/device/xml_parser.rs` to include more context for relevant elements (e.g., proximity to other elements, parent container hints) while maintaining aggressive compression.
- Filter out non-interactive layout containers that do not provide semantic value to the LLM.

### 2. Sophisticated Reasoning (Planning & Memory)
- **Multi-step Planning:** Update the system prompt and agent engine to support a two-stage reasoning process:
    1. **Plan:** Decompose the high-level goal into a sequence of logical sub-goals.
    2. **Execute:** Target the next sub-goal in the current plan.
- **Session Memory:** Introduce a `SessionHistory` tracker in the `AgentEngine` to store the last 5 actions and their outcomes. Use this context in the prompt to prevent repetitive loops.

### 3. Resilient Action Execution & Verification
- **Explicit Verification:** After executing an action, the agent must perform a mandatory "verification" observation to confirm the UI state has transitioned as expected.
- **Dynamic Loading Detection:** Implement logic to detect and wait for common loading patterns (e.g., progress bars, specific resource IDs associated with spinners) before "Thinking".

### 4. Advanced Interaction Patterns
- **Reliable List Navigation:** Improve the `swipe` logic and element targeting to handle elements that are partially visible or require multiple scrolls to reach.

## Acceptance Criteria
- **Task Success Rate:** The agent must successfully complete 90% of a benchmark set of complex app interactions (e.g., multi-page forms, nested navigation).
- **Loop Prevention:** The agent must detect and self-correct when it attempts the same failed action more than twice in a row.
- **Verifiable Reasoning:** Every action in the chat log must include a "Plan Context" showing which sub-goal is being pursued.

## Out of Scope
- Integration of Vision-Language Models (VLM) for visual validation.
- Supporting iOS devices via a new device bridge.