# Mobie Studio: Product Guide

## Initial Concept
AI-powered desktop application for automated mobile testing using an agent loop (Observe -> Think -> Act) built in Rust and GPUI.

## Vision
Mobie Studio empowers mobile-first QA, Engineers, and non-technical staff to run automated mobile tests with zero-effort setup. Instead of dealing with the complex configuration friction of traditional tools like Appium or Espresso, users can simply converse with an autonomous agent that navigates the UI to achieve high-level goals. 

## Target Audience
- **Non-technical QA:** Able to test application flows using natural language without writing code.
- **Mobile Developers:** Can perform rapid local testing and debugging iteratively.
- **QA Automation Engineers:** Can translate successful exploratory runs into stable, reliable test cases for CI/CD integration.

## Core Value Proposition
- **Zero-Effort Setup:** Eliminates the deep learning curve and configuration hell of traditional mobile testing frameworks. 
- **Conversational Interface:** Users dictate test goals in plain English.
- **Autonomous Agent Loop:** The agent uses an Observe -> Think -> Act -> Verify loop with implicit waits to handle dynamic UIs like a human would.
- **Integrated Emulator Management:** View all registered Android Virtual Devices (AVDs) and launch or stop them directly within the app, streamlining the testing workflow.
- **Deterministic CI/CD Output:** Successful exploratory AI runs generate strict, step-by-step YAML test cases, ensuring fast, deterministic replay in CI/CD pipelines without incurring LLM latency or non-determinism.

## Future Scope
While initially focused on Android (via ADB) and structural XML dumps, the product roadmap includes:
- **Visual Validation:** Leveraging vision-language models for asserting visual correctness and semantic layouts.
- **iOS Support:** Expanding device bridge capabilities beyond Android to support Apple devices.
- **Multi-device Orchestration:** Coordinating sophisticated test flows across multiple devices simultaneously.