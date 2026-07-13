import { AlertCircle, CheckCircle2, Clock3, RefreshCw, Trash2 } from "lucide-react";
import * as React from "react";

import { MediaUploadButton } from "@/components/media/media-upload-button";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api/endpoints";
import type { MediaUsage, MyUpload } from "@/lib/api/types";
import { STATIC_IMAGE_REUPLOAD_MESSAGE } from "@/lib/media-policy";

function defaultAlt(fileName: string) {
  const withoutExtension = fileName.replace(/\.[^.]+$/, "").trim();
  return withoutExtension.slice(0, 300) || "论坛图片";
}

function statusPresentation(status: MyUpload["status"]) {
  if (status === "clean") {
    return { label: "审核通过，可发布", icon: CheckCircle2, variant: "secondary" as const };
  }
  if (status === "blocked") {
    return { label: "未通过，请移除", icon: AlertCircle, variant: "destructive" as const };
  }
  return { label: "审核中，暂不可发布", icon: Clock3, variant: "outline" as const };
}

function uploadPresentation(upload: MyUpload) {
  if (upload.status === "clean" && upload.deliveryState === "processing") {
    return { label: "审核通过，正在生成安全版本", icon: Clock3, variant: "outline" as const };
  }
  if (upload.status === "clean" && upload.deliveryState === "failed") {
    return { label: "安全版本生成失败，等待运维重试", icon: AlertCircle, variant: "destructive" as const };
  }
  return statusPresentation(upload.status);
}

export function ForumImageAttachments({
  usage,
  assetIds,
  maxImages,
  disabled,
  onUpload,
  onRemove,
  onReadyChange,
}: {
  usage: Extract<MediaUsage, "forum_thread" | "forum_comment">;
  assetIds: string[];
  maxImages: number;
  disabled?: boolean;
  onUpload: (assetId: string, alt: string) => void;
  onRemove: (assetId: string) => void;
  onReadyChange?: (isReady: boolean) => void;
}) {
  const [uploads, setUploads] = React.useState<Record<string, MyUpload>>({});
  const [isLoading, setIsLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const generationRef = React.useRef(0);

  const refresh = React.useCallback(async () => {
    const generation = generationRef.current;
    if (assetIds.length === 0) {
      setUploads({});
      setError(null);
      return;
    }
    setIsLoading(true);
    try {
      const items = await Promise.all(assetIds.map((assetId) => api.myMediaUpload(assetId)));
      if (generation !== generationRef.current) return;
      setUploads(Object.fromEntries(items.map((item) => [item.id, item])));
      setError(null);
    } catch (refreshError) {
      if (generation !== generationRef.current) return;
      setError(refreshError instanceof Error ? refreshError.message : "无法读取图片审核状态");
    } finally {
      if (generation === generationRef.current) setIsLoading(false);
    }
  }, [assetIds]);

  React.useEffect(() => {
    generationRef.current += 1;
    void refresh();
    return () => {
      generationRef.current += 1;
    };
  }, [refresh]);

  const hasPending = assetIds.some((assetId) => {
    const upload = uploads[assetId];
    return upload?.status === "pending" || upload?.deliveryState === "processing";
  });
  const isReady = !error
    && assetIds.every((assetId) => {
      const upload = uploads[assetId];
      return upload?.status === "clean" && upload.deliveryState === "published";
    });
  React.useEffect(() => {
    onReadyChange?.(isReady);
  }, [isReady, onReadyChange]);
  React.useEffect(() => {
    if (!hasPending) return;
    const interval = window.setInterval(() => void refresh(), 3_000);
    return () => window.clearInterval(interval);
  }, [hasPending, refresh]);

  return (
    <section className="space-y-2 rounded-lg border bg-muted/20 p-3" aria-labelledby={`forum-images-${usage}`}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 id={`forum-images-${usage}`} className="text-sm font-medium">正文图片</h3>
          <p className="text-xs text-muted-foreground">
            最多 {maxImages} 张；{STATIC_IMAGE_REUPLOAD_MESSAGE}。上传后会插入平台引用，
            审核与去元数据安全版本均完成后才能发布。
          </p>
        </div>
        <MediaUploadButton
          kind="image"
          usage={usage}
          disabled={disabled || assetIds.length >= maxImages}
          label="添加图片"
          onUploaded={(upload) => onUpload(upload.uploadId, defaultAlt(upload.originalName))}
        />
      </div>

      {error ? (
        <div role="alert" className="flex flex-wrap items-center justify-between gap-2 text-xs text-destructive">
          <span>{error}</span>
          <Button type="button" size="sm" variant="outline" onClick={() => void refresh()}>
            <RefreshCw className="size-3.5" aria-hidden="true" />
            重试
          </Button>
        </div>
      ) : null}
      {assetIds.length === 0 ? (
        <p className="text-xs text-muted-foreground">尚未添加图片。</p>
      ) : isLoading && Object.keys(uploads).length === 0 && !error ? (
        <p role="status" className="text-xs text-muted-foreground">正在读取图片审核状态…</p>
      ) : (
        <ul className="space-y-2">
          {assetIds.map((assetId) => {
            const upload = uploads[assetId];
            const presentation = upload ? uploadPresentation(upload) : null;
            const StatusIcon = presentation?.icon;
            return (
              <li key={assetId} className="flex flex-wrap items-center justify-between gap-2 rounded-md border bg-background px-3 py-2">
                <div className="min-w-0">
                  <p className="truncate text-xs font-medium">图片 #{assetId}</p>
                  {presentation && StatusIcon ? (
                    <Badge variant={presentation.variant} className="mt-1">
                      <StatusIcon className="size-3" aria-hidden="true" />
                      {presentation.label}
                    </Badge>
                  ) : (
                    <span className="text-xs text-muted-foreground">正在读取状态…</span>
                  )}
                </div>
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  disabled={disabled}
                  onClick={() => onRemove(assetId)}
                  aria-label={`移除图片 ${assetId}`}
                >
                  <Trash2 className="size-4" aria-hidden="true" />
                  移除
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
