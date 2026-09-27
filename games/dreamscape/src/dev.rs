//! `DREAMSCAPE_*` switches (autopilot, seeds, screenshots, forced themes...)
//! are for development. Dev builds (debug, or `--features dev`) read them;
//! a customer's release build ignores every one.

use std::env::VarError;
use std::ffi::OsString;

/// Dev switches are read in debug builds and in `--features dev` builds.
pub const ENABLED: bool = cfg!(any(debug_assertions, feature = "dev"));

/// Same shape as `std::env::var`, so call sites don't change.
pub fn var(name: &str) -> Result<String, VarError> {
    var_with(ENABLED, name)
}

pub fn var_os(name: &str) -> Option<OsString> {
    var_os_with(ENABLED, name)
}

pub fn var_with(enabled: bool, name: &str) -> Result<String, VarError> {
    if enabled {
        std::env::var(name)
    } else {
        Err(VarError::NotPresent)
    }
}

pub fn var_os_with(enabled: bool, name: &str) -> Option<OsString> {
    enabled.then(|| std::env::var_os(name)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn a_release_build_ignores_switches() {
        std::env::set_var("DREAMSCAPE_DEV_TEST", "1");
        assert!(var_with(false, "DREAMSCAPE_DEV_TEST").is_err());
        assert!(var_os_with(false, "DREAMSCAPE_DEV_TEST").is_none());
        assert_eq!(var_with(true, "DREAMSCAPE_DEV_TEST").as_deref(), Ok("1"));
    }

    /// Every switch must go through here, or a customer could flip it.
    #[test]
    fn every_switch_goes_through_dev() {
        fn walk(dir: &Path, bad: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, bad);
                } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("dev.rs") {
                    let s = std::fs::read_to_string(&p).unwrap();
                    for (i, l) in s.lines().enumerate() {
                        if l.contains("env::var(\"DREAMSCAPE_")
                            || l.contains("env::var_os(\"DREAMSCAPE_")
                        {
                            bad.push(format!("{}:{}", p.display(), i + 1));
                        }
                    }
                }
            }
        }
        let mut bad = Vec::new();
        walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut bad);
        assert!(bad.is_empty(), "read with crate::dev instead: {bad:#?}");
    }
}
