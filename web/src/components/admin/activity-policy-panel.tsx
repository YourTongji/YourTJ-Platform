import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Calculator, History, Save } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { AdminSectionHeader } from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

interface WeightDraft {
  thread: number;
  comment: number;
  like: number;
}

function boundedInteger(value: string) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return Math.max(0, Math.min(1000, Math.round(parsed)));
}

export function ActivityPolicyPanel() {
  const queryClient = useQueryClient();
  const policy = useQuery({ queryKey: ["admin", "activity-policy"], queryFn: api.adminActivityPolicy });
  const history = useQuery({ queryKey: ["admin", "activity-policy", "history"], queryFn: () => api.adminActivityPolicyHistory() });
  const [weights, setWeights] = React.useState<WeightDraft>({ thread: 10, comment: 3, like: 1 });
  const [reason, setReason] = React.useState("");
  const [sample, setSample] = React.useState({ thread: 1, comment: 3, like: 5 });

  React.useEffect(() => {
    if (policy.data) {
      setWeights(policy.data.weights);
    }
  }, [policy.data]);

  const update = useMutation({
    mutationFn: () => api.updateAdminActivityPolicy({
      expectedVersion: policy.data?.version ?? 0,
      weights,
      reason: reason.trim(),
    }),
    onSuccess: async () => {
      toast.success("活跃度策略已发布为新版本");
      setReason("");
      await queryClient.invalidateQueries({ queryKey: ["admin", "activity-policy"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "策略发布失败"),
  });

  if (policy.isLoading) return <LoadingState label="加载活跃度策略" />;
  if (policy.isError || !policy.data) {
    return <ErrorState title="活跃度策略加载失败" error={policy.error} onRetry={() => void policy.refetch()} />;
  }

  const current = policy.data;
  const preview = sample.thread * weights.thread + sample.comment * weights.comment + sample.like * weights.like;
  const hasChanges = weights.thread !== current.weights.thread
    || weights.comment !== current.weights.comment
    || weights.like !== current.weights.like;

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="活跃度策略"
        description="每日原始计数不会被改写；发布新权重后，历史热力图会按当前策略重新解释。更新使用版本号防止覆盖他人的修改。"
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_20rem]">
        <Card className="rounded-xl">
          <CardHeader>
            <div className="flex flex-wrap items-center gap-2">
              <CardTitle>权重编辑器</CardTitle>
              <Badge variant="secondary">版本 {current.version}</Badge>
              <Badge variant="outline">{current.timezone}</Badge>
            </div>
            <CardDescription>权重必须是 0–1000 的整数。点赞指用户主动给出的正向点赞。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-5">
            <div className="grid gap-3 sm:grid-cols-3">
              {(["thread", "comment", "like"] as const).map((key) => {
                const labels = { thread: "发帖权重", comment: "评论权重", like: "点赞权重" };
                return (
                  <div key={key} className="space-y-2">
                    <Label htmlFor={`activity-weight-${key}`}>{labels[key]}</Label>
                    <Input
                      id={`activity-weight-${key}`}
                      type="number"
                      min={0}
                      max={1000}
                      step={1}
                      value={weights[key]}
                      onChange={(event) => setWeights((previous) => ({ ...previous, [key]: boundedInteger(event.target.value) }))}
                    />
                  </div>
                );
              })}
            </div>
            <div className="rounded-xl border bg-muted/40 p-4">
              <div className="flex items-center gap-2 text-sm font-medium"><Calculator className="size-4 text-primary" />样例日预览</div>
              <div className="mt-3 grid gap-3 sm:grid-cols-3">
                {(["thread", "comment", "like"] as const).map((key) => {
                  const labels = { thread: "发帖", comment: "评论", like: "点赞" };
                  return (
                    <div key={key} className="space-y-1">
                      <Label htmlFor={`activity-sample-${key}`} className="text-xs">{labels[key]}</Label>
                      <Input
                        id={`activity-sample-${key}`}
                        type="number"
                        min={0}
                        value={sample[key]}
                        onChange={(event) => setSample((previous) => ({ ...previous, [key]: Math.max(0, boundedInteger(event.target.value)) }))}
                      />
                    </div>
                  );
                })}
              </div>
              <p className="mt-3 text-sm">
                {sample.thread} × {weights.thread} + {sample.comment} × {weights.comment} + {sample.like} × {weights.like}
                <span className="ml-2 font-semibold text-primary">= {preview} 分</span>
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="activity-policy-reason">变更原因</Label>
              <Textarea
                id="activity-policy-reason"
                value={reason}
                onChange={(event) => setReason(event.target.value)}
                maxLength={500}
                placeholder="说明调整依据和预期影响；该内容会进入审计与版本历史"
              />
              <p className="text-xs text-muted-foreground">至少 3 个字符，最多 500 个字符。</p>
            </div>
            <Button type="button" onClick={() => update.mutate()} disabled={!hasChanges || reason.trim().length < 3 || update.isPending}>
              <Save className="size-4" />{update.isPending ? "正在发布…" : "发布新版本"}
            </Button>
          </CardContent>
        </Card>

        <Card className="rounded-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><History className="size-4 text-primary" />版本历史</CardTitle>
            <CardDescription>最近发布的策略版本及其审计原因。</CardDescription>
          </CardHeader>
          <CardContent>
            {history.isLoading ? (
              <LoadingState />
            ) : history.isError ? (
              <ErrorState error={history.error} onRetry={() => void history.refetch()} />
            ) : (history.data?.items ?? []).length === 0 ? (
              <EmptyState title="暂无版本记录" />
            ) : (
              <div className="max-h-[34rem] space-y-3 overflow-y-auto pr-1">
                {history.data?.items?.map((item) => (
                  <div key={item.version} className="rounded-lg border p-3">
                    <div className="flex items-center justify-between gap-2">
                      <Badge variant={item.version === current.version ? "secondary" : "outline"}>v{item.version}</Badge>
                      <span className="text-xs text-muted-foreground">{formatUnixTime(item.createdAt)}</span>
                    </div>
                    <p className="mt-2 text-xs">发帖 ×{item.weights.thread} · 评论 ×{item.weights.comment} · 点赞 ×{item.weights.like}</p>
                    <p className="mt-2 text-xs leading-5 text-muted-foreground">{item.reason}</p>
                    <p className="mt-1 text-[10px] text-muted-foreground">操作人 {item.changedBy}</p>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
