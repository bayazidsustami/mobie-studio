# Mobie Studio: Product Guidelines

## 1. UI/UX Principles
**Clean and Minimalist:** 
The GPUI frontend should prioritize a clean, uncluttered interface. The primary focus is the conversational chat window and essential device configuration controls (e.g., Device ID, LLM Provider settings). Secondary information like raw XML dumps or complex logs should be hidden by default but accessible via expandable panels or dedicated views to avoid overwhelming the user.

## 2. Agent Personality & Tone
**Helpful and Explanatory:** 
The autonomous agent should communicate with the user in a clear, educational manner. When taking actions, it should briefly explain its reasoning based on the observed UI state. For example, instead of just saying "Tapped login," it should say, "I see the login screen. Tapping the 'Username' field to proceed." This builds trust and helps users understand the agent's decision-making process.

## 3. Error Handling & Resilience
**Graceful Degradation:** 
Failures in ADB connections, UI dump extraction, or LLM parsing should not crash the application or present jarring error modals. Instead, the application should handle these issues gracefully within the chat interface. If the agent gets stuck, it should proactively notify the user of the obstacle and offer manual intervention options, retry prompts, or suggestions to adjust the current goal.