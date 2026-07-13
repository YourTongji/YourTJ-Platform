import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ProfilePage } from "./profile-page";

const apiMocks = vi.hoisted(() => ({
  profile: vi.fn(),
  threads: vi.fn(),
  comments: vi.fn(),
  media: vi.fn(),
  likes: vi.fn(),
  bookmarks: vi.fn(),
  bookmark: vi.fn(),
  removeBookmark: vi.fn(),
  relationship: vi.fn(),
  follow: vi.fn(),
  unfollow: vi.fn(),
  mute: vi.fn(),
  unmute: vi.fn(),
  block: vi.fn(),
  unblock: vi.fn(),
  followers: vi.fn(),
  following: vi.fn(),
  dm: vi.fn(),
  myActivity: vi.fn(),
  wallet: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({
    account: {
      id: "1",
      handle: "alice",
      role: "user",
      trustLevel: 2,
      capabilities: [],
    },
    isAuthenticated: true,
  }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    publicUser: apiMocks.profile,
    userThreads: apiMocks.threads,
    userComments: apiMocks.comments,
    userMedia: apiMocks.media,
    userLikes: apiMocks.likes,
    bookmarks: apiMocks.bookmarks,
    bookmarkPost: apiMocks.bookmark,
    removeBookmark: apiMocks.removeBookmark,
    userRelationship: apiMocks.relationship,
    followUser: apiMocks.follow,
    unfollowUser: apiMocks.unfollow,
    muteUser: apiMocks.mute,
    unmuteUser: apiMocks.unmute,
    blockUser: apiMocks.block,
    unblockUser: apiMocks.unblock,
    userFollowers: apiMocks.followers,
    userFollowing: apiMocks.following,
    createDmConversation: apiMocks.dm,
    myActivity: apiMocks.myActivity,
    wallet: apiMocks.wallet,
  },
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/profile/bob"]}>
        <Routes>
          <Route path="/profile/:handle" element={<ProfilePage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("ProfilePage social relationships", () => {
  beforeEach(() => {
    apiMocks.profile.mockReset().mockResolvedValue({
      id: "2",
      handle: "bob",
      displayName: "Bob Builder",
      school: "同济大学",
      bio: "Campus maker",
      website: "https://bob.example.test",
      avatarUrl: null,
      bannerUrl: null,
      role: "user",
      trustLevel: 2,
      badges: [],
      threadCount: 3,
      commentCount: 4,
      votesReceived: 5,
      followerCount: 2,
      followingCount: 1,
      canViewActivity: true,
      createdAt: 1_700_000_000,
    });
    apiMocks.threads.mockReset().mockResolvedValue({ items: [], hasMore: false, nextCursor: null });
    apiMocks.comments.mockReset().mockResolvedValue({ items: [], hasMore: false, nextCursor: null });
    apiMocks.media.mockReset().mockResolvedValue({ items: [], hasMore: false, nextCursor: null });
    apiMocks.likes.mockReset().mockResolvedValue({ items: [], hasMore: false, nextCursor: null });
    apiMocks.bookmarks.mockReset().mockResolvedValue({ items: [], hasMore: false, nextCursor: null });
    apiMocks.bookmark.mockReset().mockResolvedValue(undefined);
    apiMocks.removeBookmark.mockReset().mockResolvedValue(undefined);
    apiMocks.relationship.mockReset().mockResolvedValue({
      isSelf: false,
      following: false,
      followedBy: true,
      muted: false,
      blockedByMe: false,
      blockedMe: false,
      canFollow: true,
      canStartConversation: true,
      canMention: true,
    });
    apiMocks.follow.mockReset().mockResolvedValue(undefined);
    apiMocks.unfollow.mockReset().mockResolvedValue(undefined);
    apiMocks.mute.mockReset().mockResolvedValue(undefined);
    apiMocks.unmute.mockReset().mockResolvedValue(undefined);
    apiMocks.block.mockReset().mockResolvedValue(undefined);
    apiMocks.unblock.mockReset().mockResolvedValue(undefined);
    apiMocks.followers.mockReset().mockResolvedValue({
      items: [{
        id: "3",
        handle: "carol",
        displayName: "Carol",
        avatarUrl: null,
        role: "user",
        followedAt: 1_700_000_100,
      }],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.following.mockReset().mockResolvedValue({ items: [], hasMore: false, nextCursor: null });
    apiMocks.dm.mockReset().mockResolvedValue({ id: "10" });
    apiMocks.myActivity.mockReset().mockResolvedValue({ from: "2026-01-01", to: "2026-07-01", weights: { thread: 3, comment: 2, like: 1 }, days: [] });
    apiMocks.wallet.mockReset().mockResolvedValue({ balance: 0 });
  });

  it("uses relationship and list APIs for follow, mute, block, and counts", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    expect(await screen.findByRole("heading", { name: "Bob Builder" })).toBeVisible();
    expect(screen.getByText("Campus maker")).toBeVisible();
    expect(await screen.findByText("关注了你")).toBeVisible();

    await user.click(screen.getByRole("button", { name: /^关注$/ }));
    await waitFor(() => expect(apiMocks.follow).toHaveBeenCalledWith("bob"));

    await user.click(screen.getByRole("button", { name: "静音" }));
    await waitFor(() => expect(apiMocks.mute).toHaveBeenCalledWith("bob"));

    await user.click(screen.getByRole("button", { name: /^屏蔽$/ }));
    await user.click(screen.getByRole("button", { name: "确认屏蔽" }));
    await waitFor(() => expect(apiMocks.block).toHaveBeenCalledWith("bob"));

    await user.click(screen.getByRole("button", { name: /关注者/ }));
    expect(await screen.findByText("Carol")).toBeVisible();
    expect(apiMocks.followers).toHaveBeenCalledWith("bob", null);

    await expectNoAccessibilityViolations(view.container);
  });

  it("shows an honest private-activity state without requesting protected lists", async () => {
    apiMocks.profile.mockResolvedValue({
      id: "2",
      handle: "bob",
      displayName: "Bob Builder",
      school: "同济大学",
      bio: "Campus maker",
      website: null,
      avatarUrl: null,
      bannerUrl: null,
      role: "user",
      trustLevel: 2,
      badges: [],
      verifications: [],
      threadCount: 3,
      commentCount: 4,
      votesReceived: 5,
      followerCount: 2,
      followingCount: 1,
      canViewActivity: false,
      createdAt: 1_700_000_000,
    });
    const view = renderPage();

    expect(await screen.findByText("活动列表未公开")).toBeVisible();
    expect(screen.getByText(/公开内容仍可在对应板块和主题中查看/)).toBeVisible();
    expect(apiMocks.threads).not.toHaveBeenCalled();
    expect(apiMocks.comments).not.toHaveBeenCalled();
    await expectNoAccessibilityViolations(view.container);
  });

  it("loads real media and likes and bookmarks visible content", async () => {
    apiMocks.media.mockResolvedValue({
      items: [{
        targetType: "thread",
        id: "thread-1",
        threadId: "thread-1",
        title: "校园照片",
        body: "今天的校园",
        contentFormat: "plain_v1",
        boardSlug: "campus",
        authorHandle: "bob",
        authorDisplayName: "Bob Builder",
        replyCount: 2,
        voteCount: 8,
        viewerVote: null,
        isBookmarked: false,
        attachments: [{
          assetId: "asset-1",
          reference: "yourtj-asset:asset-1",
          position: 0,
          url: "https://cdn.example.test/asset-1.jpg",
          alt: "校园",
          width: 1200,
          height: 800,
          expiresAt: 1_900_000_000,
        }],
        createdAt: 1_700_000_000,
        activityAt: 1_700_000_000,
      }],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.likes.mockResolvedValue({
      items: [{
        targetType: "comment",
        id: "comment-1",
        threadId: "thread-2",
        title: "选课讨论",
        body: "很有帮助",
        contentFormat: "plain_v1",
        boardSlug: "courses",
        authorHandle: "carol",
        authorDisplayName: "Carol",
        replyCount: 1,
        voteCount: 5,
        viewerVote: "up",
        isBookmarked: false,
        attachments: [],
        createdAt: 1_700_000_100,
        activityAt: 1_700_000_200,
      }],
      hasMore: false,
      nextCursor: null,
    });
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByRole("heading", { name: "Bob Builder" })).toBeVisible();
    await user.click(screen.getByRole("tab", { name: "媒体" }));
    expect(await screen.findByText("校园照片")).toBeVisible();
    expect(apiMocks.media).toHaveBeenCalledWith("bob", null);
    await user.click(screen.getByRole("button", { name: "收藏" }));
    await waitFor(() => expect(apiMocks.bookmark).toHaveBeenCalledWith("thread-1", "thread"));

    await user.click(screen.getByRole("tab", { name: "喜欢" }));
    expect(await screen.findByText("选课讨论")).toBeVisible();
    expect(screen.getByLabelText("5 个赞")).toBeVisible();
    expect(apiMocks.likes).toHaveBeenCalledWith("bob", null);
  });
});
