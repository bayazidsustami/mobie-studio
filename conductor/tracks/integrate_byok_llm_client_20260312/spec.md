# Specification: Integrate Bring-Your-Own-Key (BYOK) LLM Client

## Overview
Implement the core LLM client that enables Mobie Studio's agent to communicate with any OpenAI-compatible HTTP API. This handles prompt formatting, UI dump compression, and action parsing.

## Requirements
- Use `reqwest` for HTTP communication.
- Create an asynchronous client struct in `src/llm/mod.rs`.
- Read API Key and Base URL configuration from standard platform config directories using `dirs`.
- Implement basic prompt templating to pass the UI XML dump + user goal.
- Parse JSON responses back into atomic `Action` structs.
- Fail gracefully with clear error types via `anyhow`.