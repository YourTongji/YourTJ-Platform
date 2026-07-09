import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RefreshCcw, Shield, SlidersHorizontal } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatDate } from "@/lib/format";

function ReviewQueue() {
  const queryClient = useQueryClient();
  const reviews = useQuery({ queryKey: ["admin", "reviews"], queryFn: () => api.adminReviews("all") });
  const toggle = useMutation({
    mutationFn: (id: string) => api.toggleReview(id),
    onSuccess: async () => {
      toast.success("点评状态已更新");
      await queryClient.invalidateQueries({ queryKey: ["admin", "reviews"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });

  if (reviews.isLoading) {
    return <LoadingState />;
  }
  if (reviews.isError) {
    return <ErrorState error={reviews.error} onRetry={() => void reviews.refetch()} />;
  }
  if ((reviews.data?.items ?? []).length === 0) {
    return <EmptyState title="暂无点评" />;
  }
  return (
    <div className="space-y-3">
      {reviews.data?.items?.map((review) => (
        <Card key={review.id}>
          <CardContent className="flex flex-col gap-3 p-4 md:flex-row md:items-center md:justify-between">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="secondary">{review.status}</Badge>
                <span className="font-medium">{review.authorHandle}</span>
                <span className="text-sm text-muted-foreground">{review.rating} 星</span>
              </div>
              <p className="mt-2 line-clamp-2 text-sm">{review.comment ?? "无正文"}</p>
            </div>
            <Button size="sm" variant="outline" onClick={() => review.id && toggle.mutate(review.id)}>
              显隐切换
            </Button>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function ReportQueue() {
  const queryClient = useQueryClient();
  const reports = useQuery({ queryKey: ["admin", "reports"], queryFn: () => api.adminReports("open") });
  const resolve = useMutation({
    mutationFn: (id: string) => api.resolveReport(id, "resolve", "handled from web admin"),
    onSuccess: async () => {
      toast.success("举报已处理");
      await queryClient.invalidateQueries({ queryKey: ["admin", "reports"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });

  if (reports.isLoading) {
    return <LoadingState />;
  }
  if (reports.isError) {
    return <ErrorState error={reports.error} onRetry={() => void reports.refetch()} />;
  }
  if ((reports.data?.items ?? []).length === 0) {
    return <EmptyState title="暂无开放举报" />;
  }
  return (
    <div className="space-y-3">
      {reports.data?.items?.map((report) => (
        <Card key={report.id}>
          <CardContent className="flex flex-col gap-3 p-4 md:flex-row md:items-center md:justify-between">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="secondary">{report.status}</Badge>
                <span className="text-sm text-muted-foreground">Review {report.reviewId}</span>
              </div>
              <p className="mt-2 text-sm">{report.reason}</p>
              <p className="mt-1 text-xs text-muted-foreground">{formatDate(report.createdAt)}</p>
            </div>
            <Button size="sm" onClick={() => report.id && resolve.mutate(report.id)}>标记处理</Button>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function SettingsPanel() {
  const queryClient = useQueryClient();
  const settings = useQuery({ queryKey: ["admin", "settings"], queryFn: api.adminSettings });
  const [drafts, setDrafts] = React.useState<Record<string, string>>({});
  React.useEffect(() => {
    const next: Record<string, string> = {};
    for (const item of settings.data ?? []) {
      if (item.key) {
        next[item.key] = item.value ?? "";
      }
    }
    setDrafts(next);
  }, [settings.data]);
  const update = useMutation({
    mutationFn: ({ key, value }: { key: string; value: string }) => api.updateAdminSetting(key, value),
    onSuccess: async () => {
      toast.success("设置已保存");
      await queryClient.invalidateQueries({ queryKey: ["admin", "settings"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });

  if (settings.isLoading) {
    return <LoadingState />;
  }
  if (settings.isError) {
    return <ErrorState error={settings.error} onRetry={() => void settings.refetch()} />;
  }
  return (
    <div className="space-y-3">
      {(settings.data ?? []).map((setting) => (
        <Card key={setting.key}>
          <CardContent className="grid gap-3 p-4 md:grid-cols-[12rem_1fr_auto] md:items-end">
            <div className="space-y-2">
              <Label>{setting.key}</Label>
              <p className="text-xs text-muted-foreground">platform.settings</p>
            </div>
            <Input
              value={drafts[setting.key ?? ""] ?? ""}
              onChange={(event) => setDrafts((prev) => ({ ...prev, [setting.key ?? ""]: event.target.value }))}
            />
            <Button
              variant="outline"
              onClick={() => setting.key && update.mutate({ key: setting.key, value: drafts[setting.key] ?? "" })}
            >
              保存
            </Button>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function OpsPanel() {
  const selectionSync = useMutation({
    mutationFn: api.triggerSelectionSync,
    onSuccess: () => toast.success("选课同步任务已触发"),
    onError: (error) => toast.error(error instanceof Error ? error.message : "触发失败"),
  });
  const reviewReindex = useMutation({
    mutationFn: api.reindexReviews,
    onSuccess: () => toast.success("点评索引重建已触发"),
    onError: (error) => toast.error(error instanceof Error ? error.message : "触发失败"),
  });
  const forumReindex = useMutation({
    mutationFn: api.reindexForum,
    onSuccess: () => toast.success("论坛索引重建已触发"),
    onError: (error) => toast.error(error instanceof Error ? error.message : "触发失败"),
  });
  return (
    <div className="grid gap-3 md:grid-cols-3">
      <Card>
        <CardHeader>
          <CardTitle>选课同步</CardTitle>
          <CardDescription>触发一系统镜像同步。</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={() => selectionSync.mutate()} disabled={selectionSync.isPending}>
            <RefreshCcw className="h-4 w-4" />
            触发同步
          </Button>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>点评索引</CardTitle>
          <CardDescription>重建 Meilisearch reviews。</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={() => reviewReindex.mutate()} disabled={reviewReindex.isPending}>
            <RefreshCcw className="h-4 w-4" />
            重建
          </Button>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>论坛索引</CardTitle>
          <CardDescription>重建 Meilisearch forum。</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={() => forumReindex.mutate()} disabled={forumReindex.isPending}>
            <RefreshCcw className="h-4 w-4" />
            重建
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}

export function AdminPage() {
  const { account, isAuthenticated } = useAuth();
  const canManage = isAuthenticated && (account?.role === "admin" || account?.role === "mod");

  if (!canManage) {
    return <EmptyState title="没有管理权限" description="需要 mod 或 admin 角色。" />;
  }

  return (
    <div>
      <PageHeader
        eyebrow="Admin"
        title="管理后台"
        description="审核、举报、平台设置和同步任务的轻量管理入口。"
      />
      <Tabs defaultValue="reviews">
        <TabsList className="w-full justify-start overflow-x-auto">
          <TabsTrigger value="reviews"><Shield className="mr-1 h-4 w-4" />点评</TabsTrigger>
          <TabsTrigger value="reports">举报</TabsTrigger>
          <TabsTrigger value="settings"><SlidersHorizontal className="mr-1 h-4 w-4" />设置</TabsTrigger>
          <TabsTrigger value="ops">运维</TabsTrigger>
        </TabsList>
        <TabsContent value="reviews"><ReviewQueue /></TabsContent>
        <TabsContent value="reports"><ReportQueue /></TabsContent>
        <TabsContent value="settings"><SettingsPanel /></TabsContent>
        <TabsContent value="ops"><OpsPanel /></TabsContent>
      </Tabs>
    </div>
  );
}
