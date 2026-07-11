import * as React from "react";

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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import type { DmMessage, DmReportReason } from "@/lib/api/types";

const reasonLabels: Record<DmReportReason, string> = {
  spam: "垃圾信息",
  abuse: "辱骂或攻击",
  harassment: "骚扰",
  fraud: "欺诈",
  illegal: "违法内容",
  other: "其他",
};

export function ReportMessageDialog({
  message,
  isPending,
  error,
  onClose,
  onReport,
}: {
  message: DmMessage | null;
  isPending: boolean;
  error?: unknown;
  onClose: () => void;
  onReport: (message: DmMessage, reason: DmReportReason, note?: string) => Promise<void>;
}) {
  const [reason, setReason] = React.useState<DmReportReason>("spam");
  const [note, setNote] = React.useState("");

  React.useEffect(() => {
    if (!message) {
      setReason("spam");
      setNote("");
    }
  }, [message]);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!message) return;
    try {
      await onReport(message, reason, note.trim() || undefined);
      onClose();
    } catch {
      // The parent mutation exposes a contextual error while the dialog stays open.
    }
  }

  return (
    <Dialog open={Boolean(message)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <form onSubmit={(event) => void submit(event)}>
          <DialogHeader>
            <DialogTitle>举报这条私信</DialogTitle>
            <DialogDescription>
              只有这条被举报的消息及你的补充说明会进入审核队列，管理员不能浏览未举报的私信会话。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-5">
            <blockquote className="line-clamp-3 rounded-lg border bg-muted/30 p-3 text-sm leading-6">
              {message?.body}
            </blockquote>
            <div className="space-y-2">
              <Label htmlFor="dm-report-reason">举报原因</Label>
              <Select value={reason} onValueChange={(value) => setReason(value as DmReportReason)}>
                <SelectTrigger id="dm-report-reason"><SelectValue /></SelectTrigger>
                <SelectContent>
                  {Object.entries(reasonLabels).map(([value, label]) => (
                    <SelectItem key={value} value={value}>{label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="dm-report-note">补充说明（可选）</Label>
              <Textarea
                id="dm-report-note"
                value={note}
                onChange={(event) => setNote(event.target.value)}
                maxLength={1000}
                rows={4}
                placeholder="提供有助于判断的上下文，请勿填写无关隐私信息。"
              />
              <p className="text-right text-xs tabular-nums text-muted-foreground">{note.length}/1000</p>
            </div>
            {error ? (
              <p role="alert" className="text-sm text-destructive">
                {error instanceof Error ? error.message : "举报提交失败"}
              </p>
            ) : null}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>取消</Button>
            <Button type="submit" variant="destructive" disabled={isPending}>
              {isPending ? "提交中" : "提交举报"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
