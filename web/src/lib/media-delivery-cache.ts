import type {
  ForumAttachment,
  MediaDelivery,
  MediaDeliveryVariant,
} from "@/lib/api/types";

const REFRESH_MARGIN_SECONDS = 30;
const MAX_CACHE_ENTRIES = 512;

type CacheableDelivery = MediaDelivery | ForumAttachment;

interface CachedDeliveryUrl {
  url: string;
  expiresAt: number;
}

const deliveryUrls = new Map<string, CachedDeliveryUrl>();
const variants = new Set<MediaDeliveryVariant>(["thumb_256", "display_1280", "full_2048"]);

function deliveryVariant(value: Record<string, unknown>): MediaDeliveryVariant | null {
  if (typeof value.variant === "string" && variants.has(value.variant as MediaDeliveryVariant)) {
    return value.variant as MediaDeliveryVariant;
  }
  return typeof value.reference === "string" ? "display_1280" : null;
}

function cacheKey(value: Record<string, unknown>) {
  if (
    typeof value.assetId !== "string"
    || typeof value.url !== "string"
    || typeof value.expiresAt !== "number"
  ) {
    return null;
  }
  const variant = deliveryVariant(value);
  return variant ? `${value.assetId}:${variant}` : null;
}

function isReusable(expiresAt: number, nowSeconds: number) {
  return expiresAt > nowSeconds + REFRESH_MARGIN_SECONDS;
}

function remember(key: string, delivery: CachedDeliveryUrl) {
  deliveryUrls.delete(key);
  deliveryUrls.set(key, delivery);
  while (deliveryUrls.size > MAX_CACHE_ENTRIES) {
    const oldestKey = deliveryUrls.keys().next().value as string | undefined;
    if (!oldestKey) break;
    deliveryUrls.delete(oldestKey);
  }
}

function reuseCandidate(value: Record<string, unknown>, nowSeconds: number) {
  const key = cacheKey(value);
  if (!key) return;

  const cached = deliveryUrls.get(key);
  if (cached && isReusable(cached.expiresAt, nowSeconds)) {
    value.url = cached.url;
    value.expiresAt = cached.expiresAt;
    remember(key, cached);
    return;
  }
  deliveryUrls.delete(key);

  if (isReusable(value.expiresAt as number, nowSeconds)) {
    remember(key, { url: value.url as string, expiresAt: value.expiresAt as number });
  }
}

/** Reuse one bearer URL per asset variant while that exact URL remains safely usable. */
export function reuseMediaDeliveryUrls<T>(payload: T, nowMs = Date.now()): T {
  const visited = new WeakSet<object>();
  const nowSeconds = Math.floor(nowMs / 1000);

  const visit = (value: unknown) => {
    if (value === null || typeof value !== "object" || visited.has(value)) return;
    visited.add(value);
    if (Array.isArray(value)) {
      value.forEach(visit);
      return;
    }

    const record = value as Record<string, unknown>;
    if (cacheKey(record)) {
      reuseCandidate(record, nowSeconds);
      return;
    }
    Object.values(record).forEach(visit);
  };

  visit(payload);
  return payload;
}

/** Evict only the failing bearer URL, without deleting a newer concurrent projection. */
export function invalidateMediaDeliveryUrl(delivery: CacheableDelivery) {
  const record = delivery as unknown as Record<string, unknown>;
  const key = cacheKey(record);
  if (key && deliveryUrls.get(key)?.url === delivery.url) {
    deliveryUrls.delete(key);
  }
}

/** Clear account-scoped bearer URLs when the authenticated principal changes. */
export function clearMediaDeliveryUrlCache() {
  deliveryUrls.clear();
}
