import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bell, Check, CheckCheck, ChevronRight, ShieldAlert } from "lucide-react";
import * as React from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

const NOTIFICATION_LABELS: Record<string, string> = {
  badge: "徽章",
  dm: "私信",
  dm_request: "消息请求",
  dm_request_accepted: "请求已接受",
  flag_auto_hide: "内容治理",
  mention: "提及",
  mod_action: "治理通知",
  quote: "引用回复",
  reply: "回复",
  vote: "点赞",
  watching: "订阅更新",
};

function payloadText(payload: Record<string, unknown> | undefined, key: string) {
  const value = payload?.[key];
  return typeof value === "string" && value.trim() ? value : undefined;
}
function payloadTitle(payload?: Record<string, unknown>) {
  return (
    payloadText(payload, "title")
    ?? payloadText(payload, "threadTitle")
    ?? payloadText(payload, "badgeName")
    ?? payloadText(payload, "body")
    ?? "系统通知"
  );
}

function payloadExcerpt(payload?: Record<string, unknown>) {
  return payloadText(payload, "bodyExcerpt") ?? payloadText(payload, "reason");
}

export function NotificationsPage() {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [unreadOnly, setUnreadOnly] = React.useState(false);
  const notifications = useInfiniteQuery({
    queryKey: ["notifications", { unreadOnly }],
    queryFn: ({ pageParam }) => api.notifications(unreadOnly || undefined, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
    enabled: isAuthenticated,
  });
  const unreadCount = useQuery({
    queryKey: ["notification-count"],
    queryFn: api.unreadNotificationCount,
    enabled: isAuthenticated,
  });
  const governanceNotices = useInfiniteQuery({
    queryKey: ["governance-notices", { unreadOnly }],
    queryFn: ({ pageParam }) => api.governanceNotices(unreadOnly || undefined, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
    enabled: isAuthenticated,
  });
  const governanceUnreadCount = useQuery({
    queryKey: ["governance-notice-count"],
    queryFn: () => api.governanceNoticeUnreadCount(),
    enabled: isAuthenticated,
  });
  const markRead = useMutation({
    mutationFn: (ids?: string[]) => api.markNotificationsRead(ids),
    onSuccess: async (_, ids) => {
      toast.success(ids ? "已标记为已读" : "全部通知已读");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["notifications"] }),
        queryClient.invalidateQueries({ queryKey: ["notification-count"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });
  const markGovernanceRead = useMutation({
    mutationFn: (ids?: string[]) => api.markGovernanceNoticesRead(ids),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["governance-notices"] }),
        queryClient.invalidateQueries({ queryKey: ["governance-notice-count"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });

  if (!isAuthenticated) {
    return <EmptyState title="登录后查看通知" />;
  }

  const items = notifications.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const governanceItems = governanceNotices.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const unreadTotal = (unreadCount.data?.count ?? 0) + (governanceUnreadCount.data?.count ?? 0);

  return (
    <div>
      <PageHeader
        eyebrow="Notifications"
        title="通知"
        description="回复、提及、点赞、私信与治理消息会集中在这里。"
        actions={(
          <Button
            variant="outline"
            disabled={unreadTotal === 0 || markRead.isPending || markGovernanceRead.isPending}
            onClick={() => {
              markRead.mutate(undefined);
              markGovernanceRead.mutate(undefined);
            }}
          >
            <CheckCheck className="h-4 w-4" />
            全部已读
          </Button>
        )}
      />

      <div className="mb-4 flex items-center gap-2" role="group" aria-label="通知筛选">
        <Button
          size="sm"
          variant={unreadOnly ? "ghost" : "secondary"}
          aria-pressed={!unreadOnly}
          onClick={() => setUnreadOnly(false)}
        >
          全部
        </Button>
        <Button
          size="sm"
          variant={unreadOnly ? "secondary" : "ghost"}
          aria-pressed={unreadOnly}
          onClick={() => setUnreadOnly(true)}
        >
          未读
          {unreadTotal > 0 ? <Badge variant="secondary">{unreadTotal}</Badge> : null}
        </Button>
      </div>

      {governanceNotices.isLoading ? (
        <LoadingState label="加载治理通知" />
      ) : governanceNotices.isError ? (
        <ErrorState error={governanceNotices.error} onRetry={() => void governanceNotices.refetch()} />
      ) : governanceItems.length > 0 ? (
        <section className="mb-5 space-y-3" aria-labelledby="governance-notices-title">
          <div>
            <h2 id="governance-notices-title" className="font-semibold">治理通知</h2>
            <p className="text-sm text-muted-foreground">安全与处置消息不会被互动通知偏好关闭。</p>
          </div>
          {governanceItems.map((item) => (
            <Card key={`governance-${item.id}`} className={!item.read ? "border-primary/50 bg-primary/[0.03]" : undefined}>
              <CardContent className="flex items-start gap-3 p-4">
                <div className="rounded-md bg-secondary p-2 text-primary" aria-hidden="true">
                  <ShieldAlert className="size-4" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium">{item.summary}</p>
                    {!item.read ? <Badge>未读</Badge> : null}
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">治理通知 · {formatUnixTime(item.createdAt)}</p>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  {!item.read ? (
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="标记治理通知为已读"
                      disabled={markGovernanceRead.isPending}
                      onClick={() => markGovernanceRead.mutate([item.id])}
                    >
                      <Check className="size-4" />
                    </Button>
                  ) : null}
                  <Button asChild variant="ghost" size="icon">
                    <Link
                      to={item.targetUrl}
                      aria-label="查看治理通知详情"
                      onClick={() => !item.read && markGovernanceRead.mutate([item.id])}
                    >
                      <ChevronRight className="size-4" />
                    </Link>
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
          {governanceNotices.hasNextPage ? (
            <div className="flex justify-center">
              <Button
                variant="outline"
                disabled={governanceNotices.isFetchingNextPage}
                onClick={() => void governanceNotices.fetchNextPage()}
              >
                {governanceNotices.isFetchingNextPage ? "加载中…" : "加载更多治理通知"}
              </Button>
            </div>
          ) : null}
        </section>
      ) : null}

      {notifications.isLoading ? (
        <LoadingState />
      ) : notifications.isError ? (
        <ErrorState error={notifications.error} onRetry={() => void notifications.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState title={unreadOnly ? "没有未读通知" : "没有通知"} />
      ) : (
        <div className="space-y-3">
          {items.map((item) => {
            const excerpt = payloadExcerpt(item.payload);
            return (
              <Card
                key={item.id}
                className={!item.read ? "border-primary/50 bg-primary/[0.03]" : undefined}
              >
                <CardContent className="flex items-start gap-3 p-4">
                  <div className="rounded-md bg-secondary p-2 text-primary" aria-hidden="true">
                    <Bell className="h-4 w-4" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="font-medium">{payloadTitle(item.payload)}</p>
                      {!item.read ? <Badge>未读</Badge> : null}
                    </div>
                    {excerpt ? (
                      <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">{excerpt}</p>
                    ) : null}
                    <p className="mt-1 text-xs text-muted-foreground">
                      {NOTIFICATION_LABELS[item.type] ?? "系统通知"} · {formatUnixTime(item.createdAt)}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    {!item.read ? (
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label="标记为已读"
                        disabled={markRead.isPending}
                        onClick={() => markRead.mutate([item.id])}
                      >
                        <Check className="h-4 w-4" />
                      </Button>
                    ) : null}
                    {item.targetUrl ? (
                      <Button asChild variant="ghost" size="icon">
                        <Link
                          to={item.targetUrl}
                          aria-label="查看通知详情"
                          onClick={() => {
                            if (!item.read) {
                              markRead.mutate([item.id]);
                            }
                          }}
                        >
                          <ChevronRight className="h-4 w-4" />
                        </Link>
                      </Button>
                    ) : null}
                  </div>
                </CardContent>
              </Card>
            );
          })}
          {notifications.hasNextPage ? (
            <div className="flex justify-center pt-2">
              <Button
                variant="outline"
                disabled={notifications.isFetchingNextPage}
                onClick={() => void notifications.fetchNextPage()}
              >
                {notifications.isFetchingNextPage ? "加载中…" : "加载更多"}
              </Button>
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}
