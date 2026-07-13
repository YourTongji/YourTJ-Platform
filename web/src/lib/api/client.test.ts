import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { clearMediaDeliveryUrlCache } from "@/lib/media-delivery-cache";

import { apiBlobRequest, apiRequest } from "./client";

describe("API media boundary", () => {
  beforeEach(() => {
    localStorage.clear();
    clearMediaDeliveryUrlCache();
  });

  afterEach(() => {
    clearMediaDeliveryUrlCache();
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

  it("reuses a typed media URL across API responses while it remains valid", async () => {
    const expiresAt = Math.floor(Date.now() / 1000) + 300;
    const response = (url: string) => new Response(JSON.stringify({
      assetId: "42",
      url,
      expiresAt,
      mime: "image/webp",
      width: 256,
      height: 256,
      variant: "thumb_256",
    }), { headers: { "Content-Type": "application/json" } });
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(response("https://media.example.test/first"))
      .mockResolvedValueOnce(response("https://media.example.test/second"));
    vi.stubGlobal("fetch", fetchMock);

    await apiRequest("/forum/threads/1", { auth: false });
    const second = await apiRequest<{ url: string }>("/forum/threads/1", { auth: false });

    expect(second.url).toBe("https://media.example.test/first");
  });
});
