import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { OnboardingPage } from "./onboarding-page";

const apiMocks = vi.hoisted(() => ({ onboarding: vi.fn(), completeOnboarding: vi.fn() }));
const authMocks = vi.hoisted(() => ({ refreshMe: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({
    account: { id: "1", handle: "first-reader", onboardingRequired: true },
    isAuthenticated: true,
    isLoading: false,
    refreshMe: authMocks.refreshMe,
  }),
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/onboarding"]}>
        <OnboardingPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("OnboardingPage", () => {
  beforeEach(() => {
    apiMocks.onboarding.mockReset().mockResolvedValue({
      required: true,
      currentTermsVersion: "2026-07-12",
      acceptedTermsVersion: null,
      handle: "first-reader",
      displayName: null,
      bio: null,
      profileVisibility: "campus",
      activityVisibility: "campus",
      discoverable: true,
      completedAt: null,
    });
    apiMocks.completeOnboarding.mockReset().mockResolvedValue({ required: false });
    authMocks.refreshMe.mockReset().mockResolvedValue(undefined);
  });

  it("requires explicit current-terms acceptance and saves profile and privacy choices together", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    expect(await screen.findByRole("button", { name: "保存并进入社区" })).toBeDisabled();
    await user.type(screen.getByLabelText("显示名称（可选）"), "同济读者");
    await user.type(screen.getByLabelText("简介（可选）"), "关注校园生活");
    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "保存并进入社区" }));

    await waitFor(() => expect(apiMocks.completeOnboarding).toHaveBeenCalledWith({
      handle: "first-reader",
      displayName: "同济读者",
      bio: "关注校园生活",
      profileVisibility: "campus",
      activityVisibility: "campus",
      discoverable: true,
      acceptedTermsVersion: "2026-07-12",
    }));
    expect(authMocks.refreshMe).toHaveBeenCalled();
    await expectNoAccessibilityViolations(view.container);
  });
});
