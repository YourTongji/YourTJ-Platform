import { afterEach, describe, expect, it, vi } from "vitest";

import { forumThreadUrl, shareForumThread } from "./forum-share";

describe("forum thread sharing", () => {
  afterEach(() => {
    Reflect.deleteProperty(navigator, "share");
    Reflect.deleteProperty(navigator, "clipboard");
  });

  it("copies a canonical deep link without current-page query parameters", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });

    await expect(shareForumThread("课程讨论", "42")).resolves.toBe("copied");

    expect(writeText).toHaveBeenCalledWith(forumThreadUrl("42"));
    expect(writeText.mock.calls[0]?.[0]).not.toContain("?");
  });

  it("uses native sharing when available", async () => {
    const share = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "share", { configurable: true, value: share });

    await expect(shareForumThread("课程讨论", "42")).resolves.toBe("shared");
    expect(share).toHaveBeenCalledWith({ title: "课程讨论", url: forumThreadUrl("42") });
  });
});
