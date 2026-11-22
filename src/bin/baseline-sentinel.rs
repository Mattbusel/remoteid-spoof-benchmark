//! Runs remoteid-sentinel against the benchmark corpus and writes its scored
//! submission to `submissions/sentinel-baseline.json`.
//!
//! This is the reference detector run that fills the leaderboard baseline. It
//! replays each scenario's already-decoded frames through the sentinel exactly
//! as a receive-only sensor would hand off what it heard. No radio, no
//! transmit path: the frames come from the in-memory corpus, not the air.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use remoteid::{Observation, Transmitter};
use remoteid_scenario::generate;
use remoteid_sentinel::{FieldAssessment, FloodLevel};
use remoteid_spoof_benchmark::{corpus, mac};
use serde_json::json;

fn main() -> std::io::Result<()> {
    let mut results = serde_json::Map::new();

    for b in corpus() {
        let frames = generate(&b.scenario);

        // Encode every frame to its on-the-wire payload once, owned, so the
        // borrowed observations below outlive the analysis call.
        // Flatten each frame to the bytes a receiver decodes, exactly as a
        // receive-only sensor would hand them off (Bluetooth 4, counter and
        // signal strength only). This mirrors the sentinel's own integration
        // path so the baseline reflects real detector behaviour.
        struct Enc {
            tx: Transmitter,
            payload: Vec<u8>,
            counter: u8,
            now_ms: u64,
            rssi_dbm: i8,
        }
        let mut encoded = Vec::with_capacity(frames.len());
        let mut last_ms = 0u64;
        for fr in &frames {
            let payload = match fr.message.encode() {
                Ok(p) => p.to_vec(),
                Err(_) => continue,
            };
            let now_ms = fr.time_us / 1000;
            last_ms = last_ms.max(now_ms);
            encoded.push(Enc {
                tx: fr.transmitter,
                payload,
                counter: fr.counter,
                now_ms,
                rssi_dbm: fr.rssi_dbm,
            });
        }

        let obs = encoded.iter().map(|e| {
            let o = Observation::new(
                remoteid::Transport::Bluetooth4,
                Some(e.counter),
                &e.payload,
                e.now_ms,
            )
            .with_rssi(e.rssi_dbm);
            (e.tx, o)
        });

        let field = FieldAssessment::from_observations(obs, last_ms + 1000);

        // Map the sentinel verdict to the benchmark submission shape. The
        // alarm is a definitive flood call: only the Flooded level counts.
        // Elevated is the sentinel's soft watch state ("do not yet treat every
        // track as fake"), which is deliberately not a flood declaration, so
        // the benchmark does not score it as one.
        let alarm = field.flood.level == FloodLevel::Flooded;
        let named_real = field
            .most_likely_real
            .as_ref()
            .and_then(|t| mac(&t.transmitter));

        println!(
            "{:16} level={:9} conf={:.2} alarm={} named_real={}",
            b.name,
            field.flood.level.name(),
            field.flood.confidence,
            alarm,
            named_real.as_deref().unwrap_or("-"),
        );

        results.insert(
            b.name.to_string(),
            json!({
                "alarm": alarm,
                "named_real": named_real,
                "flood_level": field.flood.level.name(),
                "confidence": (field.flood.confidence * 1000.0).round() / 1000.0,
                "tracks": field.tracks.len()
            }),
        );
    }

    let submission = json!({
        "detector": "remoteid-sentinel",
        "detector_version": "0.1.0",
        "notes": "Receive-only flood and spoof analyzer. Baseline run over the published corpus; alarm = a definitive Flooded verdict (the soft Elevated watch state is not scored as a flood call); named_real = the track the sentinel ranks as most likely the one genuine drone.",
        "results": results
    });

    let out_dir = Path::new("submissions");
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join("sentinel-baseline.json");
    let mut f = File::create(&path)?;
    f.write_all(serde_json::to_string_pretty(&submission)?.as_bytes())?;
    f.write_all(b"\n")?;
    println!("\nwrote {}", path.display());
    Ok(())
}
