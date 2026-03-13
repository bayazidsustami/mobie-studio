use rig::tool::Tool;
use rig::completion::ToolDefinition;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Tool error: {0}")]
pub struct ToolError(String);

// ---------------------------------------------------------------------------
// Tap Tool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct TapArgs {
    pub x: u32,
    pub y: u32,
    pub reasoning: String,
}

pub struct Tap;

impl Tool for Tap {
    const NAME: &'static str = "tap";

    type Error = ToolError;
    type Args = TapArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Tap at screen coordinates (x, y) on the mobile device.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "X coordinate" },
                    "y": { "type": "integer", "description": "Y coordinate" },
                    "reasoning": { "type": "string", "description": "Why this tap is being performed" }
                },
                "required": ["x", "y", "reasoning"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Tap performed at ({}, {}) for: {}", args.x, args.y, args.reasoning))
    }
}

// ---------------------------------------------------------------------------
// Input Tool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct InputArgs {
    pub text: String,
    pub reasoning: String,
}

pub struct Input;

impl Tool for Input {
    const NAME: &'static str = "input";

    type Error = ToolError;
    type Args = InputArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Type text into the currently focused field on the mobile device.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to type" },
                    "reasoning": { "type": "string", "description": "Why this input is being performed" }
                },
                "required": ["text", "reasoning"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Input \"{}\" performed for: {}", args.text, args.reasoning))
    }
}

// ---------------------------------------------------------------------------
// Swipe Tool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SwipeArgs {
    pub direction: String,
    pub x: u32,
    pub y: u32,
    pub distance: Option<u32>,
    pub reasoning: String,
}

pub struct Swipe;

impl Tool for Swipe {
    const NAME: &'static str = "swipe";

    type Error = ToolError;
    type Args = SwipeArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Swipe in a direction from a point on the mobile device.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "direction": { "type": "string", "enum": ["up", "down", "left", "right"], "description": "Direction to swipe" },
                    "x": { "type": "integer", "description": "Starting X coordinate" },
                    "y": { "type": "integer", "description": "Starting Y coordinate" },
                    "distance": { "type": "integer", "description": "Optional distance to swipe in pixels" },
                    "reasoning": { "type": "string", "description": "Why this swipe is being performed" }
                },
                "required": ["direction", "x", "y", "reasoning"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Swipe {} from ({}, {}) performed for: {}", args.direction, args.x, args.y, args.reasoning))
    }
}

// ---------------------------------------------------------------------------
// KeyEvent Tool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct KeyEventArgs {
    pub code: u32,
    pub reasoning: String,
}

pub struct KeyEvent;

impl Tool for KeyEvent {
    const NAME: &'static str = "key_event";

    type Error = ToolError;
    type Args = KeyEventArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Send a key event (e.g., Back=4, Home=3, Enter=66) to the mobile device.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "code": { "type": "integer", "description": "Android key code" },
                    "reasoning": { "type": "string", "description": "Why this key event is being performed" }
                },
                "required": ["code", "reasoning"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("KeyEvent {} performed for: {}", args.code, args.reasoning))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tap_tool_call() {
        let tool = Tap;
        let args = TapArgs {
            x: 100,
            y: 200,
            reasoning: "Test tap".to_string(),
        };
        let result = tool.call(args).await.unwrap();
        assert!(result.contains("100"));
        assert!(result.contains("200"));
    }

    #[tokio::test]
    async fn test_input_tool_call() {
        let tool = Input;
        let args = InputArgs {
            text: "hello".to_string(),
            reasoning: "Test input".to_string(),
        };
        let result = tool.call(args).await.unwrap();
        assert!(result.contains("hello"));
    }

    #[tokio::test]
    async fn test_swipe_tool_call() {
        let tool = Swipe;
        let args = SwipeArgs {
            direction: "up".to_string(),
            x: 540,
            y: 1200,
            distance: None,
            reasoning: "Test swipe".to_string(),
        };
        let result = tool.call(args).await.unwrap();
        assert!(result.contains("up"));
        assert!(result.contains("540"));
    }

    #[tokio::test]
    async fn test_key_event_tool_call() {
        let tool = KeyEvent;
        let args = KeyEventArgs {
            code: 4,
            reasoning: "Test back button".to_string(),
        };
        let result = tool.call(args).await.unwrap();
        assert!(result.contains("4"));
    }
}
