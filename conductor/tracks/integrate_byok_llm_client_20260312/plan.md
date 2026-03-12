# Implementation Plan: Integrate BYOK LLM Client

## Phase 1: Client Setup and Configuration [checkpoint: 96d0fcb]
- [x] Task: Set up configuration structure and load/save functions using `serde` and `dirs`. [ed66c35]
    - [x] Write Tests for config handling
    - [x] Implement Feature
- [x] Task: Conductor - User Manual Verification 'Phase 1: Client Setup and Configuration' (Protocol in workflow.md)

## Phase 2: HTTP Client and Prompting
- [x] Task: Implement async HTTP client wrapper around `reqwest` targeting generic OpenAI API endpoints. [707814a]
    - [x] Write Tests for HTTP request formatting
    - [x] Implement Feature
- [x] Task: Create prompt template module for compressing UI XML and injecting the user goal. [2840e84]
    - [x] Write Tests for XML compression and templating
    - [x] Implement Feature
- [ ] Task: Conductor - User Manual Verification 'Phase 2: HTTP Client and Prompting' (Protocol in workflow.md)

## Phase 3: Response Parsing
- [ ] Task: Implement `serde` models and parsing logic to convert LLM JSON responses into `Action` items.
    - [ ] Write Tests for JSON response parsing
    - [ ] Implement Feature
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Response Parsing' (Protocol in workflow.md)