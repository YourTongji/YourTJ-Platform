import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlarmClock,
  ArchiveRestore,
  DatabaseZap,
  LockKeyhole,
  Plus,
  RotateCcw,
  ShieldAlert,
  Unlock,
} from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { PaginationControls, ReasonDialog } from "@/components/admin/admin-primitives";
import { RecentAuthDialog } from "@/components/auth/recent-auth-dialog";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { ApiError } from "@/lib/api/client";
import { api } from "@/lib/api/endpoints";
import type { MediaDeletionJob, MediaRetentionHold } from "@/lib/api/types";
import { formatRelativeTime, formatUnixTime } from "@/lib/format";

type HoldState = "active" | "expired";
type JobStatus = "dead_letter" | "queued" | "leased" | "succeeded";
type HoldKind = "moderation" | "security";
type HoldDecision = { hold: MediaRetentionHold; action: "renew" | "release" };

const DAY_MS = 24 * 60 * 60 * 1000;
const HOLD_KIND_LABELS: Record<HoldKind, string> = {
  moderation: "治理调查",
  security: "安全事件",
};
const JOB_STATUS_LABELS: Record<JobStatus, string> = {
  dead_letter: "死信",
  queued: "等待执行",
  leased: "正在执行",
  succeeded: "已完成",
};
const JOB_SOURCE_LABELS: Record<MediaDeletionJob["requestSource"], string> = {
  retention_gc: "保留期清理",
  account_purge: "账号清除",
  intent_cleanup: "过期上传凭证清理",
};

function localDateTimeInput(timestamp: number) {
  const date = new Date(timestamp);
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(timestamp - offset).toISOString().slice(0, 16);
}

function isRecentAuthRequired(error: unknown) {
  return error instanceof ApiError
    && (error.status === 428 || error.code === "RECENT_AUTH_REQUIRED");
}

function expiryBadge(hold: MediaRetentionHold) {
  if (hold.isExpired) {
    return <Badge variant="destructive">已到期</Badge>;
  }
  if (hold.expiresAt * 1000 - Date.now() <= 3 * DAY_MS) {
    return <Badge variant="destructive">72 小时内到期</Badge>;
  }
  return <Badge variant="secondary">有效</Badge>;
}

