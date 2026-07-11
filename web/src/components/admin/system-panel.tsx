import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { DatabaseZap, RefreshCcw, Settings2 } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { AdminSectionHeader, ReasonDialog } from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api/endpoints";

type JobKind = "selection" | "courses" | "reviews" | "forum";

function SettingsPanel() {
  const queryClient = useQueryClient();
  const settings = useQuery({ queryKey: ["admin", "settings"], queryFn: api.adminSettings });
  const [drafts, setDrafts] = React.useState<Record<string, string>>({});
  const [saving, setSaving] = React.useState<{ key: string; value: string } | null>(null);
  React.useEffect(() => {
    setDrafts((current) => {
      const next = { ...current };
      for (const item of settings.data ?? []) {
        if (item.key && next[item.key] === undefined) next[item.key] = item.value ?? "";
      }
      return next;
    });
  }, [settings.data]);
  const update = useMutation({
    mutationFn: ({ key, value, reason }: { key: string; value: string; reason: string }) =>
      api.updateAdminSetting(key, { value, reason }),
    onSuccess: async () => {
      toast.success("平台设置已保存");
      setSaving(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "settings"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });

  if (settings.isLoading) return <LoadingState label="加载平台设置" />;
  if (settings.isError) return <ErrorState error={settings.error} onRetry={() => void settings.refetch()} />;
  if ((settings.data ?? []).length === 0) return <EmptyState title="没有平台设置" />;

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">当前设置仍是通用字符串，缺少字段类型、描述和版本控制；每次保存必须填写原因并进入治理审计。</p>
      {settings.data?.map((setting) => {
        const key = setting.key ?? "";
        const value = drafts[key] ?? "";
        const hasChanges = value !== (setting.value ?? "");
        return (
          <Card key={key}>
            <CardContent className="grid gap-3 p-4 md:grid-cols-[14rem_minmax(0,1fr)_auto] md:items-end">
              <div className="space-y-1">
                <Label htmlFor={`admin-setting-${key}`}>{key}</Label>
                <p className="text-xs text-muted-foreground">platform.settings</p>
              </div>
              <Input id={`admin-setting-${key}`} value={value} maxLength={20_000} onChange={(event) => setDrafts((values) => ({ ...values, [key]: event.target.value }))} />
              <Button type="button" variant="outline" onClick={() => setSaving({ key, value })} disabled={!key || !hasChanges || update.isPending}>
                保存
              </Button>
            </CardContent>
          </Card>
        );
      })}
      <ReasonDialog
        open={Boolean(saving)}
        onOpenChange={(open) => !open && setSaving(null)}
        title={`保存平台设置“${saving?.key ?? ""}”`}
        description="设置可能影响所有客户端和后台任务；请确认新值并说明变更依据。"
        confirmLabel="确认保存"
        isPending={update.isPending}
        onConfirm={(reason) => saving && update.mutate({ ...saving, reason })}
      >
        <div className="rounded-lg border bg-muted/40 p-3">
          <p className="text-xs text-muted-foreground">待保存的新值</p>
          <p className="mt-1 break-all font-mono text-sm">{saving?.value || "（空字符串）"}</p>
        </div>
      </ReasonDialog>
    </div>
  );
}

function JobsPanel() {
  const [selected, setSelected] = React.useState<JobKind | null>(null);
  const job = useMutation({
    mutationFn: ({ kind, reason }: { kind: JobKind; reason: string }) => {
      if (kind === "selection") return api.triggerSelectionSync(reason);
      if (kind === "courses") return api.reindexCourses(reason);
      if (kind === "reviews") return api.reindexReviews(reason);
      return api.reindexForum(reason);
    },
    onSuccess: () => {
      toast.success("任务已提交到后端队列");
      setSelected(null);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "任务触发失败"),
  });
  const jobs = [
    { kind: "selection" as const, title: "选课数据同步", description: "从一系统镜像选课目录。", icon: DatabaseZap },
    { kind: "courses" as const, title: "课程索引重建", description: "重建 Meilisearch course documents。", icon: RefreshCcw },
    { kind: "reviews" as const, title: "点评索引重建", description: "重建 Meilisearch reviews 索引。", icon: RefreshCcw },
    { kind: "forum" as const, title: "论坛索引重建", description: "重建 Meilisearch forum 索引。", icon: RefreshCcw },
  ];
  const selectedJob = jobs.find((item) => item.kind === selected);

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">当前接口只返回 202，没有持久任务 ID、进度、失败日志或安全重试状态。确认仅表示已提交，不表示已完成。</p>
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        {jobs.map((item) => (
          <Card key={item.kind} className="rounded-xl">
            <CardHeader>
              <CardTitle className="flex items-center gap-2"><item.icon className="size-4 text-primary" />{item.title}</CardTitle>
              <CardDescription>{item.description}</CardDescription>
            </CardHeader>
            <CardContent>
              <Button type="button" variant="outline" onClick={() => setSelected(item.kind)}>检查并触发</Button>
            </CardContent>
          </Card>
        ))}
      </div>
      <ReasonDialog
        open={Boolean(selected)}
        onOpenChange={(open) => !open && setSelected(null)}
        title={`触发${selectedJob?.title ?? "运维任务"}`}
        description="重复运行可能增加数据库和搜索服务负载。请确认没有同类任务正在执行，并说明本次操作依据。"
        confirmLabel="确认触发"
        isPending={job.isPending}
        onConfirm={(reason) => selected && job.mutate({ kind: selected, reason })}
      />
    </div>
  );
}

export function SystemPanel({ canManageSettings, canRunJobs }: { canManageSettings: boolean; canRunJobs: boolean }) {
  return (
    <div className="space-y-6">
      <AdminSectionHeader
        title="平台与任务"
        description="平台设置和高负载运维任务只对相应能力开放。积分账本不提供余额编辑或任意写入入口。"
      />
      {canManageSettings ? (
        <section className="space-y-3" aria-labelledby="platform-settings-heading">
          <h3 id="platform-settings-heading" className="flex items-center gap-2 font-semibold"><Settings2 className="size-4 text-primary" />平台设置</h3>
          <SettingsPanel />
        </section>
      ) : null}
      {canRunJobs ? (
        <section className="space-y-3" aria-labelledby="platform-jobs-heading">
          <h3 id="platform-jobs-heading" className="font-semibold">运维任务</h3>
          <JobsPanel />
        </section>
      ) : null}
    </div>
  );
}
