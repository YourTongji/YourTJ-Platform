import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, Clock3, Image as ImageIcon, ShieldX, Trash2 } from "lucide-react";
import { toast } from "sonner";
import * as React from "react";

import { MediaUploadButton } from "@/components/media/media-upload-button";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api/endpoints";
import type { MediaUsage, MyProfile, MyUpload, Page } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";
import { mediaDeliveryRefetchInterval } from "@/lib/media-delivery";
import { STATIC_IMAGE_REUPLOAD_MESSAGE } from "@/lib/media-policy";
import { cn } from "@/lib/utils";

interface ProfileMediaSlot {
  slot: "avatar" | "banner";
  usage: MediaUsage;
  title: string;
  description: string;
}

const profileMediaSlots: ProfileMediaSlot[] = [
  {
    slot: "avatar",
    usage: "profile_avatar",
    title: "头像",
    description: `建议使用清晰的正方形图片。${STATIC_IMAGE_REUPLOAD_MESSAGE}。`,
  },
  {
    slot: "banner",
    usage: "profile_banner",
    title: "封面",
    description: `建议使用横向图片，重要内容保持居中。${STATIC_IMAGE_REUPLOAD_MESSAGE}。`,
  },
];

function MediaImage({
  assetId,
  title,
  slot,
  status,
  deliveryState,
}: {
  assetId: string;
  title: string;
  slot: ProfileMediaSlot["slot"];
  status?: MyUpload["status"];
  deliveryState?: MyUpload["deliveryState"];
}) {
  const pending = useQuery({
    queryKey: ["media-owner-preview", assetId],
    queryFn: () => api.myMediaPreview(assetId),
    enabled: status === "pending",
    staleTime: 60_000,
  });
  const media = useQuery({
    queryKey: ["media-delivery", assetId],
    queryFn: () => api.mediaUrl(assetId),
    enabled: status !== "pending"
      && status !== "blocked"
      && status !== "quarantined"
      && (deliveryState == null || deliveryState === "published"),
    refetchInterval: (query) => mediaDeliveryRefetchInterval(query.state.data),
  });
  const [pendingUrl, setPendingUrl] = React.useState<string | null>(null);
  const retriedDeliveryUrl = React.useRef<string | null>(null);
  const frameClass = slot === "avatar" ? "aspect-square rounded-full" : "aspect-[3/1] rounded-lg";

  React.useEffect(() => {
    if (!pending.data) {
      setPendingUrl(null);
      return;
    }
    const nextUrl = URL.createObjectURL(pending.data);
    setPendingUrl(nextUrl);
    return () => URL.revokeObjectURL(nextUrl);
  }, [pending.data]);

  if (media.isLoading || pending.isLoading) {
    return <Skeleton className={cn("w-full", frameClass)} />;
  }
  const url = status === "pending" ? pendingUrl : media.data?.url;
  if (!url) {
    return (
      <div className={cn("flex w-full items-center justify-center bg-muted text-muted-foreground", frameClass)}>
        <ImageIcon className="size-5" aria-hidden="true" />
        <span className="sr-only">{title}预览暂不可用</span>
      </div>
    );
  }
  return (
    <img
      src={url}
      alt={status === "pending" ? `${title}（待审核预览）` : title}
      referrerPolicy="no-referrer"
      className={cn("w-full object-cover", frameClass, status === "pending" && "opacity-80")}
      onError={() => {
        if (!media.data?.url || retriedDeliveryUrl.current === media.data.url) return;
        retriedDeliveryUrl.current = media.data.url;
        void media.refetch();
      }}
    />
  );
}

