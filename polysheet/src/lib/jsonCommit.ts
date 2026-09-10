import { parseTree, type Node } from "jsonc-parser";

const IMMEDIATE_VALUE_TYPES = new Set(["string", "object", "array"]);

export function shouldCommitAtCursor(text: string, offset: number) {
  if (isStructuralSeparatorBoundary(text, offset)) return true;
  if (isCompletedStringValueBoundary(text, offset)) return true;

  const errors: Array<{ error: number; offset: number; length: number }> = [];
  const root = parseTree(text, errors, { allowTrailingComma: false });
  if (!root || errors.length > 0) return false;
  return hasCompletedValueAtOffset(root, offset);
}

function isStructuralSeparatorBoundary(text: string, offset: number) {
  const separatorIndex = offset - 1;
  if (separatorIndex < 0 || text[separatorIndex] !== ",") return false;
  return !isInsideString(text, separatorIndex);
}

function isInsideString(text: string, endExclusive: number) {
  let inside = false;
  for (let index = 0; index < endExclusive; index += 1) {
    if (text[index] === '"' && !isEscaped(text, index)) inside = !inside;
  }
  return inside;
}

function isCompletedStringValueBoundary(text: string, offset: number) {
  const quoteIndex = offset - 1;
  if (quoteIndex < 0 || text[quoteIndex] !== '"' || isEscaped(text, quoteIndex)) {
    return false;
  }

  // A property-name quote is followed by a colon. Every other unescaped
  // closing quote is treated as a value boundary so invalid JSON is also
  // dispatched to validation immediately.
  let next = offset;
  while (next < text.length && /\s/.test(text[next])) next += 1;
  return text[next] !== ":";
}

function isEscaped(text: string, index: number) {
  let backslashes = 0;
  for (let cursor = index - 1; cursor >= 0 && text[cursor] === "\\"; cursor -= 1) {
    backslashes += 1;
  }
  return backslashes % 2 === 1;
}

function hasCompletedValueAtOffset(node: Node, offset: number): boolean {
  if (
    IMMEDIATE_VALUE_TYPES.has(node.type) &&
    node.offset + node.length === offset
  ) {
    return true;
  }
  if (node.type === "property") {
    return node.children?.slice(1).some((child) => hasCompletedValueAtOffset(child, offset)) ?? false;
  }
  return node.children?.some((child) => hasCompletedValueAtOffset(child, offset)) ?? false;
}
