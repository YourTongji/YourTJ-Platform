import * as React from "react";

import type { MediaDelivery } from "@/lib/api/types";

const DELIVERY_REFRESH_MARGIN_MS = 30_000;
const MINIMUM_REFRESH_MS = 5_000;
const MAXIMUM_REFRESH_MS = 4 * 60_000;
const DELIVERY_RECOVERY_COOLDOWN_MS = 15_000;

export const COMPATIBILITY_DELIVERY_REFRESH_INTERVAL_MS = 4 * 60_000;

export function mediaDeliveryRefetchInterval(delivery?: Pick<MediaDelivery, "expiresAt">) {
  if (!delivery) return false;
  return Math.max(
    MINIMUM_REFRESH_MS,
    Math.min(delivery.expiresAt * 1000 - Date.now() - DELIVERY_REFRESH_MARGIN_MS, MAXIMUM_REFRESH_MS),
  );
}

/** Throttles an owning-resource refetch when a compatibility URL fails to load. */
export function useBoundedDeliveryRecovery(onRefresh: () => void | Promise<unknown>) {
  const refresh = React.useRef(onRefresh);
  const lastRecoveryAt = React.useRef(0);
  refresh.current = onRefresh;

  return React.useCallback(() => {
    const now = Date.now();
    if (now - lastRecoveryAt.current < DELIVERY_RECOVERY_COOLDOWN_MS) return;
    lastRecoveryAt.current = now;
    void refresh.current();
  }, []);
}
