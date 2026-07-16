import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const authState = vi.hoisted(() => ({ accountId: "account-a", isAuthenticated: true }));
const authStorageState = vi.hoisted(() => ({
  accountId: "account-a",
  accessToken: "access-a",
  contextVersion: 1,
}));
const apiMocks = vi.hoisted(() => ({
  bindWallet: vi.fn(),
  claimChallenge: vi.fn(),
  claimWallet: vi.fn(),
  ledger: vi.fn(),
  products: vi.fn(),
  purchases: vi.fn(),
  taskAction: vi.fn(),
  tasks: vi.fn(),
  verifyLedger: vi.fn(),
  wallet: vi.fn(),
}));
const walletMocks = vi.hoisted(() => ({
  clear: vi.fn(),
  create: vi.fn(),
  discardLegacy: vi.fn(),
  get: vi.fn(),
  inspectLegacy: vi.fn(),
  resolveServerKeyState: vi.fn(),
}));
const walletMutationMocks = vi.hoisted(() => ({
  perform: vi.fn(),
  reconcile: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({
    account: authState.isAuthenticated ? { id: authState.accountId } : null,
    isAuthenticated: authState.isAuthenticated,
  }),
}));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));

vi.mock("@/lib/auth-storage", () => ({
  readAccessToken: () => authStorageState.accessToken,
  readAuthContextVersion: () => authStorageState.contextVersion,
  readStoredAccount: () => ({ id: authStorageState.accountId }),
}));

vi.mock("@/lib/wallet", () => ({
  clearLocalWallet: walletMocks.clear,
  createLocalWallet: walletMocks.create,
  discardLegacyWallet: walletMocks.discardLegacy,
  getLocalWallet: walletMocks.get,
  inspectLegacyWallet: walletMocks.inspectLegacy,
  resolveWalletServerKeyState: walletMocks.resolveServerKeyState,
}));

vi.mock("@/lib/wallet-mutations", () => {
  class WalletMutationCommittedError extends Error {}
  class WalletMutationUncertainError extends Error {}
  return {
    performWalletMutation: walletMutationMocks.perform,
    reconcileWalletMutations: walletMutationMocks.reconcile,
    WalletMutationCommittedError,
    WalletMutationUncertainError,
  };
});

vi.mock("@/components/auth/recent-auth-dialog", () => ({
  RecentAuthDialog: ({
    open,
    onVerified,
  }: {
    open: boolean;
    onVerified: () => void;
  }) => open ? <button onClick={onVerified}>完成近期验证</button> : null,
}));

import { WalletMutationUncertainError } from "@/lib/wallet-mutations";
import { WalletPage, WalletSetup } from "./wallet-page";

function renderSetup(serverPublicKey: string | null, isServerStateKnown = true) {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  const view = render(
    <QueryClientProvider client={queryClient}>
      <WalletSetup
        serverPublicKey={serverPublicKey}
        isServerStateKnown={isServerStateKnown}
      />
    </QueryClientProvider>,
  );
  return { ...view, queryClient };
}

function createQueryClient() {
  return new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
}

function renderPage(queryClient = createQueryClient()) {
  const view = render(
    <QueryClientProvider client={queryClient}>
      <WalletPage />
    </QueryClientProvider>,
  );
  return { ...view, queryClient };
}

