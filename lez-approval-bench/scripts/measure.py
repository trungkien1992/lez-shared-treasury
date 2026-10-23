#!/usr/bin/env python3
"""Run one local proof and retain its output plus Linux resource observations."""
import argparse
import datetime
import json
import os
from pathlib import Path
import platform
import resource
import signal
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--m", type=int, default=5)
parser.add_argument("--n", type=int, default=7)
parser.add_argument("--r0vm", default="r0vm")
parser.add_argument("--threads", type=int, default=4)
parser.add_argument("--report-dir", default="reports")
parser.add_argument("--timeout-seconds", type=int, default=1800)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
stem = root / args.report_dir / f"m{args.m}-n{args.n}"
stem.parent.mkdir(parents=True, exist_ok=True)
if any(stem.with_suffix(s).exists() for s in [".json", ".receipt", ".log", ".resources.json"]):
    parser.error("report destination already contains a trial; select a fresh --report-dir")
if args.threads < 1 or args.timeout_seconds < 1:
    parser.error("threads and timeout-seconds must be positive")
command = [str(root / "target/release/approval-host"), "--m", str(args.m),
           "--n", str(args.n), "--out", str(stem.with_suffix(".json")),
           "--r0vm", args.r0vm, "--receipt", "composite"]
environment = os.environ.copy()
environment["RISC0_DEV_MODE"] = "0"
environment["RAYON_NUM_THREADS"] = str(args.threads)
environment["RISC0_INFO"] = "1"
environment["RUST_LOG"] = "risc0_zkvm::host::server::prove=debug"
started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
started = time.monotonic()
print(f"Running real M={args.m}, N={args.n} proof; log: {stem.with_suffix('.log')}", flush=True)
status = "completed"
with stem.with_suffix(".log").open("w") as log:
    process = subprocess.Popen(command, cwd=root, env=environment, stdout=log, stderr=log,
                               start_new_session=True)
    try:
        exit_code = process.wait(timeout=args.timeout_seconds)
    except (subprocess.TimeoutExpired, KeyboardInterrupt) as error:
        status = "timeout" if isinstance(error, subprocess.TimeoutExpired) else "operator_interrupted"
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        exit_code = 124 if status == "timeout" else 130
elapsed = time.monotonic() - started
usage = resource.getrusage(resource.RUSAGE_CHILDREN)
def read_optional(path):
    p = Path(path)
    return p.read_text().strip() if p.exists() else None
cpu_model = next((line.split(":", 1)[1].strip()
                  for line in Path("/proc/cpuinfo").read_text().splitlines()
                  if line.startswith("model name")), None)
observations = {
    "started_at_utc": started_at, "exit_code": exit_code, "run_status": status,
    "timeout_seconds": args.timeout_seconds,
    "command": command, "elapsed_wall_seconds": elapsed,
    "child_user_cpu_seconds": usage.ru_utime,
    "child_system_cpu_seconds": usage.ru_stime,
    "child_max_rss_kib": usage.ru_maxrss,
    "rss_definition": "Linux RUSAGE_CHILDREN maximum resident set size; largest process peak, not simultaneous aggregate",
    "platform": platform.platform(), "architecture": platform.machine(),
    "cpu_model": cpu_model, "visible_logical_cpus": os.cpu_count(),
    "cgroup_cpu_max": read_optional("/sys/fs/cgroup/cpu.max"),
    "cgroup_memory_max_bytes": read_optional("/sys/fs/cgroup/memory.max"),
    "rayon_num_threads": args.threads, "gpu_used": False,
    "risc0_dev_mode": False, "paid_remote_proving": False,
}
stem.with_suffix(".resources.json").write_text(json.dumps(observations, indent=2) + "\n")
print(stem.with_suffix(".log").read_text(), end="")
print(json.dumps(observations, indent=2), flush=True)
raise SystemExit(exit_code)
