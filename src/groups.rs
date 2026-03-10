/// Returns the GID for a group name by parsing /etc/group.
pub fn gid_for_name(group_name: &str) -> Option<u32> {
    let content = std::fs::read_to_string("/etc/group").ok()?;
    for line in content.lines() {
        let mut parts = line.split(':');
        let name = parts.next()?;
        parts.next(); // skip password field
        let gid: u32 = parts.next()?.parse().ok()?;
        if name == group_name {
            return Some(gid);
        }
    }
    None
}

/// Returns all supplementary GIDs of the current process from /proc/self/status.
pub fn current_gids() -> Vec<u32> {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("Groups:"))
        .map(|line| {
            line.split_whitespace()
                .skip(1) // skip "Groups:" label
                .filter_map(|g| g.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the names of required groups that the current user is missing.
pub fn missing_groups(required: &[&str]) -> Vec<String> {
    let gids = current_gids();
    required
        .iter()
        .filter(|&&name| {
            gid_for_name(name)
                .map(|gid| !gids.contains(&gid))
                .unwrap_or(true) // group doesn't exist → treat as missing
        })
        .map(|&s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_group() {
        // "root" group always exists with GID 0
        assert_eq!(gid_for_name("root"), Some(0));
    }

    #[test]
    fn unknown_group_returns_none() {
        assert_eq!(gid_for_name("__nonexistent_group_xyz__"), None);
    }

    #[test]
    fn current_gids_is_nonempty() {
        // Every process has at least one GID
        assert!(!current_gids().is_empty());
    }

    #[test]
    fn missing_groups_empty_when_no_requirements() {
        let missing = missing_groups(&[]);
        assert!(missing.is_empty());
    }
}
