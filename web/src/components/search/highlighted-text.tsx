import * as React from "react";

export interface HighlightRange {
  start: number;
  end: number;
}

interface HighlightedTextProps {
  text: string;
  ranges?: HighlightRange[];
}

export function HighlightedText({ text, ranges = [] }: HighlightedTextProps) {
  const characters = Array.from(text);
  const candidates = ranges
    .filter((range) => Number.isInteger(range.start)
      && Number.isInteger(range.end)
      && range.start >= 0
      && range.end > range.start
      && range.end <= characters.length)
    .sort((left, right) => left.start - right.start || right.end - left.end);
  const normalized: HighlightRange[] = [];
  for (const range of candidates) {
    const previous = normalized.at(-1);
    if (previous && range.start < previous.end) continue;
    normalized.push(range);
    if (normalized.length === 8) break;
  }

  if (normalized.length === 0) return text;

  const nodes: React.ReactNode[] = [];
  let cursor = 0;
  for (const range of normalized) {
    if (range.start > cursor) {
      nodes.push(characters.slice(cursor, range.start).join(""));
    }
    nodes.push(
      <mark key={`${range.start}:${range.end}`} className="rounded-sm bg-primary/15 text-inherit">
        {characters.slice(range.start, range.end).join("")}
      </mark>,
    );
    cursor = range.end;
  }
  if (cursor < characters.length) {
    nodes.push(characters.slice(cursor).join(""));
  }
  return <>{nodes}</>;
}
