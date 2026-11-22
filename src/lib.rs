//! The Remote ID spoof and flood detection benchmark: scenario definitions
//! and ground truth shared by the corpus generator and the sentinel baseline.
//!
//! This crate is **test data only**. It builds synthetic Remote ID message
//! streams with [`remoteid_scenario`] 0.1 (which itself uses the published
//! [`remoteid`] encoders) and hands a detector a known input with a known
//! right answer. It opens no radio, broadcasts nothing, and contains no
//! transmit path and no evasion guidance. It is to a counter-UAS detector
//! what a fuzzer corpus or the EICAR test file is to the software it
//! exercises.

use remoteid::Transmitter;
use remoteid_scenario::{
    AddressMode, CounterMode, GenuineTrack, Scenario, ScenarioKind, SerialTemplate, SourceModel,
    Timing,
};

/// One named benchmark scenario: a scenario description plus the ground truth
/// a correct detector should produce for it.
pub struct BenchScenario {
    /// Stable file-name / submission key, for example `templated_flood`.
    pub name: &'static str,
    /// One-line description of what the airspace contains.
    pub description: &'static str,
    /// The underlying scenario the corpus is built from.
    pub scenario: Scenario,
    /// Ground truth: should a correct detector raise an alarm (flood or
    /// spoof) for this airspace? `false` only for the clean baseline.
    pub expect_alarm: bool,
    /// Ground truth: the ANSI/CTA-2063-A serial of the one genuine drone a
    /// detector must pick out of the crowd, or `None` when there is no single
    /// real drone to name (a pure clone, a spike of fakes, a clean field).
    pub real_serial: Option<&'static str>,
}

/// The canonical MAC-style address string for a transmitter, lower case and
/// colon separated, as a detector reading the BLE advertising address sees
/// it. This is the identifier the manifest and submissions name tracks by.
pub fn mac(tx: &Transmitter) -> Option<String> {
    let a = match tx {
        Transmitter::Ble(a) | Transmitter::Wifi(a) => a,
        Transmitter::Other(_) => return None,
    };
    Some(format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        a[0], a[1], a[2], a[3], a[4], a[5]
    ))
}

// Genuine serials: valid ANSI/CTA-2063-A strings (4 char manufacturer code,
// one length character, then that many body characters) so they parse
// unchanged and are easy to pick out of the templated `FAKE` block.
const REAL_FLOOD: &str = "1581CREAL7A4F2C91"; // code 1581, len C (12), body REAL7A4F2C91
const REAL_CLONE_FIELD: &str = "1581CREAL9Z8Y7X65";

/// The full benchmark corpus: six named scenarios covering a clean baseline,
/// a single spoof, a cloned identity in a crowd, a large templated flood with
/// one real drone, co-located operators, and a count spike.
pub fn corpus() -> Vec<BenchScenario> {
    vec![
        // 1. Clean baseline: honest drones only. A detector must stay quiet.
        BenchScenario {
            name: "clean_baseline",
            description: "6 honest drones from distinct sources, sane moving tracks. No flood, no spoof. A detector must report clear.",
            scenario: Scenario::new(ScenarioKind::Clean, 6),
            expect_alarm: false,
            real_serial: None,
        },
        // 2. Single spoof: one identity, two transmitters, two positions.
        BenchScenario {
            name: "single_spoof",
            description: "One cloned identity broadcast from two transmitters at incompatible positions. A detector must flag the spoof. No lone real drone to name.",
            scenario: Scenario::new(ScenarioKind::Spoof, 1),
            expect_alarm: true,
            real_serial: None,
        },
        // 3. Cloned ID flood: dozens of copies of one fixed identity, plus
        //    one embedded real drone.
        BenchScenario {
            name: "cloned_id",
            description: "40 fakes all broadcasting the same fixed identity from one source, plus one genuine moving drone. A detector must flag the flood and name the real drone.",
            scenario: cloned_id_scenario(40),
            expect_alarm: true,
            real_serial: Some(REAL_CLONE_FIELD),
        },
        // 4. Templated flood: hundreds of sequential fakes + one real drone.
        BenchScenario {
            name: "templated_flood",
            description: "500 templated fakes (shared source, sequential serials, lockstep counters, one operator position) with one genuine moving drone hidden inside. A detector must flag the flood and name the real drone.",
            scenario: templated_flood_scenario(500),
            expect_alarm: true,
            real_serial: Some(REAL_FLOOD),
        },
        // 5. Co-located operators: many tracks under one pilot position.
        BenchScenario {
            name: "colocated",
            description: "12 tracks sharing one operator position in a tight position and signal cluster. A detector must fire the co-location alarm. All fake.",
            scenario: Scenario::new(ScenarioKind::Colocated, 12),
            expect_alarm: true,
            real_serial: None,
        },
        // 6. Count spike: a burst of transmitters appearing at once.
        BenchScenario {
            name: "count_spike",
            description: "60 transmitters appearing within one second from distinct addresses. A detector must fire the onset / count-spike alarm. All fake.",
            scenario: Scenario::new(ScenarioKind::CountSpike, 60),
            expect_alarm: true,
            real_serial: None,
        },
    ]
}

/// A templated flood of `n` sequential fakes with one embedded real drone.
fn templated_flood_scenario(n: u32) -> Scenario {
    let mut s = Scenario::new(ScenarioKind::Flood, n);
    s.genuine = GenuineTrack {
        serial: REAL_FLOOD.to_string(),
        ..GenuineTrack::default()
    };
    s
}

/// A cloned-identity flood: `n` fakes all sharing one fixed serial, plus one
/// embedded real drone.
fn cloned_id_scenario(n: u32) -> Scenario {
    let mut s = Scenario::new(ScenarioKind::Flood, n);
    s.template = SerialTemplate {
        manufacturer_code: "1581".to_string(),
        prefix: "CLONE".to_string(),
        sequential: false,
        fixed_suffix: "0001".to_string(),
    };
    // Per-unit addresses: the cloned identity is broadcast from many distinct
    // transmitters at once, which is what makes it a detectable clone rather
    // than one source repeating itself. Frozen counters across the fleet add
    // the shared-counter tell.
    s.source = SourceModel {
        address_mode: AddressMode::PerUnit,
        shared_tsf: false,
        counter_mode: CounterMode::Frozen,
    };
    s.timing = Timing {
        frames_per_s: 2,
        window_s: 10,
    };
    s.genuine = GenuineTrack {
        serial: REAL_CLONE_FIELD.to_string(),
        ..GenuineTrack::default()
    };
    s
}
