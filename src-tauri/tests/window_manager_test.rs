use yumo_lib::window_manager::{WindowPosition, WindowLayout};

// ---------------------------------------------------------------------------
// WindowPosition
// ---------------------------------------------------------------------------

#[test]
fn test_default_recorder_position() {
    let pos = WindowPosition::default_recorder(1440, 900);
    // Should be centered horizontally near top of screen
    assert!(pos.x > 0.0);
    assert!(pos.y >= 0.0 && pos.y <= 50.0, "y should be near top, got {}", pos.y);
    // Centered: x ≈ (1440 - width) / 2
    let expected_x = (1440.0 - pos.width) / 2.0;
    assert!((pos.x - expected_x).abs() < 1.0, "x should be centered, got {} expected {}", pos.x, expected_x);
}

#[test]
fn test_default_recorder_dimensions() {
    let pos = WindowPosition::default_recorder(1920, 1080);
    assert_eq!(pos.width, 200.0);
    assert_eq!(pos.height, 200.0);
}

#[test]
fn test_clamp_to_screen_within_bounds() {
    let pos = WindowPosition { x: 100.0, y: 100.0, width: 200.0, height: 200.0 };
    let clamped = pos.clamp_to_screen(1440.0, 900.0);
    assert_eq!(clamped.x, 100.0);
    assert_eq!(clamped.y, 100.0);
}

#[test]
fn test_clamp_to_screen_right_overflow() {
    let pos = WindowPosition { x: 1400.0, y: 50.0, width: 200.0, height: 200.0 };
    let clamped = pos.clamp_to_screen(1440.0, 900.0);
    assert_eq!(clamped.x, 1240.0); // 1440 - 200
}

#[test]
fn test_clamp_to_screen_bottom_overflow() {
    let pos = WindowPosition { x: 100.0, y: 800.0, width: 200.0, height: 200.0 };
    let clamped = pos.clamp_to_screen(1440.0, 900.0);
    assert_eq!(clamped.y, 700.0); // 900 - 200
}

#[test]
fn test_clamp_to_screen_negative() {
    let pos = WindowPosition { x: -50.0, y: -30.0, width: 200.0, height: 200.0 };
    let clamped = pos.clamp_to_screen(1440.0, 900.0);
    assert_eq!(clamped.x, 0.0);
    assert_eq!(clamped.y, 0.0);
}

#[test]
fn test_position_serialization_roundtrip() {
    let pos = WindowPosition { x: 123.5, y: 456.7, width: 200.0, height: 200.0 };
    let json = serde_json::to_string(&pos).unwrap();
    let restored: WindowPosition = serde_json::from_str(&json).unwrap();
    assert_eq!(pos, restored);
}

// ---------------------------------------------------------------------------
// WindowLayout
// ---------------------------------------------------------------------------

#[test]
fn test_layout_set_and_get_position() {
    let mut layout = WindowLayout::new();
    let pos = WindowPosition { x: 100.0, y: 200.0, width: 200.0, height: 200.0 };
    layout.set_position("recorder", pos.clone());
    assert_eq!(layout.get_position("recorder"), Some(&pos));
}

#[test]
fn test_layout_get_nonexistent() {
    let layout = WindowLayout::new();
    assert_eq!(layout.get_position("nonexistent"), None);
}

#[test]
fn test_layout_override_position() {
    let mut layout = WindowLayout::new();
    let pos1 = WindowPosition { x: 10.0, y: 20.0, width: 200.0, height: 200.0 };
    let pos2 = WindowPosition { x: 30.0, y: 40.0, width: 200.0, height: 200.0 };
    layout.set_position("recorder", pos1);
    layout.set_position("recorder", pos2.clone());
    assert_eq!(layout.get_position("recorder"), Some(&pos2));
}

#[test]
fn test_layout_multiple_windows() {
    let mut layout = WindowLayout::new();
    let r = WindowPosition { x: 10.0, y: 20.0, width: 200.0, height: 200.0 };
    let m = WindowPosition { x: 100.0, y: 100.0, width: 1024.0, height: 768.0 };
    layout.set_position("recorder", r.clone());
    layout.set_position("main", m.clone());
    assert_eq!(layout.get_position("recorder"), Some(&r));
    assert_eq!(layout.get_position("main"), Some(&m));
}

#[test]
fn test_layout_serialization_roundtrip() {
    let mut layout = WindowLayout::new();
    layout.set_position("recorder", WindowPosition { x: 1.0, y: 2.0, width: 200.0, height: 200.0 });
    layout.set_position("main", WindowPosition { x: 3.0, y: 4.0, width: 1024.0, height: 768.0 });

    let json = serde_json::to_string(&layout).unwrap();
    let restored: WindowLayout = serde_json::from_str(&json).unwrap();
    assert_eq!(layout.get_position("recorder"), restored.get_position("recorder"));
    assert_eq!(layout.get_position("main"), restored.get_position("main"));
}
