import { beforeEach, describe, expect, it } from "vitest";

import type { MediaDelivery } from "@/lib/api/types";
import {
  clearMediaDeliveryUrlCache,
  invalidateMediaDeliveryUrl,
  reuseMediaDeliveryUrls,
} from "@/lib/media-delivery-cache";

const NOW_MS = 1_700_000_000_000;
const NOW_SECONDS = NOW_MS / 1000;

function delivery(overrides: Partial<MediaDelivery> = {}): MediaDelivery {
  return {
    assetId: "42",
    url: "https://media.example.test/first",
    expiresAt: NOW_SECONDS + 300,
    mime: "image/webp",
    width: 256,
    height: 256,
    variant: "thumb_256",
    ...overrides,
  };
}

describe("media Delivery URL cache", () => {
  beforeEach(clearMediaDeliveryUrlCache);

  it("reuses the same URL and expiry for one asset variant before the refresh margin", () => {
    reuseMediaDeliveryUrls(delivery(), NOW_MS);
    const refreshed = delivery({
      url: "https://media.example.test/second",
      expiresAt: NOW_SECONDS + 360,
    });

    expect(reuseMediaDeliveryUrls(refreshed, NOW_MS)).toMatchObject({
      url: "https://media.example.test/first",
      expiresAt: NOW_SECONDS + 300,
    });
  });

  it("keeps variants for the same asset separate", () => {
    reuseMediaDeliveryUrls(delivery(), NOW_MS);
    const display = delivery({
      url: "https://media.example.test/display",
      variant: "display_1280",
      width: 1280,
      height: 720,
    });

    expect(reuseMediaDeliveryUrls(display, NOW_MS).url).toBe(
      "https://media.example.test/display",
    );
  });

  it("accepts a refreshed URL once the cached URL reaches the safety margin", () => {
    const expiring = delivery({ expiresAt: NOW_SECONDS + 30 });
    reuseMediaDeliveryUrls(expiring, NOW_MS);
    const refreshed = delivery({
      url: "https://media.example.test/refreshed",
      expiresAt: NOW_SECONDS + 300,
    });

    expect(reuseMediaDeliveryUrls(refreshed, NOW_MS).url).toBe(
      "https://media.example.test/refreshed",
    );
  });

  it("allows a replacement after the displayed URL fails", () => {
    const original = delivery();
    reuseMediaDeliveryUrls(original, NOW_MS);
    invalidateMediaDeliveryUrl(original);

    const replacement = delivery({ url: "https://media.example.test/replacement" });
    expect(reuseMediaDeliveryUrls(replacement, NOW_MS).url).toBe(
      "https://media.example.test/replacement",
    );
  });

  it("does not reuse a bearer URL after the account-scoped cache is cleared", () => {
    reuseMediaDeliveryUrls(delivery(), NOW_MS);
    clearMediaDeliveryUrlCache();
    const replacement = delivery({ url: "https://media.example.test/next-account" });

    expect(reuseMediaDeliveryUrls(replacement, NOW_MS).url).toBe(
      "https://media.example.test/next-account",
    );
  });

  it("reuses nested typed deliveries and display attachments without touching other URLs", () => {
    const initial = {
      authorAvatar: delivery(),
      attachments: [{
        assetId: "99",
        reference: "yourtj-asset:99",
        position: 0,
        alt: "校园",
        url: "https://media.example.test/attachment-first",
        expiresAt: NOW_SECONDS + 300,
        width: 1280,
        height: 720,
      }],
      targetUrl: "https://example.test",
    };
    reuseMediaDeliveryUrls(initial, NOW_MS);
    const refreshed = structuredClone(initial);
    refreshed.authorAvatar.url = "https://media.example.test/avatar-second";
    refreshed.attachments[0].url = "https://media.example.test/attachment-second";

    expect(reuseMediaDeliveryUrls(refreshed, NOW_MS)).toMatchObject({
      authorAvatar: { url: "https://media.example.test/first" },
      attachments: [{ url: "https://media.example.test/attachment-first" }],
      targetUrl: "https://example.test",
    });
  });
});
