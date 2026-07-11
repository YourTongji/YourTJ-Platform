export interface SseEvent {
  event: string;
  data: string;
}

export function parseSseBlock(block: string): SseEvent | null {
  let event = "message";
  const data: string[] = [];
  for (const rawLine of block.replaceAll("\r\n", "\n").split("\n")) {
    if (!rawLine || rawLine.startsWith(":")) continue;
    const separator = rawLine.indexOf(":");
    const field = separator === -1 ? rawLine : rawLine.slice(0, separator);
    const value = separator === -1 ? "" : rawLine.slice(separator + 1).replace(/^ /, "");
    if (field === "event" && value) event = value;
    if (field === "data") data.push(value);
  }
  return data.length > 0 ? { event, data: data.join("\n") } : null;
}

export function consumeSseBuffer(buffer: string) {
  const normalized = buffer.replaceAll("\r\n", "\n");
  const blocks = normalized.split("\n\n");
  const remainder = blocks.pop() ?? "";
  return {
    events: blocks.map(parseSseBlock).filter((event): event is SseEvent => event !== null),
    remainder,
  };
}
