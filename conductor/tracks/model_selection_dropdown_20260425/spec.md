# Specification: Model Selection Dropdown

## Overview
Implement a dropdown component for selecting LLM models instead of manual text input to reduce human error. The dropdown will be located in the Settings Panel and populated dynamically by fetching available models from the provider's `/models` API endpoint (e.g., OpenRouter).

## Functional Requirements
- **Dynamic Model List**: The application must fetch the list of available models from the configured LLM provider's `/models` endpoint upon initialization or when accessing the settings.
- **Dropdown UI**: Replace the current manual text input for the model name in the Settings Panel with a GPUI dropdown/select component.
- **Selection Handling**: Selecting a model from the dropdown updates the application's configuration and is used for subsequent agent interactions.
- **Error Handling**: Gracefully handle API failures when fetching models (e.g., show a default fallback list or an error message).

## Non-Functional Requirements
- **Review Requirement**: Any code changes made during this implementation must be reviewed by the `@rust-reviewer` sub-agent.
- **UX**: The dropdown should support scrolling if the list of models is long.

## Out of Scope
- Implementing custom model entry beyond what the API provides.
- Moving the selection out of the Settings Panel.