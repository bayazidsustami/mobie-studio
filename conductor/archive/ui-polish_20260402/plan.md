# Implementation Plan: UI Layout Polish & UX Refinement

## Phase 1: UX & Noise Reduction
- [x] Task: Silence redundant "Found devices" chat messages. [bbad1f6]
- [x] Task: Conductor - User Manual Verification 'Phase 1: UX & Noise Reduction' (Protocol in workflow.md)

## Phase 2: Sidebar Polish
- [x] Task: Improve device row layout and spacing. [91b2bc0]
- [x] Task: Style Start/Stop controls as interactive buttons. [91b2bc0]
- [x] Task: Conductor - User Manual Verification 'Phase 2: Sidebar Polish' (Protocol in workflow.md)

## Phase 3: Layout Stability & Text Wrapping
- [x] Task: Fix sidebar device name overlap for long AVD names.
    - [x] Apply `min_w_0()` and `flex_1()` to device name container.
    - [x] Add `overflow_hidden()` to prevent text spillover.
- [x] Task: Fix chat bubble height calculation for wrapped text.
    - [x] Add `min_h_auto()` and `flex_shrink_0()` to chat message bubbles.
    - [x] Ensure inner text containers have `w_full()` to report correct height.
    - [x] Ensure vertical spacing in `chat-list` respects dynamic bubble heights.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Layout Stability & Text Wrapping' (Protocol in workflow.md)

## Final Verification
- [x] All acceptance criteria met
- [x] UI remains responsive
- [x] Ready for review