export function MediaOperations() {
  const queryClient = useQueryClient();
  const [holdState, setHoldState] = React.useState<HoldState>("active");
  const [holdCursorStack, setHoldCursorStack] = React.useState<Array<string | null>>([null]);
  const [jobStatus, setJobStatus] = React.useState<JobStatus>("dead_letter");
  const [jobCursorStack, setJobCursorStack] = React.useState<Array<string | null>>([null]);
  const [holdDecision, setHoldDecision] = React.useState<HoldDecision | null>(null);
  const [newHoldUploadId, setNewHoldUploadId] = React.useState("");
  const [isCreateHoldOpen, setIsCreateHoldOpen] = React.useState(false);
  const [holdKind, setHoldKind] = React.useState<HoldKind>("moderation");
  const [holdUntil, setHoldUntil] = React.useState("");
  const [retryJob, setRetryJob] = React.useState<MediaDeletionJob | null>(null);
  const [recentAuthRetry, setRecentAuthRetry] = React.useState<(() => void) | null>(null);
  const handledHoldInventoryError = React.useRef<unknown>(null);
  const handledJobInventoryError = React.useRef<unknown>(null);
  const holdCursor = holdCursorStack.at(-1);
  const jobCursor = jobCursorStack.at(-1);

  const holds = useQuery({
    queryKey: ["admin", "media", "retention-holds", holdState, holdCursor],
    queryFn: () => api.adminMediaRetentionHolds(holdCursor, holdState),
    gcTime: 0,
  });
  const jobs = useQuery({
    queryKey: ["admin", "media", "deletion-jobs", jobStatus, jobCursor],
    queryFn: () => api.adminMediaDeletionJobs(jobCursor, jobStatus),
    gcTime: 0,
  });
  const holdsError = holds.error;
  const jobsError = jobs.error;
  const refetchHolds = holds.refetch;
  const refetchJobs = jobs.refetch;

  React.useEffect(() => {
    if (
      holdsError
      && holdsError !== handledHoldInventoryError.current
      && isRecentAuthRequired(holdsError)
    ) {
      handledHoldInventoryError.current = holdsError;
      setRecentAuthRetry(() => () => { void Promise.all([refetchHolds(), refetchJobs()]); });
    }
  }, [holdsError, refetchHolds, refetchJobs]);

  React.useEffect(() => {
    if (
      jobsError
      && jobsError !== handledJobInventoryError.current
      && isRecentAuthRequired(jobsError)
    ) {
      handledJobInventoryError.current = jobsError;
      setRecentAuthRetry(() => () => { void Promise.all([refetchHolds(), refetchJobs()]); });
    }
  }, [jobsError, refetchHolds, refetchJobs]);

  const holdMutation = useMutation({
    mutationFn: async ({ decision, reason, kind, expiresAt }: {
      decision: HoldDecision;
      reason: string;
      kind: HoldKind;
      expiresAt: number;
    }) => {
      if (decision.action === "release") {
        await api.releaseAdminMediaRetentionHold(
          decision.hold.uploadId,
          decision.hold.id,
          reason,
        );
        return;
      }
      await api.placeAdminMediaRetentionHold(decision.hold.uploadId, {
        holdKind: kind,
        expiresAt,
        reason,
        expectedHoldId: decision.hold.id,
      });
    },
    onSuccess: async (_data, variables) => {
      toast.success(variables.decision.action === "release" ? "媒体保留已解除" : "媒体保留已续期");
      setHoldDecision(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "media", "retention-holds"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "media", "deletion-jobs"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "media", "pending"] }),
      ]);
    },
    onError: (error, variables) => {
      if (isRecentAuthRequired(error)) {
        setHoldDecision(null);
        setRecentAuthRetry(() => () => holdMutation.mutate(variables));
        return;
      }
      if (error instanceof ApiError && error.status === 409) {
        void queryClient.invalidateQueries({ queryKey: ["admin", "media", "retention-holds"] });
      }
      toast.error(error instanceof Error ? error.message : "媒体保留操作失败");
    },
  });
  const createHoldMutation = useMutation({
    mutationFn: ({ uploadId, reason, kind, expiresAt }: {
      uploadId: string;
      reason: string;
      kind: HoldKind;
      expiresAt: number;
    }) => api.placeAdminMediaRetentionHold(uploadId, {
      holdKind: kind,
      expiresAt,
      reason,
      expectedHoldId: null,
    }),
    onSuccess: async () => {
      toast.success("媒体保留已设置");
      setIsCreateHoldOpen(false);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "media", "retention-holds"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "media", "deletion-jobs"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "media", "pending"] }),
      ]);
    },
    onError: (error, variables) => {
      if (isRecentAuthRequired(error)) {
        setIsCreateHoldOpen(false);
        setRecentAuthRetry(() => () => createHoldMutation.mutate(variables));
        return;
      }
      if (error instanceof ApiError && error.status === 409) {
        void queryClient.invalidateQueries({ queryKey: ["admin", "media", "retention-holds"] });
      }
      toast.error(error instanceof Error ? error.message : "媒体保留设置失败");
    },
  });
  const retryMutation = useMutation({
    mutationFn: ({ job, reason }: { job: MediaDeletionJob; reason: string }) =>
      api.retryAdminMediaDeletionJob(job.id, reason),
    onSuccess: async () => {
      toast.success("系统删除任务已重新排队");
      setRetryJob(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "media", "deletion-jobs"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "overview"] }),
      ]);
    },
    onError: (error, variables) => {
      if (isRecentAuthRequired(error)) {
        setRetryJob(null);
        setRecentAuthRetry(() => () => retryMutation.mutate(variables));
        return;
      }
      if (error instanceof ApiError && error.status === 409) {
        void queryClient.invalidateQueries({ queryKey: ["admin", "media", "deletion-jobs"] });
      }
      toast.error(error instanceof Error ? error.message : "删除任务重试失败");
    },
  });

  function openHoldDecision(hold: MediaRetentionHold, action: HoldDecision["action"]) {
    if (action === "renew") {
      const now = Date.now();
      const proposed = Math.max(now + 30 * DAY_MS, hold.expiresAt * 1000 + 30 * DAY_MS);
      setHoldKind(hold.holdKind);
      setHoldUntil(localDateTimeInput(Math.min(now + 365 * DAY_MS - 60_000, proposed)));
    }
    setHoldDecision({ hold, action });
  }

  function openNewHold(uploadId: string) {
    setNewHoldUploadId(uploadId);
    setHoldKind("moderation");
    setHoldUntil(localDateTimeInput(Date.now() + 30 * DAY_MS));
    setIsCreateHoldOpen(true);
  }

  const holdErrorNeedsRecentAuth = isRecentAuthRequired(holds.error);
  const jobErrorNeedsRecentAuth = isRecentAuthRequired(jobs.error);

  return (
    <div className="space-y-6">
      <Card className="border-primary/30 bg-primary/5">
        <CardContent className="flex gap-3 p-4 text-xs leading-5 text-muted-foreground">
          <ShieldAlert className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />
          <p>
            此处包含保留目的、经办人和系统清理原因，仅向运维能力开放。保留不会恢复公开访问，只会暂停物理删除；读取清单和所有变更都会审计。
          </p>
        </CardContent>
      </Card>

      <section className="space-y-3" aria-labelledby="retention-inventory-title">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h3 id="retention-inventory-title" className="flex items-center gap-2 font-semibold">
              <LockKeyhole className="size-4 text-primary" aria-hidden="true" />媒体保留清单
            </h3>
            <p className="mt-1 text-sm text-muted-foreground">按到期时间升序排列，优先处理临期记录。</p>
          </div>
          <div className="grid grid-cols-2 gap-1 rounded-lg bg-muted p-1" role="group" aria-label="媒体保留状态筛选">
            {(["active", "expired"] as const).map((state) => (
              <Button
                key={state}
                type="button"
                size="sm"
                variant={holdState === state ? "secondary" : "ghost"}
                aria-pressed={holdState === state}
                onClick={() => {
                  setHoldState(state);
                  setHoldCursorStack([null]);
                }}
              >
                {state === "active" ? "有效保留" : "已到期"}
              </Button>
            ))}
          </div>
        </div>

        <Card className="border-dashed">
          <CardContent className="flex flex-col gap-3 p-4 sm:flex-row sm:items-end">
            <div className="min-w-0 flex-1 space-y-2">
              <Label htmlFor="operations-new-hold-upload">按已复核上传 ID 设置保留</Label>
              <Input
                id="operations-new-hold-upload"
                inputMode="numeric"
                pattern="[1-9][0-9]*"
                value={newHoldUploadId}
                placeholder="如 100001"
                onChange={(event) => setNewHoldUploadId(event.target.value.trim())}
              />
              <p className="text-xs text-muted-foreground">
                可处理管理员自有对象及系统删除任务；提交时会原子确认当前没有其他有效保留。
              </p>
            </div>
            <Button
              type="button"
              size="sm"
              className="shrink-0"
              disabled={!/^[1-9][0-9]*$/.test(newHoldUploadId)}
              onClick={() => openNewHold(newHoldUploadId)}
            >
              <Plus className="size-4" aria-hidden="true" />设置保留
            </Button>
          </CardContent>
        </Card>

        {holds.isLoading ? <LoadingState label="加载媒体保留清单" /> : null}
        {holds.isError && !holdErrorNeedsRecentAuth ? (
          <ErrorState error={holds.error} onRetry={() => void holds.refetch()} />
        ) : null}
        {holdErrorNeedsRecentAuth ? (
          <Card>
            <CardContent className="flex flex-col items-start gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <p className="font-medium">查看保留详情前需要重新验证</p>
                <p className="mt-1 text-sm text-muted-foreground">验证只绑定当前可撤销会话，有效期 10 分钟。</p>
              </div>
              <Button type="button" size="sm" onClick={() => setRecentAuthRetry(() => () => { void holds.refetch(); })}>
                重新验证
              </Button>
            </CardContent>
          </Card>
        ) : null}
        {!holds.isLoading && !holds.isError && (holds.data?.items ?? []).length === 0 ? (
          <EmptyState title={holdState === "active" ? "当前没有有效媒体保留" : "当前没有待处理的到期保留"} />
        ) : null}
        <div className="grid gap-3 xl:grid-cols-2">
          {!holds.isError ? holds.data?.items?.map((hold) => (
            <Card key={hold.id} className="hover:border-primary/30 hover:shadow-sm">
              <CardHeader className="pb-3">
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div>
                    <CardTitle className="text-sm">上传 #{hold.uploadId}</CardTitle>
                    <CardDescription className="mt-1">账号 {hold.accountId} · 保留 #{hold.id}</CardDescription>
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {expiryBadge(hold)}
                    <Badge variant="outline">{HOLD_KIND_LABELS[hold.holdKind]}</Badge>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                <dl className="grid gap-x-4 gap-y-2 text-sm sm:grid-cols-2">
                  <div><dt className="text-xs text-muted-foreground">到期时间</dt><dd>{formatUnixTime(hold.expiresAt)} · {formatRelativeTime(hold.expiresAt)}</dd></div>
                  <div><dt className="text-xs text-muted-foreground">对象状态</dt><dd>{hold.uploadStatus}</dd></div>
                  <div><dt className="text-xs text-muted-foreground">设置人</dt><dd>账号 {hold.placedBy}</dd></div>
                  <div><dt className="text-xs text-muted-foreground">设置时间</dt><dd>{formatUnixTime(hold.createdAt)}</dd></div>
                </dl>
                <div className="rounded-lg border bg-muted/30 p-3">
                  <p className="text-xs font-medium text-muted-foreground">保留原因</p>
                  <p className="mt-1 whitespace-pre-wrap break-words text-sm leading-6">{hold.reason}</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button type="button" size="sm" variant="outline" onClick={() => openHoldDecision(hold, "renew")}>
                    <ArchiveRestore className="size-4" aria-hidden="true" />续期
                  </Button>
                  <Button type="button" size="sm" variant="outline" className="text-destructive hover:text-destructive" onClick={() => openHoldDecision(hold, "release")}>
                    <Unlock className="size-4" aria-hidden="true" />解除
                  </Button>
                </div>
              </CardContent>
            </Card>
          )) : null}
        </div>
        <PaginationControls
          hasPrevious={holdCursorStack.length > 1}
          hasMore={Boolean(!holds.isError && holds.data?.hasMore && holds.data.nextCursor)}
          onPrevious={() => setHoldCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
          onNext={() => holds.data?.nextCursor && setHoldCursorStack((items) => [...items, holds.data?.nextCursor ?? null])}
        />
      </section>

      <section className="space-y-3 border-t pt-6" aria-labelledby="system-deletion-title">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h3 id="system-deletion-title" className="flex items-center gap-2 font-semibold">
              <DatabaseZap className="size-4 text-primary" aria-hidden="true" />系统删除任务
            </h3>
            <p className="mt-1 text-sm text-muted-foreground">显示保留期清理、账号清除和过期上传凭证清理任务；人工审核删除仍在待审媒体队列处理。</p>
          </div>
          <Select value={jobStatus} onValueChange={(value) => {
            setJobStatus(value as JobStatus);
            setJobCursorStack([null]);
          }}>
            <SelectTrigger className="w-full sm:w-40" aria-label="系统删除任务状态"><SelectValue /></SelectTrigger>
            <SelectContent>
              {(Object.entries(JOB_STATUS_LABELS) as Array<[JobStatus, string]>).map(([value, label]) => (
                <SelectItem key={value} value={value}>{label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        {jobs.isLoading ? <LoadingState label="加载系统删除任务" /> : null}
        {jobs.isError && !jobErrorNeedsRecentAuth ? (
          <ErrorState error={jobs.error} onRetry={() => void jobs.refetch()} />
        ) : null}
        {jobErrorNeedsRecentAuth ? (
          <Card>
            <CardContent className="flex flex-col items-start gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <p className="font-medium">查看系统删除详情前需要重新验证</p>
                <p className="mt-1 text-sm text-muted-foreground">验证后会同时刷新保留与删除任务清单。</p>
              </div>
              <Button type="button" size="sm" onClick={() => setRecentAuthRetry(() => () => { void Promise.all([holds.refetch(), jobs.refetch()]); })}>
                重新验证
              </Button>
            </CardContent>
          </Card>
        ) : null}
        {!jobs.isLoading && !jobs.isError && (jobs.data?.items ?? []).length === 0 ? (
          <EmptyState title={`没有${JOB_STATUS_LABELS[jobStatus]}的系统删除任务`} />
        ) : null}
        <div className="space-y-3">
          {!jobs.isError ? jobs.data?.items?.map((job) => (
            <Card key={job.id} className="hover:border-primary/30 hover:shadow-sm">
              <CardContent className="flex flex-col gap-4 p-4 lg:flex-row lg:items-center lg:justify-between">
                <div className="min-w-0 space-y-2">
                  <div className="flex flex-wrap items-center gap-2">
                    {job.status === "dead_letter" ? <Badge variant="destructive">死信</Badge> : <Badge variant="outline">{JOB_STATUS_LABELS[job.status]}</Badge>}
                    <Badge variant="outline">{JOB_SOURCE_LABELS[job.requestSource]}</Badge>
                    <span className="text-sm font-medium">任务 #{job.id} · 上传 #{job.uploadId}</span>
                  </div>
                  <p className="text-sm leading-6">{job.reason}</p>
                  <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                    <span>账号 {job.accountId}</span>
                    <span>对象 {job.uploadStatus}</span>
                    <span>尝试 {job.attemptCount} 次</span>
                    <span>更新于 {formatUnixTime(job.updatedAt)}</span>
                    {job.lastErrorCode ? <span className="text-destructive">错误 {job.lastErrorCode}</span> : null}
                  </div>
                </div>
                <div className="flex shrink-0 flex-wrap items-center gap-2">
                  {job.status === "dead_letter" ? (
                    <Button type="button" size="sm" variant="outline" onClick={() => setRetryJob(job)}>
                      <RotateCcw className="size-4" aria-hidden="true" />重新排队
                    </Button>
                  ) : null}
                  {job.status === "queued" || job.status === "dead_letter" ? (
                    <Button type="button" size="sm" variant="outline" onClick={() => openNewHold(job.uploadId)}>
                      <LockKeyhole className="size-4" aria-hidden="true" />设置保留
                    </Button>
                  ) : null}
                  {job.status === "queued" ? (
                    <span className="flex items-center gap-1 text-xs text-muted-foreground">
                      <AlarmClock className="size-4" aria-hidden="true" />可执行 {formatUnixTime(job.availableAt)}
                    </span>
                  ) : null}
                </div>
              </CardContent>
            </Card>
          )) : null}
        </div>
        <PaginationControls
          hasPrevious={jobCursorStack.length > 1}
          hasMore={Boolean(jobs.data?.hasMore && jobs.data.nextCursor)}
          onPrevious={() => setJobCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
          onNext={() => jobs.data?.nextCursor && setJobCursorStack((items) => [...items, jobs.data?.nextCursor ?? null])}
        />
      </section>

      <ReasonDialog
        open={isCreateHoldOpen}
        onOpenChange={(open) => !open && setIsCreateHoldOpen(false)}
        title={`为上传 #${newHoldUploadId || "—"} 设置保留`}
        description="仅对已经核验的上传 ID 设置目的受限、自动到期的运维保留。提交时会以 expectedHoldId: null 原子确认没有并发保留。"
        confirmLabel="确认设置保留"
        isPending={createHoldMutation.isPending}
        confirmDisabled={
          !/^[1-9][0-9]*$/.test(newHoldUploadId)
          || !holdUntil
          || new Date(holdUntil).getTime() < Date.now() + 5 * 60 * 1000
          || new Date(holdUntil).getTime() > Date.now() + 365 * DAY_MS
        }
        onConfirm={(reason) => createHoldMutation.mutate({
          uploadId: newHoldUploadId,
          reason,
          kind: holdKind,
          expiresAt: Math.floor(new Date(holdUntil).getTime() / 1000),
        })}
      >
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="operations-create-hold-kind">保留目的</Label>
            <Select value={holdKind} onValueChange={(value) => setHoldKind(value as HoldKind)}>
              <SelectTrigger id="operations-create-hold-kind"><SelectValue /></SelectTrigger>
              <SelectContent>
                {(Object.entries(HOLD_KIND_LABELS) as Array<[HoldKind, string]>).map(([value, label]) => (
                  <SelectItem key={value} value={value}>{label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="operations-create-hold-until">到期时间</Label>
            <Input
              id="operations-create-hold-until"
              type="datetime-local"
              value={holdUntil}
              min={localDateTimeInput(Date.now() + 5 * 60 * 1000)}
              max={localDateTimeInput(Date.now() + 365 * DAY_MS)}
              onChange={(event) => setHoldUntil(event.target.value)}
            />
          </div>
        </div>
      </ReasonDialog>
      <ReasonDialog
        open={holdDecision !== null}
        onOpenChange={(open) => !open && setHoldDecision(null)}
        title={holdDecision?.action === "release" ? "解除媒体保留" : "续期媒体保留"}
        description={holdDecision?.action === "release"
          ? "解除后，已排队的物理删除可以继续执行。提交时会核对你刚刚查看的保留记录，避免解除并发创建的新保留。"
          : "系统会原子替换你刚刚查看的保留记录，不会产生无保护间隙；若记录已被他人修改，本次操作会被拒绝。"}
        confirmLabel={holdDecision?.action === "release" ? "确认解除" : "确认续期"}
        destructive={holdDecision?.action === "release"}
        isPending={holdMutation.isPending}
        confirmDisabled={holdDecision?.action === "renew" && (
          !holdUntil
          || new Date(holdUntil).getTime() < Date.now() + 5 * 60 * 1000
          || new Date(holdUntil).getTime() > Date.now() + 365 * DAY_MS
        )}
        onConfirm={(reason) => {
          if (!holdDecision) return;
          holdMutation.mutate({
            decision: holdDecision,
            reason,
            kind: holdKind,
            expiresAt: Math.floor(new Date(holdUntil).getTime() / 1000),
          });
        }}
      >
        {holdDecision?.action === "renew" ? (
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="operations-hold-kind">保留目的</Label>
              <Select value={holdKind} onValueChange={(value) => setHoldKind(value as HoldKind)}>
                <SelectTrigger id="operations-hold-kind"><SelectValue /></SelectTrigger>
                <SelectContent>
                  {(Object.entries(HOLD_KIND_LABELS) as Array<[HoldKind, string]>).map(([value, label]) => (
                    <SelectItem key={value} value={value}>{label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="operations-hold-until">新到期时间</Label>
              <Input
                id="operations-hold-until"
                type="datetime-local"
                value={holdUntil}
                min={localDateTimeInput(Date.now() + 5 * 60 * 1000)}
                max={localDateTimeInput(Date.now() + 365 * DAY_MS)}
                onChange={(event) => setHoldUntil(event.target.value)}
              />
            </div>
          </div>
        ) : null}
      </ReasonDialog>
      <ReasonDialog
        open={retryJob !== null}
        onOpenChange={(open) => !open && setRetryJob(null)}
        title="重新排队系统删除任务"
        description="只会重置这条死信的执行状态，不会改变最初的系统清理目的。重试原因会进入独立、限期保留的运维记录。"
        confirmLabel="确认重新排队"
        isPending={retryMutation.isPending}
        onConfirm={(reason) => retryJob && retryMutation.mutate({ job: retryJob, reason })}
      />
      <RecentAuthDialog
        open={recentAuthRetry !== null}
        onOpenChange={(open) => { if (!open) setRecentAuthRetry(null); }}
        onVerified={() => {
          const retry = recentAuthRetry;
          setRecentAuthRetry(null);
          retry?.();
        }}
      />
    </div>
  );
}
