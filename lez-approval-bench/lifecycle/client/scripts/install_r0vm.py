#!/usr/bin/env python3
"""Install the exact recorded CPU r0vm from the checksum-verified official release."""
import argparse, hashlib, io, os, tarfile, tempfile, urllib.request
from pathlib import Path

URL = 'https://github.com/risc0/risc0/releases/download/v3.0.5/cargo-risczero-x86_64-unknown-linux-gnu.tgz'
ARCHIVE_SHA = '936ef988b78f20e3bd9f80e375f3adc934b13addc6ae2680f2e5fc0bcc966158'
BINARY_SHA = '36c016a5bb2ded5bd1f8f92cc487e6ffaeb1e95ec05850c983081a0f716b515b'
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--output', required=True)
a = p.parse_args()
out = Path(a.output).resolve()
if out.exists():
    if hashlib.sha256(out.read_bytes()).hexdigest() != BINARY_SHA:
        p.error('existing binary differs; choose a new output path')
    print('Exact r0vm already installed')
    raise SystemExit(0)
out.parent.mkdir(parents=True, exist_ok=True)
with urllib.request.urlopen(URL, timeout=60) as response:
    data = response.read(100_000_001)
if len(data) > 100_000_000 or hashlib.sha256(data).hexdigest() != ARCHIVE_SHA:
    raise RuntimeError('official release archive failed size or digest verification')
with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
    matches = [item for item in archive.getmembers() if item.isfile() and Path(item.name).name == 'r0vm']
    if len(matches) != 1:
        raise RuntimeError('release must contain exactly one r0vm binary')
    binary = archive.extractfile(matches[0]).read()
if hashlib.sha256(binary).hexdigest() != BINARY_SHA:
    raise RuntimeError('r0vm differs from the recorded exact runtime')
with tempfile.NamedTemporaryFile(dir=out.parent, delete=False) as temp:
    temp.write(binary)
    temp.flush()
    os.fsync(temp.fileno())
    temporary = Path(temp.name)
temporary.chmod(0o700)
os.replace(temporary, out)
print('Verified CPU r0vm 3.0.5 installed')
