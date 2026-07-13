import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ProfileRelationshipListDialog } from "./profile-relationship-list-dialog";

const apiMocks = vi.hoisted(() => ({
  followers: vi.fn(),
  following: vi.fn(),
  removeFollower: vi.fn(),
}));
const avatarMock = vi.hoisted(() => ({
  onLoadingStatusChange: undefined as undefined | ((status: "error") => void),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    userFollowers: apiMocks.followers,
    userFollowing: apiMocks.following,
    removeFollower: apiMocks.removeFollower,
  },
}));

vi.mock("@/components/ui/avatar", () => ({
  Avatar: ({ children }: PropsWithChildren) => <div>{children}</div>,
  AvatarFallback: ({ children }: PropsWithChildren) => <span>{children}</span>,
  AvatarImage: ({
    onLoadingStatusChange,
    src,
  }: { onLoadingStatusChange?: (status: "error") => void; src?: string }) => {
    avatarMock.onLoadingStatusChange = onLoadingStatusChange;
    return <span data-testid="signed-avatar" data-src={src} />;
  },
}));

function renderDialog(canRemoveFollowers: boolean) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <ProfileRelationshipListDialog
          handle="alice"
          kind="followers"
          open
          canRemoveFollowers={canRemoveFollowers}
          onOpenChange={vi.fn()}
        />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("ProfileRelationshipListDialog", () => {
  beforeEach(() => {
    avatarMock.onLoadingStatusChange = undefined;
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
    apiMocks.following.mockReset().mockResolvedValue({ items: [], hasMore: false });
    apiMocks.removeFollower.mockReset().mockResolvedValue(undefined);
  });

  it("lets the profile owner remove a follower without presenting the action as a block", async () => {
    const user = userEvent.setup();
    const view = renderDialog(true);

    expect(await screen.findByText("Carol")).toBeVisible();
    expect(screen.getByText(/对方以后仍可重新关注你/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "移除 @carol 关注者" }));
    await waitFor(() => expect(apiMocks.removeFollower).toHaveBeenCalledWith("carol"));
    await expectNoAccessibilityViolations(view.container);
  });

  it("does not expose owner-only controls on someone else's follower list", async () => {
    renderDialog(false);
    expect(await screen.findByText("Carol")).toBeVisible();
    expect(screen.queryByRole("button", { name: /移除 @carol/ })).not.toBeInTheDocument();
  });

  it("refetches the owning relationship page when a signed avatar expires", async () => {
    apiMocks.followers.mockResolvedValue({
      items: [{
        id: "3",
        handle: "carol",
        displayName: "Carol",
        avatarUrl: "https://cdn.example/avatar.webp?auth_key=old",
        role: "user",
        followedAt: 1_700_000_100,
      }],
      hasMore: false,
      nextCursor: null,
    });
    renderDialog(false);

    await screen.findByText("Carol");
    expect(screen.getByTestId("signed-avatar")).toHaveAttribute(
      "data-src",
      "https://cdn.example/avatar.webp?auth_key=old",
    );
    act(() => {
      avatarMock.onLoadingStatusChange?.("error");
    });

    await waitFor(() => expect(apiMocks.followers).toHaveBeenCalledTimes(2));
  });
});
