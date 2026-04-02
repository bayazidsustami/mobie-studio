use mobie::device::xml_parser::{is_loading, parse_ui_xml};

#[test]
fn test_detect_progress_bar() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hierarchy rotation="0">
  <node index="0" class="android.widget.FrameLayout">
    <node index="0" class="android.widget.ProgressBar" bounds="[400,1000][600,1200]" />
  </node>
</hierarchy>"#;

    let elements = parse_ui_xml(xml);
    assert!(
        elements.iter().any(|e| e.class.contains("ProgressBar")),
        "Should detect ProgressBar even without text/id"
    );
}

#[test]
fn test_detect_loading_text() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hierarchy rotation="0">
  <node index="0" class="android.widget.TextView" text="Loading..." bounds="[0,0][100,100]" />
</hierarchy>"#;

    let elements = parse_ui_xml(xml);
    assert!(elements.iter().any(|e| e.text == "Loading..."));
}

#[test]
fn test_is_loading_helper() {
    let xml = r#"<node class="android.widget.ProgressBar" bounds="[0,0][10,10]" />"#;
    assert!(is_loading(xml));

    let xml_text =
        r#"<node class="android.widget.TextView" text="Please wait..." bounds="[0,0][10,10]" />"#;
    assert!(is_loading(xml_text));

    let xml_idle =
        r#"<node class="android.widget.Button" text="Click me" bounds="[0,0][10,10]" />"#;
    assert!(!is_loading(xml_idle));
}
