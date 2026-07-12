import * as React from "react";
import { ChevronLeft } from "lucide-react";
import { Link } from "react-router";

import { getTwentyWeekActivityRange } from "@/components/activity/calendar-range";
import { ProfilePostCard } from "@/components/profile/profile-post-card";
import { ProfileSidebar } from "@/components/profile/profile-sidebar";
import { ProfileSummary } from "@/components/profile/profile-summary";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { ActivityCalendar, UserProfile } from "@/lib/api/types";

const sampleProfile: UserProfile = {
  id: "preview-1",
  handle: "walkerhiller",
  displayName: "walkerhiller",
  bio: "喜欢看二次元和人机对话",
  website: "https://github.com/walkerhiller",
  avatarUrl: null,
  bannerUrl: null,
  role: "user",
  trustLevel: 3,
  badges: [
    { slug: "first-thread", name: "首次发帖" },
    { slug: "helpful", name: "乐于助人" },
    { slug: "early", name: "早期用户" },
    { slug: "contributor", name: "贡献者" },
  ],
  verifications: [],
  threadCount: 12,
  commentCount: 48,
  votesReceived: 326,
  followerCount: 914,
  followingCount: 114,
  canViewActivity: true,
  createdAt: 1_752_192_000,
};

const sampleThreads = [
  {
    id: "1",
    title: "给基米",
    body: "这是一条帖子",
    boardSlug: "校园生活",
    createdAtLabel: "12小时前",
    replyCount: 24,
    voteCount: 5,
    viewCount: 142,
    heatCount: 1300,
  },
  {
    id: "2",
    title: "代码片段分享",
    body: "这是一条很长的帖子内容预览，用来对照 Figma 信息流卡片的密度与排版。",
    boardSlug: "校园生活",
    createdAtLabel: "6小时前",
    replyCount: 8,
    voteCount: 16,
    viewCount: 386,
    heatCount: 920,
  },
];

function buildSampleActivity(): ActivityCalendar {
  const rangeHint = getTwentyWeekActivityRange();
  const days = Array.from({ length: 20 * 7 }, (_, index) => {
    const base = Date.parse(`${rangeHint.from}T00:00:00Z`);
    const date = new Date(base + index * 86_400_000).toISOString().slice(0, 10);
    const score = (index * 7 + 3) % 11 === 0 ? 0 : ((index * 3) % 5) + ((index % 4 === 0) ? 3 : 0);
    return {
      date,
      score,
      threads: score > 0 ? score % 3 : 0,
      comments: score > 0 ? score % 4 : 0,
      likes: score > 0 ? score % 5 : 0,
    };
  });
  return {
    timezone: "Asia/Shanghai",
    from: rangeHint.from,
    to: rangeHint.to,
    policyVersion: 1,
    weights: { thread: 3, comment: 2, like: 1 },
    days,
  };
}

/**
 * 本地预览页：复用真实个人主页组件，不依赖后端。
 * 路由：/dev/profile-preview
 */
export function ProfilePreviewPage() {
  const activity = React.useMemo(() => buildSampleActivity(), []);
  const [tab, setTab] = React.useState("threads");

  return (
    <div className="min-[1240px]:grid min-[1240px]:grid-cols-[minmax(0,640px)_320px]">
      <div className="space-y-4 px-4 py-5 sm:px-6 sm:py-6 min-[1360px]:!px-8">
        <Link
          to="/"
          className="inline-flex items-center gap-1 text-[13px] text-muted-foreground transition-colors hover:text-primary"
        >
          <ChevronLeft className="size-4" aria-hidden="true" />
          返回首页
        </Link>

        <ProfileSummary
          profile={sampleProfile}
          isAuthenticated
          isSelf
          relationshipLoading={false}
          relationshipPending={false}
          messagePending={false}
          canStartConversation={false}
          canManageUser={false}
          canManageVerifications={false}
          confirmBlockOpen={false}
          onConfirmBlockOpenChange={() => undefined}
          onStartConversation={() => undefined}
          onToggleFollow={() => undefined}
          onToggleMute={() => undefined}
          onToggleBlock={() => undefined}
          onOpenRelationshipList={() => undefined}
        />

        <div className="min-[1240px]:hidden">
          <ProfileSidebar
            profile={sampleProfile}
            isSelf
            ariaLabel="个人主页预览侧栏（窄屏）"
            walletBalance={1_919_810}
            activity={{
              calendar: activity,
              isLoading: false,
              onRetry: () => undefined,
            }}
          />
        </div>

        <section aria-label="用户动态">
          <Tabs value={tab} onValueChange={setTab} className="gap-0">
            <div className="mb-4 flex h-10 items-center border-b border-border/50">
              <TabsList className="h-auto min-w-0 flex-1 justify-start gap-0 rounded-none bg-transparent p-0">
                {(
                  [
                    ["threads", "帖子"],
                    ["comments", "回复"],
                    ["bookmarks", "收藏"],
                    ["media", "媒体"],
                    ["likes", "喜欢"],
                  ] as const
                ).map(([value, label]) => (
                  <TabsTrigger
                    key={value}
                    value={value}
                    className="h-10 flex-1 rounded-none border-b-2 border-transparent px-1 pb-3 pt-0 text-sm font-medium text-muted-foreground shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:shadow-none sm:px-2"
                  >
                    {label}
                  </TabsTrigger>
                ))}
              </TabsList>
            </div>

            <TabsContent value="threads" className="space-y-3">
              {sampleThreads.map((thread) => (
                <ProfilePostCard
                  key={thread.id}
                  authorName={sampleProfile.displayName || sampleProfile.handle}
                  authorHandle={sampleProfile.handle}
                  authorAvatarUrl={sampleProfile.avatarUrl}
                  trustLevel={sampleProfile.trustLevel}
                  post={{
                    id: thread.id,
                    title: thread.title,
                    body: thread.body,
                    boardSlug: thread.boardSlug,
                    createdAtLabel: thread.createdAtLabel,
                    replyCount: thread.replyCount,
                    voteCount: thread.voteCount,
                    viewCount: thread.viewCount,
                    heatCount: thread.heatCount,
                    href: "#",
                  }}
                />
              ))}
            </TabsContent>

            <TabsContent value="comments">
              <Card className="rounded-2xl border-dashed border-border/60 shadow-none">
                <CardContent className="p-6 text-sm text-muted-foreground">预览模式：回复列表可在真实个人主页查看。</CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="bookmarks">
              <Card className="rounded-2xl border-dashed border-border/60 shadow-none">
                <CardContent className="p-6 text-sm text-muted-foreground">收藏会显示在这里。</CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="media">
              <Card className="rounded-2xl border-dashed border-border/60 shadow-none">
                <CardContent className="p-6 text-sm text-muted-foreground">媒体内容即将开放。</CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="likes">
              <Card className="rounded-2xl border-dashed border-border/60 shadow-none">
                <CardContent className="p-6 text-sm text-muted-foreground">喜欢列表即将开放。</CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </section>
      </div>

      <div className="hidden pb-16 pl-6 pt-6 min-[1240px]:block">
        <ProfileSidebar
          profile={sampleProfile}
          isSelf
          ariaLabel="个人主页预览侧栏（宽屏）"
          walletBalance={1_919_810}
          activity={{
            calendar: activity,
            isLoading: false,
            onRetry: () => undefined,
          }}
        />
      </div>
    </div>
  );
}
