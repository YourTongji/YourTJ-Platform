import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { apiBlobRequest } from "./client";

describe("API binary media boundary", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("rejects a GIF preview and asks the user to re-upload a static image", async () => {
    const readBody = vi.fn();
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      headers: new Headers({ "Content-Type": "image/gif" }),
      blob: readBody,
    });
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiBlobRequest("/me/media/uploads/42/preview")).rejects.toThrow(
      "仅支持静态 JPEG、PNG 或 WebP 图片；GIF 或其他动图请转换为静态图片后重新上传",
    );
    expect(readBody).not.toHaveBeenCalled();
  });
});
