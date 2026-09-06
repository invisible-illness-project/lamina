# Lamina Validation — Potential / Confirmed Bug Log

This file records candidate defects and unexpected behaviors discovered while
validating the **current** Lamina implementation against public datasets.

**Scope: validation findings only. No Lamina source modification is performed
in this task.** Another engineering team owns triage and fixes. Each entry
must contain enough evidence to reproduce the behavior.

## Status taxonomy

| Status | Meaning |
| ------ | ------- |
| `confirmed` | Reproduced against the current implementation with a minimal input; behavior contradicts the documented/API-contract expectation. |
| `suspected` | Evidence points to an implementation defect but reproduction is incomplete or confounded. |
| `dataset-issue` | Root cause traced to the dataset/adapter (parsing, units, channel selection, annotation semantics), not Lamina. |
| `ambiguous` | Unexpected result; evidence insufficient to classify as defect or dataset issue. |
| `limitation` | Expected/by-design limitation of the current implementation (documented here for visibility, not a defect). |

Severity: `Informational` / `Low` / `Medium` / `High` / `Critical`.

## Entry format

```markdown
## BUG-XXX — Short Description

### Status
Suspected / Confirmed / Dataset-issue / Ambiguous / Limitation

### Component
Lamina component/API involved.

### Dataset
Dataset and version (or synthetic fixture).

### Reproduction
Exact command or minimal reproduction procedure.

### Input
Signal characteristics, sampling rate, relevant recording/segment.

### Expected Behavior
What should reasonably occur.

### Actual Behavior
What Lamina produced.

### Evidence
Metrics, output, plots, logs, or references.

### Severity
Informational / Low / Medium / High / Critical

### Suggested Investigation
Optional technical hypothesis about the cause.

### Scope
Validation finding only. No source modification performed.
```

## Entries

_No entries yet. Candidate sharp edges noted during framework bring-up (e.g.
`ecg_clean` ignoring its `method` argument, `eda_findpeaks`/`rsp_findpeaks`
hardcoding fs=100 Hz, `sample_entropy` returning +inf on zero matches) are
listed in `validation/API-INVENTORY.md` §"Sharp edges" and are pending
verification by the Stage 4 reviewer before being logged here._
