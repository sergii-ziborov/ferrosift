# Agent benchmark (FS-A09)

Independent comparison harness for whether FerroSift helps an agent finish
real data-transform tasks better than ordinary Python/shell and existing
CyberChef MCP surfaces.

This directory seeds the experiment. It is **not** a published scoreboard.

## Four variants (same files, permissions, model, budget)

| Id | Agent receives | Why it exists |
|----|----------------|---------------|
| `A` | Shell + Python/stdlib + docs | Strongest everyday baseline |
| `B` | Existing CyberChef MCP (recommended discovery/batching) | Market alternative |
| `C` | FerroSift CLI + skill | Library value without MCP |
| `D` | FerroSift MCP (handles + structured results) | Adapter incremental value |

Do not force competitors to load hundreds of tool definitions if they already
avoid that. Do not weaken variant `A`.

## Task groups (pilot starts small)

1. Multilayer payload → regression test  
2. Encoding / representation repair  
3. Protocol / framing bug  
4. Pattern fields / unsupported construct  
5. Negative / resource refusal  
6. Handoff into a clean-process package  

Full design target is 24 tasks (4 per group). The seed below starts with a
cheap pilot of a few tasks and an offline checker for the FerroSift paths.

## Ground truth

Expected outcomes come from specification, an independent implementation, an
upstream reference, or a manually reviewed fixture — **not** from “FerroSift
wrote expected.json and then compared to itself.” Self-observed repro packages
remain useful for CI regression (`ferrosift repro check`) but are labelled
separately from independent success criteria.

## Metrics (record, do not hide)

Primary: share of tasks completed correctly **with** a reproducible offline
test.

Secondary: time-to-correct (median + tail), tokens, cost snapshot, tool calls,
budget refusals, human interventions, clean-process replays.

Publish failures, unsupported cases, adapter errors, and timeouts.

## Layout

```text
docs/agent-benchmark/
  README.md
  variants.json
  tasks/<task-id>/
    task.json          # prompt, group, criteria, allowed surfaces
    input.bin          # shared sample bytes
    expected/          # independent checks (not FerroSift self-hash)
    notes.md           # optional human notes
  runner/
    check_offline.py   # validates manifests + FerroSift offline criteria
```

## Offline checker (no agent)

```bash
python docs/agent-benchmark/runner/check_offline.py
python docs/agent-benchmark/runner/check_offline.py --task T01-hex-space
```

Requires a built `ferrosift` on `PATH`, or set `FERROSIFT_BIN`.
