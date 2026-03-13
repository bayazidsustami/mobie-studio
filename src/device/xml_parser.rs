use tracing::warn;

/// A flattened, LLM-friendly representation of a single UI node.
#[derive(Debug, Clone)]
pub struct UiElement {
    pub index: usize,
    pub parent_index: Option<usize>,
    pub class: String,
    pub text: String,
    pub resource_id: String,
    pub content_desc: String,
    pub bounds: String,
    pub clickable: bool,
    pub scrollable: bool,
    pub checked: Option<bool>,
    pub focused: bool,
}

impl UiElement {
    /// Center coordinates derived from bounds string "[x1,y1][x2,y2]"
    pub fn center(&self) -> Option<(u32, u32)> {
        parse_bounds_center(&self.bounds)
    }
}

impl std::fmt::Display for UiElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = vec![format!("[{}]", self.index)];

        // Short class name (strip package prefix)
        let short_class = self.class.rsplit('.').next().unwrap_or(&self.class);
        parts.push(short_class.to_string());

        if !self.text.is_empty() {
            parts.push(format!("\"{}\"", self.text));
        }
        if !self.content_desc.is_empty() {
            parts.push(format!("desc=\"{}\"", self.content_desc));
        }
        if !self.resource_id.is_empty() {
            // Strip package prefix from resource ID
            let short_id = self
                .resource_id
                .rsplit('/')
                .next()
                .unwrap_or(&self.resource_id);
            parts.push(format!("id={}", short_id));
        }

        if let Some(p_idx) = self.parent_index {
            parts.push(format!("parent={}", p_idx));
        }

        if let Some((cx, cy)) = self.center() {
            parts.push(format!("center=[{},{}]", cx, cy));
        }

        parts.push(format!("bounds={}", self.bounds));

        let mut flags = Vec::new();
        if self.clickable {
            flags.push("clickable");
        }
        if self.scrollable {
            flags.push("scrollable");
        }
        if self.focused {
            flags.push("focused");
        }
        if let Some(checked) = self.checked {
            flags.push(if checked { "checked" } else { "unchecked" });
        }
        if !flags.is_empty() {
            parts.push(flags.join(","));
        }

        write!(f, "{}", parts.join(" "))
    }
}

/// Parse raw uiautomator XML and extract a flat list of meaningful UI elements.
///
/// Filters out nodes that have no text, resource-id, content-desc AND are not
/// clickable/scrollable — these are just layout containers the LLM doesn't need.
pub fn parse_ui_xml(raw_xml: &str) -> Vec<UiElement> {
    let mut elements = Vec::new();
    let mut index = 0usize;
    let mut parent_stack: Vec<usize> = Vec::new();

    // Use a simple state-machine-like approach to track nesting.
    // We look for "<node " to start a node and "/>" or "</node>" to end it.
    let mut cursor = 0;
    while let Some(node_start) = raw_xml[cursor..].find("<node ") {
        let absolute_start = cursor + node_start;
        let segment_start = absolute_start + 6; // skip "<node "
        
        // Find the end of this node's opening tag
        let tag_end = match raw_xml[segment_start..].find('>') {
            Some(e) => segment_start + e,
            None => break,
        };

        let segment = &raw_xml[segment_start..tag_end];
        
        let class = extract_attr(segment, "class").unwrap_or_default();
        let text = extract_attr(segment, "text").unwrap_or_default();
        let resource_id = extract_attr(segment, "resource-id").unwrap_or_default();
        let content_desc = extract_attr(segment, "content-desc").unwrap_or_default();
        let bounds = extract_attr(segment, "bounds").unwrap_or_default();
        let clickable = extract_attr(segment, "clickable").as_deref() == Some("true");
        let scrollable = extract_attr(segment, "scrollable").as_deref() == Some("true");
        let focused = extract_attr(segment, "focused").as_deref() == Some("true");
        let checked = match extract_attr(segment, "checked").as_deref() {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        };

        // Filter: keep only nodes that carry meaningful information
        let is_meaningful = !text.is_empty()
            || !resource_id.is_empty()
            || !content_desc.is_empty()
            || clickable
            || scrollable
            || focused
            || class.contains("ProgressBar");

        let mut current_node_index = None;
        if is_meaningful {
            let parent_index = parent_stack.last().copied();
            elements.push(UiElement {
                index,
                parent_index,
                class,
                text,
                resource_id,
                content_desc,
                bounds,
                clickable,
                scrollable,
                checked,
                focused,
            });
            current_node_index = Some(index);
            index += 1;
        }

        // Check if this node is self-closing or has children
        let is_self_closing = segment.trim_end().ends_with('/');
        if !is_self_closing {
            // Push to stack. If this node was filtered, we still push a placeholder
            // to maintain depth, but we don't have an index for it.
            // Actually, we should only track meaningful parents for "parent=" hints.
            // If a parent is filtered, we might want to link to its own parent.
            if let Some(idx) = current_node_index {
                parent_stack.push(idx);
            } else if let Some(last_meaningful) = parent_stack.last().copied() {
                parent_stack.push(last_meaningful);
            }
        }

        // Move cursor to after the tag
        cursor = tag_end + 1;

        // Simple check for closing tags before the next node start
        // This is a bit naive but should work for well-formed uiautomator XML
        let next_node = raw_xml[cursor..].find("<node ");
        let search_range = match next_node {
            Some(n) => &raw_xml[cursor..cursor + n],
            None => &raw_xml[cursor..],
        };

        // Count "</node>" occurrences
        let closings = search_range.match_indices("</node>").count();
        for _ in 0..closings {
            parent_stack.pop();
        }
    }

    elements
}

