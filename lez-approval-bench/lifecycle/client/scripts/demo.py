#!/usr/bin/env python3
"""Local test-only seven-member CLI journey. Secret keys stay outside reports."""
import argparse, datetime, hashlib, json, os, subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--client', required=True)
parser.add_argument('--key-dir', required=True, help='new private directory outside the project and report directory')
parser.add_argument('--report-dir', required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
project = root.parents[1]
report = Path(args.report_dir).resolve()
keys = Path(args.key_dir).resolve()
client = Path(args.client).resolve()
if keys.is_relative_to(project) or keys.is_relative_to(report):
    parser.error('keys must stay outside the entire deliverable project')
if report.exists() or keys.exists():
    parser.error('use fresh report and key directories')
report.mkdir(parents=True, mode=0o700)
keys.mkdir(parents=True, mode=0o700)
transcript, controls = [], []

def run(*arguments, success=True):
    result = subprocess.run([str(client), *map(str, arguments)], capture_output=True, text=True)
    safe_args = [str(v).replace(str(keys), '<LOCAL_TEST_KEYS>') for v in arguments]
    transcript.append({'arguments': safe_args, 'exit_code': result.returncode,
                       'stdout': result.stdout, 'stderr': result.stderr})
    if success and result.returncode:
        raise RuntimeError(f'client command failed: {safe_args}: {result.stderr}')
    if not success:
        if result.returncode == 0:
            raise RuntimeError(f'negative control unexpectedly passed: {safe_args}')
        return result.stderr
    return json.loads(result.stdout) if result.stdout.strip().startswith('{') else None

def save(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')

def account_hex(base58):
    alphabet = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
    value = 0
    for char in base58:
        value = value * 58 + alphabet.index(char)
    prefix = len(base58) - len(base58.lstrip('1'))
    raw = b'\0' * prefix + value.to_bytes((value.bit_length() + 7) // 8, 'big')
    if len(raw) != 32:
        raise ValueError('recorded account identifier must be 32 bytes')
    return raw.hex()

for index in range(7):
    run('keygen', keys / f'member-{index}.key', report / f'member-{index}.public.json')
public = [json.loads((report / f'member-{i}.public.json').read_text())['public_key_hex'] for i in range(7)]
save(report / 'member-public-keys.json', public)
context = report / 'context.json'
run('configure', root / 'reports/lifecycle/enrolled-descriptor.borsh', report / 'member-public-keys.json', 5, context)
recorded = json.loads((root / 'reports/2026-10-12-standalone-lifecycle/succinct.json').read_text())
expiry = 2_000_000_000_000
creation = report / 'create.request.json'
run('create', context, expiry, account_hex(recorded['funder_id']), 500, creation)
review = run('review', context, creation)
save(report / 'create.review.json', review)
enrollments = []
for index in range(7):
    out = report / f'enrollment-{index}.response.json'
    run('respond-offline', context, creation, keys / f'member-{index}.key', 'approve', review['review_digest'], out)
    enrollments.append(out)
creation_bundle = report / 'create.responses.json'
run('collect', context, creation, creation_bundle, *reversed(enrollments))
run('export', context, creation, creation_bundle, report / 'create-export')

spend = report / 'spend.request.json'
run('propose', context, account_hex(recorded['recipient_id']), 37, expiry, spend)
review = run('review', context, spend)
save(report / 'spend.review.json', review)
approved = [0, 2, 3, 5, 6]
responses = []
for index in range(7):
    out = report / f'spend-{index}.response.json'
    run('respond-offline', context, spend, keys / f'member-{index}.key', 'approve' if index in approved else 'reject', review['review_digest'], out)
    responses.append(out)
spend_bundle = report / 'spend.responses.json'
deliveries = list(reversed(responses)) + [responses[3], responses[3]]
status = run('collect', context, spend, spend_bundle, *deliveries)
assert status['approved_indices'] == approved and status['rejected_indices'] == [1, 4]
assert status['quorum']
export = run('export', context, spend, spend_bundle, report / 'spend-export')

four = report / 'below-threshold.responses.json'
run('collect', context, spend, four, *(responses[i] for i in approved[:4]))
controls.append({'case': 'four approvals cannot export a five-of-seven transfer', 'passed': True,
                 'observed': run('export', context, spend, four, report / 'must-not-export-below-threshold', success=False)})
altered = report / 'altered-amount.request.json'
run('propose', context, account_hex(recorded['recipient_id']), 38, expiry, altered)
controls.append({'case': 'responses cannot authorize a different amount', 'passed': True,
                 'observed': run('collect', context, altered, report / 'must-not-collect-altered', *responses, success=False)})
flipped = json.loads(responses[1].read_text())
flipped['decision'] = 'approve'
save(report / 'flipped-decision.response.json', flipped)
controls.append({'case': 'a signed rejection cannot be relabelled as approval', 'passed': True,
                 'observed': run('collect', context, spend, report / 'must-not-collect-flipped', report / 'flipped-decision.response.json', success=False)})
controls.append({'case': 'counter-one context rejects the counter-zero request', 'passed': True,
                 'observed': run('review', report / 'spend-export/predicted-context.json', spend, success=False)})
controls.append({'case': 'changed review digest refuses signing', 'passed': True,
                 'observed': run('respond-offline', context, spend, keys / 'member-0.key', 'approve', '00' * 32, report / 'must-not-sign-wrong-review', success=False)})
os.chmod(keys / 'member-0.key', 0o644)
try:
    observed = run('respond-offline', context, spend, keys / 'member-0.key', 'approve', review['review_digest'], report / 'must-not-read-public-key-file', success=False)
finally:
    os.chmod(keys / 'member-0.key', 0o600)
controls.append({'case': 'signing key with group or public permissions is refused', 'passed': True, 'observed': observed})
save(report / 'controls.json', controls)
save(report / 'transcript.json', transcript)
summary = {'recorded_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
           'local_test_demo': True, 'fresh_random_member_signing_keys': 7,
           'secret_keys_in_report': False, 'private_account_material': 'retained public test fixtures',
           'spending_threshold': 5, 'unanimous_enrollment_count': 7,
           'approved_indices': approved, 'rejected_indices': [1, 4],
           'duplicate_delivery_counted_once': True, 'negative_cli_controls_passed': len(controls),
           'creation_instruction_sha256': hashlib.sha256((report / 'create-export/instruction.borsh').read_bytes()).hexdigest(),
           'transfer_instruction_sha256': export['instruction_sha256'],
           'native_config_apply_passed': True, 'guest_execution': 'pending separate host run',
           'proof_generated': False, 'sequencer_submission': False, 'logos_chat_integrated': False}
save(report / 'demo.json', summary)
print(json.dumps(summary, indent=2))
