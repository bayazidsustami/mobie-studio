# Specification: UI Layout Polish & UX Refinement

## Overview
This track focuses on refining the visual layout of Mobie Studio and improving the User Experience by reducing chat noise and polishing sidebar interactions.

## Problem Statement
1. **Chat Noise:** The application currently pushes a "✅ Found X device(s)/AVDs" message to the chat every time the device list updates (including during background polling), which buries relevant conversation.
2. **Layout Tightness:** The sidebar device list lacks sufficient vertical padding, making it feel cluttered.
3. **Control Styling:** "Start" and "Stop" controls are raw text and lack visual feedback (hover states, button-like containers).
4. **Visual Hierarchy:** The sidebar needs better spacing and alignment between headers and list items.

## Success Criteria
- [ ] System messages for device discovery are only shown on manual refresh, not background polling.
- [ ] Sidebar device entries have increased vertical breathing room.
- [ ] Start/Stop controls are styled as subtle, interactive buttons.
- [ ] Status dots have improved visual clarity (e.g., subtle glow or borders).
- [ ] Overall layout feels "polished" and professional.

## Technical Requirements
- Modify `src/ui/mod.rs` to refine GPUI element styling.
- Update `AgentUpdate` or `MobieWorkspace` logic to filter redundant system messages.
