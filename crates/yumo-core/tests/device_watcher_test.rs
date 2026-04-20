use yumo_core::device_watcher::snapshot;
use yumo_core::platform::AudioInputDevice;

fn dev(id: u32, default: bool) -> AudioInputDevice {
    AudioInputDevice { id, name: format!("d{id}"), is_default: default }
}

#[test]
fn snapshot_equal_for_identical_lists() {
    let a = vec![dev(1, true), dev(2, false)];
    let b = vec![dev(1, true), dev(2, false)];
    assert_eq!(snapshot(&a), snapshot(&b));
}

#[test]
fn snapshot_equal_ignores_order() {
    let a = vec![dev(1, true), dev(2, false), dev(3, false)];
    let b = vec![dev(3, false), dev(1, true), dev(2, false)];
    assert_eq!(snapshot(&a), snapshot(&b));
}

#[test]
fn snapshot_differs_when_default_changes() {
    let a = vec![dev(1, true), dev(2, false)];
    let b = vec![dev(1, false), dev(2, true)];
    assert_ne!(snapshot(&a), snapshot(&b));
}

#[test]
fn snapshot_differs_when_device_added() {
    let a = vec![dev(1, true)];
    let b = vec![dev(1, true), dev(2, false)];
    assert_ne!(snapshot(&a), snapshot(&b));
}

#[test]
fn snapshot_differs_when_device_removed() {
    let a = vec![dev(1, true), dev(2, false)];
    let b = vec![dev(1, true)];
    assert_ne!(snapshot(&a), snapshot(&b));
}

#[test]
fn snapshot_handles_no_default() {
    let a = vec![dev(1, false), dev(2, false)];
    let b = vec![dev(1, false), dev(2, false)];
    assert_eq!(snapshot(&a), snapshot(&b));

    let c = vec![dev(1, true), dev(2, false)];
    assert_ne!(snapshot(&a), snapshot(&c));
}
