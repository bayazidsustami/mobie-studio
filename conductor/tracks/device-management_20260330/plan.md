# Implementation Plan: Device & Emulator Management

## Phase 1: Device Bridge Enhancement [checkpoint: 110211c]
- [x] Task: Extend `DeviceBridge` to support listing AVDs using `emulator -list-avds`. [9adc075]
    - [x] Write Tests for AVD listing (mocking command output)
    - [x] Implement `list_avds` in `src/device/mod.rs`
- [x] Task: Implement `launch_emulator` and `stop_emulator` methods in `DeviceBridge`. [a53af54]
    - [x] Write Tests for launch/stop commands
    - [x] Implement `launch_emulator` (using `emulator -avd`) and `stop_emulator` (using `adb emu kill`) in `src/device/mod.rs`
- [x] Task: Implement status detection logic for emulators (Offline, Launching, Online). [15b8c00]
    - [x] Write Tests for status transition logic
    - [x] Implement `get_device_status` helper in `src/device/mod.rs`
- [x] Task: Conductor - User Manual Verification 'Phase 1: Device Bridge Enhancement' (Protocol in workflow.md)

## Phase 2: UI Implementation (Sidebar) [checkpoint: ]
- [x] Task: Design and implement the Device List sidebar component in `src/ui/mod.rs`.
    - [x] Write Tests for sidebar rendering with mock device data
    - [x] Implement UI with status dots and start/stop icons
- [x] Task: Integrate sidebar with `AgentEngine` for real-time status updates.
- [x] Task: Conductor - User Manual Verification 'Phase 2: UI Implementation (Sidebar)' (Protocol in workflow.md)

## Phase 3: Integration & Final Polish [checkpoint: ]
- [x] Task: Implement "Select Device" functionality from the sidebar.
- [x] Task: Add "Refresh" button to manually update the device list.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Integration & Final Polish' (Protocol in workflow.md)

## Final Verification
- [x] All acceptance criteria met
- [x] Tests passing
- [x] Code coverage maintained
- [x] Ready for review
