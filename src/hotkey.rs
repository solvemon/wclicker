use evdev::{Device, EventType, Key};
use std::{
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

/// Scans all /dev/input/event* devices and spawns a listener thread for each
/// that has keyboard capabilities. Each listener toggles `clicking` on the
/// configured key press and optionally captures a rebind key.
pub fn spawn_listeners(
    clicking: Arc<AtomicBool>,
    hotkey: Arc<Mutex<Key>>,
    rebinding: Arc<AtomicBool>,
    new_key: Arc<Mutex<Option<Key>>>,
) {
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

        let clicking = Arc::clone(&clicking);
        let hotkey = Arc::clone(&hotkey);
        let rebinding = Arc::clone(&rebinding);
        let new_key = Arc::clone(&new_key);

        thread::spawn(move || listen(device, clicking, hotkey, rebinding, new_key));
    }
}

fn listen(
    mut device: Device,
    clicking: Arc<AtomicBool>,
    hotkey: Arc<Mutex<Key>>,
    rebinding: Arc<AtomicBool>,
    new_key: Arc<Mutex<Option<Key>>>,
) {
    loop {
        let events = match device.fetch_events() {
            Ok(e) => e,
            Err(_) => break,
        };

        for event in events {
            if event.event_type() != EventType::KEY {
                continue;
            }
            if event.value() != 1 {
                // Only keydown (value=1), not repeat (2) or release (0)
                continue;
            }

            let key = Key::new(event.code());

            if rebinding
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                let mut nk = new_key.lock().unwrap();
                *nk = Some(key);
            } else {
                let hk = hotkey.lock().unwrap();
                if key == *hk {
                    clicking.fetch_xor(true, Ordering::AcqRel);
                }
            }
        }
    }
}
