---
type: DeploymentPreparation
title: Composed Umbriel Longcat deployment candidate
description: Exact preserved base, narrow native port, verified release artifact, staged config compatibility and reversible user-selected session procedure.
resource: design/longcat/deployment-candidate0.gpt6a.md
status: prepared-not-installed
generated: { at: 2026-10-22, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e9a70297dffe6dQ5UJP0up8m7T
    title: Prepare the composed deployment candidate without installation or live compositor disruption; isolate the committed-size stream gap and verify the corrected port
  - resource: umbriel-control-stream0.gpt6a.md
    title: Earlier source/deployment findings and delivered native/Rust/ZK integration
  - resource: ../../app/zk/src/runner/README.md
    title: Consumer native-authority and headless integration contract
  - resource: ../../app/zk/tests/headless/umbriel_longcat.sh
    title: Integrated sizing, focus switch, native restore and external-edit check, with explicit no-titlebar base support
  - resource: file:///home/rektide/src/umbriel-longcat-deploy/docs/user/layout-control.md
    title: Candidate wire contract and zero-titlebar adaptation
  - resource: file:///home/rektide/src/umbriel-longcat-deploy/src/view/view_lifecycle.cpp
    title: Small committed-size notification hook required by streaming convergence
  - resource: file:///home/rektide/src/umbriel-longcat-deploy/PACKAGING.md
    title: Compiled data prefix, static umbrielfx linkage and systemd launcher mechanics
  - resource: file:///home/rektide/src/umbriel-longcat-deploy/CONTRIBUTING.md
    title: Debug/release builds and isolated headless verification
  - resource: file:///usr/local/lib/systemd/user/umbriel.service
    title: Observed deployed unit invoking /usr/local/bin/umbriel
  - resource: file:///home/rektide/src/umbriel-longcat-deploy/.test-agent/rollback/README.md
    title: Preserved actual running executable and installed-config snapshots
verification:
  method: Debug and release builds, pure scrolling suite, native allocation/IPC/output-power checks, native and integrated four-instance headless stress, read-only config validation and artifact hashes.
  limits: No installation, systemd changes, compositor restart, graphical session transition or live desktop mutation. Headless behavioral checks use the debug build of the exact candidate source; release checks establish build, version, CLI and config compatibility.
extensions:
  ticket: zk-longcat-return-native-deploy
---

# Composed deployment candidate

**Ready for a user-selected deployment/session transition, not installed.**
The isolated worktree is `/home/rektide/src/umbriel-longcat-deploy`. Its named
candidate branch is `longcat-deploy-candidate-20261022`, pointing to
**`96b118cf8b1135d7915ee3c560b17206f4d703f6`**.

## Exact composition

Base: **`1845885e622a6cd923b0155cd1f967fcfe793714`**, the clean current
`umbriel-working` composed tip. The original worktree and its `working` /
`working-20261009` references were not moved. No upstream rebase/reset was used.

The candidate adds three commits:

| Commit | Contribution |
|---|---|
| `fdb6f7ffbeecdf899d75ccfc1e8ac1a52d6cbcfe` | Port of native allocation commit `14dab546`, resolving event-family differences narrowly |
| `2038cfca2198d8bef4bcf0bfb635028214a8ffb0` | Port of raw-weight/context correction `5432ace1` |
| `96b118cf8b1135d7915ee3c560b17206f4d703f6` | Zero-titlebar compatibility and committed-content-size stream publication |

Only 15 source/build/test/document paths differ from the chosen composed base.
Its local output-power, output-arrangement and scrolling-padding work remains in
the ancestry. The newer upstream branch was not merged wholesale.

The running binary is older than that already-selected composed base:
`51e6528ecf3d` is an ancestor with **223 commits** between it and `1845885e`.
Thus the eventual session transition activates the existing composed upgrade as
well as Longcat. Those 223 commits are not introduced by this port.

## Necessary differences from the upstream-based test branch

1. **Events:** retain the seven existing families, add only `layout-state` at bit
   7, widen enum/request/subscription/broadcast masks consistently to `uint16_t`,
   and size the cache/name table to eight families. Newer output/layer families
   are not imported. Wire clients select the family by name, not by its bit.
2. **Decorations:** this base predates server-side title bars. Every member reports
   `title_bar_height: 0`; native target tile/content boxes coincide. Existing tabs
   remain unsupported for allocation. No SSD implementation was imported.
3. **Committed progress:** the initial port sized correctly, but ZK timed out
   because only target arrangement was pushed; a query showed committed pixels
   had changed while the subscription still held old sizes. The upstream-sized
   `m_ipcContentSize` cache and seven-line mapped-commit publication hook fix this
   directly. The native check now requires a stream event with committed target
   sizes, not merely an accepted basis.

This was a small compatibility port, not a broad unrelated upstream migration.

## Verification and artifacts

From the candidate directory:

```sh
just configure debug
meson compile -C build-debug -j 4 umbriel scrolling-layout-test harness-clients
meson test -C build-debug scrolling-layout --print-errorlogs
bash tests/harness/check.sh ./build-debug/umbriel \
  layout/conditional_allocation session/ipc protocol/output_power -j 3
just check-stress layout/conditional_allocation 4

ZK_NATIVE_TITLEBARS=0 \
ZK_TEST_BIN=/home/rektide/src/zooming-kittens/target/debug/zk \
CHECK_DIR=/home/rektide/src/zooming-kittens/app/zk/tests/headless \
  bash tests/harness/check.sh ./build-debug/umbriel umbriel_longcat -j 1

just configure release
meson compile -C build-release -j 4 umbriel
```

All listed checks passed. Four concurrent independent integrated ZK headless runs
also passed with nonzero edge gaps, focus switching, exact raw-weight restoration
and external same-pixel edit revocation. One earlier stress attempt overlapped a
`just`-triggered binary relink and got executable permission errors; after builds
completed, the unchanged tests passed. Finish builds before launching checks
against that binary path.

Release artifact: `build-release/umbriel`, reporting
`umbriel 0.1.0 (96b118cf8b11)`.
SHA-256: `5049ed50d8f14b19880dd917bc27344ad3190ef22626af564a9fa741eb971934`.
Its CLI exposes `layout-state` and excludes harness-only `settle`. The compiled
data prefix remains `/usr/local`; `umbrielfx` is linked statically.

## Configuration and rollback material

The running process and the research shell both had unset `XDG_CONFIG_HOME`,
`XDG_CONFIG_DIRS` and `UMBRIEL_CONFIG`, and the process had no `-c` argument.
Candidate validation selected `/usr/local/share/umbriel/config.toml` and reported
two obsolete keys: `layout.dwindle.new_exits_fullscreen` and
`layout.master.new_exits_fullscreen`, both `false`.

A staged copy removes **only those two lines**. The composed schema's global
`layout.new_exits_fullscreen` defaults to `None`, preserving their false meaning.
Debug and release validation both accept the staged file. The installed config
was not modified or replaced with the substantially different example config.

Rollback directory: `~/src/umbriel-longcat-deploy/.test-agent/rollback/` (ignored,
retained deliberately):

| File | SHA-256 |
|---|---|
| `umbriel-51e6528ecf3d` — actual `/proc/12558/exe` copy | `008125e744b9df44163c21ee72884fe717198bb8d1c40199a1f76b5af7a97356` |
| `config.toml` — installed config snapshot | `95298dc1ba9403039725de5012a5ddcd0fb51756c9c1d75dc13d4ad1e56cf2cb` |
| `config-candidate.toml` — two-line compatibility change | `e0081ad9e1e35a8d1bfefed1f707b6cd023d63851955e62b6d2739c299c02303` |

The original installed executable had the same hash as the captured running
executable. Keep these copies through the deployment decision.

## User-run installation and session transition

These commands are the prepared procedure; **none was executed by this session**.
Use side-by-side installation and a dedicated user-unit drop-in so the original
binary, config, launcher and unit remain available unchanged.

After choosing deployment, stage the named candidate:

```sh
C="$HOME/src/umbriel-longcat-deploy"
sudo install -Dm755 "$C/build-release/umbriel" \
  /usr/local/libexec/umbriel/umbriel-longcat-96b118cf
sudo install -Dm644 "$C/.test-agent/rollback/config-candidate.toml" \
  /usr/local/etc/umbriel/longcat-96b118cf.toml
/usr/local/libexec/umbriel/umbriel-longcat-96b118cf config validate \
  -c /usr/local/etc/umbriel/longcat-96b118cf.toml
```

Create `~/.config/systemd/user/umbriel.service.d/90-longcat-candidate.conf`:

```ini
[Service]
ExecStart=
ExecStart=/usr/local/libexec/umbriel/umbriel-longcat-96b118cf -c /usr/local/etc/umbriel/longcat-96b118cf.toml
```

Run `systemctl --user daemon-reload`. This changes the next launch, not the current
process. Finish/save application work, log out of the graphical session, then
launch Umbriel through the existing login entry or `start-umbriel`. An independent
TTY remains the place to revert the drop-in if startup fails.

After the new session starts:

```sh
systemctl --user show umbriel.service -p MainPID -p ExecStart
"/proc/$(systemctl --user show umbriel.service --value -p MainPID)/exe" --version
/usr/local/libexec/umbriel/umbriel-longcat-96b118cf layout-state --json
~/src/zooming-kittens/target/debug/zk longcat --backend umbriel --dry-run --limit 1
```

The existing `/usr/local/bin/umbriel` CLI remains the old binary under this
side-by-side procedure; use the named candidate for its new inspection command.
ZK talks directly to the socket. Actual sizing is then
`zk longcat --backend umbriel`; the parent-owned sizing service generator is
`zk generate-systemd --output longcat --backend umbriel` and embeds the invoked
ZK executable's absolute path. Its activation remains a separate user action.

### Rollback

Stop the Longcat sizing service/foreground process, finish the candidate session,
and from a TTY rename only the added drop-in to
`90-longcat-candidate.conf.disabled`. Run `systemctl --user daemon-reload` and
start the usual Umbriel session again. The untouched original unit selects the
untouched `/usr/local/bin/umbriel` and original default config. Candidate files
can remain as evidence; cleanup is deliberate. The captured executable/config
are additional exact rollback copies if those original paths later change.

## Connections and standing

- [Implementation account](umbriel-control-stream0.gpt6a.md): source evidence,
  native/Rust API, integrated consumer and feature verification.
- [Runner contract](../../app/zk/src/runner/README.md): native authority, receipt
  semantics and test entrypoint.
- `zk-longcat-return-native-deploy` remains open for the actual user-selected
  installation/session transition; preparation is complete, deployment is not.
