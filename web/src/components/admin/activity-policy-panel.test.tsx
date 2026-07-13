import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ActivityPolicyPanel } from "./activity-policy-panel";

const apiMocks = vi.hoisted(() => ({
  activityPolicy: vi.fn(),
  activityPolicyHistory: vi.fn(),
  trustPolicy: vi.fn(),
  trustPolicyHistory: vi.fn(),
  updateActivityPolicy: vi.fn(),
  updateTrustPolicy: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminActivityPolicy: apiMocks.activityPolicy,
    adminActivityPolicyHistory: apiMocks.activityPolicyHistory,
    adminTrustPolicy: apiMocks.trustPolicy,
    adminTrustPolicyHistory: apiMocks.trustPolicyHistory,
    updateAdminActivityPolicy: apiMocks.updateActivityPolicy,
    updateAdminTrustPolicy: apiMocks.updateTrustPolicy,
  },
}));

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ActivityPolicyPanel />
    </QueryClientProvider>,
  );
}

describe("ActivityPolicyPanel", () => {
  beforeEach(() => {
    apiMocks.activityPolicy.mockReset().mockResolvedValue({
      version: 2,
      timezone: "Asia/Shanghai",
      weights: { thread: 10, comment: 3, like: 2, checkIn: 4 },
      reason: "current score policy",
      changedBy: "1",
      createdAt: 1_700_000_000,
    });
    apiMocks.activityPolicyHistory.mockReset().mockResolvedValue({
      items: [],
      nextCursor: null,
      hasMore: false,
    });
    apiMocks.trustPolicy.mockReset().mockResolvedValue({
      version: 3,
      scorePolicyVersion: 2,
      thresholdLevel2: 30,
      thresholdLevel3: 120,
      thresholdLevel4: 400,
      thresholdLevel5: 1_200,
      thresholdLevel6: 3_000,
      likeDailyCap: 6,
      demotionCooldownDays: 14,
      reason: "current trust policy",
      changedBy: "1",
      createdAt: 1_700_000_000,
    });
    apiMocks.trustPolicyHistory.mockReset().mockResolvedValue({
      items: [],
      nextCursor: null,
      hasMore: false,
    });
    apiMocks.updateActivityPolicy.mockReset();
    apiMocks.updateTrustPolicy.mockReset();
  });

  it("applies the daily like cap and constrains check-in samples to zero or one", async () => {
    renderPanel();

    expect(await screen.findByText(/min\(5 × 2, 6\)/)).toHaveTextContent("= 29 分");
    const checkIn = screen.getByLabelText("签到");
    expect(checkIn).toHaveAttribute("max", "1");

    fireEvent.change(checkIn, { target: { value: "9" } });
    expect(checkIn).toHaveValue(1);
    expect(screen.getByText(/min\(5 × 2, 6\)/)).toHaveTextContent("= 29 分");
  });
});
