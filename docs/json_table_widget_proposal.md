# JSON ↔ Table Web Widget Proposal (Draft)

## Goals & Scope
- Bidirectional editing: toggle and sync between JSON editor and table (grid) view.
- Auto schema inference from JSON, including nested objects and lists (flattening rules).
- Reversible transform: table edits map back to JSON with minimal loss.
- Embeddable widget: ship as a single JS bundle to plug into other projects.

## Data Mapping Rules
- Nested objects: flatten using dot notation, e.g., `a.b.c`.
- Lists: dynamic K (header/data driven) and fixed K supported. Column names `field[i]`, e.g., `items[0].id`.
  - Gap mode Break (default): if `i=0` empty → stop scanning further indices.
  - Gap mode Sparse (optional): allow gaps; continue scanning indices.
- Null/Option: empty cell → `null` or property omitted (transparent Option semantics).
- Enums: render as strings; map back to original enum value using provided enumMap.
- Types: numbers/bools use invariant formatting; date/custom types can be handled via formatters.
- Validation: type/required/range enums; highlight invalid cells.

## UX Flow
- Tabs: `JSON` | `Table` toggle on top.
- JSON Editor: code editor with parsing/formatting/errors (line decorations).
- Table View: virtualized grid, column resize/sort/filter, in-cell editing.
- Import/Export: JSON upload/download, CSV download (dynamic header aware).
- Status: conversion errors badge; schema mismatch warnings.

## Components
- `JsonEditor`: JSON string ↔ object; format/error display.
- `GridView`: columns + rows, virtualized rendering, edit handlers.
- `SchemaInfer`: JSON → columns (dynamic K computation included).
- `Flattener`: object → flat row(s) with columns.
- `Unflattener`: flat row(s) → object; reversible with gap rules.
- `Validators`: type/required/pattern/range/enum checks.
- `Exporter`: CSV/JSON output with options (BOM, newline, separator).

## Suggested Stack
- Frontend: React + TypeScript.
- Editor: Monaco or CodeMirror (formatting and diagnostics).
- Grid: AG Grid (rich features) or TanStack Table + react-virtualized.
- State/Validation: React Query (async), Zod/Yup (schema), Zustand (local state).
- Build: Vite; produce UMD/ESM bundles for embedding.

## File Layout
- `/src/index.ts`: widget entry; public API `init(container, options)`.
- `/src/components/JsonEditor.tsx`
- `/src/components/GridView.tsx`
- `/src/core/schema.ts`: column types, dynamic K logic.
- `/src/core/flatten.ts`: JSON → flat rows/columns.
- `/src/core/unflatten.ts`: rows/columns → JSON.
- `/src/core/validate.ts`: validation utilities.
- `/src/core/export.ts`: CSV/JSON exporters.
- `/src/styles/*`: layout/theme.
- `/demo/index.html`: sample page and manual tests.

## Public API (Draft)
```ts
type GapMode = 'break' | 'sparse';

interface InitOptions {
  initialJson?: unknown;
  listStrategy?: 'dynamic' | 'fixed';
  fixedListMax?: number;           // used when listStrategy = 'fixed'
  gapMode?: GapMode;               // default 'break'
  enumMap?: Record<string, string[]>; // fieldPath -> allowed string values
  formatters?: Record<string, (v: unknown) => string>;
  parsers?: Record<string, (s: string) => unknown>;
  validators?: Record<string, (v: unknown) => string | null>; // return error or null
  onChange?: (state: { json: unknown; errors: string[] }) => void;
  onError?: (err: Error) => void;
}

declare function init(container: HTMLElement, options?: InitOptions): {
  getJson(): unknown;
  setJson(json: unknown): void;
  getCsv(opts?: { sep?: string; bom?: boolean; newline?: '\n' | '\r\n' }): string;
  destroy(): void;
};
```

## Algorithm Notes
- Dynamic K header inference (read): scan header tokens like `field[i].tail` → per-root max index (K).
- Dynamic K writer (export): pass 1 scans data to compute K per list field; pass 2 writes header then rows.
- Existence check for complex list items: any sub-tail non-empty → element exists.
- Reversibility: prefer empty→null or omit for optional fields; document behavior.

## Performance & Robustness
- Virtualized grid for large data sets.
- Debounced JSON parse; optional Web Worker offloading.
- Cache per-column formatters/parsers and tail lists for repeated operations.
- Guard against excessive dynamic K (cap or group columns when needed).

## Accessibility & i18n
- Keyboard navigation, focus management, ARIA labels.
- Externalized strings; RTL support.

## Testing Plan
- Unit: flatten/unflatten, dynamic K calc, validators, enum mapping, formatters/parsers.
- Snapshot: header generation and sample CSV rows.
- E2E: JSON↔table round-trip, large dataset virtualization, edit/save flows.

## Extensions
- JSON Schema input to lock columns/types; stricter validation.
- Column UI: show/hide, reorder, saved presets for filter/sort.
- Server integrations: streaming CSV (chunked), incremental saves.

## Open Decisions
- Default list strategy: dynamic vs fixed; expose both.
- Empty vs null semantics on write-back.
- Enum label vs value mapping ownership (enumMap vs inference).

