import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { parseUploadCallbackData, uploadMedia, validateMediaFile } from "./media-upload";

const apiMocks = vi.hoisted(() => ({ mediaUploadCredentials: vi.fn() }));
const ossMocks = vi.hoisted(() => ({ constructor: vi.fn(), put: vi.fn() }));
const CALLBACK_BODY =
  '{"uploadIntentId":"intent","ossKey":${object},"bytes":${size},"mime":${mimeType},"sha256":${x:sha256}}';

function uploadCredentials(region: string) {
  return {
    uploadIntentId: "intent",
    accessKeyId: "temporary-id",
    accessKeySecret: "temporary-secret",
    securityToken: "temporary-token",
    region,
    bucket: "yourtj-test",
    prefix: "uploads/1/image/",
    ossKey: "uploads/1/image/intent.png",
    callbackUrl: "https://api.example.test/api/v2/media/callback",
    callbackBody: CALLBACK_BODY,
    expiration: Math.floor(Date.now() / 1_000) + 300,
  };
}

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));

vi.mock("ali-oss", () => ({
  default: class OssClientMock {
    constructor(options: unknown) {
      ossMocks.constructor(options);
    }

    put(...args: unknown[]) {
      return ossMocks.put(...args);
    }
  },
}));

describe("media upload boundary", () => {
  beforeEach(() => {
    apiMocks.mediaUploadCredentials.mockReset().mockResolvedValue(uploadCredentials("cn-shanghai"));
    ossMocks.constructor.mockReset();
    ossMocks.put.mockReset().mockResolvedValue({ data: { uploadId: "42" } });
    vi.stubGlobal("crypto", {
      subtle: {
        digest: vi.fn().mockResolvedValue(new Uint8Array(32).buffer),
      },
    });
  });

  afterEach(() => vi.unstubAllGlobals());

  it("accepts only the server-supported media kinds and size limit", () => {
    for (const [name, contentType] of [
      ["photo.jpg", "image/jpeg"],
      ["photo.png", "image/png"],
      ["photo.webp", "image/webp"],
    ]) {
      expect(() => validateMediaFile(new File(["image"], name, { type: contentType }), "image"))
        .not.toThrow();
    }
    expect(() => validateMediaFile(new File(["script"], "photo.svg", { type: "image/svg+xml" }), "image")).toThrow(/JPEG/);
    expect(() => validateMediaFile(new File(["text"], "notes.txt", { type: "text/plain" }), "file")).toThrow(/PDF/);
  });

  it("rejects GIF before requesting credentials and asks for a static re-upload", async () => {
    const gif = new File(["GIF89a"], "animated.gif", { type: "image/gif" });

    expect(() => validateMediaFile(gif, "image")).toThrow(
      "仅支持静态 JPEG、PNG 或 WebP 图片；GIF 或其他动图请转换为静态图片后重新上传",
    );
    await expect(uploadMedia(gif, "image")).rejects.toThrow(/转换为静态图片后重新上传/);
    expect(apiMocks.mediaUploadCredentials).not.toHaveBeenCalled();
    expect(ossMocks.constructor).not.toHaveBeenCalled();
    expect(ossMocks.put).not.toHaveBeenCalled();
  });

  it("requires the signed OSS callback to return a canonical upload id", () => {
    expect(parseUploadCallbackData({ uploadId: "42" })).toBe("42");
    expect(parseUploadCallbackData('{"ok":true,"uploadId":43}')).toBe("43");
    expect(() => parseUploadCallbackData({ ok: true })).toThrow(/上传记录/);
  });

  it("uploads only the exact authorized key with STS and signed callback metadata", async () => {
    const file = new File(["image"], "photo.png", { type: "image/png" });

    await expect(uploadMedia(file, "image", "profile_avatar")).resolves.toEqual({
      uploadId: "42",
      ossKey: "uploads/1/image/intent.png",
      originalName: "photo.png",
    });
    expect(apiMocks.mediaUploadCredentials).toHaveBeenCalledWith(
      "image",
      "image/png",
      "profile_avatar",
    );
    expect(ossMocks.constructor).toHaveBeenCalledWith(expect.objectContaining({
      accessKeyId: "temporary-id",
      stsToken: "temporary-token",
      region: "oss-cn-shanghai",
      bucket: "yourtj-test",
      secure: true,
    }));
    expect(ossMocks.put).toHaveBeenCalledWith(
      "uploads/1/image/intent.png",
      file,
      expect.objectContaining({
        headers: expect.objectContaining({
          "x-oss-forbid-overwrite": "true",
        }),
        callback: expect.objectContaining({
          url: "https://api.example.test/api/v2/media/callback",
          body: CALLBACK_BODY,
          contentType: "application/json",
          customValue: { sha256: "0".repeat(64) },
          callbackSNI: true,
        }),
      }),
    );
  });

  it("keeps an already normalized OSS SDK region unchanged", async () => {
    apiMocks.mediaUploadCredentials.mockResolvedValueOnce(uploadCredentials("oss-cn-shanghai"));

    await uploadMedia(new File(["image"], "photo.png", { type: "image/png" }), "image");

    expect(ossMocks.constructor).toHaveBeenCalledWith(expect.objectContaining({
      region: "oss-cn-shanghai",
    }));
  });
});
