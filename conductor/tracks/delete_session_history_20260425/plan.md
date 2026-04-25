# Delete Session History Plan

## Objective
Implement a "Delete Session" button within the session history detail view. When triggered, the system will permanently delete the session record from the database along with all associated output artifacts, including generated YAML test cases and the folder containing step screenshots.

## Key Files & Context
- `src/ui/mod.rs`: Update `render_session_detail` to incorporate the new button and attach its `.on_mouse_down` handler.
- `src/db/mod.rs`: Exposes `delete_session` function (already implemented) which will be called upon user confirmation or direct action.
- `src/yaml_exporter.rs` & `src/device/mod.rs`: These specify that screenshots are stored under `<yaml_dir>/screenshots/<yaml_stem>/`.

- [x] **Track: Delete Session History**
  - [x] Add "Delete Session" button in UI.
  - [x] Implement database deletion logic.
  - [x] Implement artifact (YAML and screenshots) deletion logic.
  - [x] Verify build and logic.

## Verification & Testing
- Validate that the UI displays the delete button next to the active session.
- Run a quick session (or retest one) to ensure test case files and a "screenshots" folder exist on disk.
- Click "Delete" and assert that:
  - The session immediately vanishes from the sidebar.
  - The active session detail view returns to "Select a session".
  - The SQLite database no longer holds the `session_id`.
  - The corresponding YAML file is successfully deleted from disk.
  - The corresponding screenshots folder is successfully deleted from disk.