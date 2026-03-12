# Rust Code Style Guide

- Follow standard Rust formatting (`rustfmt`).
- Use `clippy` for linting and follow its recommendations.
- Keep functions small and focused.
- Prefer `Result` and `Option` for error handling over panics.
- Write unit tests for business logic in the same file using `#[cfg(test)]`.