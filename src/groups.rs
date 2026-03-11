use std::fs::OpenOptions;
use std::path::Path;

/// Checks that the required device paths are writable by the current user.
/// Returns a list of human-readable error strings for any that are not accessible.
pub fn check_device_access() -> Vec<String> {
    let devices = [
        ("/dev/uinput", "uinput"),
        ("/dev/input", "input"),
    ];

    let mut errors = Vec::new();

    for (path, label) in &devices {
        if *label == "input" {
            // Check that we can read at least one /dev/input/event* device
            let readable = Path::new(path)
                .read_dir()
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("event"))
                        .unwrap_or(false)
                })
                .any(|e| OpenOptions::new().read(true).open(e.path()).is_ok());

            if !readable {
                errors.push(format!(
                    "Cannot read /dev/input/event* — add yourself to the 'input' group"
                ));
            }
        } else {
            // Check write access
            if OpenOptions::new().write(true).open(path).is_err() {
                errors.push(format!(
                    "Cannot write to {} — add yourself to the '{}' group or set up a udev rule",
                    path, label
                ));
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_returns_vec() {
        // Just verify it doesn't panic; actual results depend on permissions
        let _ = check_device_access();
    }
}
