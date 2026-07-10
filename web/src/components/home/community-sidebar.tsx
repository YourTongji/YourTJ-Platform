import { BellRing, Flame, HandHeart, Leaf, RefreshCw, TrendingUp } from "lucide-react";
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

  return (
    <Card className="h-[180px] rounded-xl">
      <CardHeader className="flex-row items-center justify-between p-4 pb-0">
        <CardTitle className="text-sm">等级任务</CardTitle>
        <TeaBadge level={level} />
      </CardHeader>
      <CardContent className="p-4">
        <div className="flex items-center gap-4">
          <div className="flex size-16 shrink-0 items-center justify-center rounded-lg border bg-[#f9fafb] text-primary dark:bg-secondary">
            <Leaf className="size-9" strokeWidth={1.5} />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center justify-between gap-2 text-[10px] text-[#6b7280]">
              <span>社区成长进度</span>
              <span>{level} / 6</span>
            </div>
            <Progress className="mt-2 h-1.5" value={progress} />
          </div>
        </div>
        <Button asChild variant="outline" size="sm" className="mt-4 w-full bg-[#f9fafb] dark:bg-background">
          <Link to={account?.handle ? `/profile/${account.handle}` : "/login"}>
            {account ? "查看日常任务" : "登录查看任务"}
          </Link>
        </Button>
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
            <span className="truncate text-[#374151] dark:text-foreground">{thread.title || "未命名讨论"}</span>
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
    <Card className="rounded-xl">
      <CardHeader className="p-4 pb-0">
        <CardTitle className="flex items-center gap-2 text-sm">
          <BellRing className="size-4 text-primary" />
          校园公告
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 p-4">
        {announcements.slice(0, 2).map((announcement) => (
          <div key={announcement.id} className="border-l-2 border-primary pl-3">
            <p className="line-clamp-1 text-xs font-medium text-[#374151] dark:text-foreground">{announcement.title}</p>
            <p className="mt-1 text-[10px] text-[#9ca3af]">{formatDate(announcement.createdAt)}</p>
          </div>
        ))}
        {announcements.length === 0 ? <p className="text-xs text-[#9ca3af]">暂无校园公告</p> : null}
      </CardContent>
    </Card>
  );
}

function DonationCard() {
  return (
    <Card className="h-[165px] rounded-xl border-primary/20 bg-[#f0fdfa] dark:bg-secondary/40">
      <CardHeader className="flex-row items-center gap-3 p-4 pb-0">
        <span className="flex size-8 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <HandHeart className="size-5" />
        </span>
        <CardTitle className="text-sm">捐赠入口</CardTitle>
      </CardHeader>
      <CardContent className="p-4 pt-3">
        <p className="text-xs leading-5 text-[#4b5563] dark:text-muted-foreground">
          YourTJ 是一个非营利社区，捐赠将直接用于服务器托管等运营费用。
        </p>
        <Button asChild size="sm" className="mt-3 w-full">
          <a href="mailto:support@yourtj.de?subject=支持 YourTJ">支持我们</a>
        </Button>
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
      <DonationCard />
    </aside>
  );
}
