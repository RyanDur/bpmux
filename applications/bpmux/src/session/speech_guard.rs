const BANNED: [&str; 8] = [
    "standing by",
    "monitoring",
    "active-trigger-watch",
    "presence-mode",
    "watching for",
    "hold position",
    "passive watch",
    "passive-watch",
];

const NAMED_CAUSE: [&str; 13] = [
    "queue-empty",
    "blocker",
    "owner-signal",
    "red-ci",
    "red ci",
    "next-pull",
    "substrate-bound",
    "legitimate-deferral",
    "named-active-trigger",
    "named-empty",
    "awaiting-owner",
    "ci-red",
    "test-failure",
];

pub fn pause_vocab_violation(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let stripped = strip_quoted(&lower);
    let matched = BANNED
        .iter()
        .filter_map(|banned| word_bounded_find(&stripped, banned).map(|pos| (pos, *banned)))
        .min_by_key(|(pos, _)| *pos)
        .map(|(_, banned)| banned)?;
    if NAMED_CAUSE.iter().any(|cause| lower.contains(cause)) {
        return None;
    }
    let mut without_banned = stripped;
    for banned in BANNED {
        without_banned = without_banned.replace(banned, "");
    }
    if has_for_noun_clause(&without_banned) {
        return None;
    }
    Some(reason(matched))
}

fn reason(matched: &str) -> String {
    format!(
        "self-imposed-pause vocabulary detected (\"{matched}\") without a named stop-cause. \
         Per the board-speech-guard (#493): a broadcast that signals pause/standby requires a \
         NAMED cause (queue-empty, blocker, owner-signal, red-CI, next-pull, substrate-bound, \
         legitimate-deferral, named-active-trigger). Either rewrite to explicit active work, or \
         name the stop-cause explicitly."
    )
}

fn word_bounded_find(haystack: &str, needle: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before_bounded = start == 0
            || !haystack[..start]
                .chars()
                .next_back()
                .is_some_and(is_word_char);
        let after_bounded =
            end == haystack.len() || !haystack[end..].chars().next().is_some_and(is_word_char);
        if before_bounded && after_bounded {
            return Some(start);
        }
        from = start + needle.chars().next().map_or(1, char::len_utf8);
    }
    None
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn strip_quoted(text: &str) -> String {
    let mut out = text.to_string();
    for quote in ['\'', '"', '`'] {
        out = strip_spans(&out, quote);
    }
    out
}

fn strip_spans(text: &str, quote: char) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == quote
            && let Some(close) = (i + 1..chars.len()).find(|&j| chars[j] == quote)
        {
            i = close + 1;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn has_for_noun_clause(text: &str) -> bool {
    for (idx, _) in text.match_indices(" for ") {
        let rest = &text[idx + " for ".len()..];
        let mut chars = rest.chars();
        let first = chars.next();
        let second = chars.next();
        if matches!(first, Some(c) if c.is_ascii_lowercase())
            && matches!(second, Some(c) if c.is_ascii_lowercase() || c == '-')
        {
            return true;
        }
    }
    false
}
