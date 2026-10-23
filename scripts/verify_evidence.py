#!/usr/bin/env python3
"""Check release integrity and recorded correlations; does not verify zkVM proofs."""
import argparse
import hashlib
import json
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def verify(root):
    root = root.resolve()
    checks = []
    manifest = json.loads((root / "evidence/MANIFEST.json").read_text())
    require(manifest["format"] == "lez-treasury-evidence-v1", "unknown manifest format")
    seen = set()
    for item in manifest["files"]:
        relative = Path(item["path"])
        require(not relative.is_absolute() and ".." not in relative.parts, "unsafe manifest path")
        require(item["path"] not in seen, "duplicate manifest path")
        seen.add(item["path"])
        p = root / relative
        require(not p.is_symlink() and p.resolve().is_relative_to(root), "symlink/escaped manifest path")
        require(p.is_file(), "missing file: " + item["path"])
        require(p.stat().st_size == item["size_bytes"] and sha(p) == item["sha256"],
                "integrity mismatch: " + item["path"])
    checks.append({"check": "release_file_integrity", "files": len(seen), "passed": True})

    original = json.loads((root / "docs/provenance/unchanged-source.json").read_text())
    for name, digest in original["files"].items():
        require(sha(root / name) == digest, "snapshot source changed: " + name)
    rust = [name for name in original["files"] if name.endswith(".rs")]
    require(len(rust) == 31, "expected 31 original Rust source files")
    checks.append({"check": "unchanged_snapshot_source", "rust_files": len(rust), "passed": True})

    def read(name):
        return json.loads((root / name).read_text())

    life = "lez-approval-bench/lifecycle/"
    member = life + "client/reports/2026-10-13-member-workflow/"
    proof = life + "reports/2026-10-13-client-real-lifecycle/"
    summary = read(member + "verification.json")
    included = read(life + "reports/2026-10-13-client-settlement/settlement.json")
    proved = read(proof + "succinct.json")
    checked = read(proof + "check-succinct.json")
    executed = read(life + "reports/2026-10-13-client-guest-execution/execute.json")
    require(summary["real_lifecycle_receipts_verified"] == proved["real_receipts_verified"]
            == checked["real_receipts_verified"] == 2, "receipt-record count mismatch")
    require(checked["fresh_process_check"] and not summary["public_devnet"], "historical scope mismatch")
    require(included["complete"] and included["node_exit_code"] == 0, "local-node record incomplete")
    require(summary["approved_indices"] == [0, 2, 3, 5, 6]
            and summary["rejected_indices"] == [1, 4], "member record mismatch")
    for operation, field in [("create-export", "creation_instruction_sha256"),
                             ("spend-export", "transfer_instruction_sha256")]:
        digest = sha(root / (member + operation + "/instruction.borsh"))
        require(all(report[field] == digest for report in [proved, checked, executed]),
                "instruction record mismatch: " + operation)
    cases = [
        ("fund-private-source", life + "reports/2026-10-12-standalone-funding/transaction-succinct.lee-envelope"),
        ("create-and-fund-vault", proof + "create/transaction-succinct.lee-envelope"),
        ("five-of-seven-spend", proof + "spend/transaction-succinct.lee-envelope"),
    ]
    records = summary["actual_local_sequencer_transactions"]
    require(len(records) == len(included["transactions"]) == 3, "expected three local transactions")
    for operation, name in cases:
        item = next(x for x in records if x["operation"] == operation)
        node = next(x for x in included["transactions"] if x["operation"] == operation)
        p = root / name
        require(item["included_bytes_match"] and node["included_bytes_match"], "node byte-match record missing")
        require(p.stat().st_size == item["envelope_bytes"] and sha(p) == item["envelope_sha256"],
                "encoded transaction mismatch: " + operation)
        require(item["transaction_hash"] == node["transaction_hash"], "local transaction record mismatch")
    require(included["vault_balance"] == 463 and included["recipient_balance"] == 37
            and included["private_funder_balance"] == 500, "local balance record mismatch")
    controls = read(member + "controls.json")
    require(len(controls) == 6 and all(x["passed"] for x in controls), "CLI control records incomplete")
    checks.append({"check": "historical_client_proof_node_correlations", "transactions": 3, "passed": True})

    state = read(life + "client/reports/2026-10-17-state-import/validation-summary.json")
    require(state["total_rust_tests_passed"] == 35
            and state["client_state_import_tests_passed"] == 12
            and state["fresh_offline_cli_negative_controls_passed"] == 6,
            "state-import test records incomplete")
    require(state["latest_recorded_state_demo"]["balance_base_units"] == "463"
            and state["latest_recorded_state_demo"]["proposal_counter"] == 1,
            "state-import record mismatch")
    require(not state["live_network_checkpoint_authentication"]
            and not state["new_receipts_generated"] and not state["new_sequencer_started"],
            "state-import historical scope mismatch")
    guest = root / (life + "reports/lifecycle/lifecycle-guest.bin")
    require(sha(guest) == "fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c",
            "archived guest identity mismatch")
    checks.append({"check": "historical_state_import_records", "passed": True})
    return {"passed": True, "checks": checks, "cryptographic_receipts_verified_by_this_script": False,
            "network_contacted": False, "source_snapshot_date": "2026-10-17"}


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = p.parse_args()
    print(json.dumps(verify(args.root), indent=2))
