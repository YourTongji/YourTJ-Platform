import type { QueryKey } from "@tanstack/react-query";

export type ForumFeedMode = "hot" | "new" | "subscriptions" | "following" | "unread";
export type HomeFeedMode = Exclude<ForumFeedMode, "unread">;

const profileContentSegments = new Set(["threads", "comments", "media", "likes", "bookmarks"]);

export const forumQueryKeys = {
  homeFeeds: () => ["home", "threads"] as const,
  homeFeed: (feed: HomeFeedMode) => [...forumQueryKeys.homeFeeds(), feed] as const,
  feeds: () => ["forum", "threads"] as const,
  feed: (feed: ForumFeedMode, board: string, tag: string) =>
    [...forumQueryKeys.feeds(), feed, board, tag] as const,
  boards: () => ["forum", "boards"] as const,
  tags: () => ["forum", "tags"] as const,
  bookmarks: () => ["forum", "bookmarks"] as const,
  profiles: () => ["profile"] as const,
  profile: (handle: string) => [...forumQueryKeys.profiles(), handle] as const,
  profileViewer: (handle: string, viewer: string) =>
    [...forumQueryKeys.profile(handle), "viewer", viewer] as const,
  profileThreads: (handle: string, viewer: string) =>
    [...forumQueryKeys.profile(handle), "threads", viewer] as const,
  profileComments: (handle: string, viewer: string) =>
    [...forumQueryKeys.profile(handle), "comments", viewer] as const,
  profileMedia: (handle: string, viewer: string) =>
    [...forumQueryKeys.profile(handle), "media", viewer] as const,
  profileLikes: (handle: string, viewer: string) =>
    [...forumQueryKeys.profile(handle), "likes", viewer] as const,
  profileBookmarks: (handle: string, viewer: string) =>
    [...forumQueryKeys.profile(handle), "bookmarks", viewer] as const,
  threadDetails: () => ["thread"] as const,
  thread: (threadId: string) => [...forumQueryKeys.threadDetails(), threadId] as const,
  threadComments: () => ["thread-comments"] as const,
  comments: (threadId: string) => [...forumQueryKeys.threadComments(), threadId] as const,
};

export function forumRefreshQueryRoots(): QueryKey[] {
  return [
    forumQueryKeys.homeFeeds(),
    forumQueryKeys.feeds(),
    forumQueryKeys.bookmarks(),
    forumQueryKeys.profiles(),
    forumQueryKeys.threadDetails(),
    forumQueryKeys.threadComments(),
  ];
}

export function isForumInteractionQuery(queryKey: QueryKey) {
  const [root, segment, contentSegment] = queryKey;
  if (root === "home" && segment === "threads") return true;
  if (root === "forum" && (segment === "threads" || segment === "bookmarks")) return true;
  if (root === "thread" || root === "thread-comments") return true;
  return root === "profile"
    && typeof contentSegment === "string"
    && profileContentSegments.has(contentSegment);
}
