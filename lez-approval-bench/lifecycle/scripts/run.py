#!/usr/bin/env python3
"""Measured local execution, real proving and saved-proof verification. No RPC submission."""
import argparse, datetime, json, os, platform, resource, shutil, signal, subprocess, time
from pathlib import Path

def optional_limit(path):
 """A host without readable cgroup v2 limits has no value to report."""
 try: return Path(path).read_text().strip()
 except OSError: return None

p=argparse.ArgumentParser()
p.add_argument('mode', choices=['execute','validate','prove','succinct','check','check-succinct','bootstrap-execute','bootstrap','check-bootstrap'])
p.add_argument('--threads', type=int, default=8)
p.add_argument('--state-profile', choices=['isolated','standalone'], default='isolated')
p.add_argument('--timeout', type=int, default=10800)
p.add_argument('--r0vm', default='r0vm')
p.add_argument('--host', help='explicit lifecycle-host binary; defaults to the shared integration target')
p.add_argument('--create-instruction', help='client-exported creation Borsh instruction; use with --transfer-instruction')
p.add_argument('--transfer-instruction', help='client-exported transfer Borsh instruction; use with --create-instruction')
p.add_argument('--trial', default='1')
p.add_argument('--report-dir', required=True, help='fresh report directory, relative to lifecycle/')
a=p.parse_args()
if bool(a.create_instruction) != bool(a.transfer_instruction): p.error('supply both exported instructions')
if a.create_instruction and a.mode == 'validate': p.error('the historical validation suite requires the original fixture')
if a.threads < 1 or a.timeout < 1: p.error('threads and timeout must be positive')
root=Path(__file__).resolve().parents[1]
directory=root/a.report_dir
directory.mkdir(parents=True,exist_ok=True)
stem=directory/f'{a.mode}-trial-{a.trial}'
log_path=stem.with_suffix('.log')
resource_path=stem.with_suffix('.resources.json')
if log_path.exists() or resource_path.exists(): p.error('trial already exists; use a new --trial')
r0vm=shutil.which(a.r0vm) or a.r0vm
env=os.environ.copy()
env.update(RISC0_DEV_MODE='0', RISC0_PROVER='ipc',RISC0_EXECUTOR='ipc',
           LIFECYCLE_STATE_PROFILE=a.state_profile,
           RISC0_SERVER_PATH=str(Path(r0vm).resolve()), RAYON_NUM_THREADS=str(a.threads),
           RISC0_INFO='1',RUST_LOG='risc0_zkvm::host::server::prove=debug')
for key in ('LIFECYCLE_CREATE_INSTRUCTION','LIFECYCLE_TRANSFER_INSTRUCTION'): env.pop(key,None)
if a.create_instruction:
 env['LIFECYCLE_CREATE_INSTRUCTION']=str(Path(a.create_instruction).resolve())
 env['LIFECYCLE_TRANSFER_INSTRUCTION']=str(Path(a.transfer_instruction).resolve())
host=Path(a.host).resolve() if a.host else root.parent/'integration/target/release/lifecycle-host'
if not host.is_file(): p.error('lifecycle-host binary is missing; build it or supply --host')
if not Path(r0vm).is_file(): p.error('r0vm binary is missing; supply --r0vm')
command=[str(host),a.mode,str(directory)]
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
        'client_exported_instructions':bool(a.create_instruction),
        'state_profile':a.state_profile,
        'run_status':status,'timeout_seconds':a.timeout,'threads':a.threads,'command':command,
        'child_max_rss_kib':usage.ru_maxrss,'child_user_cpu_seconds':usage.ru_utime,
        'child_system_cpu_seconds':usage.ru_stime,'cpu_max':optional_limit('/sys/fs/cgroup/cpu.max'),
        'memory_max':optional_limit('/sys/fs/cgroup/memory.max'),
        'cpu_model':next(x.split(':',1)[1].strip() for x in Path('/proc/cpuinfo').read_text().splitlines() if x.startswith('model name')),
        'platform':platform.platform(),'gpu_used':False,'paid_remote_proving':False,'dev_mode':False,
        'rss_definition':'Linux RUSAGE_CHILDREN maximum per-process resident set size, not simultaneous aggregate'}
resource_path.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)
raise SystemExit(code)
