#!/usr/bin/env python3
"""Build/test the pinned snapshot or recheck saved receipts and local signers."""
import argparse
import datetime
import json
import os
import shutil
import subprocess
import tempfile
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIFECYCLE = ROOT / "lez-approval-bench/lifecycle"
PIN = "411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4"


def run(command, cwd=ROOT, env=None, log=None):
    result = subprocess.run([str(x) for x in command], cwd=cwd, env=env,
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log:
        log.write_text(result.stdout)
    if result.returncode:
        raise RuntimeError("Command failed: " + str(command[0]) + "\n" + result.stdout[-6000:])
    return result.stdout


def environment():
    env = os.environ.copy()
    for key in ("LIFECYCLE_CREATE_INSTRUCTION", "LIFECYCLE_TRANSFER_INSTRUCTION"):
        env.pop(key, None)
    env.update(RISC0_DEV_MODE="0", CARGO_BUILD_JOBS="2",
               LIFECYCLE_PREBUILT_GUEST=str(LIFECYCLE / "reports/lifecycle/lifecycle-guest.bin"),
               LIFECYCLE_STATE_PROFILE="standalone")
    return env


def fresh_directory(command):
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    p = ROOT / "var" / (stamp + "-" + command + "-" + uuid.uuid4().hex[:8])
    p.mkdir(parents=True, mode=0o700)
    return p


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("command", choices=["setup", "test", "receipts", "demo"])
    p.add_argument("--host", type=Path, help="explicit local host executable; default is the release build")
    p.add_argument("--client", type=Path, help="explicit local client executable; default is the release build")
    a = p.parse_args()
    env = environment()
    if a.command == "setup":
        if not shutil.which("rustup"):
            p.error("Rustup is required; install it from the official Rust project first")
        upstream = ROOT / "lez-upstream"
        if upstream.exists():
            head = run(["git", "rev-parse", "HEAD"], upstream).strip()
            if head != PIN or run(["git", "status", "--porcelain"], upstream).strip():
                p.error("existing lez-upstream must be the clean pinned revision; choose a separate checkout")
        else:
            upstream.mkdir()
            run(["git", "init", "--quiet"], upstream)
            run(["git", "remote", "add", "origin", "https://github.com/logos-blockchain/logos-execution-zone.git"], upstream)
            run(["git", "fetch", "--depth", "1", "origin", PIN], upstream)
            run(["git", "checkout", "--detach", "FETCH_HEAD"], upstream)
        run(["rustup", "toolchain", "install", "1.98.1", "--profile", "minimal"])
        print(json.dumps({"setup_completed": True, "upstream_revision": PIN}))
        return
    output = fresh_directory(a.command)
    if a.command == "test":
        if not shutil.which("cargo"):
            p.error("cargo is missing; run setup after installing Rustup")
        upstream = ROOT / "lez-upstream"
        if not upstream.is_dir() or run(["git", "rev-parse", "HEAD"], upstream).strip() != PIN:
            p.error("run setup to fetch the exact upstream revision")
        if run(["git", "status", "--porcelain"], upstream).strip():
            p.error("pinned upstream must be clean")
        run(["cargo", "+1.98.1", "build", "--release", "--locked", "-p", "treasury-client", "-p", "lifecycle-host"],
            LIFECYCLE, env, output / "build.log")
        run(["cargo", "+1.98.1", "test", "--release", "--locked", "-p", "treasury-client"],
            LIFECYCLE, env, output / "client-tests.log")
        run(["cargo", "+1.98.1", "test", "--release", "--locked", "-p", "approval-core"],
            ROOT / "lez-approval-bench", env, output / "core-tests.log")
        result = {"rust_build_and_tests_passed": True, "new_receipts_generated": False}
    elif a.command == "receipts":
        host = (a.host or LIFECYCLE / "target/release/lifecycle-host").resolve()
        if not host.is_file():
            p.error("build lifecycle-host first, or pass --host")
        proofs = output / "member-proofs"
        funding = output / "funding"
        shutil.copytree(LIFECYCLE / "reports/2026-10-13-client-real-lifecycle", proofs)
        shutil.copytree(LIFECYCLE / "reports/2026-10-12-standalone-funding", funding)
        member = LIFECYCLE / "client/reports/2026-10-13-member-workflow"
        env.update(LIFECYCLE_CREATE_INSTRUCTION=str(member / "create-export/instruction.borsh"),
                   LIFECYCLE_TRANSFER_INSTRUCTION=str(member / "spend-export/instruction.borsh"))
        run([host, "check-succinct", proofs], LIFECYCLE, env, output / "member-receipts.log")
        run([host, "check-bootstrap", funding], LIFECYCLE, env, output / "funding-receipt.log")
        result = {"saved_lifecycle_receipts_verified": 2, "saved_funding_receipt_verified": True,
                  "new_receipts_generated": False, "network_contacted": False}
    else:
        client = (a.client or LIFECYCLE / "target/release/treasury-client").resolve()
        if not client.is_file():
            p.error("build treasury-client first, or pass --client")
        with tempfile.TemporaryDirectory(prefix="lez-treasury-test-keys-") as private:
            run(["python3", LIFECYCLE / "client/scripts/demo.py", "--client", client,
                 "--key-dir", Path(private) / "keys", "--report-dir", output / "member-demo"],
                ROOT, env, output / "demo.log")
        controls = json.loads((output / "member-demo/controls.json").read_text())
        if len(controls) != 6 or not all(x["passed"] for x in controls):
            raise RuntimeError("fresh CLI controls incomplete")
        result = {"fresh_cli_negative_controls_passed": 6, "local_test_signers": 7,
                  "temporary_signing_keys_removed": True, "network_contacted": False}
    result.update(command=a.command, output=str(output),
                  recorded_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
