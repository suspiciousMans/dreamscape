use engine::ui::output::OutputPanel;
use std::path::PathBuf;

#[test]
fn test_output_panel_appends_lines() {
    let mut panel = OutputPanel::new();
    assert_eq!(panel.line_count_label(), "0 messages");

    panel.push("hello");
    assert_eq!(panel.line_count_label(), "1 message");

    panel.push("world");
    assert_eq!(panel.lines().len(), 2);
    assert_eq!(panel.line_count_label(), "2 messages");

    // multiline push is still one "message" in the label count
    panel.push("line1\nline2");
    assert_eq!(panel.lines().len(), 3);
    assert_eq!(panel.line_count_label(), "3 messages");
}

#[test]
fn test_output_panel_line_count_label() {
    let panel = OutputPanel::new();
    assert_eq!(panel.line_count_label(), "0 messages");

    // a single message
    let mut panel2 = OutputPanel::new();
    panel2.push("one");
    assert_eq!(panel2.line_count_label(), "1 message");
}

#[test]
fn test_output_panel_clear_removes_all_lines() {
    let mut panel = OutputPanel::new();
    panel.push("a");
    panel.push("b");
    assert_eq!(panel.lines().len(), 2);
    panel.clear();
    assert_eq!(panel.lines().len(), 0);
    assert_eq!(panel.line_count_label(), "0 messages");
}
