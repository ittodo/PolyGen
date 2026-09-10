import { RANGE_TYPE, type IRange } from "@univerjs/core";

export type InspectorSelection =
  | { kind: "create" }
  | { kind: "field"; fieldPath: string }
  | { kind: "row"; rowId: string; fieldPath?: string }
  | { kind: "compare"; rowIds: string[] }
  | { kind: "compareLimit"; count: number };

export type SelectionContext = {
  fields: string[];
  rowIds: string[];
};

export function classifyCell(
  row: number,
  column: number,
  context: SelectionContext,
): InspectorSelection {
  if (row === 0 && column >= 1 && column <= context.fields.length) {
    return { kind: "field", fieldPath: context.fields[column - 1] };
  }
  if (
    row >= 1 &&
    row <= context.rowIds.length &&
    column >= 1 &&
    column <= context.fields.length
  ) {
    const rowId = context.rowIds[row - 1];
    if (rowId) {
      return {
        kind: "row",
        rowId,
        fieldPath: context.fields[column - 1],
      };
    }
  }
  return { kind: "create" };
}

export function classifySelection(
  ranges: IRange[],
  context: SelectionContext,
): InspectorSelection {
  if (ranges.length === 0) return { kind: "create" };

  const selectedIds = new Set<string>();
  let selectedField: string | undefined;
  let touchesValue = false;

  for (const range of ranges) {
    if (range.rangeType === RANGE_TYPE.ALL) return { kind: "create" };
    if (range.rangeType === RANGE_TYPE.COLUMN) {
      if (
        range.endColumn === range.startColumn &&
        range.startColumn >= 1 &&
        range.startColumn <= context.fields.length
      ) {
        selectedField ??= context.fields[range.startColumn - 1];
      }
      continue;
    }

    const includesMappedColumn =
      range.endColumn >= 1 && range.startColumn <= context.fields.length;
    if (range.rangeType !== RANGE_TYPE.ROW && !includesMappedColumn) continue;

    const firstDataRow = Math.max(1, range.startRow);
    const lastDataRow = Math.min(range.endRow, context.rowIds.length);
    for (let row = firstDataRow; row <= lastDataRow; row += 1) {
      const rowId = context.rowIds[row - 1];
      if (rowId) selectedIds.add(rowId);
    }
    if (lastDataRow >= firstDataRow) touchesValue = true;

    if (
      range.startRow === 0 &&
      range.endRow === 0 &&
      range.endColumn === range.startColumn &&
      range.startColumn >= 1 &&
      range.startColumn <= context.fields.length
    ) {
      selectedField ??= context.fields[range.startColumn - 1];
    }
  }

  const rowIds = context.rowIds.filter((rowId) => selectedIds.has(rowId));
  if (rowIds.length > 100) return { kind: "compareLimit", count: rowIds.length };
  if (rowIds.length > 1) return { kind: "compare", rowIds };
  if (rowIds.length === 1) {
    const first = ranges[0];
    const fieldPath =
      first &&
      first.startColumn >= 1 &&
      first.startColumn <= context.fields.length &&
      first.endColumn === first.startColumn
        ? context.fields[first.startColumn - 1]
        : undefined;
    return { kind: "row", rowId: rowIds[0], fieldPath };
  }
  if (!touchesValue && selectedField) return { kind: "field", fieldPath: selectedField };
  return { kind: "create" };
}
