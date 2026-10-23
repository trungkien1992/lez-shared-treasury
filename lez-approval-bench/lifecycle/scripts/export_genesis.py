#!/usr/bin/env python3
"""Build a small exporter against the already-built, pinned sequencer dependencies."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

PIN = "411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--upstream", required=True)
parser.add_argument("--target-dir", required=True, help="Cargo target directory used for the sequencer build")
parser.add_argument("--output", default="reports/standalone-genesis.state.borsh")
args = parser.parse_args()
upstream = Path(args.upstream).resolve()
target = Path(args.target_dir).resolve()
output = Path(args.output).resolve()
revision = subprocess.check_output(["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True).strip()
if revision != PIN or subprocess.check_output(["git", "-C", str(upstream), "status", "--porcelain"], text=True):
    parser.error("unchanged pinned upstream source is required")
if output.exists():
    parser.error("genesis output exists; do not overwrite retained evidence")
if not shutil.which("rustc"):
    parser.error("use the same host Rust compiler as the sequencer build")
deps = target / "release/deps"
libraries = sorted(deps.glob("libtestnet_initial_state-*.rlib"))
borsh_libraries = sorted(deps.glob("libborsh-*.rlib"))
if not libraries or not borsh_libraries:
    parser.error("build the pinned standalone sequencer first")
source = Path(__file__).with_suffix(".rs")
output.parent.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="lez-genesis-export-") as temporary:
    executable = Path(temporary) / "export-genesis"
    last = ""
    success = False
    for library in libraries:
        for borsh in borsh_libraries:
            command = ["rustc", "--edition=2024", "-O", str(source), "--extern",
                       "testnet_initial_state=" + str(library), "--extern", "borsh=" + str(borsh),
                       "-L", "dependency=" + str(deps), "-o", str(executable)]
            result = subprocess.run(command, capture_output=True, text=True)
            if result.returncode == 0:
                success = True
                break
            last = result.stderr
        if success:
            break
    if not success:
        raise RuntimeError("Pinned exporter compilation failed: " + last[-2500:])
    root = json.loads(subprocess.check_output([str(executable), str(output)], text=True))
metadata = {"upstream_commit": PIN, "cross_zone": False, "private_root": root,
            "state_bytes": output.stat().st_size, "state_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
            "source": "pinned testnet_initial_state::initial_state(false)", "sequencer_source_modified": False}
output.with_suffix(".json").write_text(json.dumps(metadata, indent=2) + "\n")
print(json.dumps(metadata))
