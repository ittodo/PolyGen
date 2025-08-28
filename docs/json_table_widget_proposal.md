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

## Related Work (Comparative Notes)
- JSONEditor (josdejong): powerful JSON code/tree/table modes and schema validation. Great nested editing; dynamic list flattening with reversible mapping requires customization.
- AG Grid + custom flattener: robust grid feature set; JSON↔table reversibility typically implemented by bespoke flatten/unflatten logic.
- Handsontable: spreadsheet-like grid with JSON binding; dynamic index expansion (`field[i].*`) needs custom code.
- Flatfile (SaaS): strong data onboarding, mapping, validation workflows; proprietary/paid and platform-specific.
- Retool/Table UIs: enable JSON-backed tables in internal tools; less suitable as a standalone reusable widget.
- One-off converters (transform.tools, CodeBeautify): JSON↔CSV conversion utilities; not focused on bidirectional editing or dynamic K.

Differentiators
- Unified, reversible JSON↔table transform with:
  - Dynamic K list expansion (header/data driven) and GapMode (break/sparse)
  - Enum mapping and per-field formatters/parsers
  - Clear semantics for null/empty and options
  - Embeddable API with minimal external assumptions

---

## Repository Strategy (Decision: Separate Repository)

We will build and distribute this widget as a standalone, reusable web package in a separate repository.

Rationale
- Reuse: easy to consume across projects via npm/CDN (UMD/ESM bundles).
- Isolation: independent release cadence, CI, security scopes, and issue tracking.
- Simplicity: node-only toolchain; avoids mixing with Rust build/CI.

Repository Layout
- Root
  - `package.json`, `vite.config.ts`, `tsconfig.json`, `.editorconfig`, `.eslintrc`, `.prettierrc`
  - `src/` (library sources)
  - `demo/` (sample app; can be deployed with GitHub Pages)
  - `scripts/` (helper scripts: build, release, canary)
  - `README.md` (public API; moved/expanded from this proposal)
  - `LICENSE`

Build & Bundling
- Vite + TypeScript
- Output formats: `esm`, `umd` (umd global name: `JsonTableWidget`)
- CSS: extracted single file and/or CSS-in-JS with theming hooks

CI/CD (GitHub Actions)
- `ci.yml`: install → lint → typecheck → unit tests → build
- `release.yml`: on tag `v*.*.*` → build → `npm publish` (with provenance) → upload demo build as artifact
- `pages.yml` (optional): deploy `demo/` to GitHub Pages on `main`

Publishing
- `npm publish` with `access=public`
- Semantic Versioning (SemVer); conventional commits for changelog automation (e.g., `changesets` or `semantic-release` optional)

Security & QA
- Dependency review; lockfile maintenance
- `eslint` + `prettier` + `tsc --noEmit` gating in CI
- Unit and snapshot tests (Jest or Vitest)

License & Governance
- Choose OSI license (MIT recommended for broad reuse)
- CONTRIBUTING.md, CODE_OF_CONDUCT.md as needed

## Monorepo Alternative (If Co-located with Other Code)

If we ever choose to co-locate with another project (e.g., a Rust codebase), keep it in a distinct subfolder and isolate pipelines:
- `tools/json-table-widget/` for the widget
- Separate CI jobs/matrices for Node vs Rust
- Clear developer docs to install Node toolchain only when working under `tools/`

Pros
- Shared issues/PRs and tighter versioning with the host repo

Cons
- Mixed toolchains and heavier CI/cache complexity
- Harder to publish/package independently

## Immediate Next Steps
- Initialize new repository with Vite + TS template
- Port this proposal into `README.md` and refine public API
- Scaffold `src/` modules (editor, grid, core transforms)
- Add `demo/` with minimal toggle UI and sample data
- Set up CI (lint/typecheck/test/build) and Pages deploy for demo
- Prepare npm publishing metadata and MIT license
