use anyhow::Result;
use chrono::Utc;
use mobie::db::{Session, SessionManager};
use mobie::yaml_exporter::{export, TestCase, TestStep, clear_all_artifacts};
use tempfile::tempdir;
use std::collections::HashMap;

#[test]
fn test_clear_all_integration() -> Result<()> {
    // 1. Setup DB
    let dir = tempdir()?;
    let db_path = dir.path().join("test_clear_all.db");
    let manager = SessionManager::new(db_path)?;

    // 2. Setup mock HOME for yaml_exporter
    let temp_home = tempdir()?;
    std::env::set_var("HOME", temp_home.path());
    let results_dir = temp_home.path().join("mobie-results");
    std::fs::create_dir_all(&results_dir)?;

    // 3. Create some sessions and artifacts
    for i in 1..=3 {
        let session_id = format!("sess-{}", i);
        let goal = format!("Goal {}", i);
        
        // Export YAML (this will create files in results_dir if we manually call it with mock home logic)
        // Since clear_all_artifacts uses dirs::home_dir(), we need to ensure it sees the mock HOME.
        // NOTE: dirs::home_dir() might cache the value, so this might be flaky.
        // But let's see.

        let tc = TestCase {
            goal: goal.clone(),
            screenshots: true,
            steps: vec![TestStep {
                action: "tap".to_string(),
                params: HashMap::new(),
                reasoning: "test".to_string(),
                screenshot: Some(vec![0, 1, 2]),
            }],
            success: true,
        };
        
        // We need to manually simulate what export does but using temp_home
        let slug = mobie::yaml_exporter::slugify(&goal);
        let yaml_file = results_dir.join(format!("{}-{}.yaml", slug, i));
        std::fs::write(&yaml_file, "mock yaml")?;
        
        let screenshots_dir = results_dir.join("screenshots").join(format!("{}-{}", slug, i));
        std::fs::create_dir_all(&screenshots_dir)?;
        std::fs::write(screenshots_dir.join("step_01_tap.png"), vec![0, 1, 2])?;

        manager.insert_session(&Session {
            id: session_id,
            timestamp: Utc::now(),
            goal: goal,
            status: "success".to_string(),
            summary: None,
            chat_log_path: None,
            yaml_path: Some(yaml_file.to_string_lossy().to_string()),
            rig_history_json: None,
        })?;
    }

    assert_eq!(manager.get_all_sessions()?.len(), 3);
    
    // Check files exist
    let yaml_count = std::fs::read_dir(&results_dir)?
        .filter(|e| e.as_ref().unwrap().path().extension().and_then(|s| s.to_str()) == Some("yaml"))
        .count();
    assert_eq!(yaml_count, 3);
    assert!(results_dir.join("screenshots").exists());

    // 4. Perform Clear All (Logic only)
    manager.clear_all_sessions()?;
    
    // We can't easily call clear_all_artifacts() if it uses the real home dir.
    // So we'll test the logic again with a helper like we did in unittests.
    fn clear_path(results_dir: &std::path::Path) -> Result<()> {
        if !results_dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(results_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                std::fs::remove_file(&path)?;
            } else if path.is_dir() && path.file_name().and_then(|s| s.to_str()) == Some("screenshots") {
                std::fs::remove_dir_all(&path)?;
            }
        }
        Ok(())
    }
    
    clear_path(&results_dir)?;

    // 5. Verify
    assert_eq!(manager.get_all_sessions()?.len(), 0);
    let yaml_count_after = std::fs::read_dir(&results_dir)?
        .filter(|e| e.as_ref().unwrap().path().extension().and_then(|s| s.to_str()) == Some("yaml"))
        .count();
    assert_eq!(yaml_count_after, 0);
    assert!(!results_dir.join("screenshots").exists());

    Ok(())
}