describe("WalletSetup server and account boundaries", () => {
  beforeEach(() => {
    authState.accountId = "account-a";
    authState.isAuthenticated = true;
    authStorageState.accountId = "account-a";
    authStorageState.accessToken = "access-a";
    authStorageState.contextVersion = 1;
    for (const mock of Object.values(apiMocks)) mock.mockReset();
    for (const mock of Object.values(walletMocks)) mock.mockReset();
    for (const mock of Object.values(walletMutationMocks)) mock.mockReset();
    walletMocks.inspectLegacy.mockReturnValue(null);
    walletMocks.get.mockResolvedValue(null);
    walletMocks.resolveServerKeyState.mockReturnValue({
      isKnown: true,
      activePublicKey: null,
    });
    apiMocks.wallet.mockResolvedValue({
      accountId: "account-a",
      activePublicKey: null,
      balance: 100,
    });
    apiMocks.ledger.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.products.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.purchases.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.tasks.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.verifyLedger.mockResolvedValue({ ok: true, latestSeq: null, latestHash: null });
    walletMutationMocks.reconcile.mockResolvedValue({ resolvedCount: 0, unresolvedCount: 0 });
  });

  it("does not offer key creation while the server key field is unavailable", () => {
    renderSetup(null, false);

    expect(screen.queryByRole("button", { name: "生成本地钱包" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "绑定公钥" })).not.toBeInTheDocument();
    expect(walletMocks.get).not.toHaveBeenCalled();
  });

  it("offers generation only after an explicit unbound server response", async () => {
    walletMocks.get.mockResolvedValue(null);
    renderSetup(null);

    expect(await screen.findByRole("button", { name: "生成本地钱包" })).toBeEnabled();
    expect(walletMocks.get).toHaveBeenCalledWith("account-a", null);
  });

  it("shows a local key only when it exactly matches the bound server key", async () => {
    walletMocks.get.mockResolvedValue({ publicKey: "public-key-a" });
    renderSetup("public-key-a");

    expect(await screen.findByText("public-key-a")).toBeInTheDocument();
    expect(screen.getByText("已与服务端匹配")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "绑定公钥" })).not.toBeInTheDocument();
  });

  it("keeps the verified account token fixed when an A to B switch races bind dispatch", async () => {
    walletMocks.get.mockResolvedValue({ publicKey: "public-key-a" });
    apiMocks.bindWallet.mockImplementation(async (
      accountId: string,
      _publicKey: string,
      authToken: string,
    ) => {
      expect(accountId).toBe("account-a");
      expect(authToken).toBe("access-a");
      authStorageState.accountId = "account-b";
      authStorageState.accessToken = "access-b";
      authStorageState.contextVersion += 1;
    });
    const user = userEvent.setup();
    const view = renderSetup(null);
    const refetchQueries = vi.spyOn(view.queryClient, "refetchQueries");

    await screen.findByText("public-key-a");
    await user.click(screen.getByRole("button", { name: "绑定公钥" }));
    await user.click(screen.getByRole("button", { name: "完成近期验证" }));

    await waitFor(() => expect(apiMocks.wallet).toHaveBeenCalledWith("access-a"));
    await waitFor(() => expect(apiMocks.bindWallet).toHaveBeenCalledWith(
      "account-a",
      "public-key-a",
      "access-a",
    ));
    await act(async () => Promise.resolve());
    expect(refetchQueries).not.toHaveBeenCalled();
  });

  it("cancels a pending bind when the authenticated account changes", async () => {
    walletMocks.get.mockImplementation(async (accountId: string) => ({
      publicKey: accountId === "account-a" ? "public-key-a" : "public-key-b",
    }));
    const user = userEvent.setup();
    const view = renderSetup(null);

    await screen.findByText("public-key-a");
    await user.click(screen.getByRole("button", { name: "绑定公钥" }));
    expect(screen.getByRole("button", { name: "完成近期验证" })).toBeInTheDocument();

    act(() => {
      authState.accountId = "account-b";
      view.rerender(
        <QueryClientProvider client={new QueryClient()}>
          <WalletSetup serverPublicKey={null} isServerStateKnown />
        </QueryClientProvider>,
      );
    });

    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "完成近期验证" })).not.toBeInTheDocument();
    });
    expect(screen.queryByText("public-key-a")).not.toBeInTheDocument();
    expect(apiMocks.bindWallet).not.toHaveBeenCalled();
  });
});

