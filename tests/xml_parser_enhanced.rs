use mobie::device::xml_parser::{compress_xml, parse_ui_xml};

#[test]
fn test_enhanced_compression_includes_parent_context() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hierarchy rotation="0">
  <node index="0" text="" resource-id="com.android.settings:id/dashboard" class="android.widget.ScrollView" content-desc="" clickable="false" bounds="[0,0][1080,2400]">
    <node index="0" text="" resource-id="com.android.settings:id/item_container" class="android.widget.LinearLayout" content-desc="" clickable="false" bounds="[0,0][1080,200]">
        <node index="0" text="Wi-Fi" resource-id="android:id/title" class="android.widget.TextView" content-desc="" clickable="true" bounds="[100,50][300,150]" />
    </node>
  </node>
</hierarchy>"#;

    let compressed = compress_xml(xml);
    
    // We expect the output to somehow indicate that 'Wi-Fi' is inside a 'ScrollView' or 'item_container'
    // For now, let's just assert that it contains the text 'Wi-Fi' and maybe some parent hint.
    // The current implementation doesn't do this, so this test should fail once we add the expectation.
    assert!(compressed.contains("Wi-Fi"));
    assert!(compressed.contains("parent="), "Compressed output should contain parent hints for better context");
}

#[test]
fn test_filter_redundant_containers() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hierarchy>
  <node class="android.widget.FrameLayout" bounds="[0,0][100,100]" clickable="false">
    <node class="android.widget.LinearLayout" bounds="[0,0][100,100]" clickable="false">
        <node class="android.widget.TextView" text="Target" bounds="[10,10][90,90]" clickable="true" />
    </node>
  </node>
</hierarchy>"#;

    let elements = parse_ui_xml(xml);
    // We expect only the TextView to be present, the redundant containers should be filtered even if they have IDs if they don't add semantic value.
    // Actually, the current implementation filters them if they have NO text/id/desc AND are not clickable.
    // If we want to be MORE aggressive, we might filter them even if they have IDs if they just wrap a single meaningful child.
    
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].text, "Target");
}
