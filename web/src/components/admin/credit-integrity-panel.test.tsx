import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { CreditIntegrityPanel } from "./credit-integrity-panel";

const apiMocks = vi.hoisted(() => ({
  stats: vi.fn(),
  runs: vi.fn(),
  detail: vi.fn(),
  wallets: vi.fn(),
  request: vi.fn(),
  resume: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminCreditReconciliationStats: apiMocks.stats,
    adminCreditReconciliations: apiMocks.runs,
    adminCreditReconciliation: apiMocks.detail,
    adminCreditReconciliationWallets: apiMocks.wallets,
    requestAdminCreditReconciliation: apiMocks.request,
    resumeAdminCreditReconciliation: apiMocks.resume,
  },
}));

vi.mock("@/lib/random", () => ({ randomUuid: () => "stable-request-id" }));

const healthyRun = {
  id: "0190d8a5-7e4f-7000-8000-000000000001",
  status: "succeeded" as const,
  requestedBy: "7",
  reason: "scheduled verification",
  ledgerOk: true,
  ledgerLatestSeq: 12,
  ledgerLatestHash: "hash",
  ledgerFailureSeq: null,
  walletsChecked: 2,
  driftedWallets: 0,
  missingWallets: 0,
  balanceDriftedWallets: 0,
  sequenceDriftedWallets: 0,
  totalAbsoluteDrift: "0",
  errorCode: null,
  createdAt: 1_720_000_000,
  startedAt: 1_720_000_001,
  completedAt: 1_720_000_002,
};

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <CreditIntegrityPanel />
    </QueryClientProvider>,
  );
}

describe("CreditIntegrityPanel", () => {
  beforeEach(() => {
    apiMocks.stats.mockReset().mockResolvedValue({
      totalRuns: 1,
      failedRuns: 0,
      ledgerFailureRuns: 0,
      runsWithDrift: 0,
      latestRun: healthyRun,
    });
    apiMocks.runs.mockReset().mockResolvedValue({ items: [healthyRun], hasMore: false });
    apiMocks.detail.mockReset().mockResolvedValue(healthyRun);
    apiMocks.wallets.mockReset().mockResolvedValue({ items: [], hasMore: false });
    apiMocks.request.mockReset().mockResolvedValue(healthyRun);
    apiMocks.resume.mockReset().mockResolvedValue(healthyRun);
  });

  it("explains the read-only boundary and submits an audited idempotent run", async () => {
    const user = userEvent.setup();
    const view = renderPanel();

    expect(await screen.findByText("没有发现钱包漂移")).toBeInTheDocument();
    expect(screen.getAllByText(/不会.*修改|绝不会自动改/).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "运行只读检查" }));
    await user.type(screen.getByLabelText("操作原因"), "investigate integrity alert");
    await user.click(screen.getByRole("button", { name: "确认运行只读检查" }));

    await waitFor(() => expect(apiMocks.request).toHaveBeenCalledWith(
      "investigate integrity alert",
      "credit-reconciliation:stable-request-id",
    ));
    await expectNoAccessibilityViolations(view.container);
  });

  it("can resume one persisted active run without creating another request", async () => {
    const activeRun = { ...healthyRun, status: "queued" as const, ledgerOk: null, completedAt: null };
    apiMocks.stats.mockResolvedValue({
      totalRuns: 1,
      failedRuns: 0,
      ledgerFailureRuns: 0,
      runsWithDrift: 0,
      latestRun: activeRun,
    });
    apiMocks.runs.mockResolvedValue({ items: [activeRun], hasMore: false });
    apiMocks.detail.mockResolvedValue(activeRun);
    const user = userEvent.setup();
    renderPanel();

    await user.click(await screen.findByRole("button", { name: "继续未完成检查" }));
    await user.type(screen.getByLabelText("操作原因"), "recover interrupted job");
    await user.click(screen.getByRole("button", { name: "确认继续检查" }));

    await waitFor(() => expect(apiMocks.resume).toHaveBeenCalledWith(
      activeRun.id,
      "recover interrupted job",
    ));
    expect(apiMocks.request).not.toHaveBeenCalled();
  });
});
