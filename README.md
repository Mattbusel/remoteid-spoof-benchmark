# remoteid-spoof-benchmark

**Most Remote ID receivers believe every broadcast. This is the test corpus
that proves whether yours does. Pass it, or you shipped a nightlight.**

A drone's Remote ID beacon is an unauthenticated, unencrypted broadcast that
anyone can replay, clone or flood. A receiver that simply decodes and trusts
every frame will happily draw you five hundred drones that are not there, and
lose the one that is. This repository is the standardized, open test corpus
that measures whether a counter-UAS detector actually catches that, with a
scoring harness and a public leaderboard.

It is to a Remote ID detector what a fuzzer corpus or the EICAR test file is to
the software it exercises: known inputs with a known right answer.

## This is test data, not a weapon

This repository is **data only**, for testing detection software you control.

- **No radio, no transmit path.** The corpus is a set of capture files. Nothing
  here opens an SDR, a Bluetooth or Wi-Fi adapter, or any device, and nothing
  broadcasts. The frames are built in memory with the published
  [`remoteid`](https://crates.io/crates/remoteid) encoders and written to a
  file, exactly as [`remoteid-scenario`](https://crates.io/crates/remoteid-scenario)
  does.
- **No spoofing how-to and no evasion guidance.** The corpus exists to make a
  *defender's* detector provably reject spoofed and flooded broadcasts. It does
  not teach anyone to transmit into real airspace.
- **The ground truth is the point.** Every scenario ships with a manifest that
  says which track is real and which are fake, so a detector's output can be
  scored automatically.

## The corpus

Six named scenarios live in [`corpus/`](corpus/), each written as both a RIDF
capture (`*.ridf`, the native [`remoteid`] capture format) and JSON lines
(`*.jsonl`, one decoded message per line). The ground truth for all of them is
in [`corpus/manifest.json`](corpus/manifest.json).

| scenario          | what the airspace contains                                                            | a correct detector must...                        |
|-------------------|---------------------------------------------------------------------------------------|---------------------------------------------------|
| `clean_baseline`  | 6 honest drones, distinct sources, sane moving tracks                                 | stay quiet (no flood call)                         |
| `single_spoof`    | one cloned identity broadcast from two transmitters at incompatible positions         | flag the spoof                                     |
| `cloned_id`       | 40 transmitters all broadcasting one identical identity, plus one real drone          | flag the flood **and name the real drone**         |
| `templated_flood` | 500 templated fakes (shared source, sequential serials, lockstep counters) + 1 real   | flag the flood **and name the real drone**         |
| `colocated`       | 12 tracks sharing one operator position in a tight position / signal cluster          | fire the co-location alarm                          |
| `count_spike`     | 60 transmitters appearing within one second from distinct addresses                   | fire the onset / count-spike alarm                 |

## The metrics

The scorer, [`score.py`](score.py) (Python 3, standard library only), compares
one detector's submission against the manifest and reports three numbers:

- **Flood detection** — over every scenario whose ground truth is an attack,
  the fraction where the detector raised a flood or spoof alarm. *Higher is
  better; 100% is a pass.*
- **Real-drone accuracy** — over every scenario that hides one genuine drone
  (`cloned_id`, `templated_flood`), the fraction where the detector named
  **exactly** that drone by its transmitter address. *Higher is better; 100%
  is a pass.*
- **False-positive rate** — over the clean baseline, the fraction where the
  detector cried flood on honest traffic. *Lower is better; 0% is a pass.*

A detector **passes** the benchmark when flood detection is 100%, real-drone
accuracy is 100%, and the false-positive rate is 0%.

## Leaderboard

| detector | version | flood detection | real-drone accuracy | false-positive rate | result |
|----------|---------|-----------------|---------------------|---------------------|--------|
| [remoteid-sentinel](https://crates.io/crates/remoteid-sentinel) | 0.1.0 | **100%** (5/5) | **100%** (2/2) | **0%** (0/1) | **PASS** |

*Baseline produced by a real run of `remoteid-sentinel` over this exact corpus
(`cargo run --release --bin baseline-sentinel`), scored by `score.py`. The
submission is in [`submissions/sentinel-baseline.json`](submissions/sentinel-baseline.json).*

Per-scenario detail for the baseline (the sentinel reports a graded level and a
0-to-1 confidence):

| scenario          | sentinel level | confidence | alarm | named real drone      |
|-------------------|----------------|-----------:|-------|-----------------------|
| `clean_baseline`  | elevated       | 0.50       | no    | -                     |
| `single_spoof`    | flooded        | 1.00       | yes   | -                     |
| `cloned_id`       | flooded        | 1.00       | yes   | `02:00:00:00:77:77`   |
| `templated_flood` | flooded        | 1.00       | yes   | `02:00:00:00:77:77`   |
| `colocated`       | flooded        | 1.00       | yes   | -                     |
| `count_spike`     | flooded        | 1.00       | yes   | -                     |

On the clean baseline the sentinel reaches **elevated**, its soft "watch, do
not yet treat every track as fake" state, not a flood call. The benchmark
counts only a definitive **flooded** verdict as an alarm, so elevated on honest
traffic is scored as no false flood. (The elevated reading here is driven by
the baseline scenario's honest drones all carrying serials from one prefix
block, which the sentinel notes as templated; a more varied honest fleet reads
as clear.)

## Submit your detector

The benchmark is detector-neutral. Your detector can be in any language and can
read either the RIDF or the JSONL captures. To get on the leaderboard:

1. Run your detector over each scenario in `corpus/`.
2. Write a submission JSON in this shape:

   ```json
   {
     "detector": "your-detector",
     "detector_version": "1.0.0",
     "results": {
       "clean_baseline":   { "alarm": false, "named_real": null },
       "single_spoof":     { "alarm": true,  "named_real": null },
       "cloned_id":        { "alarm": true,  "named_real": "02:00:00:00:77:77" },
       "templated_flood":  { "alarm": true,  "named_real": "02:00:00:00:77:77" },
       "colocated":        { "alarm": true,  "named_real": null },
       "count_spike":      { "alarm": true,  "named_real": null }
     }
   }
   ```

   - `alarm` is your detector's flood/spoof call for that scenario.
   - `named_real` is the BLE advertising address (lower-case, colon-separated)
     of the one drone your detector believes is genuine, or `null` when there
     is no single real drone to name. Use the addresses as they appear in the
     capture.

3. Score it:

   ```
   python score.py submissions/your-detector.json
   ```

4. Open a pull request adding `submissions/your-detector.json` and your
   leaderboard row, **or** open an issue with your submission JSON and we will
   run it. We re-run every submission against the corpus before listing it, so
   the numbers on the board are numbers we reproduced.

No cherry-picking: a listed detector is scored on the whole corpus, clean
baseline included.

## Reproduce the corpus and the baseline

Everything is built from published crates.

```
# Regenerate the corpus files and manifest from the scenario definitions.
cargo run --release --bin generate-corpus

# Run the reference detector (remoteid-sentinel) and write its submission.
cargo run --release --bin baseline-sentinel

# Score it.
python score.py submissions/sentinel-baseline.json
```

The corpus is committed, so you can score your own detector without a Rust
toolchain: you only need `score.py` and the files in `corpus/`.

## Built on

- [`remoteid`](https://crates.io/crates/remoteid) — ASTM F3411 / DJI DroneID
  decode and per-unit trust grading.
- [`remoteid-scenario`](https://crates.io/crates/remoteid-scenario) — the
  test-vector generator the corpus is built with.
- [`remoteid-sentinel`](https://crates.io/crates/remoteid-sentinel) — the
  receive-only flood and spoof analyzer that holds the baseline.

More drone-defense demos: <https://mattbusel.github.io/drone-demos/> ·
Background on why Remote ID trust is hard: <https://mattbusel.github.io/remote-id-truth/>

## License

MIT OR Apache-2.0.
