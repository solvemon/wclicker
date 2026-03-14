use crate::config::ClickMode;
use crate::state::SharedState;
use evdev::{Device, EventType, Key};
use std::{
    fs,
    sync::{
        atomic::Ordering,
        Arc,
    },
    thread,
};

/// Scans all /dev/input/event* devices and spawns a listener thread for each
/// that has keyboard capabilities. Each listener toggles `clicking` on the
/// configured key press and optionally captures a rebind key.
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

fn listen(mut device: Device, state: Arc<SharedState>) {
    loop {
        let events = match device.fetch_events() {
            Ok(e) => e,
            Err(_) => break,
        };

        for event in events {
            if event.event_type() != EventType::KEY {
                continue;
            }

            let key = Key::new(event.code());
            let value = event.value(); // 0=release, 1=press, 2=repeat

            // Rebinding: capture on press, regardless of mode
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

            let hk = state.hotkey.lock().unwrap();
            if key != *hk {
                continue;
            }
            drop(hk);

            let mode = ClickMode::from_u8(state.mode.load(Ordering::Relaxed));
            match mode {
                ClickMode::Toggle => {
                    if value == 1 {
                        state.clicking.fetch_xor(true, Ordering::AcqRel);
                    }
                }
                ClickMode::Hold => match value {
                    1 => state.clicking.store(true, Ordering::Release),
                    0 => state.clicking.store(false, Ordering::Release),
                    _ => {}
                },
            }
        }
    }
}
