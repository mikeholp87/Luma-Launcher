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
            .then(entries[a.0].normalized.cmp(&entries[b.0].normalized))
            .then(entries[a.0].id.cmp(&entries[b.0].id))
    });
    matches.into_iter().take(limit).map(|m| m.0).collect()
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
