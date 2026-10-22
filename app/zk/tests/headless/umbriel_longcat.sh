#!/usr/bin/env bash
# Run only through Umbriel's isolated headless harness (see runner README).
set -euo pipefail
: "${ZK_TEST_BIN:?absolute path to the built zk binary}"
: "${UMBRIEL_RUNTIME_DIR:?must run inside the native headless harness}"

cat >> "$UMBRIEL_CONFIG" <<'CONFIG'
[[window_rule]]
match.title = "^zk-longcat-test-"
title_bar = "forced"
CONFIG
"$UMBRIEL" msg config-reload >/dev/null

python3 - <<'PY'
import copy, json, os, queue, signal, socket, subprocess, threading, time

socket_path = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'umbriel-' + os.environ['WAYLAND_DISPLAY'] + '.sock')
def call(request):
    with socket.socket(socket.AF_UNIX) as s:
        s.settimeout(5)
        s.connect(socket_path)
        s.sendall((json.dumps(request) + '\n').encode())
        reply = json.loads(s.makefile('rb').readline())
        assert 'ok' in reply, reply
        return reply['ok']
def action(value):
    call({'cmd':'msg','arg':value})
def settle():
    subprocess.run([os.environ['UMBRIEL'],'settle'],check=True,stdout=subprocess.DEVNULL)
def layout():
    return call({'cmd':'layout-state'})
def group():
    return next(c for c in layout()['columns'] if len(c['basis']['members']) == 3)

for i in range(3):
    subprocess.Popen(['foot','--title=zk-longcat-test-'+str(i),'sh','-c','sleep 120'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    deadline=time.monotonic()+10
    while len(call({'cmd':'windows'})) != i+1:
        assert time.monotonic()<deadline, 'client map timeout'
        time.sleep(.05)
    settle()
    if i:
        action('window-consume-left')
        settle()

def start():
    env = {k:v for k,v in os.environ.items() if not k.startswith('ZK_')}
    env['XDG_CONFIG_HOME'] = os.path.join(os.environ['UMBRIEL_RUNTIME_DIR'],'zk-config')
    process = subprocess.Popen([os.environ['ZK_TEST_BIN'],'longcat','--backend','umbriel','--socket',socket_path,
                               '--minimum-height','50','--active-height-proportion','0.7','--other-height-proportion','0.3',
                               '--convergence-timeout-ms','3000','--shutdown-timeout-ms','4000'],
                              env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    records=queue.Queue()
    def read():
        for line in process.stdout:
            records.put(json.loads(line))
        records.put({'eof':True})
    threading.Thread(target=read,daemon=True).start()
    return process,records,[]
def wait_phase(run,phase,active=None):
    process,records,seen=run
    deadline=time.monotonic()+10
    while time.monotonic()<deadline:
        row=records.get(timeout=max(.01,deadline-time.monotonic()))
        seen.append(row)
        assert not row.get('eof'), (process.poll(),process.stderr.read(),seen)
        if row.get('type')=='longcat' and (active is None or row.get('active_window')=='text:'+active) and any(g.get('phase')==phase for g in row.get('groups',[])):
            return row
    raise AssertionError((phase,seen))
def stop(run):
    process,records,seen=run
    process.send_signal(signal.SIGINT)
    process.wait(timeout=8)
    error=process.stderr.read()
    assert process.returncode==0, (process.returncode,error,seen)
    while True:
        row=records.get(timeout=2)
        seen.append(row)
        if row.get('eof'):
            break
    return seen

# Exercise real nonzero edge-gap preservation, not merely the zero-gap case.
initial=group()
weights=copy.deepcopy(initial['basis']['weights'])
total=sum(weights['rows'])
weights['top']=total*.05
weights['bottom']=total*.08
call({'cmd':'layout-allocate','target':initial['basis']['members'][0]['id'],
      'expected':initial['basis'],'weights':weights})
settle()
baseline=group()
assert baseline['top_gap']>0 and baseline['bottom_gap']>0
run=start()
settled=wait_phase(run,'Settled')
emphasized=group()
active=layout()['active']
index=next(i for i,m in enumerate(emphasized['basis']['members']) if m['id']==active)
assert emphasized['geometry'][index]['committed_content']['height'] > baseline['geometry'][index]['committed_content']['height']
assert emphasized['top_gap']==baseline['top_gap'] and emphasized['bottom_gap']==baseline['bottom_gap']
effect=settled['groups'][0]['requested_effect']
assert effect['commands']==[] and effect['group_command']=='Handled', effect
other=next(m['id'] for m in baseline['basis']['members'] if m['id']!=active)
action('window-focus:'+other)
wait_phase(run,'Settled',other)
seen=stop(run)
settle()
assert group()['basis']['weights']==baseline['basis']['weights'], (group(),baseline,seen)
assert len([r for r in seen if r.get('type')=='longcat-group-command'])==3, seen
assert all(r.get('type')!='longcat-command' for r in seen), 'fabricated per-window receipts'

# A manual weight edit with exactly the same rounded pixels revokes authority.
run=start()
wait_phase(run,'Settled')
current=group()
weights=copy.deepcopy(current['basis']['weights'])
weights['rows']=[w*2 for w in weights['rows']]
weights['top']*=2
weights['bottom']*=2
manual=call({'cmd':'layout-allocate','target':current['basis']['members'][0]['id'],
             'expected':current['basis'],'weights':weights})
wait_phase(run,'Lost')
seen=stop(run)
assert group()['basis']['weights']==manual['accepted']['basis']['weights']
assert len([r for r in seen if r.get('type')=='longcat-group-command'])==1, seen
print('ZK + native Umbriel: emphasis, focus switch, exact raw-weight restore, group receipts and external-edit revocation passed')
PY
