import { BellRing, Flame, Leaf, RefreshCw, TrendingUp } from "lucide-react";
import { Link } from "react-router";

import { ActivityHeatmap } from "@/components/activity/activity-heatmap";
import { DailyCheckInButton } from "@/components/activity/daily-check-in-button";
import { TeaBadge } from "@/components/common/tea-badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import type { Account, ActivityCalendar, Announcement, CheckInStatus, ThreadFeed, TrustProgress } from "@/lib/api/types";
import { formatDate, formatNumber } from "@/lib/format";

interface ActivityState {
  calendar?: ActivityCalendar;
  isLoading: boolean;
  error?: unknown;
  onRetry: () => void;
}

interface CheckInState {
  status?: CheckInStatus;
  isLoading: boolean;
  isPending: boolean;
  error?: unknown;
  onCheckIn: () => void;
  onRetry: () => void;
  onShare: () => void;
}

interface TrustProgressSummaryProps {
  account: Account | null;
  trustProgress: TrustProgress | null;
  trustError?: unknown;
  onTrustRetry: () => void;
}

export function TrustProgressSummary({
  account,
  trustProgress,
  trustError,
  onTrustRetry,
}: TrustProgressSummaryProps) {
  const level = trustProgress?.trustLevel ?? account?.trustLevel ?? 0;
  const progressPercent = trustProgress?.progressPercent ?? 0;
  const isMaxLevel = trustProgress?.isMaxLevel ?? false;
  const remainingScore = trustProgress?.remainingScore;
  const nextLevel = trustProgress?.nextLevel;
  const overrideActive = trustProgress?.overrideActive ?? false;
  const promotionBlockedUntil = trustProgress?.promotionBlockedUntil;
  const promotionBlocked =
    promotionBlockedUntil != null && promotionBlockedUntil > Math.floor(Date.now() / 1000);

  return (
    <section className="mt-4" aria-label="信任等级进度">
      <div className="flex items-center justify-between">
        <p className="text-sm font-semibold">信任等级</p>
        <TeaBadge level={level} />
      </div>

      <div className="mt-5 flex justify-center">
        <div className="flex size-16 items-center justify-center rounded-lg border border-input bg-background text-primary">
          <Leaf className="size-9" strokeWidth={1.5} />
        </div>
      </div>

      <div className="mt-3 flex items-center justify-between gap-2 text-[10px] text-muted-foreground">
        {isMaxLevel ? (
          <span>已达满级 · {trustProgress?.teaName ?? ""}</span>
        ) : promotionBlocked && promotionBlockedUntil != null ? (
          <span>治理降级冷却至 {formatDate(promotionBlockedUntil)}</span>
        ) : trustProgress?.promotionRequiresNewActivity ? (
          <span>完成新的有效社区贡献后可继续升级</span>
        ) : remainingScore === 0 ? (
          <span>已满足升级条件，等待每日评估</span>
        ) : remainingScore != null && nextLevel != null ? (
          <span>距离 Lv.{nextLevel} 还需 {remainingScore} 分</span>
        ) : trustError ? (
          <span className="text-destructive">等级进度加载失败</span>
        ) : (
          <span>加载等级进度中</span>
        )}
        <span className="font-medium text-primary">
          {trustProgress ? `${trustProgress.qualifyingScore} 分` : `${level} / 6`}
        </span>
      </div>
      <Progress className="mt-2 h-1.5" value={progressPercent} />

      {trustError ? (
        <Button
          type="button"
          variant="link"
          size="sm"
          className="mt-1 h-auto p-0 text-xs"
          onClick={onTrustRetry}
        >
          重试等级进度
        </Button>
      ) : null}

      <Button asChild variant="outline" size="sm" className="mt-4 w-full bg-transparent">
        <Link to={account?.handle ? `/profile/${account.handle}` : "/login"}>
          {account ? "查看个人成长" : "了解社区等级"}
        </Link>
      </Button>

      {overrideActive ? (
        <p className="mt-2 text-[10px] text-amber-600 dark:text-amber-400">等级当前由管理员锁定。</p>
      ) : null}
    </section>
  );
}

