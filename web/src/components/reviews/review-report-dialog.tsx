import { Flag } from "lucide-react";
import * as React from "react";

import { YourTJCaptcha } from "@/components/common/yourtj-captcha";
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

const reportCategories = {
  spam: "垃圾或推广信息",
  abuse: "辱骂或人身攻击",
  misleading: "虚假或误导信息",
  privacy: "泄露隐私",
  off_topic: "与课程无关",
  other: "其他问题",
} as const;

type ReportCategory = keyof typeof reportCategories;

export function ReviewReportDialog({
  reviewAuthor,
  open,
  isPending,
  error,
  onOpenChange,
  onSubmit,
}: {
  reviewAuthor?: string;
  open: boolean;
  isPending: boolean;
  error?: unknown;
  onOpenChange: (open: boolean) => void;
  onSubmit: (reason: string, captchaToken: string) => void;
}) {
  const [category, setCategory] = React.useState<ReportCategory>("spam");
  const [note, setNote] = React.useState("");
  const [pendingReason, setPendingReason] = React.useState("");
  const [captchaOpen, setCaptchaOpen] = React.useState(false);
  const normalizedNote = note.trim();
  const isValid = normalizedNote.length >= 3 && normalizedNote.length <= 450;

  function closeAndReset() {
    onOpenChange(false);
    setCategory("spam");
    setNote("");
    setPendingReason("");
  }

  function continueToCaptcha(event: React.FormEvent) {
    event.preventDefault();
    if (!isValid || isPending) return;
    setPendingReason(`${reportCategories[category]}：${normalizedNote}`);
    onOpenChange(false);
    setCaptchaOpen(true);
  }

  return (
    <>
      <Dialog
        open={open}
        onOpenChange={(nextOpen) => {
          if (!nextOpen && !isPending) closeAndReset();
        }}
      >
        <DialogContent>
          <form onSubmit={continueToCaptcha}>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Flag className="size-5 text-destructive" aria-hidden="true" />
                举报这条课程点评
              </DialogTitle>
              <DialogDescription>
                {reviewAuthor ? `点评作者为 ${reviewAuthor}。` : ""}请选择最符合的类别，并说明需要审核的具体内容。
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-5">
              <div className="space-y-2">
                <Label htmlFor="review-report-category">举报类别</Label>
                <Select value={category} onValueChange={(value) => setCategory(value as ReportCategory)}>
                  <SelectTrigger id="review-report-category"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {Object.entries(reportCategories).map(([value, label]) => (
                      <SelectItem key={value} value={value}>{label}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="review-report-note">具体说明</Label>
                <Textarea
                  id="review-report-note"
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                  rows={5}
                  minLength={3}
                  maxLength={450}
                  placeholder="请指出具体问题和判断依据，不要填写无关个人信息。"
                  aria-describedby={error ? "review-report-error" : "review-report-help"}
                />
                <div className="flex items-start justify-between gap-3 text-xs">
                  {error ? (
                    <p id="review-report-error" role="alert" className="text-destructive">
                      {error instanceof Error ? error.message : "举报提交失败"}
                    </p>
                  ) : (
                    <p id="review-report-help" className="text-muted-foreground">至少 3 个字符；提交前还需完成人机验证。</p>
                  )}
                  <span className="shrink-0 tabular-nums text-muted-foreground">{note.length}/450</span>
                </div>
              </div>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={closeAndReset} disabled={isPending}>取消</Button>
              <Button type="submit" disabled={!isValid || isPending}>继续人机验证</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <YourTJCaptcha
        open={captchaOpen}
        onOpenChange={setCaptchaOpen}
        onVerified={(captchaToken) => {
          setCaptchaOpen(false);
          onSubmit(pendingReason, captchaToken);
          setCategory("spam");
          setNote("");
          setPendingReason("");
        }}
      />
    </>
  );
}
