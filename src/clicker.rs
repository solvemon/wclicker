use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, BusType, EventType, InputEvent, InputId, Key};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub fn run(clicking: Arc<AtomicBool>, delay_ms: Arc<AtomicU64>) {
    let mut keys = AttributeSet::<Key>::new();
    keys.insert(Key::BTN_LEFT);

    let mut device = VirtualDeviceBuilder::new()
        .expect("failed to open /dev/uinput — are you in the 'input' group?")
        .name("wclicker-virtual-mouse")
        .input_id(InputId::new(BusType::BUS_USB, 0x1234, 0x5678, 1))
        .with_keys(&keys)
        .expect("failed to register BTN_LEFT")
        .build()
        .expect("failed to build virtual device");

    loop {
        if clicking.load(Ordering::Relaxed) {
            let delay = Duration::from_millis(delay_ms.load(Ordering::Relaxed));

            // Press
            let press = InputEvent::new(EventType::KEY, Key::BTN_LEFT.code(), 1);
            device.emit(&[press]).ok();

            thread::sleep(Duration::from_millis(1));

            // Release
            let release = InputEvent::new(EventType::KEY, Key::BTN_LEFT.code(), 0);
            device.emit(&[release]).ok();

            thread::sleep(delay);
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
