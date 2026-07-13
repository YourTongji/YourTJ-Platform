import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, Gavel, RotateCcw, Scale } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import {
  AdminSectionHeader,
  AdminStatusBadge,
  PaginationControls,
  ReasonDialog,
} from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api/endpoints";
import type { AdminAppeal, AppealStatus } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

const STATUS_OPTIONS: Array<{ value: AppealStatus | "all"; label: string }> = [
  { value: "submitted", label: "待领取" },
  { value: "in_review", label: "复核中" },
  { value: "upheld", label: "维持" },
  { value: "overturned", label: "撤销" },
  { value: "amended", label: "调整" },
  { value: "withdrawn", label: "用户撤回" },
  { value: "all", label: "全部" },
];

type Decision = {
  appeal: AdminAppeal;
  outcome: "upheld" | "overturned" | "amended";
};

function unixFromLocalDateTime(value: string) {
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? Math.floor(timestamp / 1_000) : undefined;
}

function DecisionDialog({
  decision,
  onClose,
  onSubmit,
  isPending,
}: {
  decision: Decision | null;
  onClose: () => void;
  onSubmit: (reason: string, amendedEndsAt?: number) => void;
  isPending: boolean;
}) {
  const [reason, setReason] = React.useState("");
  const [endsAt, setEndsAt] = React.useState("");
  React.useEffect(() => {
    setReason("");
    setEndsAt("");
  }, [decision]);
  const amendedEndsAt = unixFromLocalDateTime(endsAt);
  const needsEndsAt = decision?.outcome === "amended";
  const canSubmit = reason.trim().length >= 3 && (!needsEndsAt || Boolean(amendedEndsAt));
  return (
    <Dialog open={Boolean(decision)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {decision?.outcome === "upheld"
              ? "维持原处理"
              : decision?.outcome === "overturned"
                ? "撤销原处置"
                : "缩短账号制裁"}
          </DialogTitle>
          <DialogDescription>
            决定会写入不可变历史。撤销与调整必须由目标 owner domain 在同一事务中完成；无法安全恢复时服务端会拒绝。
          </DialogDescription>
        </DialogHeader>
        {needsEndsAt ? (
          <div className="space-y-2">
            <Label htmlFor="appeal-amended-ends">新的结束时间</Label>
            <Input
              id="appeal-amended-ends"
              type="datetime-local"
              value={endsAt}
              onChange={(event) => setEndsAt(event.target.value)}
            />
            <p className="text-xs text-muted-foreground">只能缩短当前制裁；若需立即结束，请选择“撤销原处置”。</p>
          </div>
        ) : null}
        <div className="space-y-2">
          <Label htmlFor="appeal-decision-reason">复核结论与理由</Label>
          <Textarea
            id="appeal-decision-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            maxLength={1000}
            placeholder="记录可向当事人解释的结论，不要复制举报人身份或私密证据。"
          />
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>取消</Button>
          <Button
            type="button"
            variant={decision?.outcome === "upheld" ? "outline" : "default"}
            disabled={!canSubmit || isPending}
            onClick={() => onSubmit(reason.trim(), needsEndsAt ? amendedEndsAt : undefined)}
          >
            {isPending ? "提交中" : "提交决定"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function AppealsPanel() {
  const queryClient = useQueryClient();
  const [status, setStatus] = React.useState<AppealStatus | "all">("submitted");
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const [claimTarget, setClaimTarget] = React.useState<AdminAppeal | null>(null);
  const [decision, setDecision] = React.useState<Decision | null>(null);
  const cursor = cursorStack.at(-1);
  const appeals = useQuery({
    queryKey: ["admin", "appeals", status, cursor],
    queryFn: () => api.adminAppeals(status === "all" ? undefined : status, cursor),
  });
  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["admin", "appeals"] }),
      queryClient.invalidateQueries({ queryKey: ["admin", "audit"] }),
    ]);
  };
  const claim = useMutation({
    mutationFn: ({ appeal, reason }: { appeal: AdminAppeal; reason: string }) =>
      api.startAdminAppealReview(appeal.id, appeal.version, reason),
    onSuccess: async () => {
      setClaimTarget(null);
      toast.success("申诉已进入你的复核队列");
      await invalidate();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "领取失败"),
  });
  const decide = useMutation({
    mutationFn: ({ appeal, outcome, reason, amendedEndsAt }: Decision & { reason: string; amendedEndsAt?: number }) =>
      api.decideAdminAppeal(appeal.id, {
        expectedVersion: appeal.version,
        outcome,
        reason,
        amendedEndsAt,
      }),
    onSuccess: async () => {
      setDecision(null);
      toast.success("复核决定已提交");
      await invalidate();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "决定失败"),
  });

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="治理申诉"
        description="复核人必须高于当事人角色且不能是原处置人。原处置与证据不会被覆盖。"
        actions={(
          <Select
            value={status}
            onValueChange={(value) => {
              setStatus(value as AppealStatus | "all");
              setCursorStack([null]);
            }}
          >
            <SelectTrigger className="w-36" aria-label="申诉状态筛选">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STATUS_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      />

      <Card className="border-primary/30 bg-primary/[0.03]">
        <CardContent className="flex gap-3 p-4 text-sm leading-6">
          <Scale className="mt-1 size-4 shrink-0 text-primary" aria-hidden="true" />
          维持只记录结论；撤销或调整会在同一数据库事务中调用账号、论坛或课评 owner API。状态已被后续操作改变时会 fail closed。
        </CardContent>
      </Card>

      {appeals.isLoading ? (
        <LoadingState label="加载申诉队列" />
      ) : appeals.isError ? (
        <ErrorState error={appeals.error} onRetry={() => void appeals.refetch()} />
      ) : (appeals.data?.items ?? []).length === 0 ? (
        <EmptyState title="当前筛选没有申诉" />
      ) : (
        <div className="space-y-3">
          {appeals.data?.items?.map((appeal) => (
            <Card key={appeal.id}>
              <CardContent className="space-y-4 p-4 sm:p-5">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <div className="flex flex-wrap items-center gap-2">
                      <AdminStatusBadge value={appeal.status} />
                      <Badge variant="outline">{appeal.targetKind}</Badge>
                      <span className="text-xs text-muted-foreground">申诉 #{appeal.id} · 当事人 #{appeal.appellantAccountId}</span>
                    </div>
                    <p className="mt-2 text-sm font-medium">原处置：{appeal.originalReason ?? appeal.originalAction}</p>
                    <p className="mt-1 text-sm text-muted-foreground">申诉理由：{appeal.submissionReason}</p>
                    <p className="mt-1 text-xs text-muted-foreground">提交于 {formatUnixTime(appeal.submittedAt)}</p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {appeal.status === "submitted" ? (
                      <Button size="sm" onClick={() => setClaimTarget(appeal)}>
                        <Gavel className="size-4" aria-hidden="true" />
                        领取复核
                      </Button>
                    ) : null}
                    {appeal.status === "in_review" ? (
                      <>
                        <Button size="sm" variant="outline" onClick={() => setDecision({ appeal, outcome: "upheld" })}>
                          <CheckCircle2 className="size-4" aria-hidden="true" />
                          维持
                        </Button>
                        <Button size="sm" onClick={() => setDecision({ appeal, outcome: "overturned" })}>
                          <RotateCcw className="size-4" aria-hidden="true" />
                          撤销
                        </Button>
                        {appeal.targetKind === "sanction" ? (
                          <Button size="sm" variant="secondary" onClick={() => setDecision({ appeal, outcome: "amended" })}>
                            缩短期限
                          </Button>
                        ) : null}
                      </>
                    ) : null}
                  </div>
                </div>
                <ol className="space-y-2 border-l pl-4" aria-label={`申诉 ${appeal.id} 状态历史`}>
                  {appeal.history.map((event) => (
                    <li key={event.id} className="text-sm">
                      <span className="font-medium">{event.toStatus}</span>
                      <span className="ml-2 text-xs text-muted-foreground">{formatUnixTime(event.createdAt)}</span>
                      <p className="text-muted-foreground">{event.reason}</p>
                    </li>
                  ))}
                </ol>
              </CardContent>
            </Card>
          ))}
          <PaginationControls
            hasPrevious={cursorStack.length > 1}
            hasMore={Boolean(appeals.data?.hasMore && appeals.data.nextCursor)}
            onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
            onNext={() => appeals.data?.nextCursor && setCursorStack((items) => [...items, appeals.data?.nextCursor ?? null])}
          />
        </div>
      )}

      <ReasonDialog
        open={Boolean(claimTarget)}
        onOpenChange={(open) => !open && setClaimTarget(null)}
        title="领取独立复核"
        description="领取后只有你能提交决定。系统会再次检查目标层级，并拒绝原处置人参与复核。"
        confirmLabel="领取复核"
        isPending={claim.isPending}
        onConfirm={(reason) => claimTarget && claim.mutate({ appeal: claimTarget, reason })}
      />
      <DecisionDialog
        decision={decision}
        onClose={() => setDecision(null)}
        isPending={decide.isPending}
        onSubmit={(reason, amendedEndsAt) => {
          if (decision) decide.mutate({ ...decision, reason, amendedEndsAt });
        }}
      />
    </div>
  );
}
