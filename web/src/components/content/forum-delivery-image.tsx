import * as React from "react";

import type { ForumAttachment } from "@/lib/api/types";
import { invalidateMediaDeliveryUrl } from "@/lib/media-delivery-cache";

const RECOVERY_COOLDOWN_MS = 15_000;

export function ForumDeliveryImage({
  attachment,
  onDeliveryRefresh,
  loading = "lazy",
  decoding = "async",
  ...imageProps
}: Omit<React.ImgHTMLAttributes<HTMLImageElement>, "src" | "alt" | "width" | "height" | "onError">
  & {
    attachment: ForumAttachment;
    onDeliveryRefresh?: () => void;
  }) {
  const lastRecoveryAt = React.useRef(0);

  return (
    <img
      {...imageProps}
      loading={loading}
      decoding={decoding}
      src={attachment.url}
      alt={attachment.alt}
      width={attachment.width ?? undefined}
      height={attachment.height ?? undefined}
      referrerPolicy="no-referrer"
      onError={() => {
        invalidateMediaDeliveryUrl(attachment);
        const now = Date.now();
        if (!onDeliveryRefresh || now - lastRecoveryAt.current < RECOVERY_COOLDOWN_MS) return;
        lastRecoveryAt.current = now;
        onDeliveryRefresh();
      }}
    />
  );
}

export function useForumDeliveryRefresh(
  deliveries: ReadonlyArray<Pick<ForumAttachment, "expiresAt"> | null | undefined>,
  onDeliveryRefresh: () => void,
) {
  const refresh = React.useRef(onDeliveryRefresh);
  refresh.current = onDeliveryRefresh;
  const earliestExpiry = deliveries.reduce<number | null>((earliest, delivery) => {
    if (!delivery) return earliest;
    return earliest === null ? delivery.expiresAt : Math.min(earliest, delivery.expiresAt);
  }, null);

  React.useEffect(() => {
    if (earliestExpiry === null) return undefined;
    const refreshAt = earliestExpiry * 1000 - 30_000;
    const timeout = window.setTimeout(() => refresh.current(), Math.max(1_000, refreshAt - Date.now()));
    return () => window.clearTimeout(timeout);
  }, [earliestExpiry]);
}
