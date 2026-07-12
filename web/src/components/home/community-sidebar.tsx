import { BellRing, CalendarCheck2, Flame, Leaf, RefreshCw, TrendingUp } from "lucide-react";
import { Link } from "react-router";

import { ActivityHeatmap } from "@/components/activity/activity-heatmap";
import { TeaBadge } from "@/components/common/tea-badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import type { Account, ActivityCalendar, Announcement, ThreadFeed, TrustProgress } from "@/lib/api/types";
import { formatDate, formatNumber } from "@/lib/format";

interface ActivityState {
  calendar?: ActivityCalendar;
  isLoading: boolean;
  error?: unknown;
  onRetry: () => void;
}

function MissionCard({
  account,
  activity,
  trustProgress,
}: {
  account: Account | null;
  activity: ActivityState;
  trustProgress: TrustProgress | null;
}) {
  const level = trustProgress?.trustLevel ?? account?.trustLevel ?? 0;
  const progressPercent = trustProgress?.progressPercent ?? 0;
  const isMaxLevel = trustProgress?.isMaxLevel ?? false;
  const remainingScore = trustProgress?.remainingScore;
  const nextLevel = trustProgress?.nextLevel;
  const overrideActive = trustProgress?.overrideActive ?? false;

  return (
    <Card className="min-h-[452px] rounded-xl">
      <CardContent className="p-4">
        <Button asChild className="h-10 w-full rounded-lg">
          <Link to={account?.handle ? "/wallet" : "/login"}>
            <CalendarCheck2 className="size-4" />
            {account ? "查看等级任务" : "登录查看任务"}
          </Link>
        </Button>

        <div className="mt-4 flex items-center justify-between">
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
          ) : remainingScore != null && nextLevel != null ? (
            <span>距离 Lv.{nextLevel} 还需 {remainingScore} 分</span>
          ) : (
            <span>加载等级进度中</span>
          )}
          <span className="font-medium text-primary">
            {trustProgress ? `${trustProgress.qualifyingScore} 分` : `${level} / 6`}
          </span>
        </div>
        <Progress className="mt-2 h-1.5" value={progressPercent} />

        <Button asChild variant="outline" size="sm" className="mt-4 w-full bg-transparent">
          <Link to={account?.handle ? `/profile/${account.handle}` : "/login"}>
            {account ? "查看个人成长" : "了解社区等级"}
          </Link>
        </Button>

        {overrideActive && trustProgress?.overrideReason ? (
          <p className="mt-2 text-[10px] text-amber-600 dark:text-amber-400">
            等级已锁定：{trustProgress.overrideReason}
          </p>
        ) : null}

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
  threads,
  announcements,
  trustProgress,
}: {
  account: Account | null;
  activity: ActivityState;
  threads: ThreadFeed[];
  announcements: Announcement[];
  trustProgress: TrustProgress | null;
}) {
  return (
    <aside className="space-y-6">
      <MissionCard account={account} activity={activity} trustProgress={trustProgress} />
      <HotTopicsCard threads={threads} />
      <NoticeCard announcements={announcements} />
    </aside>
  );
}
