import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bell, CheckCheck } from "lucide-react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

function payloadTitle(payload?: Record<string, unknown>) {
  if (!payload) {
    return "系统通知";
  }
  return String(payload.title ?? payload.threadTitle ?? payload.body ?? "系统通知");
}

export function NotificationsPage() {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const notifications = useQuery({
    queryKey: ["notifications"],
    queryFn: () => api.notifications(),
    enabled: isAuthenticated,
  });
  const markRead = useMutation({
    mutationFn: () => api.markNotificationsRead(),
    onSuccess: async () => {
      toast.success("已标记为已读");
      await queryClient.invalidateQueries({ queryKey: ["notifications"] });
      await queryClient.invalidateQueries({ queryKey: ["notification-count"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });

  if (!isAuthenticated) {
    return <EmptyState title="登录后查看通知" />;
  }

  return (
    <div>
      <PageHeader
        eyebrow="Notifications"
        title="通知"
        description="论坛回复、订阅、系统消息会聚合在这里。"
        actions={<Button variant="outline" onClick={() => markRead.mutate()}><CheckCheck className="h-4 w-4" />全部已读</Button>}
      />
      {notifications.isLoading ? (
        <LoadingState />
      ) : notifications.isError ? (
        <ErrorState error={notifications.error} onRetry={() => void notifications.refetch()} />
      ) : (notifications.data?.items ?? []).length === 0 ? (
        <EmptyState title="没有通知" />
      ) : (
        <div className="space-y-3">
          {(notifications.data?.items ?? []).map((item) => (
            <Card key={item.id} className={!item.read ? "border-primary/50" : undefined}>
              <CardContent className="flex items-start gap-3 p-4">
                <div className="rounded-md bg-secondary p-2 text-primary">
                  <Bell className="h-4 w-4" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium">{payloadTitle(item.payload)}</p>
                    {!item.read ? <Badge>未读</Badge> : null}
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">{item.type} · {formatUnixTime(item.createdAt)}</p>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
