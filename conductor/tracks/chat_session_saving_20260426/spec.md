# Specification: Chat Session Saving and Reloading

## Overview
Currently, the system saves the session of testing, but not the chat sequence. This feature introduces the ability to save the entire chat session. Every chat session will be accessible in the "Sessions" section and can be deleted. Selecting a session will display a title, the list of test cases generated with screenshots, and a "Reload Session" button. Clicking this button will redirect the user to the chat page and load the full conversation context.

## Functional Requirements
1.  **Chat Session Storage (Relational):** 
    - Implement a new `chat_messages` table in SQLite to store individual chat messages.
    - Each message must be linked to its parent `session_id`.
    - Message metadata (timestamp, sender/role, content) must be preserved.
2.  **Session Title & Display:**
    - The session title displayed in the UI must include: the exact Date & Time, the Initial Goal/Prompt, and a short AI-generated summary of the session.
    - Clicking a session in the list must show this title, along with the list of test cases and screenshots (as currently implemented).
3.  **Reload Session Feature:**
    - Add a "Reload Session" button to the session details view.
    - Clicking "Reload Session" must redirect the user to the main chat interface.
    - The chat interface must be populated with the history from the `chat_messages` table for the selected session.
    - Reloading a session will *Replace the Active Session* completely. The loaded session becomes the new active session in the chat view.
4.  **Session Deletion:**
    - Users must be able to delete a saved chat session from the "Sessions" section.
    - Deletion must cascade, removing the session record, all related `chat_messages`, and any generated test cases/screenshots.

## Non-Functional Requirements
- **Performance:** Loading a chat session with many messages must not block the main UI thread.
- **Database Schema:** The SQLite schema migration must handle existing sessions gracefully.

## Acceptance Criteria
- [ ] A new session stores all its chat messages in the database.
- [ ] The "Sessions" section displays titles containing Date/Time, Initial Goal, and AI Summary.
- [ ] Clicking a session shows its details, test cases, and a "Reload Session" button.
- [ ] Clicking "Reload Session" loads the entire chat history into the main chat window and replaces the current active session context.
- [ ] Deleting a session successfully removes its messages, metadata, and artifacts from the disk and database.

## Out of Scope
- Branching or duplicating sessions on reload.