describe("WalletPage pending reconciliation", () => {
  beforeEach(() => {
    authState.accountId = "account-a";
    authState.isAuthenticated = true;
    authStorageState.accountId = "account-a";
    authStorageState.accessToken = "access-a";
    authStorageState.contextVersion = 1;
    for (const mock of Object.values(apiMocks)) mock.mockReset();
    for (const mock of Object.values(walletMocks)) mock.mockReset();
    for (const mock of Object.values(walletMutationMocks)) mock.mockReset();
    walletMocks.inspectLegacy.mockReturnValue(null);
    walletMocks.get.mockResolvedValue(null);
    walletMocks.resolveServerKeyState.mockReturnValue({
      isKnown: true,
      activePublicKey: null,
    });
    apiMocks.wallet.mockResolvedValue({
      accountId: "account-a",
      activePublicKey: null,
      balance: 100,
    });
    apiMocks.ledger.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.products.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.purchases.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.tasks.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.verifyLedger.mockResolvedValue({ ok: true, latestSeq: null, latestHash: null });
    walletMutationMocks.reconcile.mockResolvedValue({ resolvedCount: 0, unresolvedCount: 0 });
  });

  it("re-runs reconciliation when the same account re-enters the page", async () => {
    const queryClient = createQueryClient();
    const firstView = renderPage(queryClient);
    await waitFor(() => expect(walletMutationMocks.reconcile).toHaveBeenCalledTimes(1));
    firstView.unmount();

    renderPage(queryClient);

    await waitFor(() => expect(walletMutationMocks.reconcile).toHaveBeenCalledTimes(2));
  });

  it("re-runs reconciliation immediately after an uncertain mutation result", async () => {
    apiMocks.tasks.mockResolvedValue({
      items: [{
        id: "1",
        creatorId: "account-a",
        acceptorId: null,
        title: "Task",
        description: null,
        rewardAmount: 10,
        contactInfo: null,
        status: "open",
        createdAt: 1_800_000_000,
      }],
      nextCursor: null,
      hasMore: false,
    });
    walletMutationMocks.perform.mockRejectedValueOnce(new WalletMutationUncertainError());
    const user = userEvent.setup();
    renderPage();
    await waitFor(() => expect(walletMutationMocks.reconcile).toHaveBeenCalledTimes(1));

    await user.click(await screen.findByRole("button", { name: "取消并退款" }));

    await waitFor(() => expect(walletMutationMocks.perform).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(walletMutationMocks.reconcile).toHaveBeenCalledTimes(2));
  });

  it("discards a consumed legacy challenge after a failed proof", async () => {
    const challengeId = "019f60d7-0ed8-4bc0-9843-8d101c990526";
    apiMocks.claimChallenge.mockResolvedValue({ challengeId, nonce: "claim-nonce" });
    apiMocks.claimWallet.mockRejectedValue(new Error("wallet claim proof is invalid"));
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "获取挑战" }));
    await screen.findByText(`challengeId: ${challengeId}`);
    await user.type(screen.getByLabelText("legacyUserHash"), "a".repeat(64));
    await user.type(
      screen.getByLabelText("旧钱包签名"),
      `${"A".repeat(86)}==`,
    );
    await user.click(screen.getByRole("button", { name: "认领并合并余额" }));

    await waitFor(() => expect(apiMocks.claimWallet).toHaveBeenCalledTimes(1));
    await waitFor(() => {
      expect(screen.queryByText(`challengeId: ${challengeId}`)).not.toBeInTheDocument();
    });
    expect(screen.getByLabelText("旧钱包签名")).toHaveValue("");
  });

  it("deletes a cancelled task without creating an unused wallet intent", async () => {
    apiMocks.tasks.mockResolvedValue({
      items: [{
        id: "1",
        creatorId: "account-a",
        acceptorId: null,
        title: "Cancelled task",
        description: null,
        rewardAmount: 10,
        contactInfo: null,
        status: "cancelled",
        createdAt: 1_800_000_000,
      }],
      nextCursor: null,
      hasMore: false,
    });
    apiMocks.taskAction.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "删除任务" }));

    await waitFor(() => expect(apiMocks.taskAction).toHaveBeenCalledWith("1", "delete"));
    expect(walletMutationMocks.perform).not.toHaveBeenCalled();
  });

  it.each([
    { label: "id is missing", patch: { id: undefined } },
    { label: "seller id is invalid", patch: { sellerId: "" } },
    { label: "price is invalid", patch: { price: 0 } },
    { label: "stock is unavailable", patch: { stock: 0 } },
    { label: "status is not on sale", patch: { status: "sold_out" } },
  ])("keeps purchase disabled when product $label", async ({ patch }) => {
    apiMocks.products.mockResolvedValue({
      items: [{
        id: "7",
        sellerId: "2",
        title: "Textbook",
        description: null,
        price: 10,
        stock: 1,
        status: "on_sale",
        createdAt: 1_800_000_000,
        ...patch,
      }],
      nextCursor: null,
      hasMore: false,
    });
    const user = userEvent.setup();
    renderPage();
    await user.click(await screen.findByRole("tab", { name: "商品托管" }));

    expect(await screen.findByRole("button", { name: "购买并托管" })).toBeDisabled();
    expect(walletMutationMocks.perform).not.toHaveBeenCalled();
  });
});
