import type { QueryClient, QueryKey } from "@tanstack/react-query";

import { forumRefreshQueryRoots, isForumInteractionQuery } from "@/lib/forum-query-keys";

export type ForumTargetType = "thread" | "comment";
export type ForumVote = "up" | "down" | null;

export interface ForumInteractionTarget {
  id: string;
  targetType: ForumTargetType;
}

export interface ForumInteractionPatch {
  voteCount?: number;
  viewerVote?: ForumVote;
  isBookmarked?: boolean;
}

export interface ForumOptimisticContext {
  target: ForumInteractionTarget;
  version: number;
  snapshots: Array<[QueryKey, unknown]>;
  cycle: number;
  principalScope: string;
}

interface PatchableContent {
  id?: string;
  targetId?: string;
  targetType?: ForumTargetType;
  content?: PatchableContent;
  items?: PatchableContent[];
  pages?: Array<{ items?: PatchableContent[] }>;
  voteCount?: number;
  viewerVote?: ForumVote;
  isBookmarked?: boolean;
}

interface ActiveForumUpdate {
  context: ForumOptimisticContext;
  patch: ForumInteractionPatch | null;
  resolution: "pending" | "succeeded" | "failed" | "discarded";
  isSettled: boolean;
}

interface ForumOptimisticState {
  cycle: number;
  nextVersion: number;
  pendingCount: number;
  principalScope: string | null;
  baseline: Array<[QueryKey, unknown]>;
  isBaselineReady: boolean;
  updates: ActiveForumUpdate[];
  beginChain: Promise<void>;
}

const optimisticStates = new WeakMap<QueryClient, ForumOptimisticState>();

function createOptimisticState(): ForumOptimisticState {
  return {
    cycle: 0,
    nextVersion: 0,
    pendingCount: 0,
    principalScope: null,
    baseline: [],
    isBaselineReady: false,
    updates: [],
    beginChain: Promise.resolve(),
  };
}

function optimisticState(queryClient: QueryClient) {
  const existing = optimisticStates.get(queryClient);
  if (existing) return existing;
  const state = createOptimisticState();
  optimisticStates.set(queryClient, state);
  return state;
}

function resetOptimisticState(state: ForumOptimisticState, principalScope: string | null) {
  state.cycle += 1;
  state.nextVersion = 0;
  state.pendingCount = 0;
  state.principalScope = principalScope;
  state.baseline = [];
  state.isBaselineReady = false;
  state.updates = [];
  state.beginChain = Promise.resolve();
}

function activeUpdate(
  state: ForumOptimisticState,
  context: ForumOptimisticContext | undefined,
) {
  if (!context || context.cycle !== state.cycle) return undefined;
  return state.updates.find((update) => update.context.version === context.version);
}

function rebuildOptimisticQueries(queryClient: QueryClient, state: ForumOptimisticState) {
  for (const [queryKey, data] of state.baseline) {
    queryClient.setQueryData(queryKey, data);
  }
  for (const update of state.updates) {
    if (!update.patch || update.resolution === "failed" || update.resolution === "discarded") {
      continue;
    }
    for (const [queryKey] of state.baseline) {
      queryClient.setQueryData(queryKey, (data: unknown) =>
        patchForumInteractionData(data, update.context.target, update.patch ?? {}, queryKey));
    }
  }
}

function voteValue(vote: ForumVote) {
  if (vote === "up") return 1;
  if (vote === "down") return -1;
  return 0;
}

export function nextForumVote(
  voteCount: number,
  viewerVote: ForumVote,
  requestedVote: Exclude<ForumVote, null>,
) {
  const nextVote = viewerVote === requestedVote ? null : requestedVote;
  return {
    voteCount: voteCount - voteValue(viewerVote) + voteValue(nextVote),
    viewerVote: nextVote,
  };
}

function matchesTarget(content: PatchableContent, target: ForumInteractionTarget) {
  if (content.targetId === target.id) {
    return content.targetType === undefined || content.targetType === target.targetType;
  }
  if (content.id !== target.id) return false;
  return content.targetType === undefined || content.targetType === target.targetType;
}

function patchContent(
  content: PatchableContent,
  target: ForumInteractionTarget,
  patch: ForumInteractionPatch,
): PatchableContent {
  if (!matchesTarget(content, target)) return content;
  return { ...content, ...patch };
}

function patchItems(
  items: PatchableContent[],
  target: ForumInteractionTarget,
  patch: ForumInteractionPatch,
) {
  let hasChanged = false;
  const nextItems = items.flatMap((item) => {
    if (item.content && matchesTarget(item.content, target)) {
      if (patch.isBookmarked === false) {
        hasChanged = true;
        return [];
      }
      hasChanged = true;
      return [{ ...item, content: patchContent(item.content, target, patch) }];
    }
    const nextItem = patchContent(item, target, patch);
    hasChanged ||= nextItem !== item;
    return [nextItem];
  });
  return hasChanged ? nextItems : items;
}