function MissionCard({
  account,
  activity,
  checkIn,
  trustProgress,
  trustError,
  onTrustRetry,
}: {
  account: Account | null;
  activity: ActivityState;
  checkIn: CheckInState;
  trustProgress: TrustProgress | null;
  trustError?: unknown;
  onTrustRetry: () => void;
}) {
  return (
    <Card className="min-h-[452px] rounded-xl">
      <CardContent className="p-4">
        <DailyCheckInButton account={account} {...checkIn} />
        <TrustProgressSummary
          account={account}
          trustProgress={trustProgress}
          trustError={trustError}
          onTrustRetry={onTrustRetry}
        />

        <div className="my-5 border-t border-border/70" />

        <ActivityHeatmap
          isAuthenticated={Boolean(account)}
          calendar={activity.calendar}
          isLoading={activity.isLoading}
          error={activity.error}
          onRetry={activity.onRetry}
        />
      </CardContent>
    </Card>
  );
}

function HotTopicsCard({ threads }: { threads: ThreadFeed[] }) {
  return (
    <Card className="h-[218px] rounded-xl">
      <CardHeader className="flex-row items-center justify-between p-4 pb-0">
        <CardTitle className="flex items-center gap-2 text-sm">
          <TrendingUp className="size-4 text-primary" />
          今日热榜
        </CardTitle>
        <RefreshCw className="size-3.5 text-[#6b7280]" />
      </CardHeader>
      <CardContent className="space-y-3 p-4">
        {threads.slice(0, 5).map((thread, index) => (
          <Link
            key={thread.id ?? `${thread.title}-${index}`}
            to={thread.id ? `/forum/threads/${thread.id}` : "/forum"}
            className="grid grid-cols-[16px_minmax(0,1fr)_auto] items-center gap-3 text-xs transition-colors hover:text-primary"
          >
            <span className={index < 3 ? "font-semibold text-primary" : "text-[#9ca3af]"}>{index + 1}</span>
            <span className="truncate text-[#3d4947] dark:text-foreground">{thread.title || "未命名讨论"}</span>
            <span className="inline-flex items-center gap-1 text-[10px] text-primary">
              <Flame className="size-2.5" />
              {formatNumber((thread.voteCount ?? 0) + (thread.replyCount ?? 0))}
            </span>
          </Link>
        ))}
        {threads.length === 0 ? <p className="text-xs text-[#9ca3af]">热榜正在生成中</p> : null}
      </CardContent>
    </Card>
  );
}

function NoticeCard({ announcements }: { announcements: Announcement[] }) {
  return (
    <Card className="min-h-[156px] rounded-xl">
      <CardHeader className="p-4 pb-0">
        <CardTitle className="flex items-center gap-2 text-sm">
          <BellRing className="size-4 text-primary" />
          校园公告
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 p-4">
        {announcements.slice(0, 2).map((announcement) => (
          <Link key={announcement.id} to="/announcements" className="block border-l-2 border-primary pl-3 hover:text-primary">
            <p className="line-clamp-1 text-xs font-medium text-[#3d4947] dark:text-foreground">{announcement.title}</p>
            <p className="mt-1 text-[10px] text-[#9ca3af]">版本 {announcement.revision} · {formatDate(announcement.createdAt)}</p>
          </Link>
        ))}
        {announcements.length === 0 ? <p className="text-xs text-[#9ca3af]">暂无校园公告</p> : null}
      </CardContent>
    </Card>
  );
}

export function CommunitySidebar({
  account,
  activity,
  checkIn,
  threads,
  announcements,
  trustProgress,
  trustError,
  onTrustRetry,
}: {
  account: Account | null;
  activity: ActivityState;
  checkIn: CheckInState;
  threads: ThreadFeed[];
  announcements: Announcement[];
  trustProgress: TrustProgress | null;
  trustError?: unknown;
  onTrustRetry: () => void;
}) {
  return (
    <aside className="space-y-6">
      <MissionCard
        account={account}
        activity={activity}
        checkIn={checkIn}
        trustProgress={trustProgress}
        trustError={trustError}
        onTrustRetry={onTrustRetry}
      />
      <HotTopicsCard threads={threads} />
      <NoticeCard announcements={announcements} />
    </aside>
  );
}
