# Specification: Device & Emulator Management

## Overview
This track enhances the device management capabilities of Mobie Studio. Users will be able to view both currently connected ADB devices and all registered Android Virtual Devices (AVDs) from a central sidebar. The interface will provide controls to launch, stop, and monitor the status of emulators.

## Functional Requirements
- **Device Discovery**:
    - List all currently connected Android devices via `adb devices`.
    - List all registered emulators via `emulator -list-avds`.
- **Emulator Control**:
    - **Launch**: Start a selected offline emulator using `emulator -avd <name>`.
    - **Stop**: Gracefully shut down a running emulator using `adb -s <serial> emu kill`.
- **Status Monitoring**:
    - **Offline**: Emulator is registered but not running.
    - **Launching**: Emulator has been started but is not yet fully booted or responsive.
    - **Online**: Device/Emulator is connected and ready for interaction.
- **UI Integration**:
    - Implement a dedicated **Sidebar List** for device management.
    - Use "Start" and "Stop" icons for emulator lifecycle control.
    - Provide a "Refresh" mechanism to manually update the device list.

## Non-Functional Requirements
- **Responsiveness**: The UI should not freeze while launching or stopping emulators (async execution).
- **Error Handling**: Provide feedback if an emulator fails to launch or if ADB is not found.

## Acceptance Criteria
- [ ] Sidebar displays both connected devices and all registered AVDs.
- [ ] Clicking the "Start" icon on an offline emulator launches it.
- [ ] Clicking the "Stop" icon on a running emulator closes it.
- [ ] Device status (Offline, Launching, Online) is visually indicated.
- [ ] Refreshing the list accurately reflects the current state of devices.

## Out of Scope
- Support for iOS devices or simulators.
- Advanced emulator configuration (e.g., hardware settings) within the app.
