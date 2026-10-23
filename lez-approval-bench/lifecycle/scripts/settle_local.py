#!/usr/bin/env python3
"""Settle the three proved test-fixture transactions on a fresh pinned standalone node.

RPC is restricted to the child node on loopback. No remote endpoint, wallet, real
keys, or funding service is accepted. Proofs must already exist and pass the host's
fresh-process verification before submission.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import urllib.error
import urllib.request

PIN = "411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4"
ROOT = Path(__file__).resolve().parents[1]


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def rpc(url, method, params, calls):
    request = {"jsonrpc": "2.0", "id": len(calls) + 1, "method": method, "params": params}
    payload = json.dumps(request).encode()
    req = urllib.request.Request(url, payload, {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as response:
        result = json.load(response)
    calls.append({"method": method, "response": result})
    if "error" in result:
        raise RuntimeError(f"{method}: {result['error']}")
    if result.get("jsonrpc") != "2.0" or result.get("id") != request["id"]:
        raise RuntimeError("JSON-RPC response mismatch")
    return result["result"]


def wait_for(fn, seconds, process):
    deadline = time.monotonic() + seconds
    last = None
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"sequencer exited with {process.returncode}")
        try:
            value = fn()
            if value is not None:
                return value
        except (urllib.error.URLError, TimeoutError) as error:
            last = str(error)
        time.sleep(1)
    raise RuntimeError(f"local node observation timed out: {last}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sequencer", required=True)
    parser.add_argument("--host", required=True)
    parser.add_argument("--r0vm", required=True)
    parser.add_argument("--upstream", required=True)
    parser.add_argument("--bootstrap-dir", required=True)
    parser.add_argument("--proof-dir", required=True)
    parser.add_argument("--report-dir", required=True)
    parser.add_argument("--create-instruction", help="client-exported creation instruction used by the proof")
    parser.add_argument("--transfer-instruction", help="client-exported transfer instruction used by the proof")
    parser.add_argument("--port", type=int, default=13440)
    args = parser.parse_args()
    if bool(args.create_instruction) != bool(args.transfer_instruction):
        parser.error("supply both client-exported instructions")
    node = Path(args.sequencer).resolve()
    host = Path(args.host).resolve()
    r0vm = Path(args.r0vm).resolve()
    upstream = Path(args.upstream).resolve()
    bootstrap = Path(args.bootstrap_dir).resolve()
    proof_dir = Path(args.proof_dir).resolve()
    directory = Path(args.report_dir).resolve()
    if directory.exists():
        parser.error("report directory already exists; select a fresh directory")
    for path in (node, host, r0vm):
        if not path.is_file():
            parser.error(f"binary missing: {path}")
    revision = subprocess.check_output(["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "-C", str(upstream), "status", "--porcelain"], text=True)
    if revision != PIN or dirty:
        parser.error("upstream must be the unchanged pinned checkout")
    boot = json.loads((bootstrap / "bootstrap.json").read_text())
    lifecycle = json.loads((proof_dir / "succinct.json").read_text())
    imported = lifecycle.get("member_signing_source") == "client-exported responses"
    if imported != bool(args.create_instruction):
        parser.error("client proof reports require their exported instructions; fixture reports use no overrides")
    instruction_paths = {}
    if imported:
        for label, source in (("creation", args.create_instruction), ("transfer", args.transfer_instruction)):
            path = Path(source).resolve()
            if hashlib.sha256(path.read_bytes()).hexdigest() != lifecycle[label + "_instruction_sha256"]:
                parser.error(f"{label} instruction differs from the proved client export")
            instruction_paths[label] = path
    if not boot["real_receipt_verified"] or lifecycle["real_receipts_verified"] != 2:
        parser.error("completed real proof reports are required")
    if boot.get("state_profile") != "standalone" or lifecycle.get("state_profile") != "standalone":
        parser.error("proof reports must use the pinned standalone genesis state profile")
    for report in (bootstrap / "check-bootstrap.json", proof_dir / "check-succinct.json"):
        if not report.is_file():
            parser.error(f"fresh-process verification is required: {report}")
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", args.port))
    directory.mkdir(parents=True)
    config = json.loads((upstream / "lez/sequencer/service/configs/debug/sequencer_config.json").read_text())
    config.update(home=str(directory / "node-state"), genesis=[boot["config"]["genesis_supply"]],
                  gossip=None, metrics_address=None, block_create_timeout="2s")
    config_path = directory / "sequencer-config.json"
    save(config_path, config)
    command = [str(node), str(config_path), "--listen-address", "127.0.0.1", "--port", str(args.port)]
    env = os.environ.copy()
    env.update(RISC0_DEV_MODE="0", RUST_LOG="info", RISC0_EXECUTOR="ipc", RISC0_PROVER="ipc",
               LIFECYCLE_STATE_PROFILE="standalone",
               RISC0_SERVER_PATH=str(r0vm))
    for name in ("LIFECYCLE_CREATE_INSTRUCTION", "LIFECYCLE_TRANSFER_INSTRUCTION"):
        env.pop(name, None)
    if imported:
        env["LIFECYCLE_CREATE_INSTRUCTION"] = str(instruction_paths["creation"])
        env["LIFECYCLE_TRANSFER_INSTRUCTION"] = str(instruction_paths["transfer"])
    url = f"http://127.0.0.1:{args.port}"
    calls = []
    report = {"upstream_commit": PIN, "sequencer_sha256": hashlib.sha256(node.read_bytes()).hexdigest(),
              "host_sha256": hashlib.sha256(host.read_bytes()).hexdigest(), "command": command,
              "loopback_only": True, "gossip_disabled": True, "dev_mode": False,
              "public_deterministic_test_keys": lifecycle.get("deterministic_public_test_keys", True),
              "public_test_private_account_material": True,
              "member_signing_source": lifecycle.get("member_signing_source", "fixed public fixtures"),
              "live_devnet_submission": False,
              "transactions": [], "complete": False}
    started = time.monotonic()
    with (directory / "sequencer.log").open("w") as log:
        process = subprocess.Popen(command, cwd=upstream, env=env, stdout=log, stderr=log)
        try:
            wait_for(lambda: (rpc(url, "checkHealth", [], calls), True)[1], 120, process)
            actual_channel = rpc(url, "getChannelId", [], calls)
            expected_channel = config["bedrock_config"]["channel_id"]
            if actual_channel != expected_channel:
                raise RuntimeError("local sequencer channel differs from its explicit configuration")
            report["channel_id"] = actual_channel
            sender = boot["config"]["public_sender_id"]
            def funding_ready():
                nonces = rpc(url, "getAccountsNonces", [[sender]], calls)
                balance = rpc(url, "getAccountBalance", [sender], calls)
                return (nonces, balance) if balance == 1_000_000 else None
            nonces, balance = wait_for(funding_ready, 120, process)
            if nonces != [boot["config"]["public_sender_nonce"]]:
                raise RuntimeError("genesis public funding state does not match the bootstrap proof")
            stages = [("fund-private-source", bootstrap), ("create-and-fund-vault", proof_dir / "create"),
                      ("five-of-seven-spend", proof_dir / "spend")]
            for label, stage in stages:
                envelope = (stage / "transaction-succinct.lee-envelope").read_bytes()
                encoded = base64.b64encode(envelope).decode("ascii")
                tx_hash = rpc(url, "sendTransaction", [encoded], calls)
                confirmed = wait_for(lambda: rpc(url, "getTransaction", [tx_hash], calls), 120, process)
                if confirmed[0] != encoded:
                    raise RuntimeError(f"included bytes differ: {label}")
                item = {"operation": label, "transaction_hash": tx_hash, "block_id": confirmed[1],
                        "envelope_bytes": len(envelope), "envelope_sha256": hashlib.sha256(envelope).hexdigest(),
                        "included_bytes_match": True}
                report["transactions"].append(item)
                if label == "fund-private-source":
                    commitments = [boot["config"]["expected_funder_commitment"]]
                    proofs, root = rpc(url, "getProofsAndRoot", [commitments], calls)
                    if not proofs[0] or root != boot["config"]["expected_funder_commitment_root"]:
                        raise RuntimeError("accepted private funding state differs from the lifecycle witness")
                    item["accepted_funding_root_matches_lifecycle"] = True
                elif label == "create-and-fund-vault":
                    proofs, root = rpc(url, "getProofsAndRoot", [[lifecycle["created_vault_commitment"]]], calls)
                    if root != lifecycle["after_creation_commitment_root"] or proofs[0] != lifecycle["vault_membership_after_creation"]:
                        raise RuntimeError("sequencer's accepted creation witness differs from the proved spend witness")
                    observation = {"commitment": lifecycle["created_vault_commitment"],
                                   "membership_proof": proofs[0], "commitment_root": root,
                                   "channel_id": actual_channel, "creation_transaction_hash": tx_hash,
                                   "creation_block_id": confirmed[1], "rpc_source": "local child sequencer"}
                    save(proof_dir / "node-creation-witness.json", observation)
                    with (directory / "node-witness-check.log").open("w") as check_log:
                        subprocess.run([str(host), "check-node-witness", str(proof_dir)], cwd=ROOT, env=env,
                                       stdout=check_log, stderr=subprocess.STDOUT, check=True)
                    item["accepted_vault_membership_matches_spend"] = True
                    item["node_membership_ingested_by_host"] = True
                else:
                    proofs, root = rpc(url, "getProofsAndRoot", [lifecycle["spend_commitments"]], calls)
                    if any(p is None for p in proofs) or root != lifecycle["after_spend_commitment_root"]:
                        raise RuntimeError("sequencer's accepted spend state differs from the verified post-state")
                    item["accepted_spend_root_matches_verified_state"] = True
                save(directory / "settlement-progress.json", report)
            report["public_funding_balance_after"] = rpc(url, "getAccountBalance", [sender], calls)
            if report["public_funding_balance_after"] != 999_000:
                raise RuntimeError("wrong public funding debit")
            report.update(complete=True, private_funder_balance=500, vault_balance=463, recipient_balance=37,
                          balances_from="independently decrypted proof outputs whose exact transactions were included")
        except BaseException as error:
            report["error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            process.terminate()
            try:
                process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            report["elapsed_seconds"] = time.monotonic() - started
            report["node_exit_code"] = process.returncode
            save(directory / "rpc-observations.json", calls)
            save(directory / "settlement.json", report)


if __name__ == "__main__":
    main()
