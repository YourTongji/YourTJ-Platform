import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bell, Check, CheckCheck, ChevronRight, ShieldAlert } from "lucide-react";
import * as React from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { PaginatedListState } from "@/components/common/paginated-list-state";
import { EmptyState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Notification } from "@/lib/api/types";
import { accountQueryKeys } from "@/lib/account-query-keys";
import { formatUnixTime } from "@/lib/format";

const NOTIFICATION_LABELS: Record<string, string> = {
  badge: "徽章",
  achievement_awarded: "成就",
  achievement_revoked: "成就变更",
  dm: "私信",
  dm_request: "消息请求",
  dm_request_accepted: "请求已接受",
  flag_auto_hide: "内容处理",
  follow: "新关注",
  mention: "提及",
  mod_action: "管理通知",
  quote: "引用回复",
  reply: "回复",
  vote: "点赞",
  watching: "订阅更新",
  verification_expired: "认证到期",
  verification_granted: "认证",
  verification_revoked: "认证变更",
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

function notificationDay(timestamp: number) {
  const date = new Date(timestamp * 1000);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function notificationDayLabel(timestamp: number, now = new Date()) {
  const date = new Date(timestamp * 1000);
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const target = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const dayDifference = Math.round((today.getTime() - target.getTime()) / 86_400_000);
  if (dayDifference === 0) return "今天";
  if (dayDifference === 1) return "昨天";
  return date.toLocaleDateString("zh-CN", { year: "numeric", month: "long", day: "numeric" });
}

function groupNotificationsByDay(items: Notification[]) {
  const groups = new Map<string, Notification[]>();
  for (const item of items) {
    const day = notificationDay(item.createdAt);
    const group = groups.get(day);
    if (group) group.push(item);
    else groups.set(day, [item]);
  }
  return [...groups.entries()];
}

function NotificationsSkeleton({ label }: { label: string }) {
  return (
    <div className="space-y-3" role="status" aria-label={label}>
      <span className="sr-only">{label}</span>
      {Array.from({ length: 3 }, (_, index) => (
        <Card key={index}>
          <CardContent className="flex items-start gap-3 p-4">
            <Skeleton className="size-8 shrink-0" />
            <div className="flex-1 space-y-2">
              <Skeleton className="h-4 w-2/5" />
              <Skeleton className="h-3 w-4/5" />
              <Skeleton className="h-3 w-1/3" />
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

export function NotificationsPage() {
  const { account, isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [unreadOnly, setUnreadOnly] = React.useState(false);
  const notifications = useInfiniteQuery({
    queryKey: [...accountQueryKeys.notifications(account?.id), { unreadOnly }],
    queryFn: ({ pageParam }) => api.notifications(unreadOnly || undefined, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
    enabled: isAuthenticated,
  });
  const unreadCount = useQuery({
    queryKey: accountQueryKeys.notificationCount(account?.id),
    queryFn: api.unreadNotificationCount,
    enabled: isAuthenticated,
  });
  const governanceNotices = useInfiniteQuery({
    queryKey: [...accountQueryKeys.governanceNotices(account?.id), { unreadOnly }],
    queryFn: ({ pageParam }) => api.governanceNotices(unreadOnly || undefined, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
    enabled: isAuthenticated,
  });
  const governanceUnreadCount = useQuery({
    queryKey: accountQueryKeys.governanceNoticeCount(account?.id),
    queryFn: () => api.governanceNoticeUnreadCount(),
    enabled: isAuthenticated,
  });
  const markRead = useMutation({
    mutationFn: (ids?: string[]) => api.markNotificationsRead(ids),
    onSuccess: async (_, ids) => {
      toast.success(ids ? "已标记为已读" : "全部通知已读");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: accountQueryKeys.notifications(account?.id) }),
        queryClient.invalidateQueries({ queryKey: accountQueryKeys.notificationCount(account?.id) }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });
  const markGovernanceRead = useMutation({
    mutationFn: (ids?: string[]) => api.markGovernanceNoticesRead(ids),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: accountQueryKeys.governanceNotices(account?.id) }),
        queryClient.invalidateQueries({ queryKey: accountQueryKeys.governanceNoticeCount(account?.id) }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });

  if (!isAuthenticated) {
    return <EmptyState title="登录后查看通知" />;
  }

  const items = notifications.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const notificationGroups = groupNotificationsByDay(items);
  const governanceItems = governanceNotices.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const unreadTotal = (unreadCount.data?.count ?? 0) + (governanceUnreadCount.data?.count ?? 0);

  return (
    <div>
      <PageHeader
        title="通知"
        description="回复、提及、点赞、私信与平台消息会集中在这里。"
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

      <PaginatedListState
        isLoading={governanceNotices.isLoading}
        error={governanceNotices.error}
        isEmpty={governanceItems.length === 0}
        onRetry={() => void governanceNotices.refetch()}
        hasMore={governanceNotices.hasNextPage}
        isLoadingMore={governanceNotices.isFetchingNextPage}
        onLoadMore={() => void governanceNotices.fetchNextPage()}
        loading={<NotificationsSkeleton label="加载平台通知" />}
        empty={null}
        loadMoreLabel="加载更多平台通知"
        className="mb-5"
      >
        <section className="space-y-3" aria-labelledby="governance-notices-title">
          <div>
            <h2 id="governance-notices-title" className="font-semibold">平台通知</h2>
            <p className="text-sm text-muted-foreground">账号安全与违规处理相关消息不会被关闭。</p>
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
                  <p className="mt-1 text-xs text-muted-foreground">平台通知 · {formatUnixTime(item.createdAt)}</p>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  {!item.read ? (
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="标记平台通知为已读"
                      disabled={markGovernanceRead.isPending}
                      onClick={() => markGovernanceRead.mutate([item.id])}
                    >
                      <Check className="size-4" />
                    </Button>
                  ) : null}
                  <Button asChild variant="ghost" size="icon">
                    <Link
                      to={item.targetUrl}
                      aria-label="查看平台通知详情"
                      onClick={() => !item.read && markGovernanceRead.mutate([item.id])}
                    >
                      <ChevronRight className="size-4" />
                    </Link>
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </section>
      </PaginatedListState>

      <PaginatedListState
        isLoading={notifications.isLoading}
        error={notifications.error}
        isEmpty={items.length === 0}
        onRetry={() => void notifications.refetch()}
        hasMore={notifications.hasNextPage}
        isLoadingMore={notifications.isFetchingNextPage}
        onLoadMore={() => void notifications.fetchNextPage()}
        loading={<NotificationsSkeleton label="加载通知" />}
        empty={<EmptyState title={unreadOnly ? "没有未读通知" : "没有通知"} />}
      >
        <div className="space-y-6">
          {notificationGroups.map(([day, dayItems]) => (
            <section key={day} aria-labelledby={`notifications-${day}`} className="space-y-3">
              <h2 id={`notifications-${day}`} className="text-sm font-semibold text-muted-foreground">
                {notificationDayLabel(dayItems[0].createdAt)}
              </h2>
              {dayItems.map((item) => {
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
            </section>
          ))}
        </div>
      </PaginatedListState>
    </div>
  );
}
