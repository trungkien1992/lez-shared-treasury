#!/usr/bin/env python3
"""Verify this benchmark's saved receipt and two local negative controls."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--report-dir", default="reports/accelerated")
args = parser.parse_args()
directory = root / args.report_dir
report = json.loads((directory / "m5-n7.json").read_text())
assert report["status"] == "verified_real_proof" and report["dev_mode"] is False
assert report["m"] == 5 and report["n"] == 7
host = str(root / "target/release/approval-host")
receipt = directory / "m5-n7.receipt"
program = str(directory / "approval-guest.bin")
original_sha256 = hashlib.sha256(receipt.read_bytes()).hexdigest()

def check(path, m, expected_program=program):
    return subprocess.run(
        [host, "--m", str(m), "--n", "7", "--verify-receipt", str(path),
         "--guest-program", expected_program],
        cwd=root, capture_output=True, text=True, check=False,
    )

valid = check(receipt, 5)
assert valid.returncode == 0, valid.stderr
result = json.loads(valid.stdout)
assert result["status"] == "saved_receipt_verified"
assert result["image_id_words_u32"] == report["image_id_words_u32"]
(directory / "saved-receipt-verification.json").write_text(
    json.dumps(result, indent=2) + "\n"
)

wrong = check(receipt, 4)
assert wrong.returncode != 0 and "journal does not match" in wrong.stderr, wrong
wrong_image = check(receipt, 5, str(root / "reports/approval-guest.bin"))
assert wrong_image.returncode != 0, "receipt unexpectedly verified against baseline guest image"

# Use the decoded receipt API instead of guessing a byte offset in bincode.
controlled = subprocess.run(
    [host, "--m", "5", "--n", "7", "--verify-receipt", str(receipt),
     "--guest-program", program, "--mode", "check-receipt"],
    cwd=root, capture_output=True, text=True, check=False,
)
assert controlled.returncode == 0, controlled.stderr
integrity = json.loads(controlled.stdout)
assert integrity["status"] == "saved_receipt_integrity_checked"
assert integrity["modified_journal_rejected"] is True
assert hashlib.sha256(receipt.read_bytes()).hexdigest() == original_sha256

results = {
    "scope": "local controls on this prototype's own saved receipt",
    "original_receipt_verified": True,
    "wrong_expected_threshold": {"exit_code": wrong.returncode, "stderr": wrong.stderr},
    "wrong_guest_image": {"exit_code": wrong_image.returncode, "stderr": wrong_image.stderr},
    "modified_journal": integrity,
    "original_receipt_modified": False,
    "original_receipt_sha256": original_sha256,
}
(directory / "receipt-negative-checks.json").write_text(json.dumps(results, indent=2) + "\n")
print(json.dumps(results, indent=2))