function UploadStatus({ upload }: { upload: MyUpload }) {
  if (upload.status === "pending") {
    return (
      <Badge variant="outline" className="gap-1 text-amber-700 dark:text-amber-300">
        <Clock3 className="size-3" aria-hidden="true" />待审核
      </Badge>
    );
  }
  if (upload.status === "clean" && upload.deliveryState === "processing") {
    return (
      <Badge variant="outline" className="gap-1 text-blue-700 dark:text-blue-300">
        <Clock3 className="size-3" aria-hidden="true" />正在生成安全版本
      </Badge>
    );
  }
  if (upload.status === "clean" && upload.deliveryState === "published") {
    return (
      <Badge variant="outline" className="gap-1 text-emerald-700 dark:text-emerald-300">
        <CheckCircle2 className="size-3" aria-hidden="true" />已通过
      </Badge>
    );
  }
  return (
    <Badge variant="destructive" className="gap-1">
      <ShieldX className="size-3" aria-hidden="true" />未通过
    </Badge>
  );
}

function ProfileMediaSlotPanel({
  definition,
  profile,
}: {
  definition: ProfileMediaSlot;
  profile: MyProfile;
}) {
  const queryClient = useQueryClient();
  const currentAssetId = definition.slot === "avatar"
    ? profile.avatarAssetId
    : profile.bannerAssetId;
  const uploadQueryKey = ["my-media", definition.usage] as const;
  const uploads = useQuery({
    queryKey: uploadQueryKey,
    queryFn: () => api.myMediaUploads(definition.usage),
    refetchInterval: (query) => {
      const page = query.state.data as Page<MyUpload> | undefined;
      return page?.items?.some((upload) =>
        upload.status === "pending" || upload.deliveryState === "processing") ? 4_000 : false;
    },
  });

  async function refreshProfileMedia() {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["my-profile"] }),
      queryClient.invalidateQueries({ queryKey: ["profile"] }),
    ]);
  }

  const bind = useMutation({
    mutationFn: (assetId: string) => api.bindMyProfileMedia(definition.slot, assetId),
    onSuccess: async () => {
      toast.success(`${definition.title}已更新`);
      await refreshProfileMedia();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : `${definition.title}更新失败`),
  });
  const clear = useMutation({
    mutationFn: () => api.clearMyProfileMedia(definition.slot),
    onSuccess: async () => {
      toast.success(`${definition.title}已移除`);
      await refreshProfileMedia();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : `${definition.title}移除失败`),
  });
  const items = uploads.data?.items ?? [];

  return (
    <section
      aria-labelledby={`profile-${definition.slot}-title`}
      className="rounded-xl border bg-muted/15 p-4"
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 id={`profile-${definition.slot}-title`} className="font-medium">{definition.title}</h3>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">{definition.description}</p>
        </div>
        <div className={definition.slot === "avatar" ? "w-16 shrink-0" : "w-28 shrink-0"}>
          {currentAssetId ? (
            <MediaImage
              assetId={currentAssetId}
              title={`当前${definition.title}`}
              slot={definition.slot}
            />
          ) : (
            <div
              className={cn(
                "flex w-full items-center justify-center border border-dashed bg-background text-muted-foreground",
                definition.slot === "avatar" ? "aspect-square rounded-full" : "aspect-[3/1] rounded-lg",
              )}
            >
              <ImageIcon className="size-5" aria-hidden="true" />
              <span className="sr-only">尚未设置{definition.title}</span>
            </div>
          )}
        </div>
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <MediaUploadButton
          kind="image"
          usage={definition.usage}
          label={`上传新${definition.title}`}
          onUploaded={() => void queryClient.invalidateQueries({ queryKey: uploadQueryKey })}
        />
        {currentAssetId ? (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => clear.mutate()}
            disabled={clear.isPending}
          >
            <Trash2 className="size-4" aria-hidden="true" />
            移除当前{definition.title}
          </Button>
        ) : null}
      </div>

      <div className="mt-5">
        <div className="flex items-center justify-between gap-3">
          <p className="text-sm font-medium">最近上传</p>
          {items.some((upload) =>
            upload.status === "pending" || upload.deliveryState === "processing") ? (
            <span role="status" className="text-xs text-muted-foreground">
              审核状态会自动刷新
            </span>
          ) : null}
        </div>
        {uploads.isLoading ? (
          <div className="mt-3 grid grid-cols-2 gap-3">
            <Skeleton className="h-24" />
            <Skeleton className="h-24" />
          </div>
        ) : uploads.isError ? (
          <div className="mt-3 rounded-lg border border-destructive/30 p-3 text-sm">
            <p>上传记录加载失败。</p>
            <Button type="button" variant="link" className="mt-1 h-auto p-0" onClick={() => void uploads.refetch()}>
              重试
            </Button>
          </div>
        ) : items.length === 0 ? (
          <p className="mt-3 rounded-lg border border-dashed p-3 text-sm text-muted-foreground">
            上传后会先进入安全审核；通过前不会公开，也不能设为{definition.title}。
          </p>
        ) : (
          <ul className="mt-3 grid gap-3 sm:grid-cols-2">
            {items.slice(0, 4).map((upload) => {
              const isCurrent = upload.id === currentAssetId;
              return (
                <li key={upload.id} className="rounded-lg border bg-background p-3">
                  <div className="flex gap-3">
                    <div className={definition.slot === "avatar" ? "w-12 shrink-0" : "w-20 shrink-0"}>
                      {upload.status === "blocked" ? (
                        <div className={cn(
                          "flex w-full items-center justify-center bg-destructive/10 text-destructive",
                          definition.slot === "avatar" ? "aspect-square rounded-full" : "aspect-[3/1] rounded-lg",
                        )}>
                          <ShieldX className="size-4" aria-hidden="true" />
                        </div>
                      ) : (
                        <MediaImage
                          assetId={upload.id}
                          title={`${definition.title}候选，上传于 ${formatUnixTime(upload.createdAt)}`}
                          slot={definition.slot}
                          status={upload.status}
                          deliveryState={upload.deliveryState}
                        />
                      )}
                    </div>
                    <div className="min-w-0 flex-1">
                      <UploadStatus upload={upload} />
                      <p className="mt-1 truncate text-xs text-muted-foreground">
                        {formatUnixTime(upload.createdAt)}
                      </p>
                    </div>
                  </div>
                  <div className="mt-3">
                    {isCurrent ? (
                      <span className="text-xs font-medium text-primary">当前使用</span>
                    ) : upload.status === "clean" && upload.deliveryState === "published" ? (
                      <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        className="w-full"
                        aria-label={`将 ${formatUnixTime(upload.createdAt)} 上传的图片设为${definition.title}`}
                        onClick={() => bind.mutate(upload.id)}
                        disabled={bind.isPending}
                      >
                        设为{definition.title}
                      </Button>
                    ) : (
                      <p className="text-xs leading-5 text-muted-foreground">
                        {upload.status === "pending"
                          ? "审核通过并生成安全版本后可使用"
                          : upload.deliveryState === "processing"
                            ? "安全版本生成完成后可使用"
                            : "请重新上传其他图片"}
                      </p>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </section>
  );
}

export function ProfileMediaSettings() {
  const profile = useQuery({ queryKey: ["my-profile"], queryFn: api.myProfile });

  return (
    <Card>
      <CardHeader>
        <CardTitle>头像与封面</CardTitle>
        <CardDescription>
          原图进入私有上传区；审核通过并生成去元数据的安全版本后才能绑定。待审核原图不会进入公开 CDN。
        </CardDescription>
      </CardHeader>
      <CardContent>
        {profile.isLoading ? (
          <div className="grid gap-4 sm:grid-cols-2">
            <Skeleton className="h-56" />
            <Skeleton className="h-56" />
          </div>
        ) : profile.isError || !profile.data ? (
          <div className="rounded-lg border border-destructive/30 p-4 text-sm">
            <p>资料媒体加载失败，请稍后重试。</p>
            <Button type="button" variant="link" className="mt-1 h-auto p-0" onClick={() => void profile.refetch()}>
              重试
            </Button>
          </div>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            {profileMediaSlots.map((definition) => (
              <ProfileMediaSlotPanel
                key={definition.slot}
                definition={definition}
                profile={profile.data}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
