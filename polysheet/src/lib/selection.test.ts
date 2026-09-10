import { describe, expect, it } from "vitest";
import { RANGE_TYPE, type IRange } from "@univerjs/core";
import { classifyCell, classifySelection } from "./selection";

const context = {
  fields: ["id", "name", "tags"],
  rowIds: ["row-a", "row-b"],
};
const range = (input: Partial<IRange>): IRange => ({
  startRow: 0,
  endRow: 0,
  startColumn: 0,
  endColumn: 0,
  rangeType: RANGE_TYPE.NORMAL,
  ...input,
});

describe("classifySelection", () => {
  it("shows field definitions for headers and whole columns", () => {
    expect(classifySelection([range({ startColumn: 2, endColumn: 2 })], context)).toEqual({
      kind: "field",
      fieldPath: "name",
    });
    expect(
      classifySelection(
        [range({ rangeType: RANGE_TYPE.COLUMN, startColumn: 2, endColumn: 2 })],
        context,
      ),
    ).toEqual({ kind: "field", fieldPath: "name" });
  });

  it("uses stable row IDs for value and row selections", () => {
    expect(
      classifySelection(
        [range({ startRow: 1, endRow: 1, startColumn: 2, endColumn: 2 })],
        context,
      ),
    ).toEqual({ kind: "row", rowId: "row-a", fieldPath: "name" });
    expect(
      classifySelection(
        [
          range({
            rangeType: RANGE_TYPE.ROW,
            startRow: 1,
            endRow: 2,
            startColumn: 0,
            endColumn: 3,
          }),
        ],
        context,
      ),
    ).toEqual({ kind: "compare", rowIds: ["row-a", "row-b"] });
  });

  it("shows create UI for blank and unmapped cells", () => {
    expect(
      classifySelection(
        [range({ startRow: 4, endRow: 4, startColumn: 2, endColumn: 2 })],
        context,
      ),
    ).toEqual({ kind: "create" });
    expect(
      classifySelection(
        [range({ startRow: 1, endRow: 1, startColumn: 5, endColumn: 5 })],
        context,
      ),
    ).toEqual({ kind: "create" });
  });

  it("deduplicates multiple ranges in visual order", () => {
    expect(
      classifySelection(
        [
          range({ startRow: 1, endRow: 2, startColumn: 2, endColumn: 2 }),
          range({ startRow: 2, endRow: 2, startColumn: 1, endColumn: 1 }),
        ],
        context,
      ),
    ).toEqual({ kind: "compare", rowIds: ["row-a", "row-b"] });
  });
});

describe("classifyCell", () => {
  it("maps an existing data cell to its stable row and field", () => {
    expect(classifyCell(2, 1, context)).toEqual({
      kind: "row",
      rowId: "row-b",
      fieldPath: "id",
    });
  });

  it("maps a header cell to a field definition", () => {
    expect(classifyCell(0, 2, context)).toEqual({
      kind: "field",
      fieldPath: "name",
    });
  });

  it("maps blank and unmapped cells to create", () => {
    expect(classifyCell(4, 1, context)).toEqual({ kind: "create" });
    expect(classifyCell(1, 0, context)).toEqual({ kind: "create" });
  });
});
