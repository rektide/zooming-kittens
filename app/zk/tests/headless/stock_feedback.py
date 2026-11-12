#!/usr/bin/env python3
"""Real stock snapshots and ordinary zk, never a fixture corrective controller."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import threading
import time


RUNTIME = Path(os.environ["UMBRIEL_RUNTIME_DIR"])
assert os.environ.get("UMBRIEL_HARNESS_LIB"), "isolated native harness required"
SOCKET = Path(os.environ["XDG_RUNTIME_DIR"]) / (
    "umbriel-" + os.environ["WAYLAND_DISPLAY"] + ".sock"
)
assert SOCKET.parent.resolve() == RUNTIME.resolve(), "refuse a non-harness socket"
BASE = Path(os.environ["UMBRIEL_CONFIG"]).read_text()


def call(request):
    with socket.socket(socket.AF_UNIX) as stream:
        stream.settimeout(5)
        stream.connect(str(SOCKET))
        stream.sendall((json.dumps(request) + "\n").encode())
        reply = json.loads(stream.makefile("rb").readline())
        assert "ok" in reply, reply
        return reply["ok"]


def action(name, *args):
    subprocess.run([os.environ["UMBRIEL"], "msg", name, *map(str, args)],
                   check=True, stdout=subprocess.DEVNULL)


def settle():
    subprocess.run([os.environ["UMBRIEL"], "settle"],
                   check=True, stdout=subprocess.DEVNULL)


def windows():
    return call({"cmd": "windows"})


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.01)
    raise AssertionError((description, windows()))


def configure(axis="vertical", strut=20):
    Path(os.environ["UMBRIEL_CONFIG"]).write_text(BASE + f"""
