#[cfg(target_os = "windows")]
const OS: &str = "windows";
#[cfg(target_os = "linux")]
const OS: &str = "linux";
#[cfg(target_os = "macos")]
const OS: &str = "osx";

use crate::version::Rule;

pub fn rules_allow(rules: &[Rule]) -> bool {
    if rules.is_empty() {
        return true;
    }

    let mut allowed = false;

    for rule in rules {
        let applies = match &rule.os {
            Some(os) => os.name == OS,
            None => true,
        };

        if applies {
            allowed = rule.action == "allow";
        }
    }

    allowed
}
