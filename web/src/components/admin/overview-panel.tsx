import { useQuery } from "@tanstack/react-query";
import {
  Ban,
  Flag,
  Heart,
  MessageSquare,
  MessageSquareWarning,
  RefreshCw,
  UserCheck,
  Users,
} from "lucide-react";

import { AdminSectionHeader } from "@/components/admin/admin-primitives";
import { ErrorState, LoadingState } from "@/components/common/states";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { api } from "@/lib/api/endpoints";
import { formatNumber } from "@/lib/format";

export function OverviewPanel() {
  const overview = useQuery({ queryKey: ["admin", "overview"], queryFn: api.adminOverview });

  if (overview.isLoading) {
    return <LoadingState label="加载管理概览" />;
  }
  if (overview.isError || !overview.data) {
    return <ErrorState title="管理概览加载失败" error={overview.error} onRetry={() => void overview.refetch()} />;
  }

  const item = overview.data;
  const metrics = [
    { label: "用户总数", value: item.totalUsers, detail: `${item.activeUsers} 个正常账号`, icon: Users },
    { label: "活跃用户", value: item.activeUsers, detail: `${item.moderators ?? 0} 位版主`, icon: UserCheck },
    { label: "封禁账号", value: item.suspendedUsers, detail: "需要定期复核", icon: Ban },
    { label: "点评举报", value: item.pendingReviewReports, detail: "待处理", icon: Flag },
    { label: "论坛举报", value: item.pendingForumFlags, detail: "待处理", icon: MessageSquareWarning },
    { label: "私信举报", value: item.pendingDmReports, detail: "仅显示已举报证据", icon: MessageSquare },
    { label: "今日发帖", value: item.threadsToday, detail: `${item.commentsToday} 条评论`, icon: MessageSquare },
    { label: "今日点赞", value: item.likesToday, detail: "正向点赞动作", icon: Heart },
  ];

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="治理概览"
        description="聚合账号状态、待处理举报和今日社区活动。这里展示的是操作入口，不替代各队列的证据审阅。"
        actions={
          <Button type="button" variant="outline" size="sm" onClick={() => void overview.refetch()} disabled={overview.isFetching}>
            <RefreshCw className={overview.isFetching ? "size-4 animate-spin" : "size-4"} />
            刷新
          </Button>
        }
      />
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {metrics.map((metric) => (
          <Card key={metric.label} className="rounded-xl">
            <CardContent className="p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-xs text-muted-foreground">{metric.label}</p>
                  <p className="mt-2 text-2xl font-semibold tabular-nums">{formatNumber(metric.value)}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{metric.detail}</p>
                </div>
                <span className="rounded-lg bg-primary/10 p-2 text-primary" aria-hidden="true">
                  <metric.icon className="size-4" />
                </span>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
      {item.pendingMediaUploads > 0 ? (
        <Card className="border-primary/30 bg-primary/5">
          <CardContent className="p-4 text-sm">
            还有 {item.pendingMediaUploads} 个媒体上传等待安全处理，可前往“内容资源 → 待审媒体”逐项审阅。
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}
