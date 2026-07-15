import { CalendarCheck2, LoaderCircle, RotateCcw, Share2 } from "lucide-react";
import { Link } from "react-router";

import { Button } from "@/components/ui/button";
import type { Account, CheckInStatus } from "@/lib/api/types";

interface DailyCheckInButtonProps {
  account: Account | null;
  status?: CheckInStatus;
  isLoading: boolean;
  isPending: boolean;
  error?: unknown;
  onCheckIn: () => void;
  onRetry: () => void;
  onShare?: () => void;
}

export function DailyCheckInButton({
  account,
  status,
  isLoading,
  isPending,
  error,
  onCheckIn,
  onRetry,
  onShare,
}: DailyCheckInButtonProps) {
  if (!account) {
    return (
      <Button asChild className="h-10 w-full rounded-lg">
        <Link to="/login">
          <CalendarCheck2 className="size-4" />
          登录后每日签到
        </Link>
      </Button>
    );
  }

  if (error) {
    return (
      <Button type="button" variant="outline" className="h-10 w-full rounded-lg" onClick={onRetry}>
        <RotateCcw className="size-4" />
        签到状态加载失败，重试
      </Button>
    );
  }

  if (isLoading || !status) {
    return (
      <Button type="button" className="h-10 w-full rounded-lg" disabled>
        <LoaderCircle className="size-4 animate-spin" />
        正在加载签到状态
      </Button>
    );
  }

  return (
    <Button
      type="button"
      className="h-10 w-full rounded-lg"
      variant={status.checkedIn ? "secondary" : "default"}
      disabled={isPending || (status.checkedIn && !onShare)}
      onClick={status.checkedIn ? onShare : onCheckIn}
    >
      {isPending ? (
        <LoaderCircle className="size-4 animate-spin" />
      ) : (
        status.checkedIn ? <Share2 className="size-4" /> : <CalendarCheck2 className="size-4" />
      )}
      {status.checkedIn
        ? `今日已签到 · 连续 ${status.currentStreak} 天 · 分享`
        : `每日签到 · 累计 ${status.totalDays} 天`}
    </Button>
  );
}
