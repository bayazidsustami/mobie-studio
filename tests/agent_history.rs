use mobie::agent::{SessionHistory, Action};

#[test]
fn test_history_tracking() {
    let mut history = SessionHistory::new(5);
    
    let action1 = Action::Tap { x: 100, y: 200, reasoning: "test 1".to_string(), sub_goal: "goal 1".to_string() };
    let action2 = Action::Input { text: "hello".to_string(), reasoning: "test 2".to_string(), sub_goal: "goal 2".to_string() };
    
    history.push(action1.clone());
    history.push(action2.clone());
    
    let entries = history.get_recent(2);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].reasoning(), "test 1");
    assert_eq!(entries[1].reasoning(), "test 2");
}

#[test]
fn test_loop_detection() {
    let mut history = SessionHistory::new(5);
    
    let action = Action::Tap { x: 100, y: 200, reasoning: "stuck".to_string(), sub_goal: "fix".to_string() };
    
    history.push(action.clone());
    assert!(!history.is_looping());
    
    history.push(action.clone());
    assert!(history.is_looping(), "Should detect repeated identical actions as a loop");
}

#[test]
fn test_history_limit() {
    let mut history = SessionHistory::new(2);
    
    history.push(Action::Tap { x: 1, y: 1, reasoning: "1".to_string(), sub_goal: "g1".to_string() });
    history.push(Action::Tap { x: 2, y: 2, reasoning: "2".to_string(), sub_goal: "g2".to_string() });
    history.push(Action::Tap { x: 3, y: 3, reasoning: "3".to_string(), sub_goal: "g3".to_string() });
    
    let entries = history.get_recent(5);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].reasoning(), "2");
    assert_eq!(entries[1].reasoning(), "3");
}
