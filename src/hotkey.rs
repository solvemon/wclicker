use crate::config::ClickMode;
use crate::state::SharedState;
use evdev::{
    uinput::VirtualDeviceBuilder, AttributeSet, Device, EventType, InputEvent, Key,
    RelativeAxisType,
};
use std::{
    fs,
    sync::{
        atomic::Ordering,
        Arc,
    },
    thread,
};

/// Scans all /dev/input/event* devices and spawns a listener thread for each
/// that has keyboard capabilities. Each listener toggles `active` on the
/// configured hotkey press and tracks BTN_LEFT state for Hold mode.
pub fn spawn_listeners(state: Arc<SharedState>) {
    let entries = fs::read_dir("/dev/input").unwrap_or_else(|_| {
        panic!("cannot read /dev/input — are you in the 'input' group?")
    });

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("event") {
            continue;
        }

        let device = match Device::open(&path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Only handle devices that support key events
        if device.supported_keys().map(|k| k.iter().count()).unwrap_or(0) == 0 {
            continue;
        }

        let state = Arc::clone(&state);
        thread::spawn(move || listen(device, state));
    }
}

/// Creates a virtual device that mirrors the physical device's capabilities
/// for forwarding events when the physical device is grabbed.
fn create_forwarder(physical: &Device) -> Option<evdev::uinput::VirtualDevice> {
    let mut builder = VirtualDeviceBuilder::new().ok()?;
    builder = builder.name("wclicker-forwarder");

    if let Some(keys) = physical.supported_keys() {
        let mut key_set = AttributeSet::<Key>::new();
        for key in keys.iter() {
            key_set.insert(key);
        }
        builder = builder.with_keys(&key_set).ok()?;
    }

    if let Some(rel_axes) = physical.supported_relative_axes() {
        let mut rel_set = AttributeSet::<RelativeAxisType>::new();
        for axis in rel_axes.iter() {
            rel_set.insert(axis);
        }
        builder = builder.with_relative_axes(&rel_set).ok()?;
    }

    builder.build().ok()
}

fn listen(mut device: Device, state: Arc<SharedState>) {
    let dev_name = device.name().unwrap_or("unknown").to_string();
    let has_mouse_button = device
        .supported_keys()
        .map(|k| k.contains(Key::BTN_LEFT))
        .unwrap_or(false);

    eprintln!("[hotkey] device={dev_name:?} has_mouse_button={has_mouse_button}");

    let mut forwarder = if has_mouse_button {
        create_forwarder(&device)
    } else {
        None
    };

    let mut grabbed = false;

    loop {
        let events: Vec<InputEvent> = match device.fetch_events() {
            Ok(e) => e.collect(),
            Err(_) => break,
        };

        // Update grab state: grab when Hold+Active, ungrab otherwise.
        // The grab must be in place BEFORE the user presses BTN_LEFT so that
        // the physical press never reaches the compositor (libinput). This
        // avoids the button-state refcount conflict where virtual clicks are
        // absorbed while the physical button is held.
        if forwarder.is_some() {
            let should_grab = state.active.load(Ordering::Acquire)
                && ClickMode::from_u8(state.mode.load(Ordering::Relaxed)) == ClickMode::Hold;

            if should_grab && !grabbed {
                if device.grab().is_ok() {
                    eprintln!("[hotkey] GRABBED {dev_name:?}");
                    grabbed = true;
                }
            } else if !should_grab && grabbed {
                let _ = device.ungrab();
                eprintln!("[hotkey] UNGRABBED {dev_name:?}");
                grabbed = false;
            }
        }

        // When grabbed, forward all events except BTN_LEFT through the forwarder
        // so mouse movement, scroll, and other buttons still work.
        if grabbed {
            if let Some(ref mut fwd) = forwarder {
                let to_forward: Vec<InputEvent> = events
                    .iter()
                    .filter(|e| {
                        if e.event_type() == EventType::KEY
                            && Key::new(e.code()) == Key::BTN_LEFT
                        {
                            return false;
                        }
                        // Skip SYN events — emit() appends its own SYN_REPORT
                        if e.event_type() == EventType::SYNCHRONIZATION {
                            return false;
                        }
                        true
                    })
                    .copied()
                    .collect();

                if !to_forward.is_empty() {
                    fwd.emit(&to_forward).ok();
                }
            }
        }

        // Process KEY events for hotkey, rebind, and mouse tracking
        for event in &events {
            if event.event_type() != EventType::KEY {
                continue;
            }

            let key = Key::new(event.code());
            let value = event.value(); // 0=release, 1=press, 2=repeat

            // Track physical mouse button state (for Hold mode)
            if key == Key::BTN_LEFT {
                eprintln!("[hotkey] BTN_LEFT value={value} on {dev_name:?} grabbed={grabbed}");
                match value {
                    1 => state.mouse_held.store(true, Ordering::Release),
                    0 => state.mouse_held.store(false, Ordering::Release),
                    _ => {}
                }
                continue;
            }

            // Rebinding: capture on press only
            if value == 1
                && state
                    .rebinding
                    .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
            {
                let mut nk = state.new_key.lock().unwrap();
                *nk = Some(key);
                continue;
            }

            // Hotkey: always toggles active on press
            if value == 1 {
                let hk = state.hotkey.lock().unwrap();
                if key == *hk {
                    state.active.fetch_xor(true, Ordering::AcqRel);
                }
            }
        }
    }
}
