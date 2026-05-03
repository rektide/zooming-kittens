# Niri export

The `export` command prints niri IPC state as typed records. It is a discovery tool and a machine-readable feed for consumers that want raw niri data without going through longcat or zoomer behavior.

```bash
kitty-focus-tracker export
```

With no mode flags, `export` prints a snapshot and then follows the event stream. The default format is JSON Lines because a stream has no natural end.

## Modes

| Flag | Behavior |
|---|---|
| none | Print a snapshot, then stream events. |
| `--snapshot` | Print one finite snapshot and exit. |
| `--stream` | Follow niri events. |
| `--snapshot --stream` | Print a snapshot, then follow events. |

Examples:

```bash
kitty-focus-tracker export --snapshot --format json
kitty-focus-tracker export --stream --format jsonl
kitty-focus-tracker export --snapshot --stream --filter window=4
```

## Formats

| Format | Behavior |
|---|---|
| `json` | One JSON array containing all emitted records. |
| `jsonl` | One JSON object per line. |

`--format json` requires `--limit` when streaming, otherwise the command would need to buffer an unbounded array forever. `jsonl` is the default whenever streaming is enabled.

## Record shape

Records are flat objects with a `type` field. The rest of the fields come from niri IPC, except for explicit export fields such as `stream_event_id`.

Snapshot examples:

```json
{"type":"version","version":"26.4.0"}
{"type":"workspace","id":9,"idx":5,"name":null,"output":"DP-3","is_focused":false}
{"type":"window","id":42,"title":"shell","app_id":"kitty","workspace_id":9}
```

Stream examples:

```json
{"type":"window_closed","stream_event_id":1,"id":42}
{"type":"window_layouts_changed","stream_event_id":2,"changes":[[42,{"pos_in_scrolling_layout":[1,1]}]]}
```

`stream_event_id` is assigned by zooming-kittens. It is not a niri event ID. It is monotonic within one `export --stream` run and starts at `--starting-id`, which defaults to `1`.

## Snapshot records

Snapshot mode currently emits these record types:

| Type | Source |
|---|---|
| `version` | `Request::Version` |
| `window` | `Request::Windows` |
| `workspace` | `Request::Workspaces` |
| `output` | `Request::Outputs` |
| `layer` | `Request::Layers` |
| `cast` | `Request::Casts` |
| `keyboard_layouts` | `Request::KeyboardLayouts` |
| `overview` | `Request::OverviewState` |

Output records can be noisy because niri reports every available monitor mode. Use `--current-mode` to replace the full `modes` list and `current_mode` index with the current mode object.

```bash
kitty-focus-tracker export --snapshot --filter output --current-mode
```

## Stream records

Stream mode emits one record per niri event. Batch events stay batched unless a future `--unpack` mode is added.

Common stream record types:

| Type | Meaning |
|---|---|
| `windows_changed` | Full window set changed. |
| `window_opened_or_changed` | One window opened or changed. |
| `window_closed` | One window closed. |
| `window_focus_changed` | Focus changed to a window or to none. |
| `window_layouts_changed` | Layout changed for one or more windows. |
| `workspaces_changed` | Full workspace set changed. |
| `workspace_activated` | Workspace activation changed. |
| `casts_changed` | Full cast set changed. |
| `cast_started_or_changed` | One cast started or changed. |
| `cast_stopped` | One cast stopped. |

## Filters

Filters are comma-separated and ORed. If any filter matches, the record is emitted. If no filter is provided, all records are emitted.

| Filter | Meaning |
|---|---|
| `window` | All window records and window-related events. |
| `window=42` | Records directly related to window `42`. |
| `workspace` | All workspace records and workspace-related events. |
| `workspace=9` | Records directly related to workspace `9`. |
| `output` | All output records. |
| `output=DP-3` | Output `DP-3`. |
| `layer` | All layer records. |
| `layer=namespace` | Layer surface with the given namespace. |
| `cast` | All cast records and cast-related events. |
| `cast=12` | Cast stream ID `12`. |
| `keyboard_layouts` | Keyboard layout snapshot and events. |
| `overview` | Overview snapshot and events. |
| `version` | Version record. |
| `type=window_layouts_changed` | Exact output record type. |

Examples:

```bash
kitty-focus-tracker export --filter window=42,workspace=9
kitty-focus-tracker export --stream --filter type=cast_started_or_changed
kitty-focus-tracker export --snapshot --filter output,cast --current-mode
```

Future filters may support nested field predicates such as `window.app_id=kitty` or `cast.target.window=42`.

## Pipeline architecture

Export uses a simple pipeline:

```text
niri IPC source -> export records -> filters -> writer
```

Snapshot mode gathers finite niri responses, converts them into flattened records, filters them, and writes them as JSON or JSONL.

Stream mode connects to `Request::EventStream`, converts each event into one flattened record, filters it, and writes it immediately. Stream output preserves niri event order. Batch events such as `window_layouts_changed` remain a single record containing the original batch.

Serde JSON is the production writer because niri IPC types already derive serde traits. A gated `export-facet` scaffold exists for future facet-rs experiments, but `facet-json` cannot serialize `niri_ipc` payloads directly unless upstream types implement `Facet` or local proxy types are added.
