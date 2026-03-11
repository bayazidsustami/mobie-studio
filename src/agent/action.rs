use serde::Deserialize;

/// An action decided by the LLM agent.
///
/// The LLM returns JSON with an `"action"` tag that maps to one of these variants.
/// Example JSON:
/// ```json
/// { "action": "tap", "x": 200, "y": 300, "reasoning": "Tapping Settings button" }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Tap at screen coordinates.
    Tap {
        x: u32,
        y: u32,
        #[serde(default)]
        reasoning: String,
    },

    /// Type text into the currently focused field.
    Input {
        text: String,
        #[serde(default)]
        reasoning: String,
    },

    /// Swipe in a direction from a point.
    Swipe {
        direction: SwipeDirection,
        #[serde(default = "default_swipe_x")]
        x: u32,
        #[serde(default = "default_swipe_y")]
        y: u32,
        #[serde(default)]
        reasoning: String,
    },

    /// Send a key event (e.g., Back=4, Home=3, Enter=66).
    KeyEvent {
        code: u32,
        #[serde(default)]
        reasoning: String,
    },

    /// The agent considers the goal achieved (or failed).
    Done {
        success: bool,
        reason: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

fn default_swipe_x() -> u32 {
    540
}
fn default_swipe_y() -> u32 {
    1200
}

impl Action {
    /// Returns the reasoning string if present.
    pub fn reasoning(&self) -> &str {
        match self {
            Action::Tap { reasoning, .. } => reasoning,
            Action::Input { reasoning, .. } => reasoning,
            Action::Swipe { reasoning, .. } => reasoning,
            Action::KeyEvent { reasoning, .. } => reasoning,
            Action::Done { reason, .. } => reason,
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Tap { x, y, reasoning } => {
                write!(f, "Tap({}, {}) — {}", x, y, reasoning)
            }
            Action::Input { text, reasoning } => {
                write!(f, "Input(\"{}\") — {}", text, reasoning)
            }
            Action::Swipe {
                direction,
                x,
                y,
                reasoning,
            } => {
                write!(f, "Swipe({:?} from {},{}) — {}", direction, x, y, reasoning)
            }
            Action::KeyEvent { code, reasoning } => {
                write!(f, "KeyEvent({}) — {}", code, reasoning)
            }
            Action::Done { success, reason } => {
                write!(
                    f,
                    "Done(success={}) — {}",
                    success, reason
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_tap() {
        let json = r#"{"action": "tap", "x": 200, "y": 300, "reasoning": "Tap settings"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Tap { x, y, reasoning } => {
                assert_eq!(x, 200);
                assert_eq!(y, 300);
                assert_eq!(reasoning, "Tap settings");
            }
            _ => panic!("Expected Tap"),
        }
    }

    #[test]
    fn test_deserialize_input() {
        let json = r#"{"action": "input", "text": "hello world", "reasoning": "Typing search"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Input { text, reasoning } => {
                assert_eq!(text, "hello world");
                assert_eq!(reasoning, "Typing search");
            }
            _ => panic!("Expected Input"),
        }
    }

    #[test]
    fn test_deserialize_swipe() {
        let json = r#"{"action": "swipe", "direction": "up", "x": 540, "y": 1200}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Swipe { direction, x, y, .. } => {
                assert!(matches!(direction, SwipeDirection::Up));
                assert_eq!(x, 540);
                assert_eq!(y, 1200);
            }
            _ => panic!("Expected Swipe"),
        }
    }

    #[test]
    fn test_deserialize_keyevent() {
        let json = r#"{"action": "key_event", "code": 4, "reasoning": "Press back"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::KeyEvent { code, reasoning } => {
                assert_eq!(code, 4);
                assert_eq!(reasoning, "Press back");
            }
            _ => panic!("Expected KeyEvent"),
        }
    }

    #[test]
    fn test_deserialize_done() {
        let json = r#"{"action": "done", "success": true, "reason": "Goal achieved"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Done { success, reason } => {
                assert!(success);
                assert_eq!(reason, "Goal achieved");
            }
            _ => panic!("Expected Done"),
        }
    }
}
