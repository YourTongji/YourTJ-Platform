import { describe, expect, it } from "vitest";

import { consumeSseBuffer, parseSseBlock } from "./sse";

describe("SSE parser", () => {
  it("parses typed multi-line events and ignores heartbeat comments", () => {
    expect(parseSseBlock("event: dm\ndata: {\"id\":1}\ndata: tail")).toEqual({
      event: "dm",
      data: '{"id":1}\ntail',
    });
    expect(parseSseBlock(": heartbeat")).toBeNull();
  });

  it("keeps incomplete event bytes for the next streamed chunk", () => {
    const parsed = consumeSseBuffer("event: reply\ndata: {}\n\nevent: dm\ndata:");
    expect(parsed.events).toEqual([{ event: "reply", data: "{}" }]);
    expect(parsed.remainder).toBe("event: dm\ndata:");
  });
});
