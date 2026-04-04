# Specification: Screenshot Support for Visual Documentation

## Overview
This feature introduces automated and manual screenshot capture during test execution in Mobie Studio. It aims to provide visual "proof of work" for QA audits, debugging, and verification of UI states. Screenshots will be integrated into the test execution flow and stored alongside test results.

## Functional Requirements
- **Capture Triggers:**
    - **Automatic:** The system will capture a screenshot after every successful action (Tap, Input, Swipe, etc.) if enabled.
    - **Explicit:** A new `screenshot` command/action will be added to the YAML test case format to allow manual capture at specific points.
- **Storage:**
    - Screenshots will be saved in a `screenshots/` subdirectory relative to the directory where the test report or YAML configuration is generated.
- **Toggle Capability:**
    - A global `screenshots: true/false` setting will be supported in the YAML test case header to enable or disable capture for the entire run.
- **Naming Convention:**
    - Files will follow a sequential naming pattern: `step_<NN>_<action_name>.png` (e.g., `step_01_tap_login.png`, `step_02_screenshot_custom.png`).
- **Implementation Mechanism:**
    - Use `adb exec-out screencap -p` to capture screenshots directly from the connected device via the `DeviceBridge`.
    - Screenshots should be captured immediately after an action is confirmed successful.

## Non-Functional Requirements
- **Performance:** Screenshot capture should not significantly delay the test execution loop (target < 500ms overhead per capture).
- **Storage Efficiency:** Users can disable screenshots via the global toggle to save disk space in CI environments.

## Acceptance Criteria
- [ ] A screenshot is automatically captured and saved after a `Tap` action when `screenshots: true`.
- [ ] A screenshot is captured when a `screenshot` action is encountered in the YAML test case.
- [ ] Screenshots are saved in the correct relative directory with the specified naming convention.
- [ ] Disabling `screenshots` in the YAML header prevents any files from being created.
- [ ] The `DeviceBridge` correctly handles the `screencap` command and returns the image data.

## Out of Scope
- Visual regression testing (comparing screenshots against baselines).
- Video recording of test execution.
- Advanced image compression or post-processing beyond standard PNG format.
