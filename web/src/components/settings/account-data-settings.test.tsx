import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { AccountDataSettings } from "./account-data-settings";

const apiMocks = vi.hoisted(() => ({
  accountLifecycle: vi.fn(),
  dataExports: vi.fn(),
  createDataExport: vi.fn(),
  dataExport: vi.fn(),
  createDataExportDownloadGrant: vi.fn(),
  downloadDataExport: vi.fn(),
  deactivateAccount: vi.fn(),
  deleteAccount: vi.fn(),
}));
const authMocks = vi.hoisted(() => ({ clearSession: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/context/auth-provider", () => ({ useAuth: () => authMocks }));
vi.mock("@/components/auth/recent-auth-dialog", () => ({
  RecentAuthDialog: ({ open, onVerified }: { open: boolean; onVerified: () => void }) =>
    open ? <button type="button" onClick={onVerified}>完成安全验证</button> : null,
}));

function renderSettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <AccountDataSettings />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("AccountDataSettings", () => {
  beforeEach(() => {
    sessionStorage.clear();
    apiMocks.accountLifecycle.mockReset().mockResolvedValue({ state: "active" });
    apiMocks.dataExports.mockReset().mockResolvedValue([]);
    apiMocks.createDataExport.mockReset().mockResolvedValue({
      id: "export-1",
      status: "ready",
      createdAt: 1_700_000_000,
      updatedAt: 1_700_000_000,
      expiresAt: 4_100_000_000,
      errorCode: null,
    });
    apiMocks.dataExport.mockReset().mockResolvedValue({
      id: "export-1",
      status: "ready",
      createdAt: 1_700_000_000,
      updatedAt: 1_700_000_000,
      expiresAt: 4_100_000_000,
      errorCode: null,
    });
    apiMocks.createDataExportDownloadGrant.mockReset();
    apiMocks.downloadDataExport.mockReset();
    apiMocks.deleteAccount.mockReset();
    apiMocks.deactivateAccount.mockReset();
    authMocks.clearSession.mockReset();
  });

  it("requires recent authentication before starting a durable owner export", async () => {
    const user = userEvent.setup();
    const view = renderSettings();

    await user.click(await screen.findByRole("button", { name: "创建导出" }));
    expect(apiMocks.createDataExport).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "完成安全验证" }));

    await waitFor(() => expect(apiMocks.createDataExport).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("可下载")).toBeVisible();
    await expectNoAccessibilityViolations(view.container);
  });

  it("requires an exact destructive confirmation before the recent-auth step", async () => {
    const user = userEvent.setup();
    renderSettings();

    await user.click(await screen.findByRole("button", { name: "申请删除" }));
    const continueButton = screen.getByRole("button", { name: "继续安全验证" });
    expect(continueButton).toBeDisabled();
    await user.type(screen.getByLabelText("输入“删除账号”以继续"), "删除账号");
    expect(continueButton).toBeEnabled();
    await user.click(continueButton);
    expect(screen.getByRole("button", { name: "完成安全验证" })).toBeVisible();
    expect(apiMocks.deleteAccount).not.toHaveBeenCalled();
  });

  it("requires recent authentication again before issuing a download grant", async () => {
    const user = userEvent.setup();
    sessionStorage.setItem("yourtj.latestDataExport", "export-1");
    apiMocks.createDataExportDownloadGrant.mockResolvedValue({ token: "grant", expiresAt: 100 });
    apiMocks.downloadDataExport.mockReturnValue(new Promise(() => undefined));
    renderSettings();

    await user.click(await screen.findByRole("button", { name: "一次性下载" }));
    expect(apiMocks.createDataExportDownloadGrant).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "完成安全验证" }));

    await waitFor(() =>
      expect(apiMocks.createDataExportDownloadGrant).toHaveBeenCalledWith("export-1"),
    );
    expect(apiMocks.downloadDataExport).toHaveBeenCalledWith("export-1", "grant");
  });
});
