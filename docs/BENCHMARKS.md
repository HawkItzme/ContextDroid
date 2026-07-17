# ContextDroid alpha benchmarks

## Method

The corpus is synthetic or redistributable and contains no secrets, personal data, or
proprietary identifiers. Estimated tokens are `ceil(UTF-8 bytes / 4)`; they are not exact model
token counts. Direct reduction compares raw command output with returned output. Effective
reduction also counts recoveries and detectable reruns. Neither metric represents complete
model-session billing.

The fixture benchmark runs parser plus never-worse rendering 1,000 times in a Rust test and
reports average microseconds. Preservation is established by independent typed-field, rendered
evidence, exit/signal, durable-ID, and byte-exact recovery assertions. Run it with:

```text
cargo test cmds::android::contract_tests::emit_alpha_fixture_benchmark --locked -- --ignored --nocapture
```

Machine-readable data is in `benchmarks/alpha-v0.1.0-alpha.1.json`.

## Measured fixture results (Windows, 2026-07-17)

| Case | Raw est. tokens | Returned est. tokens | Direct / effective reduction | Confidence | Decision | Avg parser + render |
| --- | ---: | ---: | ---: | --- | --- | ---: |
| Gradle success | 13 | 13 | 0.0% / 0.0% | high | raw incomplete evidence | 75 µs |
| Kotlin failure | 281 | 113 | 59.8% / 59.8% | high | semantic | 684 µs |
| Resource merge failure | 21 | 21 | 0.0% / 0.0% | high | raw not smaller | 97 µs |
| Manifest failure | 25 | 25 | 0.0% / 0.0% | high | raw not smaller | 103 µs |
| Unit-test failure | 33 | 33 | 0.0% / 0.0% | high | raw not smaller | 150 µs |
| Logcat Java crash | 92 | 92 | 0.0% / 0.0% | high | raw not smaller | 499 µs |

The small fixtures deliberately show the never-worse behavior: five cases return raw because a
semantic result would be larger or incomplete. Only the expanded Kotlin fixture selects semantic
output. No raw recovery or rerun occurred in this controlled fixture run, so direct and effective
results are equal.

## General-command evidence

A ten-run `git status --short` comparison in the clean pinned public Android sample with one
synthetic untracked file returned 35 bytes raw and 35 bytes through the release build (9 estimated
tokens, 0% reduction, exact status evidence). Median elapsed time was 73.488 ms raw and 115.260 ms
through ContextDroid on this Windows host. This small local sample is not a universal general-command
performance claim.

## Remaining evidence

The public `architecture-samples` CI job passed for the release candidate and records real
Gradle/AGP/Kotlin bytes, estimated tokens, latency, exit parity, and preservation. The permissioned
internal pilot also passed and records recoveries and reruns. These results provide release
correctness evidence, but the current corpus is not broad enough for a representative aggregate
compression claim. ContextDroid does not reuse upstream RTK percentages and does not claim to
outperform RTK or raw tools for every general command.

Correctness gates block release; compression percentage does not.
