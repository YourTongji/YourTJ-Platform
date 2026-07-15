import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { useNavigate, useSearchParams } from "react-router";
import { toast } from "sonner";

import { ActivityHeatmap } from "@/components/activity/activity-heatmap";
import { getTwentyWeekActivityRange } from "@/components/activity/calendar-range";
import { CheckInShareDialog } from "@/components/activity/check-in-share-dialog";
import { DailyCheckInButton } from "@/components/activity/daily-check-in-button";
import { useForumDeliveryRefresh } from "@/components/content/forum-delivery-image";
import {
  useForumBookmarkMutation,
  useForumVoteMutation,
} from "@/components/forum/use-forum-interactions";
import { CommunityFeed, type CommunityFeedMode } from "@/components/home/community-feed";
import {
  CommunitySidebar,
  TrustProgressSummary,
} from "@/components/home/community-sidebar";
import { Card, CardContent } from "@/components/ui/card";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { ThreadFeed } from "@/lib/api/types";
import { forumQueryKeys } from "@/lib/forum-query-keys";

const communityFeedModes: CommunityFeedMode[] = ["hot", "new", "following", "subscriptions"];
const authenticatedFeedModes: CommunityFeedMode[] = ["following", "subscriptions"];

export function HomePage() {
  const { account } = useAuth();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const [isCheckInShareOpen, setIsCheckInShareOpen] = React.useState(false);
  const requestedFeed = searchParams.get("feed");
  const selectedFeed = communityFeedModes.includes(requestedFeed as CommunityFeedMode)
    ? requestedFeed as CommunityFeedMode
    : "hot";
  const feedMode = !account && authenticatedFeedModes.includes(selectedFeed)
    ? "hot"
    : selectedFeed;

  React.useEffect(() => {
    if (requestedFeed && requestedFeed !== feedMode) {
      const nextParams = new URLSearchParams(searchParams);
      nextParams.delete("feed");
      setSearchParams(nextParams, { replace: true });
    }
  }, [feedMode, requestedFeed, searchParams, setSearchParams]);

  function setFeedMode(nextFeed: CommunityFeedMode) {
    const nextParams = new URLSearchParams(searchParams);
    if (nextFeed === "hot") {
      nextParams.delete("feed");
    } else {
      nextParams.set("feed", nextFeed);
    }
    setSearchParams(nextParams);
  }

  const vote = useForumVoteMutation();
  const bookmark = useForumBookmarkMutation();

  function requireForumAccount(message: string) {
    if (account) return true;
    toast.error(message);
    navigate("/login");
    return false;
  }

  function voteOnThread(thread: ThreadFeed, requestedVote: "up" | "down") {
    if (!requireForumAccount("登录后才能参与投票")) return;
    if (vote.isTargetPending({ id: thread.id, targetType: "thread" })) return;
    vote.mutate({
      id: thread.id,
      targetType: "thread",
      requestedVote,
      voteCount: thread.voteCount,
      viewerVote: thread.viewerVote,
    });
  }

  function toggleThreadBookmark(thread: ThreadFeed) {
    if (!requireForumAccount("登录后才能收藏")) return;
    if (bookmark.isTargetPending({ id: thread.id, targetType: "thread" })) return;
    bookmark.mutate({
      id: thread.id,
      targetType: "thread",
      isBookmarked: thread.isBookmarked,
    });
  }

  const [activityRange, setActivityRange] = React.useState(getTwentyWeekActivityRange);
  const threads = useInfiniteQuery({
    queryKey: forumQueryKeys.homeFeed(feedMode),
    queryFn: ({ pageParam }) => api.threads({ feed: feedMode, cursor: pageParam }),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
  });
  const announcements = useQuery({
    queryKey: ["announcements", "active"],
    queryFn: api.announcements,
  });
  const activity = useQuery({
    queryKey: ["home", "activity", account?.id, activityRange.from, activityRange.to],
    queryFn: () => api.myActivity(activityRange.from, activityRange.to),
    enabled: Boolean(account),
  });
  const trustProgress = useQuery({
    queryKey: ["home", "trust-progress", account?.id],
    queryFn: api.myTrustProgress,
    enabled: Boolean(account),
    staleTime: 5 * 60 * 1000,
  });
  const checkInStatus = useQuery({
    queryKey: ["home", "check-in", account?.id],
    queryFn: api.myCheckInStatus,
    enabled: Boolean(account),
    staleTime: 5 * 60 * 1000,
  });
  React.useEffect(() => {
    const accountId = account?.id;
    const nextResetAt = checkInStatus.data?.nextResetAt;
    if (!accountId || nextResetAt == null) return undefined;

    const refreshDelay = Math.max(1_000, nextResetAt * 1_000 - Date.now() + 1_000);
    const timer = window.setTimeout(() => {
      setActivityRange(getTwentyWeekActivityRange());
      void Promise.all([
        queryClient.invalidateQueries({ queryKey: ["home", "check-in", accountId] }),
        queryClient.invalidateQueries({ queryKey: ["home", "activity", accountId] }),
        queryClient.invalidateQueries({ queryKey: ["home", "trust-progress", accountId] }),
      ]);
    }, refreshDelay);
    return () => window.clearTimeout(timer);
  }, [account?.id, checkInStatus.data?.nextResetAt, queryClient]);
  const checkIn = useMutation({
    mutationFn: api.checkIn,
    onSuccess: async (status) => {
      queryClient.setQueryData(["home", "check-in", account?.id], status);
      toast.success(status.newlyCheckedIn ? `签到成功，已连续 ${status.currentStreak} 天` : "今天已经签到");
      setIsCheckInShareOpen(true);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["home", "activity", account?.id] }),
        queryClient.invalidateQueries({ queryKey: ["home", "trust-progress", account?.id] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "签到失败，请稍后重试"),
  });

  const checkInState = {
    status: checkInStatus.data,
    isLoading: checkInStatus.isLoading,
    isPending: checkIn.isPending,
    error: checkInStatus.error,
    onCheckIn: () => checkIn.mutate(),
    onRetry: () => void checkInStatus.refetch(),
    onShare: () => setIsCheckInShareOpen(true),
  };

  const threadItems = threads.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  useForumDeliveryRefresh(
    threadItems.flatMap((thread) => [thread.authorAvatar, thread.attachments?.[0]]),
    () => void threads.refetch(),
  );

  return (
    <div className="min-[1240px]:grid min-[1240px]:grid-cols-[minmax(0,640px)_320px]">
      <div className="px-4 py-5 sm:px-6 sm:py-6 min-[1360px]:!px-8">
        <Card
          className="mb-5 rounded-xl min-[1240px]:hidden"
          aria-label="移动端每日签到与成长"
        >
          <CardContent className="space-y-4 p-4">
            <DailyCheckInButton account={account} {...checkInState} />
            <TrustProgressSummary
              account={account}
              trustProgress={trustProgress.data ?? null}
              trustError={trustProgress.error}
              onTrustRetry={() => void trustProgress.refetch()}
            />
            <div className="border-t border-border/70" />
            <ActivityHeatmap
              isAuthenticated={Boolean(account)}
              calendar={activity.data}
              isLoading={activity.isLoading}
              error={activity.error}
              onRetry={() => void activity.refetch()}
            />
          </CardContent>
        </Card>
        <CommunityFeed
          mode={feedMode}
          onModeChange={setFeedMode}
          items={threadItems}
          isLoading={threads.isLoading}
          error={threads.error}
          onRetry={() => void threads.refetch()}
          hasMore={threads.hasNextPage}
          isLoadingMore={threads.isFetchingNextPage}
          onLoadMore={() => void threads.fetchNextPage()}
          isAuthenticated={Boolean(account)}
          onAttachmentDeliveryRefresh={() => void threads.refetch()}
          onVote={voteOnThread}
          onToggleBookmark={toggleThreadBookmark}
          isVotePending={(thread) => vote.isTargetPending({
            id: thread.id,
            targetType: "thread",
          })}
          isBookmarkPending={(thread) => bookmark.isTargetPending({
            id: thread.id,
            targetType: "thread",
          })}
        />
      </div>

      <div className="hidden pb-16 pl-6 pt-6 min-[1240px]:block">
        <CommunitySidebar
          account={account}
          activity={{
            calendar: activity.data,
            isLoading: activity.isLoading,
            error: activity.error,
            onRetry: () => void activity.refetch(),
          }}
          checkIn={checkInState}
          threads={threadItems}
          announcements={announcements.data ?? []}
          trustProgress={trustProgress.data ?? null}
          trustError={trustProgress.error}
          onTrustRetry={() => void trustProgress.refetch()}
        />
      </div>
      <CheckInShareDialog
        open={isCheckInShareOpen}
        onOpenChange={setIsCheckInShareOpen}
        status={checkInStatus.data}
        trustProgress={trustProgress.data ?? null}
        fallbackTrustLevel={account?.trustLevel}
      />
    </div>
  );
}
