use crate::config::ClickMode;
use crate::state::SharedState;
use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, BusType, EventType, InputEvent, InputId, Key};
use std::{
    sync::{
        atomic::Ordering,
        Arc,
    },
    thread,
    time::Duration,
};

pub fn run(state: Arc<SharedState>) {
    let mut keys = AttributeSet::<Key>::new();
    keys.insert(Key::BTN_LEFT);

    let mut device = VirtualDeviceBuilder::new()
        .expect("failed to open /dev/uinput — are you in the 'uinput' group?")
        .name("wclicker-virtual-mouse")
        .input_id(InputId::new(BusType::BUS_USB, 0x1234, 0x5678, 1))
        .with_keys(&keys)
        .expect("failed to register BTN_LEFT")
        .build()
        .expect("failed to build virtual device");

    loop {
        let active = state.active.load(Ordering::Acquire);
        let mode = ClickMode::from_u8(state.mode.load(Ordering::Relaxed));
        let should_click = active
            && match mode {
                ClickMode::Auto => true,
                ClickMode::Hold => state.mouse_held.load(Ordering::Acquire),
            };

        if should_click {
            let delay = Duration::from_millis(state.delay_ms.load(Ordering::Relaxed).max(1));

            // Press
            let press = InputEvent::new(EventType::KEY, Key::BTN_LEFT.code(), 1);
            let p_res = device.emit(&[press]);

            thread::sleep(Duration::from_millis(1));

            // Release
            let release = InputEvent::new(EventType::KEY, Key::BTN_LEFT.code(), 0);
            let r_res = device.emit(&[release]);

            // Log first emit to confirm virtual device works
            use std::sync::atomic::AtomicBool;
            static LOGGED: AtomicBool = AtomicBool::new(false);
            if !LOGGED.swap(true, Ordering::Relaxed) {
                eprintln!("[clicker] emit press={p_res:?} release={r_res:?} mode={mode:?}");
            }

            thread::sleep(delay);
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
