use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

// ---------------------------------------------------------------------------
// Test-case data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
    pub action: String,
    pub params: HashMap<String, serde_json::Value>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub goal: String,
    pub steps: Vec<TestStep>,
    pub success: bool,
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Slugify a string for use as a filename component.
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Export a `TestCase` to `~/mobie-results/<goal-slug>-<timestamp>.yaml`.
/// Returns the path of the written file.
pub fn export(tc: &TestCase) -> Result<PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let results_dir = home.join("mobie-results");
    std::fs::create_dir_all(&results_dir)
        .context("Failed to create ~/mobie-results directory")?;

    // Timestamp: YYYY-MM-DDTHH-MM-SS (safe for filenames)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let slug = slugify(&tc.goal);
    let filename = format!("{}-{}.yaml", slug, now);
    let path = results_dir.join(&filename);

    let yaml = serde_yaml::to_string(tc).context("Failed to serialize TestCase to YAML")?;
    std::fs::write(&path, yaml)
        .with_context(|| format!("Failed to write YAML to {:?}", path))?;

    info!("Exported test case to {:?}", path);
    Ok(path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Open Settings App"), "open-settings-app");
        assert_eq!(slugify("  multiple   spaces  "), "multiple-spaces");
        assert_eq!(slugify("Special!@#$chars"), "special-chars");
    }

    #[test]
    fn test_testcase_yaml_round_trip() {
        let mut params = HashMap::new();
        params.insert("x".to_string(), serde_json::json!(540));
        params.insert("y".to_string(), serde_json::json!(1200));

        let tc = TestCase {
            goal: "Open Settings".to_string(),
            steps: vec![TestStep {
                action: "tap".to_string(),
                params,
                reasoning: "Tapping the Settings icon".to_string(),
            }],
            success: true,
        };

        let yaml = serde_yaml::to_string(&tc).expect("serialize");
        let loaded: TestCase = serde_yaml::from_str(&yaml).expect("deserialize");
        assert_eq!(loaded.goal, "Open Settings");
        assert!(loaded.success);
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(loaded.steps[0].action, "tap");
    }
}
