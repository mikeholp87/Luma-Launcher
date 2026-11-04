use luma_launcher::{
    search::{search, Entry},
    state::State,
};
fn main() {
    for n in [100, 1000, 10000] {
        let entries: Vec<_> = (0..n)
            .map(|i| {
                Entry::new(
                    format!("app-{i}.desktop"),
                    format!("Application {i}"),
                    "Synthetic application".into(),
                    String::new(),
                    "utility tool".into(),
                )
            })
            .collect();
        let state = State::default();
        let mut samples = Vec::new();
        for _ in 0..100 {
            let now = std::time::Instant::now();
            std::hint::black_box(search(&entries, "app 99", &state, true, true, 8));
            samples.push(now.elapsed().as_micros());
        }
        samples.sort();
        println!(
            "entries={n} search_p50_us={} search_p95_us={}",
            samples[50], samples[95]
        );
    }
}