/// Compress raw uiautomator XML into a compact, indexed text format for the LLM.
///
/// Output example:
/// ```text
/// [0] Button "Settings" bounds=[100,200][300,400] clickable
/// [1] TextView "Battery: 85%" id=battery_text bounds=[50,500][400,550]
/// [2] EditText "" id=search_box bounds=[0,0][1080,100] clickable,focused
/// ```
pub fn compress_xml(raw_xml: &str) -> String {
    let elements = parse_ui_xml(raw_xml);

    if elements.is_empty() {
        warn!("XML compression produced 0 elements — the screen may be blank or XML is malformed");
        return "No UI elements detected on screen.".to_string();
    }

    let lines: Vec<String> = elements.iter().map(|e| e.to_string()).collect();
    lines.join("\n")
}

/// Detect if the screen contains common loading indicators (spinners, progress bars, etc.)
pub fn is_loading(raw_xml: &str) -> bool {
    let elements = parse_ui_xml(raw_xml);
    elements.iter().any(|e| {
        let class_lower = e.class.to_lowercase();
        let text_lower = e.text.to_lowercase();
        let desc_lower = e.content_desc.to_lowercase();

        class_lower.contains("progressbar")
            || text_lower.contains("loading")
            || desc_lower.contains("loading")
            || text_lower.contains("please wait")
            || desc_lower.contains("please wait")
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract an XML attribute value from a segment like `class="android.widget.Button"`.
fn extract_attr(segment: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    let start = segment.find(&pattern)?;
    let value_start = start + pattern.len();
    let rest = &segment[value_start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Parse bounds string "[x1,y1][x2,y2]" and return the center (x, y).
fn parse_bounds_center(bounds: &str) -> Option<(u32, u32)> {
    // Format: "[x1,y1][x2,y2]"
    let cleaned = bounds.replace("][", ",").replace(['[', ']'], "");
    let parts: Vec<u32> = cleaned.split(',').filter_map(|s| s.parse().ok()).collect();
    if parts.len() == 4 {
        let cx = (parts[0] + parts[2]) / 2;
        let cy = (parts[1] + parts[3]) / 2;
        Some((cx, cy))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<hierarchy rotation="0">
  <node index="0" text="" resource-id="" class="android.widget.FrameLayout" content-desc="" clickable="false" bounds="[0,0][1080,2400]">
    <node index="0" text="Settings" resource-id="com.android.settings:id/title" class="android.widget.TextView" content-desc="" clickable="true" bounds="[100,200][300,400]" scrollable="false" focused="false" checked="false" />
    <node index="1" text="" resource-id="" class="android.widget.LinearLayout" content-desc="" clickable="false" bounds="[0,400][1080,500]" scrollable="false" focused="false" />
    <node index="2" text="Battery 85%" resource-id="com.android.settings:id/battery" class="android.widget.TextView" content-desc="Battery level" clickable="false" bounds="[50,500][400,550]" scrollable="false" focused="false" checked="false" />
  </node>
</hierarchy>"#;

    #[test]
    fn test_parse_ui_xml_filters_empty_nodes() {
        let elements = parse_ui_xml(SAMPLE_XML);
        // FrameLayout (no text/id/desc, not clickable) should be filtered out
        // LinearLayout (no text/id/desc, not clickable) should be filtered out
        // Settings TextView and Battery TextView should remain
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].text, "Settings");
        assert_eq!(elements[1].text, "Battery 85%");
    }

    #[test]
    fn test_compress_xml_output_format() {
        let compressed = compress_xml(SAMPLE_XML);
        assert!(compressed.contains("[0]"));
        assert!(compressed.contains("\"Settings\""));
        assert!(compressed.contains("center=[200,300]"));
        assert!(compressed.contains("clickable"));
        assert!(compressed.contains("[1]"));
        assert!(compressed.contains("\"Battery 85%\""));
    }

    #[test]
    fn test_element_center_calculation() {
        let elements = parse_ui_xml(SAMPLE_XML);
        let settings = &elements[0];
        let center = settings.center().unwrap();
        assert_eq!(center, (200, 300)); // (100+300)/2, (200+400)/2
    }

    #[test]
    fn test_extract_attr() {
        let segment = r#"text="Hello" class="android.widget.Button" clickable="true""#;
        assert_eq!(extract_attr(segment, "text"), Some("Hello".to_string()));
        assert_eq!(
            extract_attr(segment, "class"),
            Some("android.widget.Button".to_string())
        );
        assert_eq!(extract_attr(segment, "clickable"), Some("true".to_string()));
        assert_eq!(extract_attr(segment, "missing"), None);
    }

    #[test]
    fn test_parse_bounds_center() {
        assert_eq!(parse_bounds_center("[0,0][100,200]"), Some((50, 100)));
        assert_eq!(parse_bounds_center("[100,200][300,400]"), Some((200, 300)));
        assert_eq!(parse_bounds_center("invalid"), None);
    }
}
