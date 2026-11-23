use crate::state::State;

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub fields: String,
    pub normalized: String,
    pub normalized_description: String,
    pub acronym: String,
    /// A system settings page rather than an application. Hidden from the
    /// empty view and ranked after applications that match equally well.
    pub setting: bool,
}
impl Entry {
    pub fn new(
        id: String,
        name: String,
        description: String,
        icon: String,
        fields: String,
    ) -> Self {
        let normalized = normalize(&name);
        let normalized_description = normalize(&description);
        let acronym = normalized
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .collect();
        Self {
            id,
            name,
            description,
            icon,
            fields: normalize(&fields),
            normalized,
            normalized_description,
            acronym,
            setting: false,
        }
    }
}
fn normalize(s: &str) -> String {
    glib::normalize(glib::casefold(s), glib::NormalizeMode::AllCompose).into()
}
/// Char index where a greedy in-order match of `query` ends, if one exists.
/// Earlier ends mean a tighter, more prefix-like match.
fn subsequence(query: &str, text: &str) -> Option<usize> {
    let mut chars = text.chars().enumerate();
    let mut end = 0;
    for q in query.chars() {
        end = chars.find(|&(_, c)| c == q)?.0;
    }
    Some(end)
}
/// Coarse log-scale frequency bucket: similar counts tie so recency decides,
/// and large desktop-reported tallies cannot saturate or drown launch counts.
fn frequency(count: u64) -> u64 {
    (2.0 * (1.0 + count as f64).ln()) as u64
}
pub fn search(
    entries: &[Entry],
    query: &str,
    state: &State,
    descriptions: bool,
    fuzzy: bool,
    limit: usize,
) -> Vec<usize> {
    if query.chars().count() > 512 || query.contains('\0') {
        return vec![];
    }
    let q = normalize(query.trim());
    let compact_query = q.replace(' ', "");
    let word_query = format!(" {q}");
    let mut matches: Vec<_> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            // Dozens of settings pages would otherwise fill the default view.
            if q.is_empty() && e.setting {
                return None;
            }
            let mut spread = 0;
            let tier = if q.is_empty() {
                0
            } else if e.normalized == q {
                8
            } else if e.normalized.starts_with(&q) {
                7
            } else if e.normalized.contains(&word_query) {
                6
            } else if e.normalized.contains(&q) {
                5
            } else if e.acronym.starts_with(&q) {
                4
            } else if let Some(end) = fuzzy
                .then(|| subsequence(&compact_query, &e.normalized))
                .flatten()
            {
                spread = end;
                3
            } else if q.split_whitespace().all(|part| e.fields.contains(part)) {
                2
            } else if descriptions && e.normalized_description.contains(&q) {
                1
            } else {
                return None;
            };
            let usage = state.usage.get(&e.id).copied().unwrap_or_default();
            Some((
                i,
                tier,
                std::cmp::Reverse(spread),
                state.favorites.contains(&e.id),
                frequency(usage.count),
                usage.last,
            ))
        })
        .collect();
    matches.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.2.cmp(&a.2))
            .then(b.3.cmp(&a.3))
            .then(b.4.cmp(&a.4))
            .then(b.5.cmp(&a.5))
            .then(entries[a.0].setting.cmp(&entries[b.0].setting))
            .then(entries[a.0].normalized.cmp(&entries[b.0].normalized))
            .then(entries[a.0].id.cmp(&entries[b.0].id))
    });
    matches.into_iter().take(limit).map(|m| m.0).collect()
}

/// Whether a displayed string and its normalized form line up one char to one
/// char, so positions found in the normalized text apply to what is shown.
/// Case folding can change the length ("Straße" is "strasse"); such strings
/// are left unhighlighted and uncompleted rather than marked in the wrong place.
fn aligned(shown: &str, normalized: &str) -> bool {
    shown.chars().count() == normalized.chars().count()
}

fn char_index(text: &str, byte: usize) -> usize {
    text[..byte].chars().count()
}

/// Merge ascending char indices into ranges.
fn coalesce(indices: impl IntoIterator<Item = usize>) -> Vec<std::ops::Range<usize>> {
    let mut out: Vec<std::ops::Range<usize>> = vec![];
    for i in indices {
        match out.last_mut() {
            Some(range) if range.end == i => range.end = i + 1,
            _ => out.push(i..i + 1),
        }
    }
    out
}