[animation]
enabled = false
[input.focus]
follows_mouse = false
[appearance.title_bar]
mode = "on"
height = 28
[layout]
gap = 10
[layout.scrolling]
center_underfull_strip = false
[layout.struts]
top = {strut}
bottom = {strut}
left = {strut}
right = {strut}
[[window_rule]]
match.title = "^zk-stock-"
default_floating = false
[[window_rule]]
match.title = "^zk-stock-1$"
title_bar = "off"
[output."HEADLESS-1"]
workspace_axis = "{axis}"
""")
    action("config-reload")


def spawn(title, **extra):
    env = dict(os.environ, **{k: str(v) for k, v in extra.items()})
    log = (RUNTIME / (title + ".log")).open("w")
    subprocess.Popen([os.environ["ZK_EDGE_CLIENT"], title], env=env,
                     stdout=log, stderr=subprocess.STDOUT)
    log.close()
    row = wait(lambda: next((w for w in windows() if w["title"] == title), None),
               "client map " + title)
    settle()
    return row["id"]


def group(ids):
    rows = {w["id"]: w for w in windows()}
    return sorted((rows[i] for i in ids), key=lambda w: w["row"])


def extents(ids, axis="vertical"):
    return [w["h" if axis == "vertical" else "w"] for w in group(ids)]


def invariants(ids, axis="vertical"):
    rows = group(ids)
    start, size, primary, psize = (("y", "h", "x", "w") if axis == "vertical"
                                    else ("x", "w", "y", "h"))
    return dict(start=rows[0][start], end=rows[-1][start]+rows[-1][size],
                strips=[b[start]-a[start]-a[size] for a, b in zip(rows, rows[1:])],
                primary=[(w[primary], w[psize]) for w in rows])


class Run:
    def __init__(self, name, timeout=4000, debounce=500):
        self.rows = []
        self.lock = threading.Lock()
        self.name = name
        self.output = (RUNTIME / (name + ".jsonl")).open("w")
        self.error = (RUNTIME / (name + ".stderr")).open("w+")
        env = {k: v for k, v in os.environ.items() if not k.startswith("ZK_")}
        env["XDG_CONFIG_HOME"] = str(RUNTIME / "zk-config")
        self.process = subprocess.Popen([
            os.environ["ZK_TEST_BIN"], "longcat", "--backend", "umbriel",
            "--socket", str(SOCKET), "--minimum-height", "60", "--minimum-width", "60",
            "--resize-debounce-ms", str(debounce),
            "--convergence-timeout-ms", str(timeout), "--shutdown-timeout-ms", "5000",
        ], env=env, stdout=subprocess.PIPE, stderr=self.error, text=True)
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()

    def read(self):
        for line in self.process.stdout:
            self.output.write(line)
            self.output.flush()
            row = json.loads(line)
            with self.lock:
                self.rows.append(row)

    def snapshot(self):
        with self.lock:
            return list(self.rows)

    def mark(self):
        return len(self.snapshot())

    def await_row(self, predicate, after=0):
        def found():
            assert self.process.poll() is None, self.failure()
            return next((r for r in self.snapshot()[after:] if predicate(r)), None)
        try:
            return wait(found, self.name + " diagnostic", timeout=12)
        except AssertionError:
            raise AssertionError(self.failure()) from None

    def failure(self):
        self.error.flush()
        rows = self.snapshot()[-8:]
        summary = [{k: r.get(k) for k in ("type", "decision", "counters", "assessment")}
                   for r in rows]
        return (self.name, self.process.poll(), summary,
                (RUNTIME / (self.name + ".stderr")).read_text(), windows())

    def completed(self, active, after=0, limited=False):
        row = self.await_row(lambda r: r.get("type") == "longcat-feedback-completed"
                             and r["attempt"]["goal"]["inputs"]["active"] == active
                             and ("Limited" in r["attempt"]["result"]["end"] if limited
                                  else r["attempt"]["result"]["end"] == "Settled"), after)
        attempt = row["attempt"]
        assert len(attempt["commands"]) <= attempt["goal"]["submission_limit"], attempt
        return attempt

    def feedback(self):
        return next(r for r in reversed(self.snapshot()) if r.get("type") == "longcat-feedback")

    def stop(self):
        self.process.send_signal(signal.SIGINT)
        self.process.wait(timeout=8)
        self.reader.join(timeout=2)
        assert self.process.returncode == 0, self.failure()
        assert self.snapshot()[-1]["type"] == "longcat-feedback-stopped", self.failure()
        self.output.close()
        self.error.close()


def focus(id):
    action("window-focus", id)
    settle()


def check_vector(attempt, ids, axis="vertical", exact=False):
    targets = {t["member"].removeprefix("text:"): t["extent"]
               for t in attempt["goal"]["targets"]}
    actual = {w["id"]: w["h" if axis == "vertical" else "w"] for w in group(ids)}
    tolerance = 0 if exact else 2
    assert all(abs(actual[i]-targets[i]) <= tolerance for i in ids), (actual, targets)
    assert attempt["result"]["achieved"]["coherent"], attempt


def preferences(run):
    return {p["window"]["id"].removeprefix("text:"): p for p in run.feedback()["preferences"]}


def intentions(run):
    return [r for r in run.snapshot() if r.get("type") == "longcat-edge-intent"]


def stable(run, after=0, ids=None):
    return run.await_row(lambda r: r.get("type") == "longcat-feedback"
                         and r["decision"].get("Wait", {}).get("reason") == "Stable"
                         and (ids is None or any(
                             [m["id"].removeprefix("text:") for m in g["observed"]["members"]]
                             == [w["id"] for w in group(ids)] for g in r["groups"])), after)


def stable_for(run, seconds):
    # Real time: prove that a stable residual/continuous drag does not retry.
    count = len(intentions(run))
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        assert run.process.poll() is None, run.failure()
        assert len(intentions(run)) == count, run.failure()
        time.sleep(.01)


def active_id():
    return next(w["id"] for w in windows() if w["active"])


def anchor_box(anchor):
    w = next(w for w in windows() if w["id"] == anchor)
    return [w[k] for k in ("x", "y", "w", "h")]


def clear():
    for w in windows():
        action("window-kill", w["id"])
    wait(lambda: not windows(), "all fixture windows closed")
    settle()


def basic():
    configure()
    anchor = spawn("zk-stock-anchor")
    ids = []
    for i in range(3):
        ids.append(spawn(f"zk-stock-{i}"))
        if i:
            action("window-consume-left")
            settle()
    # Native fixture setup only: heterogeneous rows, nonzero outer gaps.
    action("window-modify-height-up", "-0.04", "--window", ids[0])
    action("window-modify-height-down", "-0.06", "--window", ids[-1])
    action("window-set-secondary-extent", "0.40", "--window", ids[0])
    settle()
    focus(anchor)
    baseline = extents(ids)
    envelope = invariants(ids)
    run = Run("ordinary")
    run.await_row(lambda r: r.get("type") == "longcat-feedback" and len(r["preferences"]) == 3)
    mark = run.mark()
    focus(ids[0])
    attempt = run.completed("text:" + ids[0], mark)
    check_vector(attempt, ids, exact=True)
    assert invariants(ids) == envelope, (invariants(ids), envelope)
    assert active_id() == ids[0], "qualified sizing changed seat activation"
    anchor_geometry = anchor_box(anchor)
    original_preferences = preferences(run)
    print("exact initial emphasis", extents(ids), "baseline", baseline, flush=True)

    # Accepted user edit: same native action a keybinding invokes, not a ZK goal.
    mark = run.mark()
    action("window-modify-height-down", "-0.05", "--window", ids[0])
    settle()
    user = extents(ids)
    run.await_row(lambda r: r.get("type") == "longcat-feedback"
                  and r["counters"]["guidance"] == 1, mark)
    run.await_row(lambda r: r.get("type") == "longcat-feedback"
                  and r["counters"]["guidance"] == 1
                  and r["decision"].get("Wait", {}).get("reason") == "Stable", mark)
    assert extents(ids) == user, "matching accepted guidance required no correction"
    learned = preferences(run)
    assert learned[ids[0]]["neutral_weight"] == original_preferences[ids[0]]["neutral_weight"]
    assert abs(learned[ids[0]]["active_share"] - user[0]/sum(user)) < 1e-9, (learned, user)
    assert anchor_box(anchor) == anchor_geometry
    assert invariants(ids) == envelope

    mark = run.mark()
    focus(anchor)
    neutral = run.completed(None, mark)
    check_vector(neutral, ids)
    assert extents(ids) != baseline, "blur incorrectly restored process-start proportions"
    neutral_vector = extents(ids)
    assert preferences(run) == learned, "owned neutral restoration taught preferences"
    mark = run.mark()
    focus(ids[0])
    returned = run.completed("text:" + ids[0], mark)
    check_vector(returned, ids)
    assert all(abs(a-b) <= 2 for a, b in zip(extents(ids), user)), (extents(ids), user)
    assert preferences(run) == learned
    assert anchor_box(anchor) == anchor_geometry
    print("accepted resize / current-neutral blur / learned focus return", user,
          neutral_vector, flush=True)

    # Continued motion crosses the default 500ms quiet interval. Focus changes
    # inside that motion must not bypass its trailing timer.
    mark = run.mark()
    action("window-modify-height-down", "-0.004", "--window", ids[0])
    settle()
    run.await_row(lambda r: r.get("type") == "longcat-feedback"
                  and r["decision"].get("Wait", {}).get("phase") == "Yielding", mark)
    count = len(intentions(run))
    started = time.monotonic()
    for i in range(12):
        action("window-modify-height-down", "-0.004", "--window", ids[0])
        settle()
        if i == 4:
            focus(ids[1])
        if i == 6:
            focus(ids[0])
        stable_for(run, .06)
        assert len(intentions(run)) == count, "controller fought continuous user motion"
    assert time.monotonic()-started > .5
    drag = extents(ids)
    run.await_row(lambda r: r.get("type") == "longcat-feedback"
                  and r["counters"]["guidance"] == 2, mark)
    run.await_row(lambda r: r.get("type") == "longcat-feedback"
                  and r["counters"]["guidance"] == 2
                  and r["decision"].get("Wait", {}).get("reason") == "Stable", mark)
    assert extents(ids) == drag
    assert abs(preferences(run)[ids[0]]["active_share"]-drag[0]/sum(drag)) < 1e-9
    assert invariants(ids) == envelope
    assert active_id() == ids[0]
    assert anchor_box(anchor) == anchor_geometry
    print("continuous native resize / focus during yield / trailing guidance", drag, flush=True)

    # Reorder and cross-group moves keep identity-owned weights/share, not the
    # emphasized pixels or ordinal column keys. All edits are native actions.
    retained = preferences(run)
    mark = run.mark()
    action("window-move-down")
    settle()
    stable(run, mark, ids)
    assert [w["id"] for w in group(ids)] == [ids[1], ids[0], ids[2]]
    assert preferences(run) == retained
    mark = run.mark()
    action("window-consume-or-expel-right")
    settle()
    remaining = [ids[1], ids[2]]
    stable(run, mark, remaining)
    assert preferences(run) == retained
    assert len({w["column"] for w in group(ids)}) == 2
    mark = run.mark()
    action("window-consume-left")
    settle()
    stable(run, mark, ids)
    assert preferences(run) == retained

    mark = run.mark()
    incoming = spawn("zk-stock-incoming")
    action("window-consume-left")
    settle()
    expanded = ids + [incoming]
    stable(run, mark, expanded)
    assert all(preferences(run)[id] == p for id, p in retained.items())
    assert incoming in preferences(run)
    mark = run.mark()
    action("window-kill", incoming)
    settle()
    stable(run, mark, ids)
    assert preferences(run) == retained
    mark = run.mark()
    focus(anchor)
    stable(run, mark, ids)
    anchor_geometry = anchor_box(anchor)
    # The same structured qualified action used by the executable must reject a
    # genuinely stale ID, never fall back to the unrelated seat-focused anchor.
    with socket.socket(socket.AF_UNIX) as stream:
        stream.connect(str(SOCKET))
        stream.sendall((json.dumps({"cmd": "action", "action": "window-modify-height-down",
                                   "delta": .1, "window": incoming}) + "\n").encode())
        rejected = json.loads(stream.makefile("rb").readline())
    assert rejected == {"err": "unknown window: " + incoming}, rejected
    assert active_id() == anchor and anchor_box(anchor) == anchor_geometry
    print("native reorder / expel-rejoin / open-close / stale qualified target", flush=True)

    mark = run.mark()
    focus(ids[0])
    stable(run, mark, ids)
    run.stop()
    # SIGINT restores the latest neutral preferences via the same real executor.
    stopped = next(r for r in reversed(run.snapshot())
                   if r.get("type") == "longcat-feedback-completed"
                   and r["attempt"]["goal"]["inputs"]["active"] is None)
    check_vector(stopped["attempt"], ids)
    clear()


def churn():
    configure()
    anchor = spawn("zk-stock-anchor")
    ids = [spawn("zk-stock-0"), spawn("zk-stock-1")]
    action("window-consume-left")
    settle()
    focus(anchor)
    run = Run("churn", debounce=40)
    run.await_row(lambda r: r.get("type") == "longcat-feedback" and len(r["preferences"]) == 2)
    acquired = preferences(run)
    mark = run.mark()
    focus(ids[0])
    run.await_row(lambda r: r.get("type") == "longcat-feedback"
                  and r["decision"].get("Wait", {}).get("reason") == "Quiet"
                  and r["current"] is not None, mark)
    mark = run.mark()
    focus(ids[1])
    stable(run, mark, ids)
    assert preferences(run) == acquired
    for i in range(70):
        mark = run.mark()
        focus(ids[i % 2])
        attempt = run.completed("text:" + ids[i % 2], mark)
        check_vector(attempt, ids)
        assert preferences(run) == acquired
        assert active_id() == ids[i % 2]
    assert run.feedback()["counters"]["settled"] >= 70
    assert run.feedback()["counters"]["guidance"] == 0
    print("70 real focus changes: neutral preferences unchanged, no lifetime veto", flush=True)
    run.stop()
    clear()


def matrix():
    cases = [
        ("negative-vertical", "vertical", -80, 4, "none", 0),
        ("positive-horizontal", "horizontal", 20, 4, "none", 0),
        ("negative-horizontal", "horizontal", -80, 3, "none", 0),
        ("slow", "vertical", 0, 3, "none", 120),
        ("fixed-middle", "vertical", 0, 3, "fixed", 0),
        ("recipient-maximum", "vertical", 0, 3, "maximum", 0),
        ("donor-minimum", "vertical", 0, 3, "minimum", 0),
    ]
    for name, axis, strut, count, constraint, delay in cases:
        configure(axis, strut)
        anchor = spawn("zk-stock-anchor")
        ids = []
        for i in range(count):
            extra = {"COMMIT_DELAY_MS": delay}
            if constraint == "fixed" and i == 1:
                extra.update(MIN_H=100, MAX_H=100)
            if constraint == "maximum" and i == 0:
                extra.update(MAX_H=180)
            if constraint == "minimum" and i == 1:
                extra.update(MIN_H=230)
            ids.append(spawn(f"zk-stock-{i}", **extra))
            if i:
                action("window-consume-left")
                settle()
        focus(anchor)
        run = Run(name)
        run.await_row(lambda r: r.get("type") == "longcat-feedback"
                      and len(r["preferences"]) == count)
        before = preferences(run)
        generation = run.feedback()["preference_generation"]
        envelope = invariants(ids, axis)
        anchor_geometry = anchor_box(anchor)
        mark = run.mark()
        focus(ids[0])
        result = run.await_row(lambda r: r.get("type") == "longcat-feedback-completed"
                               and r["attempt"]["goal"]["inputs"]["active"] == "text:"+ids[0]
                               and (r["attempt"]["result"]["end"] == "Settled"
                                    or "Limited" in r["attempt"]["result"]["end"]), mark)
        attempt = result["attempt"]
        end = attempt["result"]["end"]
        assert len(attempt["commands"]) <= attempt["goal"]["submission_limit"], attempt
        assert attempt["commands"], "native scenario must exercise actual stock edges"
        assert all(c["outcome"] == "Handled" for c in attempt["commands"]), attempt
        assert all(r["axis"] == axis.capitalize() for r in intentions(run))
        if constraint == "none":
            assert end == "Settled", (name, end)
            check_vector(attempt, ids, axis)
            actual = invariants(ids, axis)
            assert actual["primary"] == envelope["primary"], (name, actual, envelope)
            assert actual["strips"] == envelope["strips"], (name, actual, envelope)
            assert abs(actual["start"]-envelope["start"]) <= 2
            assert abs(actual["end"]-envelope["end"]) <= 2
        else:
            assert "Limited" in end, (name, end)
            assert attempt["result"]["achieved"] is not None
            assert extents(ids, axis) != [t["extent"] for t in attempt["goal"]["targets"]]
        run.await_row(lambda r: r.get("type") == "longcat-feedback" and r["current"] is None, mark)
        stable_for(run, 1.1)
        assert preferences(run) == before, (name, preferences(run), before)
        assert run.feedback()["preference_generation"] == generation
        assert run.feedback()["counters"]["guidance"] == 0
        assert active_id() == ids[0] and anchor_box(anchor) == anchor_geometry
        print(name, end, "committed", extents(ids, axis), "edges", len(attempt["commands"]), flush=True)
        run.stop()
        clear()


selected = os.environ.get("ZK_STOCK_CASE", "all")
if selected in ("all", "basic"):
    basic()
if selected in ("all", "churn"):
    churn()
if selected in ("all", "matrix"):
    matrix()
