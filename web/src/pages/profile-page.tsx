import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronLeft, Loader2 } from "lucide-react";
import * as React from "react";
import { Link, useNavigate, useParams } from "react-router";
import { toast } from "sonner";

import {
  ADMIN_CAPABILITIES,
  capabilitiesForAccount,
  hasCapability,
} from "@/components/admin/capabilities";
import { getTwentyWeekActivityRange } from "@/components/activity/calendar-range";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import {
  ProfileRelationshipListDialog,
  type ProfileRelationshipListKind,
} from "@/components/profile/profile-relationship-list-dialog";
import { ProfilePostCard } from "@/components/profile/profile-post-card";
import { ProfileSidebar } from "@/components/profile/profile-sidebar";
import { ProfileSummary } from "@/components/profile/profile-summary";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAuth } from "@/context/auth-provider";
import { accountQueryKeys } from "@/lib/account-query-keys";
import { api } from "@/lib/api/endpoints";
import { formatRelativeTime } from "@/lib/format";

type ProfileActivityTab = "threads" | "comments" | "bookmarks" | "media" | "likes";

export function ProfilePage() {
  const { handle } = useParams();
  const name = handle ?? "";
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { account, isAuthenticated } = useAuth();
  const [confirmBlockOpen, setConfirmBlockOpen] = React.useState(false);
  const [relationshipList, setRelationshipList] = React.useState<ProfileRelationshipListKind | null>(null);
  const [activityTab, setActivityTab] = React.useState<ProfileActivityTab>("threads");
  const capabilities = React.useMemo(() => capabilitiesForAccount(account), [account]);
  const viewerCacheKey = account?.id ?? "anonymous";
  const activityRange = React.useMemo(() => getTwentyWeekActivityRange(), []);

  const profile = useQuery({
    queryKey: ["profile", name, "viewer", viewerCacheKey],
    queryFn: () => api.publicUser(name),
    enabled: Boolean(name),
  });
  const threads = useInfiniteQuery({
    queryKey: ["profile", name, "threads", viewerCacheKey],
    queryFn: ({ pageParam }) => api.userThreads(name, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: Boolean(name) && profile.data?.canViewActivity === true,
  });
  const comments = useInfiniteQuery({
    queryKey: ["profile", name, "comments", viewerCacheKey],
    queryFn: ({ pageParam }) => api.userComments(name, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: Boolean(name) && profile.data?.canViewActivity === true,
  });
  const isSelf = Boolean(
    profile.data && account && (
      profile.data.id === account.id
      || profile.data.handle.toLowerCase() === account.handle?.toLowerCase()
    ),
  );
  const socialRelationship = useQuery({
    queryKey: ["profile", name, "relationship", viewerCacheKey],
    queryFn: () => api.userRelationship(profile.data?.handle ?? name),
    enabled: isAuthenticated && Boolean(profile.data) && !isSelf,
  });
  const activity = useQuery({
    queryKey: ["profile", name, "activity", account?.id, activityRange.from, activityRange.to],
    queryFn: () => api.myActivity(activityRange.from, activityRange.to),
    enabled: Boolean(account) && isSelf,
  });
  const wallet = useQuery({
    queryKey: ["profile", name, "wallet", account?.id],
    queryFn: () => api.wallet(),
    enabled: Boolean(account) && isSelf,
  });

  const followRelationship = useMutation({
    mutationFn: async () => {
      if (!profile.data) return;
      if (socialRelationship.data?.following) {
        await api.unfollowUser(profile.data.handle);
      } else {
        await api.followUser(profile.data.handle);
      }
    },
    onSuccess: async () => {
      toast.success(socialRelationship.data?.following ? "已取消关注" : "已关注");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["profile", name] }),
        queryClient.invalidateQueries({ queryKey: ["profile", name, "relationship"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "关注设置失败"),
  });
  const muteRelationship = useMutation({
    mutationFn: () => socialRelationship.data?.muted
      ? api.unmuteUser(profile.data?.handle ?? name)
      : api.muteUser(profile.data?.handle ?? name),
    onSuccess: async () => {
      toast.success(socialRelationship.data?.muted ? "已取消静音" : "已静音");
      await queryClient.invalidateQueries({ queryKey: ["profile", name, "relationship"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "静音设置失败"),
  });
  const blockRelationship = useMutation({
    mutationFn: () => socialRelationship.data?.blockedByMe
      ? api.unblockUser(profile.data?.handle ?? name)
      : api.blockUser(profile.data?.handle ?? name),
    onSuccess: async () => {
      setConfirmBlockOpen(false);
      toast.success(socialRelationship.data?.blockedByMe ? "已解除屏蔽" : "已屏蔽该用户");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["profile", name] }),
        queryClient.invalidateQueries({ queryKey: ["profile", name, "relationship"] }),
        queryClient.invalidateQueries({
          queryKey: [...accountQueryKeys.directMessages(account?.id), "conversations"],
        }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "屏蔽设置失败"),
  });

  if (profile.isLoading) {
    return <LoadingState label="加载用户主页" />;
  }
  if (profile.isError || !profile.data) {
    return <ErrorState title="找不到这个用户" error={profile.error} onRetry={() => void profile.refetch()} />;
  }

  const threadItems = threads.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const commentItems = comments.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const canManageUser = hasCapability(capabilities, ADMIN_CAPABILITIES.searchUsers)
    || hasCapability(capabilities, ADMIN_CAPABILITIES.changeRoles)
    || hasCapability(capabilities, ADMIN_CAPABILITIES.silenceUsers)
    || hasCapability(capabilities, ADMIN_CAPABILITIES.suspendUsers);
  const relationshipPending = followRelationship.isPending
    || muteRelationship.isPending
    || blockRelationship.isPending;
  const contentHidden = Boolean(
    socialRelationship.data?.blockedByMe || socialRelationship.data?.blockedMe,
  );
  const canManageVerifications = hasCapability(
    capabilities,
    ADMIN_CAPABILITIES.manageVerifications,
  ) && account?.role === "admin" && profile.data.role !== "admin";
  const authorLabel = profile.data.displayName || profile.data.handle;

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
          profile={profile.data}
          relationship={socialRelationship.data}
          isAuthenticated={isAuthenticated}
          isSelf={isSelf}
          relationshipLoading={socialRelationship.isLoading}
          relationshipPending={relationshipPending}
          messagePending={false}
          canStartConversation={(account?.trustLevel ?? 0) >= 1 && Boolean(socialRelationship.data?.canStartConversation)}
          canManageUser={canManageUser}
          canManageVerifications={canManageVerifications}
          confirmBlockOpen={confirmBlockOpen}
          onConfirmBlockOpenChange={setConfirmBlockOpen}
          onStartConversation={() => {
            navigate(`/messages?recipient=${encodeURIComponent(profile.data.handle)}`);
          }}
          onToggleFollow={() => followRelationship.mutate()}
          onToggleMute={() => muteRelationship.mutate()}
          onToggleBlock={() => blockRelationship.mutate()}
          onOpenRelationshipList={setRelationshipList}
        />

        <div className="min-[1240px]:hidden">
          <ProfileSidebar
            profile={profile.data}
            isSelf={isSelf}
            ariaLabel="个人主页侧栏（窄屏）"
            walletBalance={wallet.data?.balance ?? null}
            walletLoading={wallet.isLoading}
            activity={isSelf ? {
              calendar: activity.data,
              isLoading: activity.isLoading,
              error: activity.error,
              onRetry: () => void activity.refetch(),
            } : undefined}
          />
        </div>

        {!contentHidden && profile.data.canViewActivity ? (
          <section aria-label="用户动态">
            <Tabs
              value={activityTab}
              onValueChange={(value) => setActivityTab(value as ProfileActivityTab)}
              className="gap-0"
            >
              {/* Figma: 帖子 / 回复 / 收藏 / 媒体 / 喜欢 */}
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
                {threads.isLoading ? (
                  <LoadingState label="加载主题" />
                ) : threads.error ? (
                  <ErrorState error={threads.error} onRetry={() => void threads.refetch()} />
                ) : threadItems.length === 0 ? (
                  <EmptyState
                    title="暂无公开主题"
                    description="该用户还没有发布可见主题。"
                    className="border-0 bg-muted/20 shadow-none"
                  />
                ) : (
                  <>
                    {threadItems.flatMap((thread) => thread.id ? [(
                      <ProfilePostCard
                        key={thread.id}
                        authorName={authorLabel}
                        authorHandle={profile.data.handle}
                        authorAvatarUrl={profile.data.avatarUrl}
                        trustLevel={profile.data.trustLevel}
                        post={{
                          id: thread.id,
                          title: thread.title || "未命名主题",
                          boardSlug: thread.boardSlug,
                          createdAtLabel: formatRelativeTime(thread.createdAt),
                          replyCount: thread.replyCount ?? 0,
                          voteCount: thread.voteCount ?? 0,
                          href: `/forum/threads/${thread.id}`,
                        }}
                      />
                    )] : [])}
                    {threads.hasNextPage ? (
                      <Button
                        type="button"
                        variant="outline"
                        className="w-full"
                        onClick={() => void threads.fetchNextPage()}
                        disabled={threads.isFetchingNextPage}
                      >
                        {threads.isFetchingNextPage ? <Loader2 className="size-4 animate-spin" /> : null}
                        {threads.isFetchingNextPage ? "加载中" : "加载更多主题"}
                      </Button>
                    ) : null}
                  </>
                )}
              </TabsContent>

              <TabsContent value="comments" className="space-y-3">
                {comments.isLoading ? (
                  <LoadingState label="加载回复" />
                ) : comments.error ? (
                  <ErrorState error={comments.error} onRetry={() => void comments.refetch()} />
                ) : commentItems.length === 0 ? (
                  <EmptyState
                    title="暂无公开回复"
                    description="该用户还没有发布可见回复。"
                    className="border-0 bg-muted/20 shadow-none"
                  />
                ) : (
                  <>
                    {commentItems.flatMap((comment) => comment.id && comment.threadId ? [(
                      <ProfilePostCard
                        key={comment.id}
                        authorName={authorLabel}
                        authorHandle={profile.data.handle}
                        authorAvatarUrl={profile.data.avatarUrl}
                        trustLevel={profile.data.trustLevel}
                        post={{
                          id: comment.id,
                          title: comment.threadTitle || "查看所在主题",
                          body: comment.body || "该回复没有可展示内容",
                          createdAtLabel: formatRelativeTime(comment.createdAt),
                          href: `/forum/threads/${comment.threadId}`,
                        }}
                      />
                    )] : [])}
                    {comments.hasNextPage ? (
                      <Button
                        type="button"
                        variant="outline"
                        className="w-full"
                        onClick={() => void comments.fetchNextPage()}
                        disabled={comments.isFetchingNextPage}
                      >
                        {comments.isFetchingNextPage ? <Loader2 className="size-4 animate-spin" /> : null}
                        {comments.isFetchingNextPage ? "加载中" : "加载更多回复"}
                      </Button>
                    ) : null}
                  </>
                )}
              </TabsContent>

              <TabsContent value="bookmarks" className="space-y-3">
                <EmptyState
                  title={isSelf ? "收藏会显示在这里" : "收藏列表未开放"}
                  description={isSelf ? "你收藏的帖子会集中展示在个人主页。" : "目前仅展示公开帖子与回复。"}
                  className="border-0 bg-muted/20 shadow-none"
                />
              </TabsContent>

              <TabsContent value="media" className="space-y-3">
                <EmptyState
                  title="媒体内容即将开放"
                  description="图片与视频内容会在后续版本展示在这里。"
                  className="border-0 bg-muted/20 shadow-none"
                />
              </TabsContent>

              <TabsContent value="likes" className="space-y-3">
                <EmptyState
                  title="喜欢列表即将开放"
                  description="点赞过的内容会在后续版本展示在这里。"
                  className="border-0 bg-muted/20 shadow-none"
                />
              </TabsContent>
            </Tabs>
          </section>
        ) : null}

        {!contentHidden && !profile.data.canViewActivity ? (
          <EmptyState
            title="活动列表未公开"
            description="该用户限制了个人主页上的主题与回复列表；公开内容仍可在对应板块和主题中查看。"
          />
        ) : null}
      </div>

      <div className="hidden pb-16 pl-6 pt-6 min-[1240px]:block">
        <ProfileSidebar
          profile={profile.data}
          isSelf={isSelf}
          ariaLabel="个人主页侧栏（宽屏）"
          walletBalance={wallet.data?.balance ?? null}
          walletLoading={wallet.isLoading}
          activity={isSelf ? {
            calendar: activity.data,
            isLoading: activity.isLoading,
            error: activity.error,
            onRetry: () => void activity.refetch(),
          } : undefined}
        />
      </div>

      <ProfileRelationshipListDialog
        handle={profile.data.handle}
        kind={relationshipList ?? "followers"}
        open={relationshipList !== null}
        canRemoveFollowers={isSelf}
        viewerCacheKey={viewerCacheKey}
        onOpenChange={(open) => !open && setRelationshipList(null)}
      />
    </div>
  );
}
