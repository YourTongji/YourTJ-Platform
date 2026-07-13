import * as React from "react";

import { LightboxableImage } from "@/components/ui/image-lightbox";
import type { ForumAttachment } from "@/lib/api/types";

const RECOVERY_COOLDOWN_MS = 15_000;

export function ForumDeliveryImage({
  attachment,
  onDeliveryRefresh,
  enableLightbox = true,
  ...imageProps
}: Omit<React.ImgHTMLAttributes<HTMLImageElement>, "src" | "alt" | "width" | "height" | "onError">
  & {
    attachment: ForumAttachment;
    onDeliveryRefresh?: () => void;
    enableLightbox?: boolean;
  }) {
  const lastRecoveryAt = React.useRef(0);
  const handleError = () => {
    const now = Date.now();
    if (!onDeliveryRefresh || now - lastRecoveryAt.current < RECOVERY_COOLDOWN_MS) return;
    lastRecoveryAt.current = now;
    onDeliveryRefresh();
  };

  if (enableLightbox) {
    return (
      <LightboxableImage
        {...imageProps}
        src={attachment.url}
        alt={attachment.alt}
        width={attachment.width}
        height={attachment.height}
        referrerPolicy="no-referrer"
        onError={handleError}
      />
    );
  }

  return (
    <img
      {...imageProps}
      src={attachment.url}
      alt={attachment.alt}
      width={attachment.width ?? undefined}
      height={attachment.height ?? undefined}
      referrerPolicy="no-referrer"
      onError={handleError}
    />
  );
}

export function useForumDeliveryRefresh(
  attachments: ReadonlyArray<ForumAttachment | null | undefined>,
  onDeliveryRefresh: () => void,
) {
  const refresh = React.useRef(onDeliveryRefresh);
  refresh.current = onDeliveryRefresh;
  const earliestExpiry = attachments.reduce<number | null>((earliest, attachment) => {
    if (!attachment) return earliest;
    return earliest === null ? attachment.expiresAt : Math.min(earliest, attachment.expiresAt);
  }, null);

  React.useEffect(() => {
    if (earliestExpiry === null) return undefined;
    const refreshAt = earliestExpiry * 1000 - 30_000;
    const timeout = window.setTimeout(() => refresh.current(), Math.max(1_000, refreshAt - Date.now()));
    return () => window.clearTimeout(timeout);
  }, [earliestExpiry]);
}
