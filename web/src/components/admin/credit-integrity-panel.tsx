import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, ListChecks, PlayCircle, Scale } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { AdminSectionHeader, AdminStatusBadge, PaginationControls, ReasonDialog } from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { api } from "@/lib/api/endpoints";
import type { CreditReconciliationRun } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";
import { randomUuid } from "@/lib/random";

function runLabel(run: CreditReconciliationRun) {
  if (run.status === "failed") return "任务失败";
  if (run.status === "queued") return "等待执行";
  if (run.status === "running") return "正在校验";
  if (run.ledgerOk === false) return "账本异常";
  if ((run.driftedWallets ?? 0) > 0) return "发现投影漂移";
  return "完整性正常";
}

function SummaryCard({
  label,
  value,
  description,
}: {
  label: string;
  value: React.ReactNode;
  description: string;
}) {
  return (
    <Card className="rounded-xl">
      <CardContent className="p-4">
        <p className="text-xs text-muted-foreground">{label}</p>
        <div className="mt-2 text-2xl font-semibold tracking-tight">{value}</div>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p>
      </CardContent>
    </Card>
  );
}

export function CreditIntegrityPanel() {
  const queryClient = useQueryClient();
  const stats = useQuery({
    queryKey: ["admin", "credit-integrity", "stats"],
    queryFn: api.adminCreditReconciliationStats,
  });
  const [runCursor, setRunCursor] = React.useState<string | null>(null);
  const [runHistory, setRunHistory] = React.useState<Array<string | null>>([]);
  const runs = useQuery({
    queryKey: ["admin", "credit-integrity", "runs", runCursor],
    queryFn: () => api.adminCreditReconciliations(runCursor),
  });
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [driftOnly, setDriftOnly] = React.useState(true);
  const [dialogMode, setDialogMode] = React.useState<"request" | "resume" | null>(null);
  const [requestKey, setRequestKey] = React.useState<string | null>(null);
  const [walletCursor, setWalletCursor] = React.useState<string | null>(null);
  const [walletHistory, setWalletHistory] = React.useState<Array<string | null>>([]);

  React.useEffect(() => {
    const latestId = stats.data?.latestRun?.id ?? runs.data?.items?.[0]?.id;
    if (!selectedId && latestId) setSelectedId(latestId);
  }, [runs.data?.items, selectedId, stats.data?.latestRun?.id]);

  const detail = useQuery({
    queryKey: ["admin", "credit-integrity", "run", selectedId],
    queryFn: () => api.adminCreditReconciliation(selectedId ?? ""),
    enabled: Boolean(selectedId),
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      return status === "queued" || status === "running" ? 2_000 : false;
    },
  });
  const wallets = useQuery({
    queryKey: ["admin", "credit-integrity", "wallets", selectedId, driftOnly, walletCursor],
    queryFn: () => api.adminCreditReconciliationWallets(selectedId ?? "", walletCursor, driftOnly),
    enabled: Boolean(selectedId) && detail.data?.status === "succeeded" && detail.data.ledgerOk === true,
  });

  const requestRun = useMutation({
    mutationFn: ({ reason, key }: { reason: string; key: string }) =>
      api.requestAdminCreditReconciliation(reason, key),
    onSuccess: async (run) => {
      const isActive = run.status === "queued" || run.status === "running";
      toast.success(isActive
        ? "检查已登记，等待执行"
        : run.ledgerOk === false ? "检查完成：账本验证未通过" : "只读完整性检查已完成");
      setDialogMode(null);
      setRequestKey(null);
      setSelectedId(run.id ?? null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "credit-integrity", "stats"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "credit-integrity", "runs"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "credit-integrity", "run", run.id] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "完整性检查未能执行"),
  });

  const resumeRun = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) =>
      api.resumeAdminCreditReconciliation(id, reason),
    onSuccess: async (run) => {
      toast.success(run.status === "succeeded" ? "检查已恢复并完成" : "检查仍在执行");
      setDialogMode(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "credit-integrity", "stats"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "credit-integrity", "runs"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "credit-integrity", "run", run.id] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "未能继续完整性检查"),
  });

  const openRequestDialog = () => {
    try {
      setRequestKey(`credit-reconciliation:${randomUuid()}`);
      setDialogMode("request");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "无法生成安全的请求标识");
    }
  };

  const latest = stats.data?.latestRun;
  const selected = detail.data;
  const isLoading = stats.isLoading || runs.isLoading;
  const hasInitialError = stats.isError || runs.isError;

  return (
    <div className="space-y-6">
      <AdminSectionHeader
        title="积分完整性"
        description="从不可改写账本重新推导每个钱包，只读比较余额和最后序号。检查只记录证据与告警，绝不会自动改余额、补流水或覆盖历史。"
        actions={
          <Button type="button" onClick={openRequestDialog} disabled={requestRun.isPending || resumeRun.isPending}>
            <PlayCircle className="size-4" />运行只读检查
          </Button>
        }
      />

      {isLoading ? (
        <LoadingState label="加载积分完整性状态" />
      ) : hasInitialError ? (
        <ErrorState
          title="积分完整性状态加载失败"
          error={stats.error ?? runs.error}
          onRetry={() => {
            void stats.refetch();
            void runs.refetch();
          }}
        />
      ) : (
        <>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <SummaryCard
              label="历史检查"
              value={stats.data?.totalRuns ?? 0}
              description={`任务失败 ${stats.data?.failedRuns ?? 0} 次`}
            />
            <SummaryCard
              label="最新账本验证"
              value={latest ? runLabel(latest) : "尚未检查"}
              description={latest?.completedAt ? formatUnixTime(latest.completedAt) : "运行后才会生成完整性快照"}
            />
            <SummaryCard
              label="最新漂移钱包"
              value={latest?.driftedWallets ?? 0}
              description={`历史有漂移的检查 ${stats.data?.runsWithDrift ?? 0} 次`}
            />
            <SummaryCard
              label="最新绝对差额"
              value={latest?.totalAbsoluteDrift ?? "0"}
              description="仅是观测指标，不会触发自动修复"
            />
          </div>

          {latest?.ledgerOk === false ? (
            <div role="alert" className="flex gap-3 rounded-xl border border-destructive/30 bg-destructive/5 p-4 text-sm">
              <AlertTriangle className="mt-0.5 size-5 shrink-0 text-destructive" />
              <div>
                <p className="font-medium">账本链或签名验证未通过</p>
                <p className="mt-1 text-muted-foreground">系统已停止钱包推导，避免把不可信账本误当作修复依据。请按审计事件升级处理。</p>
              </div>
            </div>
          ) : null}

          <div className="grid gap-4 xl:grid-cols-[20rem_minmax(0,1fr)]">
            <Card className="rounded-xl">
              <CardHeader>
                <CardTitle className="flex items-center gap-2"><ListChecks className="size-4 text-primary" />检查历史</CardTitle>
                <CardDescription>请求原因、执行状态与只读结果会持久保存。</CardDescription>
              </CardHeader>
              <CardContent>
                {(runs.data?.items ?? []).length === 0 ? (
                  <EmptyState title="尚无完整性检查" />
                ) : (
                  <div className="space-y-2">
                    {runs.data?.items?.map((run) => (
                      <button
                        key={run.id}
                        type="button"
                        onClick={() => {
                          setSelectedId(run.id ?? null);
                          setDriftOnly(true);
                          setWalletCursor(null);
                          setWalletHistory([]);
                        }}
                        aria-pressed={selectedId === run.id}
                        className="w-full rounded-lg border p-3 text-left transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring data-[selected=true]:border-primary/40 data-[selected=true]:bg-primary/5"
                        data-selected={selectedId === run.id}
                      >
                        <div className="flex items-center justify-between gap-2">
                          <AdminStatusBadge value={run.status} />
                          <span className="text-xs text-muted-foreground">{run.createdAt ? formatUnixTime(run.createdAt) : "—"}</span>
                        </div>
                        <p className="mt-2 text-sm font-medium">{runLabel(run)}</p>
                        <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{run.reason}</p>
                      </button>
                    ))}
                    <PaginationControls
                      hasPrevious={runHistory.length > 0}
                      hasMore={Boolean(runs.data?.hasMore && runs.data.nextCursor)}
                      onPrevious={() => {
                        const previous = runHistory.at(-1) ?? null;
                        setRunHistory((history) => history.slice(0, -1));
                        setRunCursor(previous);
                      }}
                      onNext={() => {
                        if (!runs.data?.nextCursor) return;
                        setRunHistory((history) => [...history, runCursor]);
                        setRunCursor(runs.data.nextCursor ?? null);
                      }}
                    />
                  </div>
                )}
              </CardContent>
            </Card>

            <Card className="rounded-xl">
              <CardHeader>
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <CardTitle className="flex items-center gap-2"><Scale className="size-4 text-primary" />钱包比较结果</CardTitle>
                    <CardDescription className="mt-1">差额 = 当前 wallet cache − 账本推导值。</CardDescription>
                  </div>
                  {selected?.status === "queued" || selected?.status === "running" ? (
                    <Button type="button" variant="outline" size="sm" onClick={() => setDialogMode("resume")}>
                      继续未完成检查
                    </Button>
                  ) : selected?.ledgerOk === true ? (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setDriftOnly((value) => !value);
                        setWalletCursor(null);
                        setWalletHistory([]);
                      }}
                    >
                      {driftOnly ? "查看全部钱包" : "仅看漂移"}
                    </Button>
                  ) : null}
                </div>
              </CardHeader>
              <CardContent>
                {!selectedId || detail.isLoading ? (
                  <LoadingState label="加载检查详情" />
                ) : detail.isError || !selected ? (
                  <ErrorState error={detail.error} onRetry={() => void detail.refetch()} />
                ) : selected.status === "queued" || selected.status === "running" ? (
                  <LoadingState label={selected.status === "queued" ? "检查等待执行" : "正在只读校验账本与钱包"} />
                ) : selected.status === "failed" ? (
                  <EmptyState title="检查任务失败" description={`稳定错误码：${selected.errorCode ?? "UNKNOWN"}。失败不会改动任何积分数据。`} />
                ) : selected.ledgerOk === false ? (
                  <EmptyState title="账本验证未通过" description={`在序号 ${selected.ledgerFailureSeq ?? "未知"} 附近发现异常，未继续生成钱包比较。`} />
                ) : wallets.isLoading ? (
                  <LoadingState label="加载钱包比较" />
                ) : wallets.isError ? (
                  <ErrorState error={wallets.error} onRetry={() => void wallets.refetch()} />
                ) : (wallets.data?.items ?? []).length === 0 ? (
                  <div className="rounded-xl border border-dashed p-8 text-center">
                    <CheckCircle2 className="mx-auto size-7 text-primary" />
                    <p className="mt-3 font-medium">{driftOnly ? "没有发现钱包漂移" : "该快照没有钱包记录"}</p>
                    <p className="mt-1 text-sm text-muted-foreground">检查结果是只读证据，不会静默修改投影。</p>
                  </div>
                ) : (
                  <div className="overflow-x-auto rounded-lg border">
                    <table className="w-full min-w-[42rem] text-left text-sm">
                      <caption className="sr-only">积分账本推导值与钱包缓存比较结果</caption>
                      <thead className="bg-muted/60 text-xs text-muted-foreground">
                        <tr>
                          <th scope="col" className="px-3 py-2 font-medium">账号 ID</th>
                          <th scope="col" className="px-3 py-2 font-medium">账本推导</th>
                          <th scope="col" className="px-3 py-2 font-medium">钱包缓存</th>
                          <th scope="col" className="px-3 py-2 font-medium">差额</th>
                          <th scope="col" className="px-3 py-2 font-medium">序号</th>
                          <th scope="col" className="px-3 py-2 font-medium">状态</th>
                        </tr>
                      </thead>
                      <tbody>
                        {wallets.data?.items?.map((wallet) => {
                          const hasDrift = !wallet.walletExists || wallet.hasBalanceDrift || wallet.hasSequenceDrift;
                          return (
                            <tr key={wallet.accountId} className="border-t">
                              <td className="px-3 py-3 font-mono text-xs">{wallet.accountId}</td>
                              <td className="px-3 py-3 font-mono">{wallet.expectedBalance}</td>
                              <td className="px-3 py-3 font-mono">{wallet.actualBalance ?? "缺失"}</td>
                              <td className="px-3 py-3 font-mono">{wallet.delta}</td>
                              <td className="px-3 py-3 text-xs text-muted-foreground">{wallet.expectedLastSeq} / {wallet.actualLastSeq ?? "缺失"}</td>
                              <td className="px-3 py-3"><Badge variant={hasDrift ? "destructive" : "secondary"}>{hasDrift ? "漂移" : "一致"}</Badge></td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                )}
                <PaginationControls
                  hasPrevious={walletHistory.length > 0}
                  hasMore={Boolean(wallets.data?.hasMore && wallets.data.nextCursor)}
                  onPrevious={() => {
                    const previous = walletHistory.at(-1) ?? null;
                    setWalletHistory((history) => history.slice(0, -1));
                    setWalletCursor(previous);
                  }}
                  onNext={() => {
                    if (!wallets.data?.nextCursor) return;
                    setWalletHistory((history) => [...history, walletCursor]);
                    setWalletCursor(wallets.data.nextCursor ?? null);
                  }}
                />
              </CardContent>
            </Card>
          </div>
        </>
      )}

      <ReasonDialog
        open={dialogMode !== null}
        onOpenChange={(open) => {
          if (!open) {
            setDialogMode(null);
            if (!requestRun.isPending) setRequestKey(null);
          }
        }}
        title={dialogMode === "resume" ? "继续未完成的完整性检查" : "运行积分完整性检查"}
        description={dialogMode === "resume"
          ? "仅继续当前 queued/running run，不创建第二份快照。请说明恢复依据。"
          : "这会验证整条账本并逐钱包比较派生投影，可能增加数据库读取负载。它不会修改余额或账本。"}
        confirmLabel={dialogMode === "resume" ? "确认继续检查" : "确认运行只读检查"}
        isPending={requestRun.isPending || resumeRun.isPending}
        onConfirm={(reason) => {
          if (dialogMode === "resume" && selectedId) {
            resumeRun.mutate({ id: selectedId, reason });
          } else if (dialogMode === "request" && requestKey) {
            requestRun.mutate({ reason, key: requestKey });
          }
        }}
      />
    </div>
  );
}
