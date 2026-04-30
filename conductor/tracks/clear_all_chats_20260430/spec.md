# Specification: Clear All Chats

## Overview
This track implements a "Clear All" functionality in the Mobie Studio chat interface. It allows users to quickly delete all chat sessions, associated database records, and generated artifacts (YAML files and screenshots) with a single action.

## Functional Requirements
- **Clear All Button:** Add a prominent "Clear All" button in the Chat Header.
- **Cleanup Logic:**
    - Delete all records from the `sessions` table in the SQLite database.
    - Delete all associated YAML test cases from the configured export directory.
    - Delete all screenshot folders associated with the sessions.
- **UI Update:**
    - Immediately refresh the sidebar to remove all sessions.
    - If a session was active, reset the chat view to an empty state.
    - Show an "Empty State" message with a "Start New Session" button in the center of the screen when no sessions exist.

## Non-Functional Requirements
- **Performance:** The deletion process should be handled asynchronously to prevent UI freezing.
- **Atomicity:** Ensure that DB and file system deletions are coordinated.

## Acceptance Criteria
- Clicking "Clear All" in the Chat Header removes all sessions from the sidebar.
- The chat view shows an empty state after clearing.
- The database is empty of session data.
- Relevant YAML and screenshot files are deleted from disk.
- A "Start New Session" button is visible when no sessions are present.

## Out of Scope
- Selective mass deletion.
- Undo functionality.