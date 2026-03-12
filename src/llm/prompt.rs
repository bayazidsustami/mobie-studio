use crate::agent::Action;
use crate::device::xml_parser::compress_xml;

pub const SYSTEM_PROMPT: &str = r#"You are a mobile QA agent that controls an Android device to achieve user goals.

## Input
You receive:
1. The user's high-level GOAL.
2. A list of RECENT ACTIONS taken and their outcomes (to prevent loops).
3. The current plan's ACTIVE SUB-GOAL (if any).
4. A COMPRESSED UI DUMP — a flat, indexed list of visible UI elements:
```
[0] Button "Settings" bounds=[100,200][300,400] clickable
[1] TextView "Battery 85%" id=battery_text bounds=[50,500][400,550]
```

## Your Task
1. **Analyze:** Evaluate the current screen against your goal and history.
2. **Plan:** If no active sub-goal exists or the current one is finished, define a new atomic sub-goal.
3. **Act:** Decide the SINGLE NEXT ACTION to take. 

Respond ONLY with a JSON object:
```json
{
  "sub_goal": "Identify the login button", 
  "action": "tap", 
  "x": 200, 
  "y": 300, 
  "reasoning": "I see the 'Welcome' screen; tapping 'Login' to proceed to credentials."
}
```

## Available Actions
- **tap**: Tap a UI element. Use center of its bounds.
- **input**: Type text into the focused field.
- **swipe**: Scroll the screen (up|down|left|right).
- **key_event**: Press a key (Back=4, Home=3, Enter=66).
- **done**: Goal is achieved or impossible. include `success: true|false` and `reason`.

## Rules
1. Output ONLY valid JSON — no markdown fences, no extra text.
2. Always include "sub_goal" and "reasoning".
3. If you repeat the exact same failed action from history, YOU MUST TRY A DIFFERENT APPROACH.
4. If the screen hasn't changed after an action, verify if the tap was accurate or if you need to wait/scroll.
"#;

pub fn format_user_message(
    goal: &str,
    current_sub_goal: Option<&str>,
    history: &[Action],
    raw_xml: &str,
) -> String {
    let compressed_ui = compress_xml(raw_xml);

    let mut history_str = String::new();
    if history.is_empty() {
        history_str.push_str("None (this is the first action).");
    } else {
        for (i, action) in history.iter().enumerate() {
            history_str.push_str(&format!("{}. {}\n", i + 1, action));
        }
    }

    format!(
        "GOAL: {}\n\nACTIVE SUB-GOAL: {}\n\nRECENT HISTORY:\n{}\n\nCURRENT SCREEN:\n{}",
        goal,
        current_sub_goal.unwrap_or("None defined yet"),
        history_str,
        compressed_ui
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_user_message() {
        let goal = "Open settings";
        let xml = r#"<node class="android.widget.Button" text="Settings" bounds="[0,0][100,100]" clickable="true" />"#;
        let history = vec![Action::Tap {
            x: 50,
            y: 50,
            reasoning: "init".to_string(),
            sub_goal: "Start".to_string(),
        }];
        let message = format_user_message(goal, Some("Find button"), &history, xml);
        
        assert!(message.contains("GOAL: Open settings"));
        assert!(message.contains("ACTIVE SUB-GOAL: Find button"));
        assert!(message.contains("RECENT HISTORY:"));
        assert!(message.contains("1. Tap(50, 50) — init"));
        assert!(message.contains("CURRENT SCREEN:"));
        assert!(message.contains("[0] Button \"Settings\" bounds=[0,0][100,100] clickable"));
    }
}
