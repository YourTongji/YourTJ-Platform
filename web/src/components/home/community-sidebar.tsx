import { BellRing, CalendarCheck2, Flame, Leaf, RefreshCw, TrendingUp } from "lucide-react";
import { Link } from "react-router";

import { TeaBadge } from "@/components/common/tea-badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import type { Account, Announcement, ThreadFeed } from "@/lib/api/types";
import { formatDate, formatNumber } from "@/lib/format";

function MissionCard({ account }: { account: Account | null }) {
  const level = Math.max(0, Math.min(account?.trustLevel ?? 0, 6));
  const progress = (level / 6) * 100;
  const reachedCells = Math.round((level / 6) * 140);

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
          <p className="text-sm font-semibold">等级任务</p>
          <TeaBadge level={level} />
        </div>

        <div className="mt-5 flex justify-center">
          <div className="flex size-16 items-center justify-center rounded-lg border border-input bg-background text-primary">
            <Leaf className="size-9" strokeWidth={1.5} />
          </div>
        </div>

        <div className="mt-3 flex items-center justify-between gap-2 text-[10px] text-muted-foreground">
          <span>距离 Lv.{Math.min(level + 1, 6)} 进度</span>
          <span className="font-medium text-primary">{level} / 6</span>
        </div>
        <Progress className="mt-2 h-1.5" value={progress} />

        <Button asChild variant="outline" size="sm" className="mt-4 w-full bg-transparent">
          <Link to={account?.handle ? `/profile/${account.handle}` : "/login"}>
            {account ? "查看个人成长" : "了解社区等级"}
          </Link>
        </Button>

        <div className="my-5 border-t border-border/70" />

        <div className="flex items-center justify-between text-xs">
          <span className="font-medium">等级成长</span>
          <span className="text-[10px] text-muted-foreground">当前 Lv.{level}</span>
        </div>
        <div className="mt-3 grid grid-cols-20 gap-0.5" aria-label={`等级成长进度 ${level} / 6`}>
          {Array.from({ length: 140 }, (_, index) => {
            const isReached = index < reachedCells;
            const intensity = index % 4;
            return (
              <span
                key={index}
                className={
                  isReached
                    ? intensity === 0
                      ? "aspect-square rounded-[1px] bg-primary/35"
                      : intensity === 1
                        ? "aspect-square rounded-[1px] bg-primary/55"
                        : intensity === 2
                          ? "aspect-square rounded-[1px] bg-primary/75"
                          : "aspect-square rounded-[1px] bg-primary"
                    : "aspect-square rounded-[1px] bg-muted"
                }
              />
            );
          })}
        </div>
        <div className="mt-2 flex items-center justify-end gap-1.5 text-[9px] text-muted-foreground">
          <span>起步</span>
          <span className="size-2.5 rounded-[1px] bg-muted" />
          <span className="size-2.5 rounded-[1px] bg-primary/35" />
          <span className="size-2.5 rounded-[1px] bg-primary/55" />
          <span className="size-2.5 rounded-[1px] bg-primary/75" />
          <span className="size-2.5 rounded-[1px] bg-primary" />
          <span>进阶</span>
        </div>
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
          <div key={announcement.id} className="border-l-2 border-primary pl-3">
            <p className="line-clamp-1 text-xs font-medium text-[#3d4947] dark:text-foreground">{announcement.title}</p>
            <p className="mt-1 text-[10px] text-[#9ca3af]">{formatDate(announcement.createdAt)}</p>
          </div>
        ))}
        {announcements.length === 0 ? <p className="text-xs text-[#9ca3af]">暂无校园公告</p> : null}
      </CardContent>
    </Card>
  );
}

export function CommunitySidebar({
  account,
  threads,
  announcements,
}: {
  account: Account | null;
  threads: ThreadFeed[];
  announcements: Announcement[];
}) {
  return (
    <aside className="space-y-6">
      <MissionCard account={account} />
      <HotTopicsCard threads={threads} />
      <NoticeCard announcements={announcements} />
    </aside>
  );
}
