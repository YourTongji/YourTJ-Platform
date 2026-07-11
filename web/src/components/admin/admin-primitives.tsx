import type { ReactNode } from "react";
import * as React from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

export function AdminSectionHeader({
  title,
  description,
  actions,
}: {
  title: string;
  description: string;
  actions?: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <h2 className="text-lg font-semibold tracking-tight">{title}</h2>
        <p className="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
      {actions ? <div className="flex shrink-0 flex-wrap gap-2">{actions}</div> : null}
    </div>
  );
}

export function AdminStatusBadge({ value }: { value?: string | null }) {
  const labels: Record<string, string> = {
    active: "正常",
    suspended: "已封禁",
    deleted: "已删除",
    open: "待处理",
    pending: "待审核",
    visible: "公开",
    hidden: "已隐藏",
    upheld: "举报成立",
    rejected: "举报驳回",
    ignored: "已忽略",
    clean: "已批准",
    blocked: "已阻止",
    block: "阻止",
    censor: "替换",
    queue: "送审",
    silence: "禁言",
    suspend: "封禁",
    user: "用户",
    mod: "版主",
    admin: "管理员",
    expired: "已到期",
    revoked: "已撤销",
    retired: "已停用",
    queued: "等待执行",
    running: "执行中",
    succeeded: "已完成",
    failed: "失败",
    dead: "已停止重试",
    cancelled: "已取消",
  };
  const normalized = value ?? "unknown";
  const isRisk = ["suspended", "deleted", "hidden", "upheld", "suspend", "blocked", "block", "failed", "dead"].includes(normalized);
  return (
    <Badge variant={isRisk ? "destructive" : normalized === "active" || normalized === "visible" ? "secondary" : "outline"}>
      {labels[normalized] ?? normalized}
    </Badge>
  );
}

export function PaginationControls({
  hasPrevious,
  hasMore,
  onPrevious,
  onNext,
}: {
  hasPrevious: boolean;
  hasMore: boolean;
  onPrevious: () => void;
  onNext: () => void;
}) {
  if (!hasPrevious && !hasMore) {
    return null;
  }
  return (
    <div className="flex justify-end gap-2 pt-2" aria-label="分页">
      <Button type="button" variant="outline" size="sm" onClick={onPrevious} disabled={!hasPrevious}>
        上一页
      </Button>
      <Button type="button" variant="outline" size="sm" onClick={onNext} disabled={!hasMore}>
        下一页
      </Button>
    </div>
  );
}

export function ReasonDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  isPending,
  destructive = false,
  minimumLength = 3,
  confirmDisabled = false,
  children,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel: string;
  isPending: boolean;
  destructive?: boolean;
  minimumLength?: number;
  confirmDisabled?: boolean;
  children?: ReactNode;
  onConfirm: (reason: string) => void;
}) {
  const [reason, setReason] = React.useState("");

  React.useEffect(() => {
    if (!open) {
      setReason("");
    }
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        {children}
        <div className="space-y-2">
          <Label htmlFor="admin-action-reason">操作原因</Label>
          <Textarea
            id="admin-action-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            placeholder="原因将进入不可变审计记录"
            maxLength={500}
          />
          <p className="text-xs text-muted-foreground">至少 {minimumLength} 个字符，不要填写口令或无关个人信息。</p>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={isPending}>
            取消
          </Button>
          <Button
            type="button"
            variant={destructive ? "destructive" : "default"}
            onClick={() => onConfirm(reason.trim())}
            disabled={reason.trim().length < minimumLength || isPending || confirmDisabled}
          >
            {isPending ? "正在提交…" : confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
