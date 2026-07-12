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
import type { TrustLevelPolicy } from "@/lib/api/types";
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

function boundedTrustThreshold(value: string) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return Math.max(1, Math.min(100000, Math.round(parsed)));
}

function TrustPolicyCard({ trustPolicy }: { trustPolicy: TrustLevelPolicy }) {
  const queryClient = useQueryClient();
  const history = useQuery({
    queryKey: ["admin", "trust-policy", "history"],
    queryFn: () => api.adminTrustPolicyHistory(),
  });
  const [thresholds, setThresholds] = React.useState({
    thresholdLevel2: trustPolicy.thresholdLevel2,
    thresholdLevel3: trustPolicy.thresholdLevel3,
    thresholdLevel4: trustPolicy.thresholdLevel4,
    thresholdLevel5: trustPolicy.thresholdLevel5,
    thresholdLevel6: trustPolicy.thresholdLevel6,
  });
  const [likeDailyCap, setLikeDailyCap] = React.useState(trustPolicy.likeDailyCap);
  const [reason, setReason] = React.useState("");

  const hasChanges =
    thresholds.thresholdLevel2 !== trustPolicy.thresholdLevel2 ||
    thresholds.thresholdLevel3 !== trustPolicy.thresholdLevel3 ||
    thresholds.thresholdLevel4 !== trustPolicy.thresholdLevel4 ||
    thresholds.thresholdLevel5 !== trustPolicy.thresholdLevel5 ||
    thresholds.thresholdLevel6 !== trustPolicy.thresholdLevel6 ||
    likeDailyCap !== trustPolicy.likeDailyCap;

  const update = useMutation({
    mutationFn: () =>
      api.updateAdminTrustPolicy({
        expectedVersion: trustPolicy.version,
        ...thresholds,
        likeDailyCap,
        reason: reason.trim(),
      }),
    onSuccess: async () => {
      toast.success("信任等级策略已发布为新版本");
      setReason("");
      await queryClient.invalidateQueries({ queryKey: ["admin", "trust-policy"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "策略发布失败"),
  });

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="信任等级策略"
        description="设置各等级所需的累计得分阈值。活跃度策略的权重决定了用户每天能积累多少分。发布新版本后所有用户重新按新阈值评估。"
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_20rem]">
        <Card className="rounded-xl">
          <CardHeader>
            <div className="flex flex-wrap items-center gap-2">
              <CardTitle>等级阈值</CardTitle>
              <Badge variant="secondary">版本 {trustPolicy.version}</Badge>
            </div>
            <CardDescription>
              新注册用户初始等级为 Lv.1。当前活跃度策略版本：v{trustPolicy.scorePolicyVersion}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-5">
            <div className="grid gap-3 sm:grid-cols-5">
              {([
                { key: "thresholdLevel2" as const, label: "Lv.2", desc: "白茶" },
                { key: "thresholdLevel3" as const, label: "Lv.3", desc: "黄茶" },
                { key: "thresholdLevel4" as const, label: "Lv.4", desc: "青茶" },
                { key: "thresholdLevel5" as const, label: "Lv.5", desc: "红茶" },
                { key: "thresholdLevel6" as const, label: "Lv.6", desc: "黑茶" },
              ]).map(({ key, label, desc }) => (
                <div key={key} className="space-y-2">
                  <Label htmlFor={`trust-${key}`}>{label} ({desc})</Label>
                  <Input
                    id={`trust-${key}`}
                    type="number"
                    min={1}
                    step={1}
                    value={thresholds[key]}
                    onChange={(event) =>
                      setThresholds((prev) => ({
                        ...prev,
                        [key]: boundedTrustThreshold(event.target.value),
                      }))
                    }
                  />
                </div>
              ))}
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="trust-like-cap">每日点赞上限</Label>
                <Input
                  id="trust-like-cap"
                  type="number"
                  min={0}
                  max={100000}
                  step={1}
                  value={likeDailyCap}
                  onChange={(event) =>
                    setLikeDailyCap(boundedTrustThreshold(event.target.value))
                  }
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="trust-policy-reason">变更原因</Label>
              <Textarea
                id="trust-policy-reason"
                value={reason}
                onChange={(event) => setReason(event.target.value)}
                maxLength={500}
                placeholder="说明调整依据和预期影响；该内容会进入审计与版本历史"
              />
              <p className="text-xs text-muted-foreground">至少 3 个字符，最多 500 个字符。</p>
            </div>
            <Button
              type="button"
              onClick={() => update.mutate()}
              disabled={!hasChanges || reason.trim().length < 3 || update.isPending}
            >
              <Save className="size-4" />
              {update.isPending ? "正在发布…" : "发布新版本"}
            </Button>
          </CardContent>
        </Card>

        <Card className="rounded-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <History className="size-4 text-primary" />
              版本历史
            </CardTitle>
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
                      <Badge variant={item.version === trustPolicy.version ? "secondary" : "outline"}>
                        v{item.version}
                      </Badge>
                      <span className="text-xs text-muted-foreground">
                        {formatUnixTime(item.createdAt)}
                      </span>
                    </div>
                    <p className="mt-2 text-xs">
                      Lv2: {item.thresholdLevel2} · Lv3: {item.thresholdLevel3} · Lv4:{" "}
                      {item.thresholdLevel4} · Lv5: {item.thresholdLevel5} · Lv6:{" "}
                      {item.thresholdLevel6}
                    </p>
                    {item.likeDailyCap > 0 ? (
                      <p className="mt-1 text-xs">每日点赞上限：{item.likeDailyCap}</p>
                    ) : null}
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

export function ActivityPolicyPanel() {
  const queryClient = useQueryClient();
  const policy = useQuery({ queryKey: ["admin", "activity-policy"], queryFn: api.adminActivityPolicy });
  const history = useQuery({ queryKey: ["admin", "activity-policy", "history"], queryFn: () => api.adminActivityPolicyHistory() });
  const trustPolicy = useQuery({ queryKey: ["admin", "trust-policy"], queryFn: api.adminTrustPolicy });
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
    <div className="space-y-10">
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

      {trustPolicy.data ? <TrustPolicyCard trustPolicy={trustPolicy.data} /> : trustPolicy.isLoading ? <LoadingState label="加载信任等级策略" /> : trustPolicy.isError ? <ErrorState title="信任等级策略加载失败" error={trustPolicy.error} onRetry={() => void trustPolicy.refetch()} /> : null}
    </div>
  );
}