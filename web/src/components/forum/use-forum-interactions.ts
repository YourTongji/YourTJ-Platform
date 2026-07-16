import { useMutation, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { toast } from "sonner";

import { useAuth } from "@/context/auth-provider";
import { accountQueryScope } from "@/lib/account-query-keys";
import { api } from "@/lib/api/endpoints";
import {
  beginForumOptimisticUpdate,
  commitForumOptimisticUpdate,
  discardForumOptimisticUpdate,
  nextForumVote,
  reconcileForumInteractionQueries,
  rollbackForumOptimisticUpdate,
  type ForumInteractionTarget,
  type ForumVote,
} from "@/lib/forum-cache";

interface VoteMutationInput extends ForumInteractionTarget {
  requestedVote: Exclude<ForumVote, null>;
  voteCount: number;
  viewerVote: ForumVote;
}

interface BookmarkMutationInput extends ForumInteractionTarget {
  isBookmarked: boolean;
}

function pendingTargetKey(
  principalScope: string,
  target: ForumInteractionTarget,
) {
  return `${principalScope}:${target.targetType}:${target.id}`;
}

function usePendingForumTargets(principalScope: string) {
  const [, setPendingCounts] = React.useState(
    () => new Map<string, number>(),
  );
  const pendingCountsRef = React.useRef(new Map<string, number>());
  const markPending = React.useCallback((key: string) => {
    const next = new Map(pendingCountsRef.current);
    next.set(key, (next.get(key) ?? 0) + 1);
    pendingCountsRef.current = next;
    setPendingCounts(next);
  }, []);
  const markSettled = React.useCallback((key: string) => {
    const count = pendingCountsRef.current.get(key) ?? 0;
    if (count === 0) return;
    const next = new Map(pendingCountsRef.current);
    if (count === 1) next.delete(key);
    else next.set(key, count - 1);
    pendingCountsRef.current = next;
    setPendingCounts(next);
  }, []);
  const isTargetPending = React.useCallback(
    (target: ForumInteractionTarget) =>
      pendingCountsRef.current.has(pendingTargetKey(principalScope, target)),
    [principalScope],
  );
  return { isTargetPending, markPending, markSettled };
}

export function useForumVoteMutation() {
  const queryClient = useQueryClient();
  const { account } = useAuth();
  const principalScope = accountQueryScope(account?.id);
  const principalScopeRef = React.useRef(principalScope);
  principalScopeRef.current = principalScope;
  const pendingTargets = usePendingForumTargets(principalScope);
  const mutation = useMutation({
    mutationFn: ({ id, targetType, requestedVote, viewerVote }: VoteMutationInput) =>
      viewerVote === requestedVote
        ? api.removePostVote(id, targetType)
        : api.votePost(id, requestedVote, targetType),
    onMutate: async (input) => {
      const pendingKey = pendingTargetKey(principalScope, input);
      pendingTargets.markPending(pendingKey);
      try {
        return {
          ...await beginForumOptimisticUpdate(
            queryClient,
            input,
            nextForumVote(
              input.voteCount,
              input.viewerVote,
              input.requestedVote,
            ),
            principalScope,
          ),
          pendingKey,
        };
      } catch (error) {
        pendingTargets.markSettled(pendingKey);
        throw error;
      }
    },
    onSuccess: (response, _input, context) => {
      if (context?.principalScope !== principalScopeRef.current) {
        discardForumOptimisticUpdate(queryClient, context);
        return;
      }
      commitForumOptimisticUpdate(queryClient, context, {
        voteCount: response.voteCount,
        viewerVote: response.viewerVote,
      });
    },
    onError: (error, _input, context) => {
      if (context?.principalScope !== principalScopeRef.current) {
        discardForumOptimisticUpdate(queryClient, context);
        return;
      }
      rollbackForumOptimisticUpdate(queryClient, context);
      toast.error(error instanceof Error ? error.message : "投票失败");
    },
    onSettled: (_data, _error, _input, context) => {
      if (context?.pendingKey) pendingTargets.markSettled(context.pendingKey);
      if (context?.principalScope !== principalScopeRef.current) {
        discardForumOptimisticUpdate(queryClient, context);
      }
      return reconcileForumInteractionQueries(queryClient, context);
    },
  });
  return { ...mutation, isTargetPending: pendingTargets.isTargetPending };
}

export function useForumBookmarkMutation() {
  const queryClient = useQueryClient();
  const { account } = useAuth();
  const principalScope = accountQueryScope(account?.id);
  const principalScopeRef = React.useRef(principalScope);
  principalScopeRef.current = principalScope;
  const pendingTargets = usePendingForumTargets(principalScope);
  const mutation = useMutation({
    mutationFn: ({ id, targetType, isBookmarked }: BookmarkMutationInput) =>
      isBookmarked
        ? api.removeBookmark(id, targetType)
        : api.bookmarkPost(id, targetType),
    onMutate: async (input) => {
      const pendingKey = pendingTargetKey(principalScope, input);
      pendingTargets.markPending(pendingKey);
      try {
        return {
          ...await beginForumOptimisticUpdate(queryClient, input, {
            isBookmarked: !input.isBookmarked,
          }, principalScope),
          pendingKey,
        };
      } catch (error) {
        pendingTargets.markSettled(pendingKey);
        throw error;
      }
    },
    onSuccess: (_response, input, context) => {
      if (context?.principalScope !== principalScopeRef.current) {
        discardForumOptimisticUpdate(queryClient, context);
        return;
      }
      commitForumOptimisticUpdate(queryClient, context, {
        isBookmarked: !input.isBookmarked,
      });
      toast.success(input.isBookmarked ? "已取消收藏" : "已收藏");
    },
    onError: (error, _input, context) => {
      if (context?.principalScope !== principalScopeRef.current) {
        discardForumOptimisticUpdate(queryClient, context);
        return;
      }
      rollbackForumOptimisticUpdate(queryClient, context);
      toast.error(error instanceof Error ? error.message : "收藏操作失败");
    },
    onSettled: (_data, _error, _input, context) => {
      if (context?.pendingKey) pendingTargets.markSettled(context.pendingKey);
      if (context?.principalScope !== principalScopeRef.current) {
        discardForumOptimisticUpdate(queryClient, context);
      }
      return reconcileForumInteractionQueries(queryClient, context);
    },
  });
  return { ...mutation, isTargetPending: pendingTargets.isTargetPending };
}
