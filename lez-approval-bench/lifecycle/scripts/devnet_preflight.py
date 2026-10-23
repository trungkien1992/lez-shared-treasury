#!/usr/bin/env python3
"""Pinned-source readiness report; optional READ-ONLY JSON-RPC checks, never submission."""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import urllib.error
import urllib.parse
import urllib.request

PIN = "411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4"
PPE_SHA = "2d0e691386987e0bd0f708557730445c7ebee03a5c922ae37b897a939b8a82f5"
READ_METHODS = {"checkHealth", "getChannelId", "getLastBlockId", "getProgramIds", "getProofsAndRoot"}
MAX_RESPONSE = 2 * 1024 * 1024


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("RPC redirects are not accepted; use the confirmed endpoint")


def channel_hex(value):
    if isinstance(value, list) and len(value) == 32 and all(type(x) is int and 0 <= x <= 255 for x in value):
        return bytes(value).hex()
    if isinstance(value, str):
        value = value.removeprefix("0x")
        if len(value) == 64:
            try:
                return bytes.fromhex(value).hex()
            except ValueError:
                pass
    raise ValueError("expected a 32-byte channel ID")


def rpc(url, method, params, request_id):
    if method not in READ_METHODS:
        raise ValueError("only the documented read-only methods are permitted")
    encoded = json.dumps({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}).encode()
    request = urllib.request.Request(url, data=encoded, headers={"Content-Type": "application/json"})
    with urllib.request.build_opener(NoRedirect).open(request, timeout=10) as response:
        raw = response.read(MAX_RESPONSE + 1)
    if len(raw) > MAX_RESPONSE:
        raise ValueError("RPC response exceeds size bound")
    obj = json.loads(raw)
    if obj.get("jsonrpc") != "2.0" or obj.get("id") != request_id:
        raise ValueError("RPC response envelope does not match request")
    if "error" in obj:
        raise ValueError(f"RPC error: {obj['error']}")
    if "result" not in obj:
        raise ValueError("missing RPC result")
    return obj["result"]


def remote_checks(url, expected_channel):
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password or parsed.fragment or parsed.query:
        raise ValueError("use an HTTP(S) RPC URL without embedded credentials, query or fragment")
    if parsed.scheme == "http" and parsed.hostname not in ("localhost", "127.0.0.1", "::1"):
        raise ValueError("remote endpoints require HTTPS")
    expected = channel_hex(expected_channel)
    result = {"endpoint": url, "read_only": True, "expected_channel": expected, "methods": {}}
    for idx, (method, params) in enumerate([
        ("checkHealth", []), ("getChannelId", []), ("getLastBlockId", []),
        ("getProgramIds", []), ("getProofsAndRoot", [[]]),
    ], 1):
        value = rpc(url, method, params, idx)
        result["methods"][method] = value
        if method == "checkHealth" and value is not None:
            raise ValueError("unexpected health response")
        if method == "getChannelId" and channel_hex(value) != expected:
            raise ValueError("RPC channel differs from the explicitly expected channel")
        if method == "getLastBlockId" and (type(value) is not int or value < 0):
            raise ValueError("unexpected block ID")
        if method == "getProgramIds" and not isinstance(value, dict):
            raise ValueError("unexpected program ID map")
        if method == "getProofsAndRoot" and (not isinstance(value, list) or len(value) != 2 or value[0] != []):
            raise ValueError("unexpected empty-commitment proof response")
    result["channel_and_rpc_checks_passed"] = True
    result["runtime_binary_compatibility_verified"] = False
    result["live_transaction_size_limit_verified"] = False
    return result


def main():
    root = Path(__file__).resolve().parents[1]
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--upstream", type=Path, default=root.parents[1] / "lez-upstream")
    p.add_argument("--rpc-url", help="explicit operator-supplied endpoint; optional")
    p.add_argument("--expected-channel", help="independently confirmed 32-byte hex channel ID")
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    if bool(args.rpc_url) != bool(args.expected_channel):
        p.error("--rpc-url and --expected-channel must be supplied together")
    if args.output.exists():
        p.error("output exists; choose a fresh report filename")
    report = {"checked_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "transactions_submitted": 0, "ready_to_submit": False,
              "commands_available": {name: shutil.which(name) is not None for name in ("git", "docker", "just")},
              "pinned_commit": PIN, "checks": {}, "blockers": []}
    try:
        head = subprocess.run(["git", "-C", str(args.upstream), "rev-parse", "HEAD"],
                              check=True, capture_output=True, text=True, timeout=10).stdout.strip()
        clean = subprocess.run(["git", "-C", str(args.upstream), "status", "--porcelain"],
                               check=True, capture_output=True, text=True, timeout=10).stdout == ""
        report["checks"]["upstream_commit_matches"] = head == PIN
        report["checks"]["upstream_worktree_clean"] = clean
        ppe = args.upstream / "artifacts/lee/privacy_preserving_circuit/privacy_preserving_circuit.bin"
        report["checks"]["ppe_sha256_matches"] = hashlib.sha256(ppe.read_bytes()).hexdigest() == PPE_SHA
        config_path = args.upstream / "lez/configs/docker-all-in-one/devnet/sequencer_config.json"
        config = json.loads(config_path.read_text())
        report["repository_config"] = {"path": str(config_path), "max_block_size": config["max_block_size"],
                                       "max_num_tx_in_block": config["max_num_tx_in_block"],
                                       "is_live_configuration": False}
        # This comparison applies only to the inspected pinned 1 MiB configuration.
        if config["max_block_size"] != "1.0 MiB":
            raise ValueError("repository block size changed; inspect its encoding before comparison")
        envelope = root.parent / "integration/reports/m5-n7/transaction-succinct.lee-envelope"
        report["archived_v1_envelope"] = {"bytes": envelope.stat().st_size,
                                         "repository_cap_bytes": 1048576 - 2048,
                                         "fits_repository_cap": envelope.stat().st_size <= 1048576 - 2048,
                                         "usable_on_live_network": False,
                                         "reason": "uses a synthetic genesis commitment root and public fixture keys"}
        if not all(report["checks"].values()):
            report["blockers"].append("Local upstream source or PPE binary differs from the pinned clean build.")
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        report["blockers"].append(f"Local source check failed: {error}")
    if args.rpc_url:
        try:
            report["rpc"] = remote_checks(args.rpc_url, args.expected_channel)
        except (OSError, ValueError, urllib.error.URLError) as error:
            report["rpc"] = {"read_only": True, "channel_and_rpc_checks_passed": False, "error": str(error)}
            report["blockers"].append("RPC preflight did not pass.")
    else:
        report["rpc"] = {"attempted": False, "reason": "No confirmed endpoint and channel supplied."}
        report["blockers"].append("Confirmed LEZ RPC endpoint and channel ID are missing.")
    report["blockers"].extend([
        "Lifecycle guest has execution evidence, not a completed real proof.",
        "Network runtime/PPE compatibility and live transaction size limit need operator confirmation.",
        "Create fresh test-only member and viewing/nullifier keys, fund an account, sync its real commitment proof, and generate new transactions.",
        "Independent review and encrypted key-distribution/account-enrollment workflow remain outstanding.",
    ])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return 2 if report["blockers"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
