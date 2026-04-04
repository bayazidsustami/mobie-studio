use crate::agent::action::SwipeDirection;
use crate::device::DeviceBridge;
use crate::yaml_exporter::TestStep;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
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

pub struct Tap {
    pub device: Arc<DeviceBridge>,
    pub history: Arc<Mutex<Vec<TestStep>>>,
    pub screenshots: bool,
}

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
        self.device
            .tap(args.x, args.y)
            .await
            .map_err(|e| ToolError(e.to_string()))?;
        
        let mut params = HashMap::new();
        params.insert("x".to_string(), json!(args.x));
        params.insert("y".to_string(), json!(args.y));
        if let Ok(mut h) = self.history.lock() {
            h.push(TestStep {
                action: "tap".to_string(),
                params,
                reasoning: args.reasoning.clone(),
            });
        }

        if self.screenshots {
            let _ = self.device.screenshot().await;
        }

        Ok(format!(
            "Tap performed at ({}, {}) for: {}",
            args.x, args.y, args.reasoning
        ))
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

pub struct Input {
    pub device: Arc<DeviceBridge>,
    pub history: Arc<Mutex<Vec<TestStep>>>,
    pub screenshots: bool,
}

impl Tool for Input {
    const NAME: &'static str = "input";

    type Error = ToolError;
    type Args = InputArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Type text into the currently focused field on the mobile device."
                .to_string(),
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
        self.device
            .input_text(&args.text)
            .await
            .map_err(|e| ToolError(e.to_string()))?;
            
        let mut params = HashMap::new();
        params.insert("text".to_string(), json!(args.text));
        if let Ok(mut h) = self.history.lock() {
            h.push(TestStep {
                action: "input".to_string(),
                params,
                reasoning: args.reasoning.clone(),
            });
        }

        if self.screenshots {
            let _ = self.device.screenshot().await;
        }

        Ok(format!(
            "Input \"{}\" performed for: {}",
            args.text, args.reasoning
        ))
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

pub struct Swipe {
    pub device: Arc<DeviceBridge>,
    pub history: Arc<Mutex<Vec<TestStep>>>,
    pub screenshots: bool,
}

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
        let dir = match args.direction.as_str() {
            "up" => SwipeDirection::Up,
            "down" => SwipeDirection::Down,
            "left" => SwipeDirection::Left,
            "right" => SwipeDirection::Right,
            _ => return Err(ToolError("Invalid direction".to_string())),
        };

        let (w, h) = self.device.get_screen_size().await.unwrap_or((1080, 2400));
        let dist = args.distance.unwrap_or(match dir {
            SwipeDirection::Up | SwipeDirection::Down => h / 3,
            SwipeDirection::Left | SwipeDirection::Right => w / 2,
        });
        let (x2, y2) = match dir {
            SwipeDirection::Up => (args.x, args.y.saturating_sub(dist)),
            SwipeDirection::Down => (args.x, (args.y + dist).min(h - 1)),
            SwipeDirection::Left => (args.x.saturating_sub(dist), args.y),
            SwipeDirection::Right => ((args.x + dist).min(w - 1), args.y),
        };

        self.device
            .swipe(args.x, args.y, x2, y2, 300)
            .await
            .map_err(|e| ToolError(e.to_string()))?;

        let mut params = HashMap::new();
        params.insert("direction".to_string(), json!(args.direction));
        params.insert("x".to_string(), json!(args.x));
        params.insert("y".to_string(), json!(args.y));
        if let Some(d) = args.distance {
            params.insert("distance".to_string(), json!(d));
        }
        if let Ok(mut h) = self.history.lock() {
            h.push(TestStep {
                action: "swipe".to_string(),
                params,
                reasoning: args.reasoning.clone(),
            });
        }

        if self.screenshots {
            let _ = self.device.screenshot().await;
        }

        Ok(format!(
            "Swipe {} from ({}, {}) performed for: {}",
            args.direction, args.x, args.y, args.reasoning
        ))
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

pub struct KeyEvent {
    pub device: Arc<DeviceBridge>,
    pub history: Arc<Mutex<Vec<TestStep>>>,
    pub screenshots: bool,
}

impl Tool for KeyEvent {
    const NAME: &'static str = "key_event";

    type Error = ToolError;
    type Args = KeyEventArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Send a key event (e.g., Back=4, Home=3, Enter=66) to the mobile device."
                .to_string(),
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
        self.device
            .keyevent(args.code)
            .await
            .map_err(|e| ToolError(e.to_string()))?;
            
        let mut params = HashMap::new();
        params.insert("code".to_string(), json!(args.code));
        if let Ok(mut h) = self.history.lock() {
            h.push(TestStep {
                action: "key_event".to_string(),
                params,
                reasoning: args.reasoning.clone(),
            });
        }

        if self.screenshots {
            let _ = self.device.screenshot().await;
        }

        Ok(format!(
            "KeyEvent {} performed for: {}",
            args.code, args.reasoning
        ))
    }
}

// ---------------------------------------------------------------------------
// Screenshot Tool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ScreenshotArgs {
    pub reasoning: String,
}

pub struct Screenshot {
    pub device: Arc<DeviceBridge>,
    pub history: Arc<Mutex<Vec<TestStep>>>,
}

impl Tool for Screenshot {
    const NAME: &'static str = "screenshot";

    type Error = ToolError;
    type Args = ScreenshotArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Capture a screenshot of the current screen on the mobile device.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "reasoning": { "type": "string", "description": "Why this screenshot is being captured" }
                },
                "required": ["reasoning"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.device
            .screenshot()
            .await
            .map_err(|e| ToolError(e.to_string()))?;
        
        let params = HashMap::new();
        if let Ok(mut h) = self.history.lock() {
            h.push(TestStep {
                action: "screenshot".to_string(),
                params,
                reasoning: args.reasoning.clone(),
            });
        }

        Ok(format!(
            "Screenshot captured for: {}",
            args.reasoning
        ))
    }
}

// ---------------------------------------------------------------------------
// Observe Tool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ObserveArgs {
    pub reasoning: String,
}

pub struct Observe {
    pub device: Arc<DeviceBridge>,
    pub history: Arc<Mutex<Vec<TestStep>>>,
}

impl Tool for Observe {
    const NAME: &'static str = "observe";

    type Error = ToolError;
    type Args = ObserveArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Observe the current UI state of the mobile device. Returns a compressed XML representation of the screen.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "reasoning": { "type": "string", "description": "Why this observation is being performed" }
                },
                "required": ["reasoning"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let xml = self
            .device
            .observe_ui()
            .await
            .map_err(|e| ToolError(e.to_string()))?;
        let compressed = crate::device::compress_xml(&xml);
        
        let params = HashMap::new();
        // observe has no action parameters to replay other than reasoning
        if let Ok(mut h) = self.history.lock() {
            h.push(TestStep {
                action: "observe".to_string(),
                params,
                reasoning: args.reasoning.clone(),
            });
        }

        Ok(format!(
            "Current UI State (reasoning: {}):\n{}",
            args.reasoning, compressed
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tap_tool_exists() {
        let device = Arc::new(DeviceBridge::new());
        let history = Arc::new(Mutex::new(Vec::<TestStep>::new()));
        let _tool = Tap { device, history, screenshots: false };
    }
}
