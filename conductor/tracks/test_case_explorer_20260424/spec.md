# Specification: Test Case Explorer

## 1. Overview
The "Test Case Explorer" will enhance the Session History view by allowing users to read the detailed steps of a past exploratory run and view the screenshots captured during each action.

## 2. Requirements

### 2.1 YAML Parsing
- When a session is selected, the application must attempt to read and parse the YAML file at `session.yaml_path`.
- The parsed `TestCase` should be stored in the UI state for rendering.

### 2.2 Execution Timeline
- Render a vertical timeline of `TestStep` items in the Session Detail view.
- Each step must display:
    - Action type (e.g., Tap, Input, Swipe).
    - Parameters (coordinates, text, direction).
    - The agent's reasoning for that specific step.

### 2.3 Screenshot Explorer
- For each step, the UI must check if a corresponding screenshot exists in the `mobie-results/screenshots/` folder.
- If found, the screenshot should be displayed as a thumbnail or clickable image next to the step details.

## 3. Scope & Constraints
- **Scope**: Visualization only. Editing test cases or re-running individual steps is out of scope for this track.
- **Performance**: Large YAML files or many high-res screenshots should not freeze the UI. Loading should be done efficiently.