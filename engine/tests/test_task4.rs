use std::path::PathBuf;

use engine::ui::output::OutputPanel;

#[test]
fn test_output_panel_appends_lines() {
    let mut panel = OutputPanel::new();
    assert_eq!(panel.lines().len(), 0, "fresh panel should have no lines");

    panel.push("first message");
    panel.push("second message");
    panel.push("third message");

    let lines = panel.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "first message");
    assert_eq!(lines[1], "second message");
    assert_eq!(lines[2], "third message");
}

#[test]
fn test_output_panel_clear_removes_all_lines() {
    let mut panel = OutputPanel::new();
    panel.push("one");
    panel.push("two");
    assert_eq!(panel.lines().len(), 2);

    panel.clear();
    assert_eq!(panel.lines().len(), 0);
}

#[test]
fn test_output_panel_line_count_label() {
    let mut panel = OutputPanel::new();
    assert_eq!(panel.line_count_label(), "0 messages");

    panel.push("a");
    panel.push("b");
    assert_eq!(panel.line_count_label(), "2 messages");

    panel.clear();
    assert_eq!(panel.line_count_label(), "0 messages");
}
