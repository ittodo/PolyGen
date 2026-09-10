import { describe, expect, it, vi } from "vitest";
import {
  InsertRowCommand,
  SetBoldCommand,
  SetColDataCommand,
  SetRangeValuesCommand,
  SetRowDataCommand,
  SetWorksheetNameCommand,
} from "@univerjs/sheets";
import {
  collectChangedCalculationUpdates,
  isBlockedUnmanagedCommand,
  POLYSHEET_UNIVER_UI_POLICY,
} from "./gridSync";

describe("PolySheet Univer surface policy", () => {
  it("hides unmanaged structure controls and blocks structural commands", () => {
    expect(POLYSHEET_UNIVER_UI_POLICY.contextMenu).toBe(false);
    expect(POLYSHEET_UNIVER_UI_POLICY.toolbar).toBe(false);
    expect(POLYSHEET_UNIVER_UI_POLICY.footer.sheetBar).toBe(false);
    expect(isBlockedUnmanagedCommand(InsertRowCommand.id)).toBe(true);
    expect(isBlockedUnmanagedCommand(SetWorksheetNameCommand.id)).toBe(true);
    expect(isBlockedUnmanagedCommand(SetBoldCommand.id)).toBe(true);
    expect(isBlockedUnmanagedCommand(SetColDataCommand.id)).toBe(true);
    expect(isBlockedUnmanagedCommand(SetRowDataCommand.id)).toBe(true);
    expect(
      isBlockedUnmanagedCommand("sheet.command.set-col-is-auto-width"),
    ).toBe(true);
    expect(isBlockedUnmanagedCommand("sheet.command.numfmt.set.percent")).toBe(true);
    expect(isBlockedUnmanagedCommand(SetRangeValuesCommand.id)).toBe(false);
  });
});

describe("collectChangedCalculationUpdates", () => {
  it("collects every dirty calculation sheet, not only the selected sheet", () => {
    const sheets = [
      { id: "data", kind: "data" as const },
      { id: "calc-a", kind: "calculation" as const },
      { id: "calc-b", kind: "calculation" as const },
    ];
    const current = new Map([
      ["calc-a", { cells: { A1: "before-a" } }],
      ["calc-b", { cells: { A1: "before-b" } }],
    ]);
    const readDocument = vi.fn((sheet: (typeof sheets)[number]) => ({
      cells: { A1: `after-${sheet.id}` },
    }));

    const updates = collectChangedCalculationUpdates(
      sheets,
      current,
      readDocument,
    );

    expect(updates.map(({ sheet }) => sheet.id)).toEqual(["calc-a", "calc-b"]);
    expect(readDocument).toHaveBeenCalledTimes(2);
  });

  it("omits calculation sheets whose workbook document is unchanged", () => {
    const sheets = [{ id: "calc", kind: "calculation" as const }];
    const document = { cells: { A1: 1 } };

    expect(
      collectChangedCalculationUpdates(
        sheets,
        new Map([["calc", document]]),
        () => ({ cells: { A1: 1 } }),
      ),
    ).toEqual([]);
  });
});
