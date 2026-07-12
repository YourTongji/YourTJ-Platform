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
});
