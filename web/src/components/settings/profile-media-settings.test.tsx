import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ProfileMediaSettings } from "./profile-media-settings";

const apiMocks = vi.hoisted(() => ({
  profile: vi.fn(),
  uploads: vi.fn(),
  mediaUrl: vi.fn(),
  preview: vi.fn(),
  bind: vi.fn(),
  clear: vi.fn(),
}));

vi.mock("@/components/media/media-upload-button", () => ({
  MediaUploadButton: ({ label, onUploaded }: { label: string; onUploaded: () => void }) => (
    <button type="button" onClick={onUploaded}>{label}</button>
  ),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    myProfile: apiMocks.profile,
    myMediaUploads: apiMocks.uploads,
    mediaUrl: apiMocks.mediaUrl,
    myMediaPreview: apiMocks.preview,
    bindMyProfileMedia: apiMocks.bind,
    clearMyProfileMedia: apiMocks.clear,
  },
}));

const originalCreateObjectURL = URL.createObjectURL;
const originalRevokeObjectURL = URL.revokeObjectURL;

function renderSettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ProfileMediaSettings />
    </QueryClientProvider>,
  );
}

describe("ProfileMediaSettings", () => {
  beforeEach(() => {
    apiMocks.profile.mockReset().mockResolvedValue({
      accountId: "1",
      displayName: "Alice",
      bio: null,
      website: null,
      avatarAssetId: "10",
      bannerAssetId: null,
    });
    apiMocks.uploads.mockReset().mockImplementation((usage: string) => Promise.resolve({
      items: usage === "profile_avatar" ? [
        {
          id: "11",
          kind: "image",
          usage,
          bytes: 100,
          mime: "image/png",
          status: "pending",
          deliveryState: "unpublished",
          imageWidth: null,
          imageHeight: null,
          createdAt: 1_720_000_000,
        },
        {
          id: "12",
          kind: "image",
          usage,
          bytes: 100,
          mime: "image/png",
          status: "clean",
          deliveryState: "published",
          imageWidth: 256,
          imageHeight: 256,
          createdAt: 1_719_000_000,
        },
        {
          id: "13",
          kind: "image",
          usage,
          bytes: 100,
          mime: "image/png",
          status: "blocked",
          deliveryState: "blocked",
          imageWidth: null,
          imageHeight: null,
          createdAt: 1_718_000_000,
        },
      ] : [],
      nextCursor: null,
      hasMore: false,
    }));
    apiMocks.mediaUrl.mockReset().mockImplementation((id: string) => Promise.resolve({
      assetId: id,
      url: `https://cdn.example.test/${id}.png`,
      variant: "display_1280",
      expiresAt: Math.floor(Date.now() / 1000) + 300,
      mime: "image/webp",
      width: 256,
      height: 256,
    }));
    apiMocks.preview.mockReset().mockResolvedValue(new Blob(["preview"], { type: "image/png" }));
    apiMocks.bind.mockReset().mockResolvedValue(undefined);
    apiMocks.clear.mockReset().mockResolvedValue(undefined);
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => "blob:owner-preview"),
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: vi.fn(),
    });
  });

  afterEach(() => {
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: originalCreateObjectURL,
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: originalRevokeObjectURL,
    });
  });

  it("keeps pending and rejected uploads unbound while allowing clean apply and removal", async () => {
    const user = userEvent.setup();
    const view = renderSettings();

    expect(await screen.findByText("待审核")).toBeInTheDocument();
    expect(screen.getByText("未通过")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /设为头像/ })).toHaveLength(1);

    await user.click(screen.getByRole("button", { name: /设为头像/ }));
    await waitFor(() => expect(apiMocks.bind).toHaveBeenCalledWith("avatar", "12"));

    await user.click(screen.getByRole("button", { name: "移除当前头像" }));
    await waitFor(() => expect(apiMocks.clear).toHaveBeenCalledWith("avatar"));
    await waitFor(() => expect(apiMocks.preview).toHaveBeenCalledWith("11"));

    await expectNoAccessibilityViolations(view.container);
  });
});