function fixedTargetType(queryKey?: QueryKey): ForumTargetType | undefined {
  if (!queryKey) return undefined;
  const [root, segment, contentSegment] = queryKey;
  if (root === "home" && segment === "threads") return "thread";
  if (root === "forum" && segment === "threads") return "thread";
  if (root === "thread") return "thread";
  if (root === "thread-comments") return "comment";
  if (root === "profile" && contentSegment === "threads") return "thread";
  if (root === "profile" && contentSegment === "comments") return "comment";
  return undefined;
}

export function patchForumInteractionData(
  data: unknown,
  target: ForumInteractionTarget,
  patch: ForumInteractionPatch,
  queryKey?: QueryKey,
): unknown {
  if (!data || typeof data !== "object") return data;
  const queryTargetType = fixedTargetType(queryKey);
  if (queryTargetType && queryTargetType !== target.targetType) return data;
  const content = data as PatchableContent;

  if (Array.isArray(content.pages)) {
    let hasChanged = false;
    const pages = content.pages.map((page) => {
      if (!Array.isArray(page.items)) return page;
      const items = patchItems(page.items, target, patch);
      hasChanged ||= items !== page.items;
      return items === page.items ? page : { ...page, items };
    });
    return hasChanged ? { ...content, pages } : data;
  }

  if (Array.isArray(content.items)) {
    const items = patchItems(content.items, target, patch);
    return items === content.items ? data : { ...content, items };
  }

  return patchContent(content, target, patch);
}

function interactionQuery(query: { queryKey: QueryKey }) {
  return isForumInteractionQuery(query.queryKey);
}

export async function beginForumOptimisticUpdate(
  queryClient: QueryClient,
  target: ForumInteractionTarget,
  patch: ForumInteractionPatch,
  principalScope = "global",
): Promise<ForumOptimisticContext> {
  const state = optimisticState(queryClient);
  if (state.pendingCount === 0 || state.principalScope !== principalScope) {
    resetOptimisticState(state, principalScope);
  }
  const context: ForumOptimisticContext = {
    target,
    version: state.nextVersion + 1,
    snapshots: [],
    cycle: state.cycle,
    principalScope,
  };
  state.nextVersion = context.version;
  state.pendingCount += 1;
  const update: ActiveForumUpdate = {
    context,
    patch,
    resolution: "pending",
    isSettled: false,
  };
  state.updates.push(update);
  const previousBegin = state.beginChain;
  const setup = previousBegin.catch(() => undefined).then(async () => {
    await queryClient.cancelQueries({ predicate: interactionQuery });
    if (!activeUpdate(state, context)) return;
    if (!state.isBaselineReady) {
      state.baseline = queryClient.getQueriesData({ predicate: interactionQuery });
      state.isBaselineReady = true;
    }
    context.snapshots = state.baseline;
    for (const [queryKey] of state.baseline) {
      queryClient.setQueryData(queryKey, (data: unknown) =>
        patchForumInteractionData(data, target, patch, queryKey));
    }
  });
  state.beginChain = setup.then(() => undefined, () => undefined);
  try {
    await setup;
  } catch (error) {
    const failedUpdate = activeUpdate(state, context);
    if (failedUpdate) {
      state.updates = state.updates.filter((candidate) => candidate !== failedUpdate);
      state.pendingCount = Math.max(0, state.pendingCount - 1);
      if (state.pendingCount === 0) resetOptimisticState(state, null);
    }
    throw error;
  }
  return context;
}

export function commitForumOptimisticUpdate(
  queryClient: QueryClient,
  context: ForumOptimisticContext | undefined,
  patch: ForumInteractionPatch,
) {
  const state = optimisticState(queryClient);
  const update = activeUpdate(state, context);
  if (!update || update.resolution !== "pending") return;
  update.patch = patch;
  update.resolution = "succeeded";
  rebuildOptimisticQueries(queryClient, state);
}

export function rollbackForumOptimisticUpdate(
  queryClient: QueryClient,
  context: ForumOptimisticContext | undefined,
) {
  const state = optimisticState(queryClient);
  const update = activeUpdate(state, context);
  if (!update || update.resolution !== "pending") return;
  update.patch = null;
  update.resolution = "failed";
  rebuildOptimisticQueries(queryClient, state);
}

export function discardForumOptimisticUpdate(
  queryClient: QueryClient,
  context: ForumOptimisticContext | undefined,
) {
  const state = optimisticState(queryClient);
  const update = activeUpdate(state, context);
  if (!update || update.resolution === "discarded") return;
  update.patch = null;
  update.resolution = "discarded";
}

export function discardAllForumOptimisticUpdates(queryClient: QueryClient) {
  resetOptimisticState(optimisticState(queryClient), null);
}

export async function reconcileForumInteractionQueries(
  queryClient: QueryClient,
  context?: ForumOptimisticContext,
) {
  const state = optimisticState(queryClient);
  const update = activeUpdate(state, context);
  if (!update || update.isSettled) return;
  update.isSettled = true;
  state.pendingCount = Math.max(0, state.pendingCount - 1);
  if (state.pendingCount > 0) return;
  const shouldInvalidate = state.updates.every((candidate) =>
    candidate.resolution !== "discarded");
  resetOptimisticState(state, null);
  if (!shouldInvalidate) return;
  await Promise.all(forumRefreshQueryRoots().map((queryKey) =>
    queryClient.invalidateQueries({ queryKey })));
}
