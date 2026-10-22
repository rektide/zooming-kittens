---
type: Research
title: Recent local journald readers and activity-tracking connections
description: Dated June–October 2026 survey of local journal replay, binary-safe ingestion, activity proposals, and adjacent observation work for ZK and Umbriel.
resource: /design/activity/journald-survey0.gpt6a.md
status: draft
generated:
  at: 2026-10-22
  by: model:openai/gpt-6-astra#medium
sources:
  - resource: opencode://ses_e9a70537bffeI1H23pKsQ37jBa
    title: Surveyed local source, hidden design directories, ticket snapshots and path-specific Git history; compared reader reuse and linked native work items
  - resource: file:///home/rektide/src/cyclrs/source/journald/README.md
    title: Native bounded reader contract, implementation and verification limits
  - resource: file:///home/rektide/src/cyclrs/source/journald/src/lib.rs
    title: Feed, delivery, native field enumeration and bounded wait implementation
  - resource: file:///home/rektide/src/cyclrs/source/journald/src/read.rs
    title: Exact cursor validation and finite replay algorithm
  - resource: file:///home/rektide/src/cyclrs/design/architecture-refresh/journal-syn0.gpt6astra.md
    title: September research comparison and corrected durability/completeness claims
  - resource: file:///home/rektide/src/phyllited/experiments/journal-export-receiver/src/lib.rs
    title: Incremental binary-safe parser retaining repeated fields and exact wire bytes
  - resource: file:///home/rektide/src/phyllited/experiments/journal-oracle/README.md
    title: Differential oracle and privileged live-run limitations
  - resource: file:///home/rektide/src/phyllited/.design/journald/otel0.gpt56.md
    title: Lossless journal-to-OTel mapping proposal
  - resource: file:///home/rektide/src/umbriel-rs/design/activity/prior-art0.glm53fx.md
    title: Earlier October survey, read as prior research rather than independently reverified exhaustive claims
  - resource: file:///home/rektide/src/umbriel-rs/design/activity/records0.gpt6a.md
    title: Selected field-first tracing output and accepted-identity plus metadata direction
  - resource: file:///home/rektide/src/umbriel-rs/design/activity/service0.gpt6a.md
    title: Service/history commission and ticket ownership
verification:
  method: Read-only source and document inspection, path-specific Git author/committer dates, native bd show and JSONL ticket snapshot inspection; document checker run separately
  limits: No journal runtime probe, build, deployment, source test rerun or session-transcript inspection. Git evidence dates are not deployment dates; external ticket snapshots may lag native stores.
extensions:
  related_work: [umbrs-activity-history, zk-export-stream-id-resume, phyllited-contract-oracle]
---

# Recent journald work for activity tracking

**Cyclrs is the strongest existing native-reader precedent. Phyllited contributes
binary-preserving ingestion and differential fixtures. Umbriel already owns the
activity-history undertaking; ZK should connect to it rather than create a second
undifferentiated history project.**

## Scope and date evidence

The requested four-month interval is **2026-06-22 through 2026-10-22**. Discovery
searched `~/src` for `journalctl`, `journald`, and `sd.journal`, then separately
searched hidden `.design`, `.doc`, and `.beads/issues.jsonl` locations. Upstream
systemd workspaces and vendored trees produce many hits and are not counted as
independent first-party efforts. This is a ranked survey, not proof that no other
reader exists. No filesystem mtime was used as evidence of authorship or recency.

Dates below come from path-specific `git log --format='%H %aI | %cI | %s'`
unless explicitly labeled document or ticket dates. Both author and committer
dates were inspected; table dates are their local calendar dates when they agree.
The source paths link current working files; the commits identify dated history,
not necessarily every later byte in those files.

