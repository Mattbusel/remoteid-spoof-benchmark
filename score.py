#!/usr/bin/env python3
"""Score a detector submission against the Remote ID spoof/flood benchmark.

The benchmark is test data only: a synthetic corpus with known ground truth.
This script reads the ground-truth manifest and one detector's submission and
reports how well the detector did. It touches no radio and no network.

Submission format (JSON):

    {
      "detector": "my-detector",
      "detector_version": "1.2.3",
      "results": {
        "<scenario name>": {
          "alarm": true,              # did you raise a flood/spoof alarm?
          "named_real": "aa:bb:..."   # MAC of the one real drone, or null
        },
        ...
      }
    }

Metrics
  flood_detection     over every scenario whose ground truth is expect_alarm,
                      the fraction where the detector raised an alarm.
  real_id_accuracy    over every scenario that hides one genuine drone, the
                      fraction where the detector named exactly that drone.
  false_positive_rate over the clean baseline scenarios, the fraction where
                      the detector raised an alarm (lower is better; 0 is a
                      pass).
  result              PASS when flood_detection == 100%, real_id_accuracy ==
                      100% and false_positive_rate == 0%; otherwise FAIL.
"""

import argparse
import json
import sys


def load(path):
    with open(path, "r", encoding="utf-8") as fh:
        return json.load(fh)


def norm_mac(value):
    if value is None:
        return None
    return str(value).strip().lower()


def score(manifest, submission):
    results = submission.get("results", {})
    rows = []
    flood_total = flood_hit = 0
    real_total = real_hit = 0
    clean_total = clean_fp = 0

    for sc in manifest["scenarios"]:
        name = sc["name"]
        expect_alarm = bool(sc["expect_alarm"])
        real_tx = norm_mac(sc.get("real_transmitter"))

        entry = results.get(name, {})
        got_alarm = bool(entry.get("alarm", False))
        got_real = norm_mac(entry.get("named_real"))

        detail = {"scenario": name, "expect_alarm": expect_alarm,
                  "got_alarm": got_alarm}

        if expect_alarm:
            flood_total += 1
            if got_alarm:
                flood_hit += 1
            detail["alarm_ok"] = got_alarm
        else:
            clean_total += 1
            if got_alarm:
                clean_fp += 1
            detail["alarm_ok"] = not got_alarm

        if real_tx is not None:
            real_total += 1
            hit = got_real == real_tx
            if hit:
                real_hit += 1
            detail["real_transmitter"] = real_tx
            detail["named_real"] = got_real
            detail["real_ok"] = hit

        rows.append(detail)

    def rate(hit, total):
        return (hit / total) if total else None

    flood_detection = rate(flood_hit, flood_total)
    real_id_accuracy = rate(real_hit, real_total)
    false_positive_rate = rate(clean_fp, clean_total)

    passed = (
        (flood_detection in (None, 1.0))
        and (real_id_accuracy in (None, 1.0))
        and (false_positive_rate in (None, 0.0))
    )

    return {
        "detector": submission.get("detector", "unknown"),
        "detector_version": submission.get("detector_version", ""),
        "flood_detection": flood_detection,
        "flood_detection_counts": [flood_hit, flood_total],
        "real_id_accuracy": real_id_accuracy,
        "real_id_counts": [real_hit, real_total],
        "false_positive_rate": false_positive_rate,
        "false_positive_counts": [clean_fp, clean_total],
        "result": "PASS" if passed else "FAIL",
        "scenarios": rows,
    }


def pct(x):
    return "n/a" if x is None else f"{x * 100:.0f}%"


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("submission", help="path to a detector submission JSON")
    ap.add_argument("--manifest", default="corpus/manifest.json",
                    help="path to the corpus manifest (default: corpus/manifest.json)")
    ap.add_argument("--json", action="store_true",
                    help="emit the scorecard as JSON")
    args = ap.parse_args()

    manifest = load(args.manifest)
    submission = load(args.submission)
    card = score(manifest, submission)

    if args.json:
        print(json.dumps(card, indent=2))
        return 0 if card["result"] == "PASS" else 1

    name = card["detector"]
    if card["detector_version"]:
        name += " " + card["detector_version"]
    print(f"Detector: {name}")
    print("-" * 60)
    for r in card["scenarios"]:
        bits = [f"alarm={'yes' if r['got_alarm'] else 'no '}"]
        if "real_ok" in r:
            bits.append(f"named_real={'HIT ' if r['real_ok'] else 'MISS'}")
        mark = "ok " if r["alarm_ok"] and r.get("real_ok", True) else "ERR"
        print(f"  [{mark}] {r['scenario']:16} " + "  ".join(bits))
    print("-" * 60)
    fh, ft = card["flood_detection_counts"]
    rh, rt = card["real_id_counts"]
    ch, ct = card["false_positive_counts"]
    print(f"  flood detection      {pct(card['flood_detection'])}  ({fh}/{ft})")
    print(f"  real-drone accuracy  {pct(card['real_id_accuracy'])}  ({rh}/{rt})")
    print(f"  false-positive rate  {pct(card['false_positive_rate'])}  ({ch}/{ct})")
    print("-" * 60)
    print(f"  RESULT: {card['result']}")
    return 0 if card["result"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
