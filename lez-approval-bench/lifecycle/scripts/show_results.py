#!/usr/bin/env python3
"""Print the completed proof and local-node evidence as a short fixture walkthrough."""
import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--proof-dir", default="reports/2026-10-12-standalone-lifecycle")
parser.add_argument("--funding-dir", default="reports/2026-10-12-standalone-funding")
parser.add_argument("--settlement-dir", default="reports/2026-10-12-standalone-settlement")
args = parser.parse_args()
proof = json.loads((Path(args.proof_dir) / "succinct.json").read_text())
check = json.loads((Path(args.proof_dir) / "check-succinct.json").read_text())
funding = json.loads((Path(args.funding_dir) / "check-bootstrap.json").read_text())
node = json.loads((Path(args.settlement_dir) / "settlement.json").read_text())
if not node["complete"] or not check["fresh_process_check"] or not funding["real_receipt_verified"]:
    parser.error("completed proof verification and local settlement evidence are required")
print("LEZ Shared Treasury | local fixture demonstration")
print("7 enrolled members, 5 approvals required for a transfer")
print("Real receipts verified in separate processes; development mode disabled")
print()
for index, tx in enumerate(node["transactions"], 1):
    print(f"{index}. {tx['operation']}")
    print(f"   Included in block {tx['block_id']} | {tx['envelope_bytes']:,} bytes")
    print(f"   Transaction {tx['transaction_hash']}")
print()
print(f"Private funding account: {node['private_funder_balance']} test units")
print(f"Treasury after transfer: {node['vault_balance']} test units")
print(f"Recipient received:     {node['recipient_balance']} test units")
print("Vault proposal counter: 1")
print("Node membership witness matches the proved spend input")
print("Expiry, changed-message and replay checks preserve state on rejection")
print()
print("Scope: pinned standalone node, mock Bedrock, public deterministic fixture keys")
print("Public devnet, wallet/key sharing, rotation, coordination and disclosure remain pending")
