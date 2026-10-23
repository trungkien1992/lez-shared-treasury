#!/usr/bin/env python3
"""Bounded local LEZ benchmark runner, retaining real stage checkpoints."""
import argparse, datetime, json, os, platform, resource, shutil, signal, subprocess, time
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('mode', choices=['execute','prove','succinct','check','check-succinct','validate'])
p.add_argument('--m', type=int, default=5)
p.add_argument('--threads', type=int, default=8)
p.add_argument('--timeout', type=int, default=5400)
p.add_argument('--r0vm', default='r0vm')
p.add_argument('--trial', default='1')
p.add_argument('--report-dir', help='fresh or existing report directory, relative to integration/')
a=p.parse_args()
if a.threads < 1 or a.timeout < 1: p.error('threads and timeout must be positive')
root=Path(__file__).resolve().parents[1]
directory=root/a.report_dir if a.report_dir else root/'reports'/f'm{a.m}-n7'
directory.mkdir(parents=True,exist_ok=True)
stem=directory/f'{a.mode}-trial-{a.trial}'
log_path=stem.with_suffix('.log')
resource_path=stem.with_suffix('.resources.json')
if log_path.exists() or resource_path.exists(): p.error('trial already exists; use a new --trial')
r0vm=shutil.which(a.r0vm) or a.r0vm
env=os.environ.copy()
env.update(RISC0_DEV_MODE='0', RISC0_PROVER='ipc',RISC0_EXECUTOR='ipc',
           RISC0_SERVER_PATH=str(Path(r0vm).resolve()), RAYON_NUM_THREADS=str(a.threads),
           RISC0_INFO='1',RUST_LOG='risc0_zkvm::host::server::prove=debug')
command=[str(root/'target/release/vault-host'),a.mode,str(a.m),str(directory)]
started=time.monotonic()
started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat()
with log_path.open('w') as log:
 proc=subprocess.Popen(command,cwd=root,env=env,stdout=log,stderr=log,start_new_session=True)
 print(json.dumps({'pid':proc.pid,'log':str(log_path),'command':command}),flush=True)
 status='completed'
 try: code=proc.wait(timeout=a.timeout)
 except (subprocess.TimeoutExpired, KeyboardInterrupt) as exc:
  status='timeout' if isinstance(exc,subprocess.TimeoutExpired) else 'interrupted'
  os.killpg(proc.pid,signal.SIGTERM)
  try: proc.wait(timeout=10)
  except subprocess.TimeoutExpired:
   os.killpg(proc.pid,signal.SIGKILL);proc.wait()
  code=124 if status=='timeout' else 130
usage=resource.getrusage(resource.RUSAGE_CHILDREN)
report={'started_at_utc':started_utc,'elapsed_seconds':time.monotonic()-started,'exit_code':code,
        'run_status':status,'timeout_seconds':a.timeout,'threads':a.threads,'command':command,
        'child_max_rss_kib':usage.ru_maxrss,'child_user_cpu_seconds':usage.ru_utime,
        'child_system_cpu_seconds':usage.ru_stime,'cpu_max':Path('/sys/fs/cgroup/cpu.max').read_text().strip(),
        'memory_max':Path('/sys/fs/cgroup/memory.max').read_text().strip(),
        'cpu_model':next(x.split(':',1)[1].strip() for x in Path('/proc/cpuinfo').read_text().splitlines() if x.startswith('model name')),
        'platform':platform.platform(),'gpu_used':False,'paid_remote_proving':False,'dev_mode':False,
        'rss_definition':'Linux RUSAGE_CHILDREN maximum per-process resident set size, not simultaneous aggregate'}
resource_path.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)
raise SystemExit(code)
