#!/usr/bin/env python3
"""Correlate client exports with independently checked guest/proof/node evidence.

Use lifecycle-host check-succinct first. This script does not replace cryptographic
receipt verification; it checks that the dated evidence describes the same inputs.
"""
import argparse, datetime, hashlib, json, subprocess
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--client', required=True)
p.add_argument('--demo-dir', required=True)
p.add_argument('--execution-dir', required=True)
p.add_argument('--proof-dir', required=True)
p.add_argument('--settlement-dir', required=True)
p.add_argument('--output', required=True)
a = p.parse_args()
demo, execution, proofs, settlement = [Path(value).resolve() for value in
                                      (a.demo_dir, a.execution_dir, a.proof_dir, a.settlement_dir)]

def read(path):
    return json.loads(path.read_text())

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def require(condition, message):
    if not condition:
        raise RuntimeError(message)

summary = read(demo / 'demo.json')
guest = read(execution / 'execute.json')
proved = read(proofs / 'succinct.json')
checked = read(proofs / 'check-succinct.json')
included = read(settlement / 'settlement.json')
node_witness = read(proofs / 'check-node-witness.json')
controls = read(demo / 'controls.json')
require(summary['fresh_random_member_signing_keys'] == 7, 'missing seven-member local demo')
require(guest['all_four_notes_decrypted_and_checked'] and guest['member_signing_source'] == 'client-exported responses',
        'guest did not execute the imported client requests')
require(not guest['proof_generated'], 'execution report must distinguish execution from proving')
require(proved['real_receipts_verified'] == checked['real_receipts_verified'] == 2 and checked['fresh_process_check'],
        'real proofs or independent saved-receipt verification are missing')
require(proved['proof_format'] == checked['proof_format'] == 'succinct', 'node workflow needs succinct proofs')
require(proved['unanimous_enrollment'] == 7 and proved['spending_threshold'] == 5, 'wrong member configuration')
require(all(control['passed'] for control in controls) and len(controls) == 6, 'CLI controls incomplete')
for operation, field in [('create-export', 'creation_instruction_sha256'), ('spend-export', 'transfer_instruction_sha256')]:
    actual = sha(demo / operation / 'instruction.borsh')
    for report in (summary, guest, proved, checked):
        require(report[field] == actual, field + ' differs between client and checked proof path')
require(guest['guest_sha256'] == proved['guest_sha256'] == checked['guest_sha256'] ==
        'fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c', 'guest changed')
require(node_witness['node_membership_ingested'] and node_witness['input_bytes_match_proved_spend'],
        'actual node membership was not checked against the spend inputs')
require(included['complete'] and included['node_exit_code'] == 0 and len(included['transactions']) == 3,
        'three-transaction local sequencer settlement is incomplete')
require(all(tx['included_bytes_match'] for tx in included['transactions']), 'included bytes differ')
require(included['member_signing_source'] == 'client-exported responses', 'node report is for a fixture-generated approval run')
require(included['vault_balance'] == 463 and included['recipient_balance'] == 37 and included['private_funder_balance'] == 500,
        'wrong final balances')
for name in ('creation', 'spend'):
    require(len(proved[name]['stages']) == 3 and {stage['stage'] for stage in proved[name]['stages']} ==
            {'vault-plan', 'vault-apply', 'ppe-composite'}, 'expected three proving stages per operation')
    # Some stages may be verified checkpoints from the earlier trial of this same
    # member workflow. The host rechecks their exact input hash and real receipt.

review = json.loads(subprocess.check_output([a.client, 'review', str(demo / 'context.json'),
                                           str(demo / 'spend.request.json')], text=True))
status = json.loads(subprocess.check_output([a.client, 'status', str(demo / 'context.json'),
                                           str(demo / 'spend.request.json'), str(demo / 'spend.responses.json')], text=True))
require(review['action']['amount_base_units'] == '37' and status['approved_indices'] == [0, 2, 3, 5, 6]
        and status['rejected_indices'] == [1, 4] and status['quorum'], 'saved client response set failed independent recheck')
result = {'recorded_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
          'member_workflow_complete': True, 'fresh_random_member_signing_keys': 7,
          'guest_execution_passed': True, 'real_lifecycle_receipts_verified': 2,
          'proving_stage_checkpoint_provenance': {
              name: [{'stage': stage['stage'], 'checkpoint_reused': stage['checkpoint_reused']}
                     for stage in proved[name]['stages']] for name in ('creation', 'spend')},
          'fresh_saved_receipt_check_passed': True, 'actual_local_sequencer_transactions': included['transactions'],
          'node_membership_ingested': True, 'client_negative_cli_controls_passed': 6,
          'approved_indices': status['approved_indices'], 'rejected_indices': status['rejected_indices'],
          'vault_balance': 463, 'recipient_balance': 37, 'private_funder_balance': 500,
          'guest_sha256': guest['guest_sha256'], 'genesis_snapshot_sha256': proved['genesis_snapshot_sha256'],
          'public_devnet': False, 'logos_chat_integrated': False, 'general_wallet_adapter': False,
          'private_account_material': 'public test fixtures', 'secret_member_signing_keys_in_bundle': False}
out = Path(a.output).resolve()
with out.open('x') as file:
    file.write(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
