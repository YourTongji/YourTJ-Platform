import {
  Award,
  Coins,
  Heart,
  MessageCircle,
  MessageSquare,
  Sparkles,
  Star,
  Trophy,
  Wallet,
} from "lucide-react";
import { Link } from "react-router";

import { ActivityHeatmap } from "@/components/activity/activity-heatmap";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { ActivityCalendar, UserProfile } from "@/lib/api/types";
import { formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

interface ProfileSidebarProps {
  profile: UserProfile;
  isSelf: boolean;
  ariaLabel?: string;
  walletBalance?: number | null;
  walletLoading?: boolean;
  activity?: {
    calendar?: ActivityCalendar;
    isLoading: boolean;
    error?: unknown;
    onRetry: () => void;
  };
}

const achievementRingStyles = [
  "border-primary/35 bg-primary/10 text-primary",
  "border-amber-400/40 bg-amber-50 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300",
  "border-sky-400/40 bg-sky-50 text-sky-700 dark:bg-sky-950/40 dark:text-sky-300",
  "border-violet-400/40 bg-violet-50 text-violet-700 dark:bg-violet-950/40 dark:text-violet-300",
  "border-rose-400/40 bg-rose-50 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300",
  "border-emerald-400/40 bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300",
] as const;

const achievementIcons = [Trophy, Star, Award, Sparkles, Heart, Coins] as const;

function AchievementRing({
  name,
  index,
}: {
  name: string;
  index: number;
}) {
  const Icon = achievementIcons[index % achievementIcons.length];
  const ringStyle = achievementRingStyles[index % achievementRingStyles.length];

  return (
    <div className="flex w-[56px] flex-col items-center gap-1.5" title={name}>
      <div
        className={cn(
          "flex size-12 items-center justify-center rounded-full border-2 shadow-sm",
          ringStyle,
        )}
        aria-hidden="true"
      >
        <Icon className="size-5" strokeWidth={1.75} />
      </div>
      <span className="line-clamp-2 w-full text-center text-[10px] leading-tight text-muted-foreground">
        {name}
      </span>
    </div>
  );
}

export function ProfileSidebar({
  profile,
  isSelf,
  ariaLabel = "个人主页侧栏",
  walletBalance,
  walletLoading = false,
  activity,
}: ProfileSidebarProps) {
  const visibleBadges = profile.badges.slice(0, 6);
  const activityDays = activity?.calendar?.days ?? [];
  const totalScore = activityDays.reduce((sum, day) => sum + day.score, 0);
  const activeDays = activityDays.filter((day) => day.score > 0).length;

  return (
    <aside className="space-y-4" aria-label={ariaLabel}>
      {/* 成就 — Figma: circular medal grid + 查看全部 */}
      <Card className="rounded-2xl border-border/60 shadow-none">
        <CardHeader className="flex-row items-center justify-between space-y-0 p-4 pb-2">
          <CardTitle className="text-sm font-semibold">成就</CardTitle>
          <span className="text-[11px] text-muted-foreground">
            {profile.badges.length > 0 ? `${profile.badges.length} 枚` : "查看全部"}
          </span>
        </CardHeader>
        <CardContent className="p-4 pt-2">
          {visibleBadges.length > 0 ? (
            <div className="flex flex-wrap gap-x-3 gap-y-3" aria-label="用户徽章">
              {visibleBadges.map((badge, index) => (
                <AchievementRing key={badge.slug} name={badge.name} index={index} />
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">尚未获得社区徽章</p>
          )}
        </CardContent>
      </Card>

      {/* 积分钱包 — Figma: only meaningful for self; show public contribution otherwise */}
      {isSelf ? (
        <Card className="rounded-2xl border-border/60 shadow-none">
          <CardHeader className="flex-row items-center justify-between space-y-0 p-4 pb-2">
            <CardTitle className="flex items-center gap-2 text-sm font-semibold">
              <Wallet className="size-4 text-primary" aria-hidden="true" />
              积分钱包
            </CardTitle>
            <Button asChild variant="ghost" size="sm" className="h-7 px-2 text-[11px] text-muted-foreground">
              <Link to="/wallet">查看详情</Link>
            </Button>
          </CardHeader>
          <CardContent className="space-y-3 p-4 pt-1">
            <div>
              <p className="text-[11px] text-muted-foreground">当前余额</p>
              <p className="mt-1 text-[28px] font-bold tabular-nums tracking-tight text-foreground">
                {walletLoading ? "…" : formatNumber(walletBalance ?? 0)}
              </p>
              <p className="mt-1 text-[11px] text-muted-foreground">平台闭环积分 · 不可提现</p>
            </div>
            <div className="grid grid-cols-3 gap-2">
              <div className="rounded-xl border border-border/60 bg-muted/20 px-2 py-2 text-center">
                <p className="text-[10px] text-muted-foreground">主题</p>
                <p className="mt-0.5 text-sm font-semibold tabular-nums">{formatNumber(profile.threadCount)}</p>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/20 px-2 py-2 text-center">
                <p className="text-[10px] text-muted-foreground">回复</p>
                <p className="mt-0.5 text-sm font-semibold tabular-nums">{formatNumber(profile.commentCount)}</p>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/20 px-2 py-2 text-center">
                <p className="text-[10px] text-muted-foreground">获赞</p>
                <p className="mt-0.5 text-sm font-semibold tabular-nums">{formatNumber(profile.votesReceived)}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      ) : (
        <Card className="rounded-2xl border-border/60 shadow-none">
          <CardHeader className="p-4 pb-2">
            <CardTitle className="text-sm font-semibold">社区贡献</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 p-4 pt-1">
            <div className="grid grid-cols-3 gap-2">
              <div className="rounded-xl border border-border/60 bg-muted/20 px-2 py-2.5 text-center">
                <div className="flex items-center justify-center gap-1 text-[10px] text-muted-foreground">
                  <MessageSquare className="size-3" aria-hidden="true" />
                  主题
                </div>
                <p className="mt-1 text-base font-semibold tabular-nums">{formatNumber(profile.threadCount)}</p>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/20 px-2 py-2.5 text-center">
                <div className="flex items-center justify-center gap-1 text-[10px] text-muted-foreground">
                  <MessageCircle className="size-3" aria-hidden="true" />
                  回复
                </div>
                <p className="mt-1 text-base font-semibold tabular-nums">{formatNumber(profile.commentCount)}</p>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/20 px-2 py-2.5 text-center">
                <div className="flex items-center justify-center gap-1 text-[10px] text-muted-foreground">
                  <Heart className="size-3" aria-hidden="true" />
                  获赞
                </div>
                <p className="mt-1 text-base font-semibold tabular-nums">{formatNumber(profile.votesReceived)}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 活跃度 — self only (API is /me/activity) */}
      {isSelf && activity ? (
        <Card className="rounded-2xl border-border/60 shadow-none">
          <CardContent className="space-y-3 p-4">
            <ActivityHeatmap
              isAuthenticated
              calendar={activity.calendar}
              isLoading={activity.isLoading}
              error={activity.error}
              onRetry={activity.onRetry}
            />
            {!activity.isLoading && activity.calendar ? (
              <p className="text-[11px] leading-5 text-muted-foreground">
                近 20 周累计 {formatNumber(totalScore)} 分，有互动 {activeDays} 天。
                继续发帖、回复和点赞可以提升活跃度。
              </p>
            ) : null}
          </CardContent>
        </Card>
      ) : null}
    </aside>
  );
}
