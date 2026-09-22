use super::*;

fn rule(spoken: &str, replacement: &str) -> Rule {
    Rule { spoken: spoken.into(), replacement: replacement.into(), enabled: true }
}

#[test]
fn replaces_misheard_phrases_verbatim() {
    let rules = vec![rule("lobia", "Logia"), rule("use state", "useState"), rule("hard key", "hotkey")];
    assert_eq!(
        apply_rules("Lobia keeps the use state hook behind a hard key.", &rules),
        "Logia keeps the useState hook behind a hotkey."
    );
}

#[test]
fn matches_whole_phrases_only() {
    let rules = vec![rule("cat", "dog")];
    assert_eq!(apply_rules("The category of cat is catalogued.", &rules), "The category of dog is catalogued.");
}

#[test]
fn repeated_fragments_are_not_a_whole_word() {
    assert_eq!(apply_rules("catcat", &[rule("cat", "dog")]), "catcat");
}

#[test]
fn phrase_edges_cannot_skip_unmatched_characters() {
    for text in ["useful state", "use interstate", "use, state"] {
        assert_eq!(apply_rules(text, &[rule("use state", "useState")]), text);
    }
}

#[test]
fn lowercase_expansion_preserves_original_source_coordinates() {
    assert_eq!(apply_rules("İ and İ!", &[rule("i\u{307}", "I")]), "I and I!");
    assert_eq!(apply_rules("İ", &[rule("i", "wrong")]), "İ");
}

#[test]
fn punctuation_separated_occurrences_are_independent() {
    assert_eq!(apply_rules("cat,cat; cat", &[rule("cat", "dog")]), "dog,dog; dog");
}

#[test]
fn tolerates_any_spacing_between_words() {
    assert_eq!(apply_rules("use   state", &[rule("use state", "useState")]), "useState");
}

#[test]
fn longer_phrase_wins_and_replacements_are_never_rematched() {
    let rules = vec![rule("state", "STATE"), rule("use state", "useState")];
    assert_eq!(apply_rules("use state here", &rules), "useState here");
    assert_eq!(apply_rules("alpha", &[rule("alpha", "beta"), rule("beta", "gamma")]), "beta");
}

#[test]
fn disabled_rules_and_empty_lists_change_nothing() {
    let off = Rule { spoken: "lobia".into(), replacement: "Logia".into(), enabled: false };
    assert_eq!(apply_rules("Lobia", &[off]), "Lobia");
    assert_eq!(apply_rules("Lobia", &[]), "Lobia");
}

#[test]
fn matching_ignores_case_and_inserts_replacement_exactly() {
    assert_eq!(apply_rules("LOBIA and lobia", &[rule("Lobia", "Logia")]), "Logia and Logia");
}

#[test]
fn non_latin_phrases_match() {
    assert_eq!(apply_rules("मैं हिंदी बोलता हूँ", &[rule("हिंदी", "Hindi")]), "मैं Hindi बोलता हूँ");
}

#[test]
fn malformed_rules_are_dropped_individually() {
    let kept = sanitize(vec![
        rule("lobia", "Logia"),
        rule("   ", "x"),
        rule("y", ""),
        rule("LOBIA", "duplicate ignored"),
        rule("hard key", "hotkey"),
    ]);
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].spoken, "lobia");
    assert_eq!(kept[1].spoken, "hard key");
}
