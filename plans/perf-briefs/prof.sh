#!/bin/bash
# usage: prof.sh name [ENV=VAL ...]  -> prints the last perf report of a 25s run
name=$1; shift
cd /home/_l/dreamscape
env DISPLAY=:1 ENGINE_PROF=1 DREAMSCAPE_BOOKLET=$TMPDIR/bkp.ron DREAMSCAPE_AUTOPILOT=1 DREAMSCAPE_SKIP_WARNING=1 DREAMSCAPE_SEED=5 RUST_LOG=info "$@" timeout 28 ./target/profiling/dreamscape >/dev/null 2>&1
python3 - "$name" <<'PY'
import re,os,sys
t=open(os.path.expanduser('~/.local/share/dreamscape/dreamscape.log')).read()
blocks=[b for b in re.split(r'(?=\[20\d\d-)',t) if 'perf over' in b]
print('===',sys.argv[1],len(blocks),'reports; last 2 averaged lines of the final one:')
b=blocks[-1].strip().split('\n')
print('\n'.join(l[:100] for l in b[:12]))
PY
