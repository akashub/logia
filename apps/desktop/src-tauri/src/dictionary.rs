//! claude 2026-09-11: R07 vocabulary rules, applied in the parent process.
//!
//! These must live here, not in the webview: the final transcript is delivered
//! to the target application from `session_output`, so rules applied only in
//! the UI would fix what the user reads and not what actually gets typed. One
//! implementation, applied once, keeps the shown text and the delivered text
//! identical by construction.
//!
//! Deliberate constraints from `docs/product-scope.md` R07:
//!   * whole-phrase matches only, so `cat` never corrupts `category`;
//!   * nothing invented — no grammar, articles, punctuation or capitalisation;
//!   * final text only, never live captions, which the model is still revising;
//!   * a replacement is never re-matched, so rules cannot chain.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

const MAX_RULES: usize = 200;
const MAX_FIELD: usize = 120;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub spoken: String,
    pub replacement: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Default)]
pub struct Dictionary(Mutex<Vec<Rule>>);

impl Dictionary {
    pub fn replace(&self, rules: Vec<Rule>) {
        let mut stored = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *stored = sanitize(rules);
    }

    pub fn apply(&self, text: &str) -> String {
        let stored = self.0.lock().unwrap_or_else(|e| e.into_inner());
        apply_rules(text, &stored)
    }
}

fn sanitize(rules: Vec<Rule>) -> Vec<Rule> {
    let mut seen: Vec<String> = Vec::new();
    let mut kept = Vec::new();
    for rule in rules {
        let spoken: String = rule.spoken.trim().chars().take(MAX_FIELD).collect();
        let replacement: String = rule.replacement.trim().chars().take(MAX_FIELD).collect();
        if spoken.is_empty() || replacement.is_empty() {
            continue;
        }
        let key = spoken.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        kept.push(Rule {
            spoken,
            replacement,
            enabled: rule.enabled,
        });
        if kept.len() >= MAX_RULES {
            break;
        }
    }
    kept
}

#[path = "dictionary_match.rs"]
mod matching;

pub fn apply_rules(text: &str, rules: &[Rule]) -> String {
    if text.is_empty() {
        return text.to_owned();
    }
    let mut active: Vec<&Rule> = rules
        .iter()
        .filter(|rule| rule.enabled && !rule.spoken.trim().is_empty() && !rule.replacement.trim().is_empty())
        .collect();
    if active.is_empty() {
        return text.to_owned();
    }
    // Longer phrases first, so a specific rule is never pre-empted by a shorter
    // overlapping one.
    active.sort_by_key(|rule| std::cmp::Reverse(rule.spoken.trim().chars().count()));

    let mut claimed: Vec<(usize, usize, &str)> = Vec::new();
    for rule in active {
        for (start, end) in matching::matches(text, &rule.spoken) {
            if claimed.iter().any(|(s, e, _)| start < *e && *s < end) {
                continue;
            }
            claimed.push((start, end, rule.replacement.as_str()));
        }
    }
    claimed.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));

    let mut chars: Vec<char> = text.chars().collect();
    for (start, end, replacement) in claimed {
        chars.splice(start..end, replacement.chars());
    }
    chars.into_iter().collect()
}

#[tauri::command]
pub fn set_dictionary(dictionary: tauri::State<'_, Dictionary>, rules: Vec<Rule>) {
    dictionary.replace(rules);
}

/// Lets the settings UI preview a rule against this exact implementation,
/// rather than a second copy of the matching logic that could drift from it.
#[tauri::command]
pub fn preview_dictionary(text: String, rules: Vec<Rule>) -> String {
    apply_rules(&text, &sanitize(rules))
}

#[cfg(test)]
#[path = "dictionary_tests.rs"]
mod tests;
