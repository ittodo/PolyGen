import { describe, expect, it } from "vitest";
import { shouldCommitAtCursor } from "./jsonCommit";

describe("shouldCommitAtCursor", () => {
  it("commits immediately after a completed string value", () => {
    const text = '{\n  "description": "고급 모험가용 검"\n}';
    const offset = text.indexOf('검"') + '검"'.length;
    expect(shouldCommitAtCursor(text, offset)).toBe(true);
  });

  it("does not commit while the caret is still inside a string", () => {
    const text = '{\n  "description": "고급 모험가용 검"\n}';
    const offset = text.indexOf("검");
    expect(shouldCommitAtCursor(text, offset)).toBe(false);
  });

  it("waits for a boundary when editing a number", () => {
    const text = '{ "price": 1200 }';
    const offset = text.indexOf("1200") + "1200".length;
    expect(shouldCommitAtCursor(text, offset)).toBe(false);
  });

  it("does not commit syntactically invalid JSON", () => {
    const text = '{ "description": "닫히지 않음 }';
    expect(shouldCommitAtCursor(text, text.length)).toBe(false);
  });

  it("commits a closed value even when the surrounding JSON is invalid", () => {
    const text = '{ "description": "검", broken }';
    const offset = text.indexOf('검"') + '검"'.length;
    expect(shouldCommitAtCursor(text, offset)).toBe(true);
  });

  it("does not commit after a property-name quote", () => {
    const text = '{ "description": "검" }';
    const offset = text.indexOf('description"') + 'description"'.length;
    expect(shouldCommitAtCursor(text, offset)).toBe(false);
  });

  it("does not treat an escaped quote as the end of a value", () => {
    const text = '{ "description": "고급 \\"검\\"입니다" }';
    const offset = text.indexOf('\\"검') + 2;
    expect(shouldCommitAtCursor(text, offset)).toBe(false);
  });

  it("commits a boolean when the caret passes its comma", () => {
    const text = '{ "enabled": false, "name": "검" }';
    const offset = text.indexOf("false,") + "false,".length;
    expect(shouldCommitAtCursor(text, offset)).toBe(true);
  });

  it("commits invalid uppercase boolean syntax at the comma for validation", () => {
    const text = '{ "enabled": FALSE, "name": "검" }';
    const offset = text.indexOf("FALSE,") + "FALSE,".length;
    expect(shouldCommitAtCursor(text, offset)).toBe(true);
  });

  it("does not commit for a comma inside a string", () => {
    const text = '{ "description": "검, 방패" }';
    const offset = text.indexOf("검,") + "검,".length;
    expect(shouldCommitAtCursor(text, offset)).toBe(false);
  });
});