| Effort | Actual dated evidence | Kind and relevance |
|---|---|---|
| Cyclrs native replay | `65d5d85c850ca9adfefc21ea4cf27d3142bf5a82`, Oct 1, “Implement exact-cursor bounded native journal replay”; `fcb93c9771556b60458170357e29dab81b531b1b`, Oct 1, wait/visibility corrections | Implemented Rust reader; directly reusable algorithm and failure semantics |
| Cyclrs journal research | [comparison](file:///home/rektide/src/cyclrs/design/architecture-refresh/journal-syn0.gpt6astra.md), document date Sep 23 | Earlier inquiry explains why seek, send success and old readable snapshots are insufficient |
| Phyllited ingestion/oracle | `bfdf8cc852a26d57f90f411b152e601ca86dc921`, Aug 13, differential harness; `14e2bf187d2cc462a4e8da78c50718c90b9a4582`, Aug 13, fixture assertions | Implemented export parser and oracle, broader backend replacement effort |
| Umbriel activity | `1c31836`, Oct 18, service plan; `acc4472`, Oct 19, accepted metadata and prior-art integration | Proposed activity history, selected tracing-first direction; five-second default already recorded as delivered |
| ZK journal helper | `e4612ce`, author Oct 15 UTC / committer Oct 14 -04:00, launch-tool naming update | Existing shell log follower; most behavioral history is Feb 2, outside interval |
| Watchmanmon relay | `bf8151118e43b083e4e86e731dd2c01e76cfeb8c`, Aug 13, optional Vector profile; `b335b300e0699156ebfe9bbb9ccaedcbc4461809`, same day, signal verification docs | Implemented OTLP relay precedent, not journald ingestion |
| Interlock observation interest | `f37549dd9591a388a5ac5e9f80db0d3d430e60da`, Aug 2 | Design proposes logind sleep observation and best-effort journal wake attribution |
| Nodeon observability | [architecture refinement](file:///home/rektide/src/nodeon/design/observability/architecture1.gpt6a.md), document date Oct 20; path history `2b2722d`, author Oct 19 -04:00 / committer Oct 20 UTC | Proposal for bounded saved-evidence tools, referencing Cyclrs and delivered tracing work |

## 1. Cyclrs: reuse the reader mechanics

Read [reader home](file:///home/rektide/src/cyclrs/source/journald/README.md),
[types/native implementation](file:///home/rektide/src/cyclrs/source/journald/src/lib.rs),
and [range algorithm](file:///home/rektide/src/cyclrs/source/journald/src/read.rs).
The current implementation is tightly coupled to Cyclrs provider/target URIs and
its event codec. It is not a ready-made generic Umbriel dependency.

Useful mechanisms:

- `FeedCursor` binds an opaque cursor to its exact selection. Resume validates
  `seek_cursor` → `next` → `test_cursor`; native seek success alone can land near
  a missing entry. No parsing or ordering cursor strings.
- Capture an inclusive upper cut before traversing, resume exclusively after the
  lower cut, revalidate both endpoints and reject range invalidation. This bounds
  the historical pass while writers continue; it is not an atomic snapshot.
- Preserve immutable event data separately from `Delivery` (cursor, journal
  receive time, trusted origin). Original activity time/run/sequence survive export.
- Enumerate raw fields with the data threshold disabled. Avoid convenience maps
  that erase duplicate names or perform lossy UTF-8 conversion.
- Entry, byte and elapsed budgets return failures rather than silently returning
  a newest-N subset. Defaults are 1,024 entries, 4 MiB copied native fields and
  two seconds between operations; native blocking I/O is not interrupted.
- The native handle is thread-local, not Send/Sync. A future asynchronous activity
  follower needs a worker that creates and owns its handle and bounded delivery.

`Reader::wait` supplies a finite wait, but this is **not a complete historical-to-
live streaming application**. The [probe CLI](file:///home/rektide/src/cyclrs/cmd/journal/src/main.rs)
does finite reads/record/init/advance/inspect. Umbriel still owns follow-loop
cancellation, output backpressure, cursor checkpoint timing and initial-tail policy.
Cyclrs's README distinguishes deterministic position fixtures from actual
rotation/vacuum/access-loss injection; do not upgrade the former into the latter.

For observational activity, retain the chosen stock tracing layer. The
[September synthesis](file:///home/rektide/src/cyclrs/design/architecture-refresh/journal-syn0.gpt6astra.md)
explains optional exact readback when an operation requires visibility, and why
that is still not durability. A stored event or surviving old snapshot cannot
certify the absence of a lost suffix.

## 2. Phyllited: ingestion fidelity and a test oracle

The [export receiver](file:///home/rektide/src/phyllited/experiments/journal-export-receiver/README.md)
accepts Journal Export Format through `ForwardToSocket=`. Its
[parser](file:///home/rektide/src/phyllited/experiments/journal-export-receiver/src/lib.rs)
retains ordered raw fields, repeated names, binary values and exact wire bytes.
Its test splits a fixture at every byte boundary. This is a useful fixture and
codec precedent if a journalctl export backend is considered.

It is not a retained-history tailer and does not promise native cursor/query
compatibility. The parser's `Vec` buffer has no explicit size budget in the read
implementation: do not lift it unchanged into a bounded long-running consumer.
`ForwardToSocket=` is documented as synchronous with lazy reconnect after a failed
write; that is a different deployment and failure model from a user-service reader.

The [differential oracle](file:///home/rektide/src/phyllited/experiments/journal-oracle/README.md)
compares forwarded entries with native `journalctl -o export` results. It removes
only storage-generated cursor/sequence metadata, retains timestamps/trusted fields,
and sorts without deduplicating field pairs. Fixtures include syslog/stdout and a
large sealed-memfd entry. Reuse that comparison discipline for native decode vs
rendered export. The live script changes journal configuration with privilege;
none of it was executed in this survey.

The [OTel mapping proposal](file:///home/rektide/src/phyllited/.design/journald/otel0.gpt56.md)
is a meaningful future link-out: normalized attributes must not destroy original
repeated/binary data; boot, invocation and cursor IDs must not become invented
trace IDs. Snapshot tickets date to Aug 13–15: `phyllited-write-demo-export` is
closed, `phyllited-contract-oracle` is in progress, and `phyllited-read-compat-query`
and `phyllited-storage-otel` remain open. These are snapshot statuses, not fresh
native acceptance checks.

## 3. Activity ownership and adjacent interests

Umbriel's [records refinement](file:///home/rektide/src/umbriel-rs/design/activity/records0.gpt6a.md)
selects field-first tracing and accepted identities plus metadata updates. Its
[earlier survey](file:///home/rektide/src/umbriel-rs/design/activity/prior-art0.glm53fx.md)
already found Cyclrs and OSC work. This research adds dated history, Phyllited
fidelity/oracle work, and explicit limits on treating bounded replay as tailing.
It does not reopen the superseded canonical-JSON-blob recommendation.

Native `bd show umbrs-activity-history` confirms an existing open feature with
cursor-aware follow, original-field export, an export-only dispatcher and
rotation/cancellation tests. It already links Cyclrs. Append this survey there,
not a duplicate feature ticket. The original record's `run_id/seq`, compositor
epoch, journal cursor and ZK synthetic stream ID remain different coordinates.

ZK's [journal.sh](/journal.sh) chooses a user unit from `ZOOMING_APPNAME`, follows
it when MainPID is present, otherwise shows 50 recent entries. It is an operational
human view, without structured activity decoding or durable resume. Native
`zk-export-stream-id-resume` was created May 3 and explicitly concerns synthetic
ZK counters, not niri event IDs. Its design warning transfers to journal cursors,
but the implementations must not share a misleading universal “event ID.”

Adjacent work worth following, with narrower claims:

- [Watchmanmon Vector relay](file:///home/rektide/src/watchmanmon/deploy/vector/README.md):
  bounded optional telemetry independent of core collection. Useful downstream
  isolation precedent, not a journald source implementation.
- [Interlock synthesis](file:///home/rektide/src/interlock/.design/interlock/syn2-0.glm52.md):
  proposes `PrepareForSleep` observation and best-effort kernel-journal wake
  attribution. Connect to `umbrs-activity-session-awareness` if suspend-aware
  intervals are pursued; journal clues do not prove human attention.
- [Nodeon refinement](file:///home/rektide/src/nodeon/design/observability/architecture1.gpt6a.md):
  bounded reader/query/check tools and explicit evidence limits are an adjacent
  verification-tools interest, not another journald tailer.

## Exclusions and old-code distinction

The [Systememe introspector](file:///home/rektide/src/systememe/package/introspector/src/cli.ts)
mentions `systemd-journald.socket` only as a D-Bus socket introspection target;
its inspected path history ends May 22 (`e311756`), outside the interval.
[Opencodance's August journal experiment](file:///home/rektide/src/opencodance/experiments/general-tui-journal/src/storage.ts)
is application JSONL/TUI fact storage, not systemd-journal consumption (Aug 12
history `7319dca`, `48cd6ee`).
[Gmellon's broker harness](file:///home/rektide/src/gmellon/dev/harness/broker.py)
checks the host journal socket as a broker-launch prerequisite (Sep 11 history
`471ab6b` through `1c9eef9`); it does not tail journal records. These are real
recent efforts but would be false positives in a journald-reader inventory.

## Recommended next use

1. Keep activity/history ownership in Umbriel and link ZK's consumer inquiry to it.
2. Adapt Cyclrs's native cursor/range machinery to Umbriel's field-first codec;
   make historical-to-live handoff and bounded cancellation an explicit slice.
3. Borrow Phyllited's binary/repeated-field and differential-test ideas, with
   hard input budgets and unprivileged fixtures as the default verification path.
4. Keep resume counter persistence in `zk-export-stream-id-resume` scoped to ZK;
   borrow selection-keying principles without representing it as journal replay.

No implementation or runtime verification is claimed by this document.