/// Char positions where a greedy in-order match of `query` lands in `text`.
fn subsequence_positions(query: &str, text: &str) -> Option<Vec<usize>> {
    let mut chars = text.chars().enumerate();
    query
        .chars()
        .map(|q| chars.find(|&(_, c)| c == q).map(|(i, _)| i))
        .collect()
}

/// Char ranges of the entry's name that `query` matched, using the same
/// order of match kinds as `search` (prefix, word start, substring, acronym,
/// fuzzy) so the marked letters are the reason the row was found.
pub fn highlight_name(e: &Entry, query: &str, fuzzy: bool) -> Vec<std::ops::Range<usize>> {
    let q = normalize(query.trim());
    if q.is_empty() || query.chars().count() > 512 || !aligned(&e.name, &e.normalized) {
        return vec![];
    }
    let n = &e.normalized;
    let len = q.chars().count();
    let span = |byte: usize| {
        let start = char_index(n, byte);
        std::iter::once(start..start + len).collect::<Vec<_>>()
    };
    if n.starts_with(&q) {
        span(0)
    } else if let Some(byte) = n.find(&format!(" {q}")) {
        span(byte + 1)
    } else if let Some(byte) = n.find(&q) {
        span(byte)
    } else if e.acronym.starts_with(&q) {
        let mut previous_space = true;
        let mut starts = vec![];
        for (i, c) in n.chars().enumerate() {
            if previous_space && !c.is_whitespace() {
                starts.push(i);
            }
            previous_space = c.is_whitespace();
        }
        coalesce(starts.into_iter().take(len))
    } else if fuzzy {
        subsequence_positions(&q.replace(' ', ""), n).map_or(vec![], coalesce)
    } else {
        vec![]
    }
}

/// Char ranges of the description that contain `query`.
pub fn highlight_description(e: &Entry, query: &str) -> Vec<std::ops::Range<usize>> {
    let q = normalize(query.trim());
    let d = &e.normalized_description;
    if q.is_empty() || query.chars().count() > 512 || !aligned(&e.description, d) {
        return vec![];
    }
    let len = q.chars().count();
    d.match_indices(&q)
        .map(|(byte, _)| {
            let start = char_index(d, byte);
            start..start + len
        })
        .collect()
}

/// The rest of the entry's name to offer as inline autocomplete after the
/// user's `typed` text, when the name starts with what was typed. The typed
/// text keeps its own capitalisation. Settings pages never complete: they are
/// reached by searching, not by rewriting what the user typed.
pub fn completion(e: &Entry, typed: &str) -> Option<String> {
    let q = normalize(typed);
    if q.trim().is_empty()
        || typed.chars().count() > 512
        || e.setting
        || !aligned(&e.name, &e.normalized)
        || !e.normalized.starts_with(&q)
    {
        return None;
    }
    let rest: String = e.name.chars().skip(q.chars().count()).collect();
    (!rest.is_empty()).then_some(rest)
}

