fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Compare folded characters, but advance only in ORIGINAL character offsets.
/// A lowercase expansion must match in full: `İ` can match `i̇`, never half of it.
fn word_end(source: &[char], start: usize, wanted: &[char]) -> Option<usize> {
    let mut end = start;
    let mut matched = 0;
    while matched < wanted.len() {
        for folded in source.get(end)?.to_lowercase() {
            if wanted.get(matched) != Some(&folded) {
                return None;
            }
            matched += 1;
        }
        end += 1;
    }
    Some(end)
}

/// Contiguous whole phrases; whitespace may vary only BETWEEN phrase words.
/// Returned ranges always address the original text, never its folded version.
pub(super) fn matches(text: &str, phrase: &str) -> Vec<(usize, usize)> {
    let words: Vec<Vec<char>> = phrase
        .split_whitespace()
        .map(|word| word.chars().flat_map(char::to_lowercase).collect())
        .collect();
    let Some(first) = words.first().and_then(|word| word.first()) else {
        return Vec::new();
    };
    let last = words.last().and_then(|word| word.last()).unwrap();
    let source: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    for start in 0..source.len() {
        if is_word(*first) && start > 0 && is_word(source[start - 1]) {
            continue;
        }
        let mut end = start;
        let mut complete = true;
        for (index, word) in words.iter().enumerate() {
            if index > 0 {
                let before = end;
                while source.get(end).is_some_and(|c| c.is_whitespace()) {
                    end += 1;
                }
                if before == end {
                    complete = false;
                    break;
                }
            }
            let Some(next) = word_end(&source, end, word) else {
                complete = false;
                break;
            };
            end = next;
        }
        if complete && !(is_word(*last) && source.get(end).is_some_and(|c| is_word(*c))) {
            found.push((start, end));
        }
    }
    found
}
