import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ProfileSidebar } from "./profile-sidebar";

const profile = {
  id: "1",
  handle: "alice",
  displayName: "Alice",
  school: "同济大学",
  bio: null,
  website: null,
  avatarUrl: null,
  bannerUrl: null,
  role: "user" as const,
  trustLevel: 2,
  badges: [],
  verifications: [],
  threadCount: 3,
  commentCount: 5,
  votesReceived: 8,
  followerCount: 2,
  followingCount: 1,
  canViewActivity: true,
  createdAt: 1_700_000_000,
};

describe("ProfileSidebar wallet state", () => {
  it("shows a retryable failure instead of presenting a false zero balance", async () => {
    const user = userEvent.setup();
    const onWalletRetry = vi.fn();
    const view = render(
      <MemoryRouter>
        <ProfileSidebar
          profile={profile}
          isSelf
          walletBalance={null}
          walletError={new Error("request failed")}
          onWalletRetry={onWalletRetry}
        />
      </MemoryRouter>,
    );

    expect(screen.getByText("暂时无法获取余额")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "重试" }));
    expect(onWalletRetry).toHaveBeenCalledOnce();
    await expectNoAccessibilityViolations(view.container);
  });
});