/// Pango markup for `text` with `ranges` (char positions) emphasised: bold on
/// a translucent wash of the accent colour, so contrast with the text holds
/// for any accent. A malformed accent falls back to bold alone.
pub fn highlighted_markup(text: &str, ranges: &[std::ops::Range<usize>], accent: &str) -> String {
    let valid = accent.len() == 7
        && accent.starts_with('#')
        && accent[1..].chars().all(|c| c.is_ascii_hexdigit());
    let open = if valid {
        format!("<span weight=\"bold\" background=\"{accent}\" background_alpha=\"35%\">")
    } else {
        "<span weight=\"bold\">".to_string()
    };
    let mut out = String::new();
    let mut on = false;
    for (i, c) in text.chars().enumerate() {
        let hit = ranges.iter().any(|r| r.contains(&i));
        if hit != on {
            out.push_str(if hit { &open } else { "</span>" });
            on = hit;
        }
        out.push_str(&glib::markup_escape_text(&c.to_string()));
    }
    if on {
        out.push_str("</span>");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(name: &str) -> Entry {
        Entry::new(
            name.into(),
            name.into(),
            "browser".into(),
            String::new(),
            "internet".into(),
        )
    }
    #[test]
    fn matching() {
        let entries = vec![
            entry("Firefox"),
            entry("Visual Studio Code"),
            entry("Brave Browser"),
            entry("Éditeur"),
        ];
        for (q, expected) in [
            ("fire", 0),
            ("vsc", 1),
            ("brve", 2),
            ("ÉDI", 3),
            ("internet", 2),
        ] {
            let found = search(&entries, q, &State::default(), true, true, 8);
            assert!(found.contains(&expected), "{q}");
        }
        assert!(search(&entries, "absent", &State::default(), true, true, 8).is_empty());
    }
    #[test]
    fn exact_beats_favorite() {
        let entries = vec![entry("Code"), entry("Code Editor")];
        let mut state = State::default();
        state.favorites.insert("Code Editor".into());
        assert_eq!(search(&entries, "code", &state, true, true, 8)[0], 0);
        assert_eq!(search(&entries, "", &state, true, true, 8)[0], 1);
    }

    #[test]
    fn empty_view_orders_by_frequency_then_recency() {
        fn usage(count: u64, last: u64) -> crate::state::Usage {
            crate::state::Usage { count, last }
        }
        let entries = vec![entry("Alpha"), entry("Beta"), entry("Gamma")];
        let state = State {
            usage: [
                ("Alpha".to_string(), usage(5, 300)),
                ("Beta".to_string(), usage(9, 100)),
                ("Gamma".to_string(), usage(5, 400)),
            ]
            .into_iter()
            .collect(),
            ..State::default()
        };
        assert_eq!(search(&entries, "", &state, true, true, 8), vec![1, 2, 0]);
    }

    #[test]
    fn unicode_casefold_and_limits() {
        let entries = vec![entry("Straße"), entry("Éditeur")];
        assert_eq!(
            search(&entries, "STRASSE", &State::default(), true, true, 8),
            vec![0]
        );
        assert_eq!(
            search(&entries, "E\u{301}DI", &State::default(), true, true, 8),
            vec![1]
        );
        assert!(search(&entries, &"x".repeat(513), &State::default(), true, true, 8).is_empty());
        assert!(search(&entries, "", &State::default(), true, true, 0).is_empty());
    }

    #[test]
    fn word_start_and_acronym_beat_loose_fuzzy() {
        let entries = vec![
            entry("Fantastic Fox Browser"),
            entry("Visual Studio Code"),
            entry("Studio Tools"),
            entry("Bastudio"),
        ];
        let found = |q| search(&entries, q, &State::default(), false, true, 8);
        // Word start ("Visual Studio") beats mid-word ("Bastudio").
        assert_eq!(found("studio"), vec![2, 1, 3]);
        // Acronym beats fuzzy; "ff" fuzzy-matches only the loose candidate.
        assert_eq!(found("vsc")[0], 1);
        assert_eq!(found("ffb"), vec![0]);
    }

    #[test]
    fn tighter_fuzzy_match_ranks_first() {
        let entries = vec![entry("Alpha Beta Gamma Delta"), entry("Adelphi")];
        let found = search(&entries, "adl", &State::default(), false, true, 8);
        assert_eq!(found, vec![1, 0]);
    }

    #[test]
    fn heavy_usage_does_not_saturate() {
        fn usage(count: u64) -> crate::state::Usage {
            crate::state::Usage { count, last: 1 }
        }
        let entries = vec![entry("Alpha"), entry("Beta")];
        let state = State {
            usage: [
                ("Alpha".to_string(), usage(1000)),
                ("Beta".to_string(), usage(5000)),
            ]
            .into_iter()
            .collect(),
            ..State::default()
        };
        assert_eq!(search(&entries, "", &state, true, true, 8), vec![1, 0]);
    }

    fn setting(name: &str) -> Entry {
        Entry {
            setting: true,
            ..entry(name)
        }
    }

    #[test]
    fn settings_are_searchable_but_not_listed_by_default() {
        let entries = vec![
            entry("Mousepad"),
            setting("Mouse"),
            setting("About this System"),
        ];
        let state = State::default();
        // The empty view lists applications only.
        assert_eq!(search(&entries, "", &state, true, true, 8), vec![0]);
        // A query finds the settings page ("Mouse" is the exact match).
        assert_eq!(search(&entries, "mouse", &state, true, true, 8), vec![1, 0]);
        assert!(search(&entries, "about", &state, true, true, 8).contains(&2));
    }

    #[test]
    fn applications_rank_before_equal_settings() {
        // Same tier, no usage: the application wins even though it sorts later by name.
        let entries = vec![setting("Network Settings"), entry("Network Tools")];
        let found = search(&entries, "network", &State::default(), true, true, 8);
        assert_eq!(found, vec![1, 0]);
    }

    fn marked(name: &str, query: &str, fuzzy: bool) -> String {
        let e = entry(name);
        let ranges = highlight_name(&e, query, fuzzy);
        name.chars()
            .enumerate()
            .map(|(i, c)| {
                if ranges.iter().any(|r| r.contains(&i)) {
                    c.to_uppercase().next().unwrap()
                } else {
                    c.to_lowercase().next().unwrap()
                }
            })
            .collect()
    }

    #[test]
    fn highlights_follow_the_kind_of_match() {
        // Prefix, word start, substring.
        assert_eq!(marked("Firefox", "fire", true), "FIREfox");
        assert_eq!(
            marked("Visual Studio Code", "studio", true),
            "visual STUDIO code"
        );
        assert_eq!(marked("Bastudio", "stud", true), "baSTUDio");
        // Acronym marks the first letter of each word; fuzzy marks the letters in order.
        assert_eq!(
            marked("Visual Studio Code", "vsc", true),
            "Visual Studio Code"
        );
        assert_eq!(marked("Brave Browser", "brve", true), "BRaVE browser");
        // Fuzzy off: a loose subsequence is not a match, so nothing is marked.
        assert_eq!(marked("Brave Browser", "brve", false), "brave browser");
        // Case, accents and surrounding spaces do not matter.
        assert_eq!(marked("Éditeur", "  édi ", true), "ÉDIteur");
        // Empty and unmatched queries mark nothing.
        assert!(highlight_name(&entry("Firefox"), "", true).is_empty());
        assert!(highlight_name(&entry("Firefox"), "zzz", true).is_empty());
    }

    #[test]
    fn highlights_skip_names_whose_case_folding_changes_length() {
        // "Straße" folds to "strasse": positions would not line up with what is shown.
        assert!(highlight_name(&entry("Straße"), "stra", true).is_empty());
        assert!(highlight_description(&app_with_description("Straße"), "stra").is_empty());
        assert_eq!(completion(&entry("Straße"), "stra"), None);
    }

    fn app_with_description(description: &str) -> Entry {
        Entry::new(
            "id".into(),
            "Name".into(),
            description.into(),
            String::new(),
            String::new(),
        )
    }

    #[test]
    fn description_matches_are_marked_wherever_they_occur() {
        let e = app_with_description("Edit video, then export Video");
        assert_eq!(highlight_description(&e, "video"), vec![5..10, 24..29]);
        assert!(highlight_description(&e, "audio").is_empty());
        assert!(highlight_description(&e, " ").is_empty());
    }

    #[test]
    fn completion_extends_only_names_that_start_with_the_text() {
        let e = entry("Firefox");
        // The typed capitalisation is kept; only the remainder comes from the name.
        assert_eq!(completion(&e, "fire").as_deref(), Some("fox"));
        assert_eq!(completion(&e, "FIRE").as_deref(), Some("fox"));
        assert_eq!(
            completion(&entry("Visual Studio Code"), "visual s").as_deref(),
            Some("tudio Code")
        );
        // Nothing left to add, empty or blank text, no prefix, leading space.
        assert_eq!(completion(&e, "firefox"), None);
        assert_eq!(completion(&e, ""), None);
        assert_eq!(completion(&e, "  "), None);
        assert_eq!(completion(&e, "ire"), None);
        assert_eq!(completion(&e, " fire"), None);
        // Word-start, acronym and fuzzy matches never rewrite the text.
        let code = entry("Visual Studio Code");
        for q in ["studio", "vsc", "vsco"] {
            assert_eq!(completion(&code, q), None, "{q}");
        }
        assert_eq!(completion(&e, &"f".repeat(513)), None);
    }

    #[test]
    fn settings_pages_never_complete() {
        assert_eq!(completion(&setting("Screen Locking"), "screen"), None);
        assert_eq!(
            completion(&entry("Screen Locking"), "screen").as_deref(),
            Some(" Locking")
        );
    }

    #[test]
    fn markup_escapes_text_and_marks_ranges() {
        let open = "<span weight=\"bold\" background=\"#12abef\" background_alpha=\"35%\">";
        assert_eq!(
            highlighted_markup("A&B <C>", &[0..1, 5..7], "#12abef"),
            format!("{open}A</span>&amp;B &lt;{open}C&gt;</span>")
        );
        // A malformed accent must not produce invalid markup.
        assert_eq!(
            highlighted_markup("ab", &[0..1, 5..9], "not a colour"),
            "<span weight=\"bold\">a</span>b"
        );
        assert_eq!(highlighted_markup("plain", &[], "#12abef"), "plain");
    }

    #[test]
    fn query_limit_counts_characters_not_utf8_bytes() {
        let name = "界".repeat(200);
        let entries = vec![entry(&name)];
        assert_eq!(
            search(&entries, &name, &State::default(), true, true, 8),
            vec![0]
        );
    }
}
