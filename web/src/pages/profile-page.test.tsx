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
  });

  it("uses relationship and list APIs for follow, mute, block, and counts", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    expect(await screen.findByRole("heading", { name: "Bob Builder", level: 1 })).toBeVisible();
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
});
