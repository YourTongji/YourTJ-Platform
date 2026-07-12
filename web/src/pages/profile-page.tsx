import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { MessageCircle, MessageSquare, ThumbsUp } from "lucide-react";
import * as React from "react";
import { Link, useNavigate, useParams } from "react-router";
import { toast } from "sonner";

import {
  ADMIN_CAPABILITIES,
  capabilitiesForAccount,
  hasCapability,
} from "@/components/admin/capabilities";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { ProfileActivitySection } from "@/components/profile/profile-activity-section";
import {
  ProfileRelationshipListDialog,
  type ProfileRelationshipListKind,
} from "@/components/profile/profile-relationship-list-dialog";
import { ProfileSummary } from "@/components/profile/profile-summary";
import { Badge } from "@/components/ui/badge";
import { useAuth } from "@/context/auth-provider";
import { accountQueryKeys } from "@/lib/account-query-keys";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

export function ProfilePage() {
  const { handle } = useParams();
  const name = handle ?? "";
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { account, isAuthenticated } = useAuth();
  const [confirmBlockOpen, setConfirmBlockOpen] = React.useState(false);
  const [relationshipList, setRelationshipList] = React.useState<ProfileRelationshipListKind | null>(null);
  const capabilities = React.useMemo(() => capabilitiesForAccount(account), [account]);
  const viewerCacheKey = account?.id ?? "anonymous";

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

  return (
    <div className="space-y-5">
      <PageHeader
        eyebrow="Community Profile"
        title={profile.data.displayName || profile.data.handle}
        description="公开社区身份、贡献与最近参与；校园邮箱等身份信息始终不会显示在这里。"
      />

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

      {!contentHidden && profile.data.canViewActivity ? <div className="grid items-start gap-5 lg:grid-cols-2">
        <ProfileActivitySection
          title="主题"
          icon={MessageSquare}
          isLoading={threads.isLoading}
          error={threads.error}
          items={threadItems.flatMap((thread) => thread.id ? [(
            <Link
              key={thread.id}
              to={`/forum/threads/${thread.id}`}
              className="block rounded-lg border p-3 outline-none transition-colors hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50"
            >
              <div className="flex items-start justify-between gap-3">
                <p className="min-w-0 font-medium leading-6">{thread.title || "未命名主题"}</p>
                {thread.boardSlug ? <Badge variant="outline">{thread.boardSlug}</Badge> : null}
              </div>
              <div className="mt-2 flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                <span>{formatUnixTime(thread.createdAt)}</span>
                <span className="flex items-center gap-1"><MessageCircle className="size-3" />{thread.replyCount ?? 0}</span>
                <span className="flex items-center gap-1"><ThumbsUp className="size-3" />{thread.voteCount ?? 0}</span>
              </div>
            </Link>
          )] : [])}
          emptyTitle="暂无公开主题"
          emptyDescription="该用户还没有发布可见主题。"
          hasMore={Boolean(threads.hasNextPage)}
          isLoadingMore={threads.isFetchingNextPage}
          onRetry={() => void threads.refetch()}
          onLoadMore={() => void threads.fetchNextPage()}
        />

        <ProfileActivitySection
          title="回复"
          icon={MessageCircle}
          isLoading={comments.isLoading}
          error={comments.error}
          items={commentItems.flatMap((comment) => comment.id && comment.threadId ? [(
            <Link
              key={comment.id}
              to={`/forum/threads/${comment.threadId}`}
              className="block rounded-lg border p-3 outline-none transition-colors hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50"
            >
              <p className="text-xs font-medium text-primary">
                {comment.threadTitle || "查看所在主题"}
              </p>
              <p className="mt-1 line-clamp-3 whitespace-pre-wrap text-sm leading-6">
                {comment.body || "该回复没有可展示内容"}
              </p>
              <p className="mt-2 text-xs text-muted-foreground">{formatUnixTime(comment.createdAt)}</p>
            </Link>
          )] : [])}
          emptyTitle="暂无公开回复"
          emptyDescription="该用户还没有发布可见回复。"
          hasMore={Boolean(comments.hasNextPage)}
          isLoadingMore={comments.isFetchingNextPage}
          onRetry={() => void comments.refetch()}
          onLoadMore={() => void comments.fetchNextPage()}
        />
      </div> : null}

      {!contentHidden && !profile.data.canViewActivity ? (
        <EmptyState
          title="活动列表未公开"
          description="该用户限制了个人主页上的主题与回复列表；公开内容仍可在对应板块和主题中查看。"
        />
      ) : null}

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
