import { Copy, Download, Flame, Leaf } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { CheckInStatus, TrustProgress } from "@/lib/api/types";

export interface CheckInShareData {
  date: string;
  currentStreak: number;
  totalDays: number;
  trustLevel: number | null;
  teaName: string | null;
  progressPercent: number | null;
}

function boundedInteger(value: number, maximum: number) {
  if (!Number.isFinite(value)) return 0;
  return Math.min(maximum, Math.max(0, Math.floor(value)));
}

/** Builds the explicit export allowlist; account, device, URL, and credential fields cannot enter it. */
export function buildCheckInShareData(
  status: CheckInStatus,
  trustProgress: TrustProgress | null,
  fallbackTrustLevel?: number,
): CheckInShareData {
  const trustLevel = trustProgress?.trustLevel ?? fallbackTrustLevel;
  return {
    date: /^\d{4}-\d{2}-\d{2}$/.test(status.date) ? status.date : "今日",
    currentStreak: boundedInteger(status.currentStreak, 100_000),
    totalDays: boundedInteger(status.totalDays, 100_000),
    trustLevel: trustLevel == null ? null : boundedInteger(trustLevel, 6),
    teaName: trustProgress?.teaName.trim().slice(0, 24) || null,
    progressPercent: trustProgress == null
      ? null
      : boundedInteger(trustProgress.progressPercent, 100),
  };
}

export function createCheckInShareText(data: CheckInShareData) {
  const growth = data.trustLevel == null
    ? ""
    : `\n当前成长：Lv.${data.trustLevel}${data.teaName ? ` · ${data.teaName}` : ""}`;
  return `我在 YourTJ 完成了 ${data.date} 每日签到！\n连续 ${data.currentStreak} 天 · 累计 ${data.totalDays} 天${growth}`;
}

function escapeXml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

export function createCheckInShareSvg(data: CheckInShareData) {
  const growth = data.trustLevel == null
    ? "继续在校园社区留下有价值的贡献"
    : `Lv.${data.trustLevel}${data.teaName ? ` · ${data.teaName}` : ""}${data.progressPercent == null ? "" : ` · ${data.progressPercent}%`}`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1080" viewBox="0 0 1080 1080">
  <defs><linearGradient id="background" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#ecfdf5"/><stop offset="1" stop-color="#dbeafe"/></linearGradient></defs>
  <rect width="1080" height="1080" rx="72" fill="url(#background)"/>
  <circle cx="540" cy="250" r="112" fill="#0f766e"/>
  <path d="M492 260c64-94 154-78 154-78-2 90-62 154-154 154 30-26 62-58 91-100-43 29-71 47-91 24Z" fill="#f0fdfa"/>
  <text x="540" y="435" text-anchor="middle" font-family="system-ui, sans-serif" font-size="44" fill="#115e59">${escapeXml(data.date)} · 每日签到</text>
  <text x="540" y="560" text-anchor="middle" font-family="system-ui, sans-serif" font-size="86" font-weight="700" fill="#0f172a">连续 ${data.currentStreak} 天</text>
  <text x="540" y="650" text-anchor="middle" font-family="system-ui, sans-serif" font-size="44" fill="#334155">累计签到 ${data.totalDays} 天</text>
  <rect x="150" y="730" width="780" height="112" rx="32" fill="#ffffff" fill-opacity="0.72"/>
  <text x="540" y="800" text-anchor="middle" font-family="system-ui, sans-serif" font-size="34" fill="#334155">${escapeXml(growth)}</text>
  <text x="540" y="960" text-anchor="middle" font-family="system-ui, sans-serif" font-size="32" font-weight="600" fill="#0f766e">YourTJ 校园社区</text>
</svg>`;
}

function downloadShareImage(data: CheckInShareData) {
  const blob = new Blob([createCheckInShareSvg(data)], { type: "image/svg+xml;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `yourtj-check-in-${data.date === "今日" ? "today" : data.date}.svg`;
  link.click();
  URL.revokeObjectURL(url);
}

interface CheckInShareDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  status?: CheckInStatus;
  trustProgress: TrustProgress | null;
  fallbackTrustLevel?: number;
}

export function CheckInShareDialog({
  open,
  onOpenChange,
  status,
  trustProgress,
  fallbackTrustLevel,
}: CheckInShareDialogProps) {
  if (!status?.checkedIn) return null;
  const data = buildCheckInShareData(status, trustProgress, fallbackTrustLevel);

  const copy = async () => {
    if (!navigator.clipboard?.writeText) {
      toast.error("当前浏览器无法复制签到文案");
      return;
    }
    try {
      await navigator.clipboard.writeText(createCheckInShareText(data));
      toast.success("签到文案已复制");
    } catch {
      toast.error("复制失败，请稍后重试");
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>今日签到完成</DialogTitle>
          <DialogDescription>分享卡只包含日期、签到天数和成长等级，不包含账号或设备信息。</DialogDescription>
        </DialogHeader>
        <div
          data-testid="check-in-share-card"
          className="overflow-hidden rounded-2xl border bg-gradient-to-br from-emerald-50 to-blue-50 p-6 text-slate-900"
        >
          <div className="mx-auto flex size-16 items-center justify-center rounded-full bg-teal-700 text-white">
            <Leaf className="size-9" />
          </div>
          <p className="mt-4 text-center text-sm font-medium text-teal-800">{data.date} · 每日签到</p>
          <p className="mt-2 text-center text-3xl font-bold">连续 {data.currentStreak} 天</p>
          <p className="mt-2 text-center text-sm text-slate-600">累计签到 {data.totalDays} 天</p>
          {data.trustLevel == null ? null : (
            <div className="mt-5 flex items-center justify-center gap-2 rounded-xl bg-white/70 p-3 text-sm">
              <Flame className="size-4 text-amber-600" />
              Lv.{data.trustLevel}{data.teaName ? ` · ${data.teaName}` : ""}
              {data.progressPercent == null ? null : ` · ${data.progressPercent}%`}
            </div>
          )}
          <p className="mt-5 text-center text-sm font-semibold text-teal-700">YourTJ 校园社区</p>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => void copy()}>
            <Copy className="size-4" />
            复制文案
          </Button>
          <Button type="button" onClick={() => downloadShareImage(data)}>
            <Download className="size-4" />
            保存分享图
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
