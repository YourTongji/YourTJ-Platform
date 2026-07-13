import * as React from "react";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import type { MediaDelivery } from "@/lib/api/types";
import { cn } from "@/lib/utils";

const RECOVERY_COOLDOWN_MS = 15_000;

export function ForumAuthorAvatar({
  avatar,
  handle,
  onDeliveryRefresh,
  className,
  fallbackClassName,
}: {
  avatar: MediaDelivery | null;
  handle: string;
  onDeliveryRefresh?: () => void;
  className?: string;
  fallbackClassName?: string;
}) {
  const lastRecoveryAt = React.useRef(0);
  const fallback = handle.trim().slice(0, 1).toUpperCase() || "用";
  const recoverDelivery = React.useCallback(() => {
    const now = Date.now();
    if (!onDeliveryRefresh || now - lastRecoveryAt.current < RECOVERY_COOLDOWN_MS) return;
    lastRecoveryAt.current = now;
    onDeliveryRefresh();
  }, [onDeliveryRefresh]);

  return (
    <Avatar className={className}>
      {avatar ? (
        <AvatarImage
          src={avatar.url}
          alt={`${handle} 的头像`}
          width={avatar.width}
          height={avatar.height}
          referrerPolicy="no-referrer"
          onLoadingStatusChange={(status) => {
            if (status === "error") recoverDelivery();
          }}
        />
      ) : null}
      <AvatarFallback className={cn("text-xs", fallbackClassName)}>{fallback}</AvatarFallback>
    </Avatar>
  );
}
