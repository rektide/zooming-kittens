---
type: Reference
title: Native inspection exports
description: Ownership of existing niri JSON and JSONL exports and the optional Facet scaffold.
resource: inspection/README.md
status: implemented
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#medium' }
sources:
  - resource: opencode://ses_ec310f020ffe5d6kIV42ehdO2z
    title: Native export crate extraction and compatibility tests
  - resource: ../design/remake/architecture0.gpt6a.md#domain-grouped-workspace
    title: Native inspection separated from neutral desktop policy
---

# Native inspection

[`export`](export/src/lib.rs), package `zk-export`, owns the existing native
niri snapshot/event record shapes, filters, serialization, and
[`ExportCommand` / `run_export`](export/src/command.rs) imperative shell.
It intentionally depends on niri-ipc rather than the neutral desktop model.
The app reexports its command for `zk export`.

JSON/JSONL formats, native identifiers, synthetic stream-event IDs and defaults
are preserved: no mode flags selects snapshot plus stream; streaming defaults
to JSONL; snapshot-only defaults to JSON. Unbounded streaming JSON is rejected.
The `export-facet` feature retains the existing Facet scaffold and is forwarded
by the application; it does not add a new working export format.

The 13 original export tests and three command/default compatibility tests pass
in the extracted crate. The all-features check passes, with the existing unused
`facet-json` dependency warning. Live niri capture and bounded slow-output
handling were not introduced or verified by this extraction.
