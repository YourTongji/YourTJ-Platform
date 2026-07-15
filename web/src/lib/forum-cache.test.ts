import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import {
  beginForumOptimisticUpdate,
  commitForumOptimisticUpdate,
  discardAllForumOptimisticUpdates,
  nextForumVote,
  patchForumInteractionData,
  reconcileForumInteractionQueries,
  rollbackForumOptimisticUpdate,
} from "@/lib/forum-cache";
import { forumQueryKeys } from "@/lib/forum-query-keys";

function thread(id: string) {
  return {
    id,
    voteCount: 4,
    viewerVote: null,
    isBookmarked: true,
  };
}

describe("forum cache interactions", () => {
  it("derives vote transitions without assuming votes only increase", () => {
    expect(nextForumVote(8, null, "up")).toEqual({ voteCount: 9, viewerVote: "up" });
    expect(nextForumVote(8, "up", "up")).toEqual({ voteCount: 7, viewerVote: null });
    expect(nextForumVote(8, "down", "up")).toEqual({ voteCount: 10, viewerVote: "up" });
  });

  it("patches feed, detail, comments, profile, and bookmark cache shapes", () => {
    const target = { id: "thread-42", targetType: "thread" as const };
    const patch = { voteCount: 9, viewerVote: "up" as const, isBookmarked: false };
    const infiniteFeed = { pages: [{ items: [thread("thread-42")] }], pageParams: [null] };
    const detail = thread("thread-42");
    const bookmarkPage = {
      pages: [{ items: [{
        targetId: "thread-42",
        targetType: "thread" as const,
        content: thread("thread-42"),
      }] }],
      pageParams: [null],
    };

    expect(patchForumInteractionData(infiniteFeed, target, patch)).toMatchObject({
      pages: [{ items: [{ voteCount: 9, viewerVote: "up", isBookmarked: false }] }],
    });
    expect(patchForumInteractionData(detail, target, patch)).toMatchObject(patch);
    expect(patchForumInteractionData(bookmarkPage, target, patch)).toMatchObject({
      pages: [{ items: [] }],
    });
    expect(patchForumInteractionData(
      infiniteFeed,
      { id: "thread-42", targetType: "comment" },
      { voteCount: 99 },
      forumQueryKeys.homeFeed("hot"),
    )).toBe(infiniteFeed);
  });

  it("updates every mounted forum surface and rolls all of them back on failure", async () => {
    const queryClient = new QueryClient();
    const cancelQueries = vi.spyOn(queryClient, "cancelQueries");
    const target = { id: "thread-rollback", targetType: "thread" as const };
    const feedKey = forumQueryKeys.homeFeed("hot");
    const detailKey = forumQueryKeys.thread(target.id);
    queryClient.setQueryData(feedKey, {
      pages: [{ items: [thread(target.id)] }],
      pageParams: [null],
    });
    queryClient.setQueryData(detailKey, thread(target.id));

    const context = await beginForumOptimisticUpdate(queryClient, target, {
      voteCount: 5,
      viewerVote: "up",
    });

    expect(cancelQueries).toHaveBeenCalledWith({ predicate: expect.any(Function) });
    expect(queryClient.getQueryData<{ voteCount: number }>(detailKey)?.voteCount).toBe(5);
    expect(queryClient.getQueryData<{ pages: Array<{ items: Array<{ voteCount: number }> }> }>(
      feedKey,
    )?.pages[0]?.items[0]?.voteCount).toBe(5);

    rollbackForumOptimisticUpdate(queryClient, context);
    expect(queryClient.getQueryData<{ voteCount: number }>(detailKey)?.voteCount).toBe(4);
    expect(queryClient.getQueryData<{ pages: Array<{ items: Array<{ voteCount: number }> }> }>(
      feedKey,
    )?.pages[0]?.items[0]?.voteCount).toBe(4);
    await reconcileForumInteractionQueries(queryClient, context);
  });

  it("prevents an older response or rollback from overwriting a newer interaction", async () => {
    const queryClient = new QueryClient();
    const target = { id: "thread-concurrent", targetType: "thread" as const };
    const detailKey = forumQueryKeys.thread(target.id);
    queryClient.setQueryData(detailKey, thread(target.id));

    const first = await beginForumOptimisticUpdate(queryClient, target, {
      voteCount: 5,
      viewerVote: "up",
    });
    const second = await beginForumOptimisticUpdate(queryClient, target, {
      voteCount: 3,
      viewerVote: "down",
    });

    rollbackForumOptimisticUpdate(queryClient, first);
    commitForumOptimisticUpdate(queryClient, first, { voteCount: 99, viewerVote: "up" });
    expect(queryClient.getQueryData(detailKey)).toMatchObject({
      voteCount: 3,
      viewerVote: "down",
    });

    rollbackForumOptimisticUpdate(queryClient, second);
    expect(queryClient.getQueryData(detailKey)).toMatchObject({
      voteCount: 4,
      viewerVote: null,
    });

    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");
    await reconcileForumInteractionQueries(queryClient, second);
    expect(invalidateQueries).not.toHaveBeenCalled();
    await reconcileForumInteractionQueries(queryClient, first);
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: forumQueryKeys.homeFeeds() });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: forumQueryKeys.profiles() });
  });

  it("does not let a failure on one card erase a newer optimistic change on another card", async () => {
    const queryClient = new QueryClient();
    const firstTarget = { id: "thread-first", targetType: "thread" as const };
    const secondTarget = { id: "thread-second", targetType: "thread" as const };
    const feedKey = forumQueryKeys.homeFeed("hot");
    queryClient.setQueryData(feedKey, {
      pages: [{ items: [thread(firstTarget.id), thread(secondTarget.id)] }],
      pageParams: [null],
    });

    const first = await beginForumOptimisticUpdate(queryClient, firstTarget, {
      voteCount: 5,
      viewerVote: "up",
    });
    const second = await beginForumOptimisticUpdate(queryClient, secondTarget, {
      isBookmarked: false,
    });
    rollbackForumOptimisticUpdate(queryClient, first);

    const items = queryClient.getQueryData<{
      pages: Array<{ items: Array<{ id: string; voteCount: number; isBookmarked: boolean }> }>;
    }>(feedKey)?.pages[0]?.items;
    expect(items?.find((item) => item.id === firstTarget.id)?.voteCount).toBe(4);
    expect(items?.find((item) => item.id === secondTarget.id)?.isBookmarked).toBe(false);

    await reconcileForumInteractionQueries(queryClient, first);
    await reconcileForumInteractionQueries(queryClient, second);
  });

  it("keeps an earlier confirmed server value when a newer change fails", async () => {
    const queryClient = new QueryClient();
    const target = { id: "thread-rebase", targetType: "thread" as const };
    const detailKey = forumQueryKeys.thread(target.id);
    queryClient.setQueryData(detailKey, thread(target.id));

    const first = await beginForumOptimisticUpdate(queryClient, target, {
      voteCount: 5,
      viewerVote: "up",
    });
    const second = await beginForumOptimisticUpdate(queryClient, target, {
      voteCount: 3,
      viewerVote: "down",
    });
    commitForumOptimisticUpdate(queryClient, first, { voteCount: 6, viewerVote: "up" });
    rollbackForumOptimisticUpdate(queryClient, second);

    expect(queryClient.getQueryData(detailKey)).toMatchObject({
      voteCount: 6,
      viewerVote: "up",
    });
    await reconcileForumInteractionQueries(queryClient, second);
    await reconcileForumInteractionQueries(queryClient, first);
  });

  it("does not recreate a cleared principal cache when an old response arrives", async () => {
    const queryClient = new QueryClient();
    const target = { id: "thread-old-account", targetType: "thread" as const };
    const detailKey = forumQueryKeys.thread(target.id);
    queryClient.setQueryData(detailKey, thread(target.id));
    const context = await beginForumOptimisticUpdate(
      queryClient,
      target,
      { voteCount: 5, viewerVote: "up" },
      "account-old",
    );

    discardAllForumOptimisticUpdates(queryClient);
    queryClient.clear();
    commitForumOptimisticUpdate(queryClient, context, { voteCount: 5, viewerVote: "up" });
    rollbackForumOptimisticUpdate(queryClient, context);
    await reconcileForumInteractionQueries(queryClient, context);

    expect(queryClient.getQueryData(detailKey)).toBeUndefined();
  });
});
