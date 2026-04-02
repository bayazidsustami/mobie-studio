# Implementation Plan: Device & Emulator Management

## Phase 1: Device Bridge Enhancement [checkpoint: ]
- [x] Task: Extend `DeviceBridge` to support listing AVDs using `emulator -list-avds`. [9adc075]
    - [x] Write Tests for AVD listing (mocking command output)
    - [x] Implement `list_avds` in `src/device/mod.rs`
- [x] Task: Implement `launch_emulator` and `stop_emulator` methods in `DeviceBridge`. [a53af54]
    - [x] Write Tests for launch/stop commands
    - [x] Implement `launch_emulator` (using `emulator -avd`) and `stop_emulator` (using `adb emu kill`) in `src/device/mod.rs`
- [x] Task: Implement status detection logic for emulators (Offline, Launching, Online). [15b8c00]
    - [x] Write Tests for status transition logic
    - [x] Implement `get_device_status` helper in `src/device/mod.rs`
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Device Bridge Enhancement' (Protocol in workflow.md)

## Phase 2: UI Implementation (Sidebar) [checkpoint: ]
- [ ] Task: Design and implement the Device List sidebar component in `src/ui/mod.rs`.
    - [ ] Write Tests for sidebar rendering with mock device data
    - [ ] Implement `DeviceSidebar` view and its basic layout
- [ ] Task: Integrate "Start" and "Stop" icons for emulator control in the sidebar.
    - [ ] Write Tests for icon actions (start/stop dispatch)
    - [ ] Implement action handlers for the icons in the UI
- [ ] Task: Implement real-time status indicators (labels/colors) in the sidebar.
    - [ ] Write Tests for status-based styling
    - [ ] Update the sidebar UI to reflect the current status of each device
- [ ] Task: Conductor - User Manual Verification 'Phase 2: UI Implementation (Sidebar)' (Protocol in workflow.md)

## Phase 3: Integration & Final Polish [checkpoint: ]
- [ ] Task: Connect the `DeviceSidebar` to the main `MobieWorkspace` and handle automatic/manual refreshes.
    - [ ] Write Tests for periodic refresh logic
    - [ ] Implement `RefreshDevices` action and background refresh task
- [ ] Task: Final UI polish, ensuring consistent styling with the existing chat interface.
    - [ ] Write Tests for overall layout and responsiveness
    - [ ] Refine sidebar layout, padding, and hover states
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Integration & Final Polish' (Protocol in workflow.md)

## Final Verification
- [ ] All acceptance criteria met
- [ ] Tests passing
- [ ] Code coverage maintained
- [ ] Ready for review
