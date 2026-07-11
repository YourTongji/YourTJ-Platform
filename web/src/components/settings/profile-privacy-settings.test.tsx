import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ProfilePrivacySettings } from "./profile-privacy-settings";

const apiMocks = vi.hoisted(() => ({
  profile: vi.fn(),
  privacy: vi.fn(),
  updateProfile: vi.fn(),
  updatePrivacy: vi.fn(),
  updateMe: vi.fn(),
  refreshMe: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({
    account: { id: "1", handle: "alice", role: "user" },
    refreshMe: apiMocks.refreshMe,
  }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    myProfile: apiMocks.profile,
    myPrivacy: apiMocks.privacy,
    updateMyProfile: apiMocks.updateProfile,
    updateMyPrivacy: apiMocks.updatePrivacy,
    updateMe: apiMocks.updateMe,
  },
}));

function renderSettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ProfilePrivacySettings />
    </QueryClientProvider>,
  );
}

describe("ProfilePrivacySettings", () => {
  beforeEach(() => {
    apiMocks.profile.mockReset().mockResolvedValue({
      accountId: "1",
      displayName: "Alice",
      bio: "Hello campus",
      website: "https://alice.example.test",
      avatarAssetId: null,
      bannerAssetId: null,
    });
    apiMocks.privacy.mockReset().mockResolvedValue({
      profileVisibility: "campus",
      followersVisibility: "followers",
      followingVisibility: "followers",
      discoverable: true,
      dmPolicy: "following",
    });
    apiMocks.updateProfile.mockReset().mockResolvedValue({});
    apiMocks.updatePrivacy.mockReset().mockResolvedValue({});
    apiMocks.updateMe.mockReset().mockResolvedValue({});
    apiMocks.refreshMe.mockReset().mockResolvedValue(undefined);
  });

  it("persists controlled text fields and conservative privacy settings", async () => {
    const user = userEvent.setup();
    const view = renderSettings();

    const displayName = await screen.findByRole("textbox", { name: "显示名称" });
    await waitFor(() => expect(displayName).toHaveValue("Alice"));
    await user.clear(displayName);
    await user.type(displayName, "Alice Chen");
    await user.click(screen.getByRole("button", { name: "保存公开资料" }));
    await waitFor(() => expect(apiMocks.updateProfile).toHaveBeenCalledWith({
      displayName: "Alice Chen",
      bio: "Hello campus",
      website: "https://alice.example.test",
    }));

    const discoverability = await screen.findByRole("switch", { name: "允许被发现" });
    await user.click(discoverability);
    await user.click(screen.getByRole("button", { name: "保存隐私设置" }));
    await waitFor(() => expect(apiMocks.updatePrivacy).toHaveBeenCalledWith({
      profileVisibility: "campus",
      followersVisibility: "followers",
      followingVisibility: "followers",
      discoverable: false,
      dmPolicy: "following",
    }));

    expect(screen.queryByLabelText("头像 URL")).not.toBeInTheDocument();
    await expectNoAccessibilityViolations(view.container);
  });
});
