import * as React from "react";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import type { MediaDelivery } from "@/lib/api/types";

export function NavigationAvatar({
  delivery,
  legacyUrl,
  handle,
  isResolving,
  onDeliveryError,
}: {
  delivery?: MediaDelivery | null;
  legacyUrl?: string | null;
  handle?: string | null;
  isResolving: boolean;
  onDeliveryError: (delivery: MediaDelivery) => void;
}) {
  const candidateUrl = delivery?.url ?? legacyUrl ?? undefined;
  const [displayedUrl, setDisplayedUrl] = React.useState<string>();
  const [failedCandidateUrl, setFailedCandidateUrl] = React.useState<string>();
  const deliveryError = React.useRef(onDeliveryError);
  deliveryError.current = onDeliveryError;

  React.useEffect(() => {
    if (failedCandidateUrl && failedCandidateUrl !== candidateUrl) {
      setFailedCandidateUrl(undefined);
    }
  }, [candidateUrl, failedCandidateUrl]);

  React.useEffect(() => {
    if (!candidateUrl) {
      if (!isResolving) setDisplayedUrl(undefined);
      return undefined;
    }
    if (candidateUrl === displayedUrl) return undefined;
    if (candidateUrl === failedCandidateUrl) return undefined;

    let isCurrent = true;
    const image = new window.Image();
    image.referrerPolicy = "no-referrer";
    image.onload = () => {
      if (isCurrent) setDisplayedUrl(candidateUrl);
    };
    image.onerror = () => {
      if (isCurrent) {
        setFailedCandidateUrl(candidateUrl);
        if (delivery?.url === candidateUrl) deliveryError.current(delivery);
      }
    };
    image.src = candidateUrl;
    return () => {
      isCurrent = false;
    };
  }, [candidateUrl, delivery, displayedUrl, failedCandidateUrl, isResolving]);

  const displayedDelivery = delivery?.url === displayedUrl ? delivery : null;
  const isWaitingForFirstImage = !displayedUrl && (isResolving || Boolean(candidateUrl));
  const fallback = handle?.trim().slice(0, 1).toUpperCase() || "我";

  return (
    <Avatar className="size-8">
      {displayedUrl ? (
        <AvatarImage
          src={displayedUrl}
          alt={handle ? `${handle} 的头像` : "我的头像"}
          width={displayedDelivery?.width ?? undefined}
          height={displayedDelivery?.height ?? undefined}
          loading="eager"
          decoding="async"
          referrerPolicy="no-referrer"
          onLoadingStatusChange={(status) => {
            if (status !== "error") return;
            setFailedCandidateUrl(displayedUrl);
            setDisplayedUrl(undefined);
            if (displayedDelivery) deliveryError.current(displayedDelivery);
          }}
        />
      ) : null}
      <AvatarFallback
        role="img"
        aria-label={isWaitingForFirstImage ? "头像加载中" : `${handle ?? "当前用户"} 的默认头像`}
        className={isWaitingForFirstImage ? "animate-pulse text-transparent" : undefined}
      >
        {isWaitingForFirstImage ? null : fallback}
      </AvatarFallback>
    </Avatar>
  );
}
