//! Generates the benchmark corpus: for every named scenario it writes a RIDF
//! capture and a JSON-lines capture into `corpus/`, and a single
//! `corpus/manifest.json` holding the ground truth for each.
//!
//! Test data only. This binary builds synthetic message streams in memory and
//! writes them to files. It opens no radio and broadcasts nothing.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use remoteid::Message;
use remoteid_scenario::{generate, ScenarioWriter};
use remoteid_spoof_benchmark::{corpus, mac, BenchScenario};
use serde_json::json;

fn main() -> std::io::Result<()> {
    let out_dir = Path::new("corpus");
    std::fs::create_dir_all(out_dir)?;

    let mut entries = Vec::new();

    for b in corpus() {
        let frames = generate(&b.scenario);

        // Write the two capture formats a detector ingests.
        let ridf_path = out_dir.join(format!("{}.ridf", b.name));
        let jsonl_path = out_dir.join(format!("{}.jsonl", b.name));
        let writer = ScenarioWriter::new(frames.clone());
        {
            let mut f = File::create(&ridf_path)?;
            writer.to_ridf(&mut f)?;
        }
        {
            let mut f = File::create(&jsonl_path)?;
            writer.to_jsonl(&mut f)?;
        }

        // Derive ground truth from the generated frames so the manifest can
        // never drift from the corpus files.
        let mut transmitters: BTreeSet<String> = BTreeSet::new();
        let mut real_mac: Option<String> = None;
        for fr in &frames {
            if let Some(m) = mac(&fr.transmitter) {
                transmitters.insert(m.clone());
                if let Message::BasicId(bid) = &fr.message {
                    if let (Some(real), Some(id)) = (b.real_serial, bid.uas_id_str()) {
                        if id == real {
                            real_mac = Some(m);
                        }
                    }
                }
            }
        }

        if b.real_serial.is_some() && real_mac.is_none() {
            panic!(
                "scenario {}: expected a genuine serial {:?} but no transmitter emitted it",
                b.name, b.real_serial
            );
        }

        println!(
            "{:16} frames={:6} transmitters={:3} expect_alarm={} real={}",
            b.name,
            frames.len(),
            transmitters.len(),
            b.expect_alarm,
            real_mac.as_deref().unwrap_or("-"),
        );

        entries.push(manifest_entry(&b, &transmitters, real_mac));
    }

    let manifest = json!({
        "benchmark": "remoteid-spoof-benchmark",
        "version": "0.1.0",
        "about": "Standardized Remote ID spoof and flood detection test corpus with ground truth. Test data only: no radio, no transmit path.",
        "generated_with": {
            "remoteid": "0.3.3",
            "remoteid-scenario": "0.1.0"
        },
        "address_format": "lower-case colon-separated BLE advertising address (MAC), as a detector reading the capture sees it",
        "fields": {
            "expect_alarm": "a correct detector must raise a flood or spoof alarm (false only for the clean baseline)",
            "real_transmitter": "the MAC of the one genuine drone a detector must name, or null when there is no single real drone to find",
            "all_transmitters": "every transmitter address present in the capture"
        },
        "scenarios": entries
    });

    let manifest_path = out_dir.join("manifest.json");
    let mut f = File::create(&manifest_path)?;
    f.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    f.write_all(b"\n")?;
    println!("\nwrote {}", manifest_path.display());
    Ok(())
}

fn manifest_entry(
    b: &BenchScenario,
    transmitters: &BTreeSet<String>,
    real_mac: Option<String>,
) -> serde_json::Value {
    json!({
        "name": b.name,
        "description": b.description,
        "kind": b.scenario.kind.name(),
        "fake_count": b.scenario.fake_count,
        "files": {
            "ridf": format!("corpus/{}.ridf", b.name),
            "jsonl": format!("corpus/{}.jsonl", b.name)
        },
        "expect_alarm": b.expect_alarm,
        "real_transmitter": real_mac,
        "all_transmitters": transmitters.iter().cloned().collect::<Vec<_>>()
    })
}
