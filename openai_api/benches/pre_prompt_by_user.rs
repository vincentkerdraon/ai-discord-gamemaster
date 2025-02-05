use criterion::{black_box, criterion_group, criterion_main, Criterion};
use openai_api::PrePromptByUser;
use std::collections::HashMap;

//(This is a very stupid benchmark, but I am learning)

// $ cargo bench
//
// Running benches/pre_prompt_by_user.rs (target/release/deps/pre_prompt_by_user-e6ed003cfa4f5cfb)
// prompt known user       time:   [16.045 ns 16.047 ns 16.048 ns]
//                         change: [-0.1681% -0.1141% -0.0694%] (p = 0.00 < 0.05)
//                         Change within noise threshold.
// Found 3 outliers among 100 measurements (3.00%)
//   3 (3.00%) high mild
//
// prompt unknown user     time:   [15.515 ns 15.519 ns 15.525 ns]
//                         change: [-0.4031% -0.3776% -0.3500%] (p = 0.00 < 0.05)
//                         Change within noise threshold.
// Found 3 outliers among 100 measurements (3.00%)
//   1 (1.00%) low mild
//   1 (1.00%) high mild
//   1 (1.00%) high severe

fn bench_prompt(c: &mut Criterion) {
    let mut users = HashMap::new();
    users.insert(12345, "Hello, user 12345!".to_string());

    let pre_prompt = PrePromptByUser {
        default: "Default prompt".to_string(),
        users,
    };

    c.bench_function("prompt known user", |b| {
        b.iter(|| pre_prompt.prompt(black_box(&12345)))
    });

    c.bench_function("prompt unknown user", |b| {
        b.iter(|| pre_prompt.prompt(black_box(&99999)))
    });
}

// Criterion macro to generate the main benchmark function
criterion_group!(benches, bench_prompt);
criterion_main!(benches);
