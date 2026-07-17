import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMocks = vi.hoisted(() => ({
  wallet: vi.fn(),
  creditSigningIntent: vi.fn(),
  creditSigningIntentOutcome: vi.fn(),
}));

const pendingMocks = vi.hoisted(() => ({
  claim: vi.fn(),
  submit: vi.fn(),
  commit: vi.fn(),
  delete: vi.fn(),
  list: vi.fn(),
  operationKey: vi.fn(),
}));

const walletMocks = vi.hoisted(() => ({
  hasLocalWallet: vi.fn(),
  sign: vi.fn(),
  environmentNamespace: vi.fn(),
  resolveServerKeyState: vi.fn(),
}));

const authMocks = vi.hoisted(() => ({
  readAccessToken: vi.fn(),
  readContextVersion: vi.fn(),
  readStoredAccount: vi.fn(),
}));

const randomMocks = vi.hoisted(() => ({ uuid: vi.fn() }));
const envelopeMocks = vi.hoisted(() => ({ matches: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));

vi.mock("@/lib/wallet-pending", () => ({
  claimWalletPendingMutation: pendingMocks.claim,
  commitWalletPendingMutation: pendingMocks.commit,
  deleteWalletPendingMutation: pendingMocks.delete,
  listWalletPendingMutations: pendingMocks.list,
  submitWalletPendingMutation: pendingMocks.submit,
  walletOperationKey: pendingMocks.operationKey,
}));

vi.mock("@/lib/wallet", () => ({
  hasLocalWallet: walletMocks.hasLocalWallet,
  resolveWalletServerKeyState: walletMocks.resolveServerKeyState,
  signExactBytes: walletMocks.sign,
  walletEnvironmentNamespace: walletMocks.environmentNamespace,
}));

vi.mock("@/lib/auth-storage", () => ({
  readAccessToken: authMocks.readAccessToken,
  readAuthContextVersion: authMocks.readContextVersion,
  readStoredAccount: authMocks.readStoredAccount,
}));

vi.mock("@/lib/random", () => ({ randomUuid: randomMocks.uuid }));
vi.mock("@/lib/wallet-signing-envelope", () => ({
  walletSigningEnvelopeMatches: envelopeMocks.matches,
}));

import { ApiError } from "@/lib/api/client";
import type { WalletPendingMutation } from "@/lib/wallet-pending";
import {
  performWalletMutation,
  reconcileWalletMutations,
  WalletMutationCommittedError,
  WalletMutationUncertainError,
} from "./wallet-mutations";

const accountId = "account-1";
const operationKey = "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const environmentNamespace = "https://api.example.test/api/v2";
const activePublicKey = "active-public-key";
const expiresAt = 1_800_000_300;
const authToken = "access-token-a";
const records = new Map<string, WalletPendingMutation>();

let authContextVersion = 1;
let currentAuthToken = authToken;
let currentAccountId = accountId;
let uuidSequence = 0;
let intentSequence = 0;

function submitted(
  overrides: Partial<WalletPendingMutation> = {},
): WalletPendingMutation {
  return {
    operationKey,
    claimId: "existing-claim",
    action: "credit.tip",
    phase: "submitted",
    intentId: "existing-intent",
    expiresAt,
    ...overrides,
  };
}

function performMutation(
  execute: (authorization: unknown, explicitToken: string) => Promise<unknown>,
) {
  return performWalletMutation(
    accountId,
    "credit.tip",
    { amount: 7, targetId: "thread-42" },
    { kind: "tip" },
    execute,
  );
}

function installSharedPendingStore() {
  pendingMocks.claim.mockImplementation(async (
    _scope: unknown,
    key: string,
    claimId: string,
    action: string,
    claimedAt: number,
  ) => {
    const existing = records.get(key);
    if (existing) {
      const canRecover = existing.phase !== "submitted" && existing.expiresAt <= claimedAt;
      if (!canRecover) return { claimed: false, mutation: existing };
    }
    const mutation: WalletPendingMutation = {
      operationKey: key,
      claimId,
      action,
      phase: "preparing",
      intentId: null,
      expiresAt: claimedAt + 120,
    };
    records.set(key, mutation);
    return { claimed: true, mutation };
  });
  pendingMocks.submit.mockImplementation(async (
    _scope: unknown,
    key: string,
    claimId: string,
    intentId: string,
    intentExpiresAt: number,
  ) => {
    const current = records.get(key);
    if (!current || current.claimId !== claimId || current.phase !== "preparing") return null;
    const mutation: WalletPendingMutation = {
      ...current,
      phase: "submitted",
      intentId,
      expiresAt: intentExpiresAt,
    };
    records.set(key, mutation);
    return mutation;
  });
  pendingMocks.commit.mockImplementation(async (
    _scope: unknown,
    key: string,
    claimId: string,
    intentId: string,
    provedAt: number,
  ) => {
    const current = records.get(key);
    if (!current
      || current.claimId !== claimId
      || current.phase !== "submitted"
      || current.intentId !== intentId) return null;
    const mutation: WalletPendingMutation = {
      ...current,
      phase: "committed",
      expiresAt: provedAt + 300,
    };
    records.set(key, mutation);
    return mutation;
  });
  pendingMocks.delete.mockImplementation(async (
    _scope: unknown,
    key: string,
    claimId: string,
    phase: string,
  ) => {
    const current = records.get(key);
    if (!current || current.claimId !== claimId || current.phase !== phase) return false;
    records.delete(key);
    return true;
  });
  pendingMocks.list.mockImplementation(async () => [...records.values()]);
}

describe("wallet mutation coordinator", () => {
  beforeEach(() => {
    vi.spyOn(Date, "now").mockReturnValue(1_800_000_000_000);
    records.clear();
    for (const mock of Object.values(apiMocks)) mock.mockReset();
    for (const mock of Object.values(pendingMocks)) mock.mockReset();
    for (const mock of Object.values(walletMocks)) mock.mockReset();
    for (const mock of Object.values(authMocks)) mock.mockReset();
    randomMocks.uuid.mockReset();
    envelopeMocks.matches.mockReset().mockResolvedValue(true);

    authContextVersion = 1;
    currentAuthToken = authToken;
    currentAccountId = accountId;
    uuidSequence = 0;
    intentSequence = 0;

    authMocks.readAccessToken.mockImplementation(() => currentAuthToken);
    authMocks.readContextVersion.mockImplementation(() => authContextVersion);
    authMocks.readStoredAccount.mockImplementation(() => ({ id: currentAccountId }));
    randomMocks.uuid.mockImplementation(() => `uuid-${++uuidSequence}`);
    pendingMocks.operationKey.mockResolvedValue(operationKey);
    installSharedPendingStore();

    apiMocks.wallet.mockResolvedValue({ accountId, balance: 100, activePublicKey });
    apiMocks.creditSigningIntent.mockImplementation(async (
      action: string,
      _request: unknown,
      idempotencyKey: string,
      explicitToken: string,
    ) => {
      const intentId = `intent-${++intentSequence}`;
      return {
        intentId,
        expiresAt,
        signingBytes: JSON.stringify({
          version: 1,
          accountId,
          action,
          expiresAt,
          idempotencyKey,
          intentId,
          publicKey: activePublicKey,
          authToken: explicitToken,
        }),
      };
    });
    apiMocks.creditSigningIntentOutcome.mockResolvedValue({
      intentId: "existing-intent",
      status: "pending",
      expiresAt,
    });
    walletMocks.hasLocalWallet.mockResolvedValue(true);
    walletMocks.sign.mockResolvedValue("signature-1");
    walletMocks.environmentNamespace.mockReturnValue(environmentNamespace);
    walletMocks.resolveServerKeyState.mockImplementation(
      (value: unknown, expectedAccountId: string) => {
        const response = value as Record<string, unknown> | null;
        const isKnown = Boolean(response)
          && response?.accountId === expectedAccountId
          && Object.prototype.hasOwnProperty.call(response, "activePublicKey");
        return {
          isKnown,
          activePublicKey: isKnown && typeof response?.activePublicKey === "string"
            ? response.activePublicKey
            : null,
        };
      },
    );
  });

  afterEach(() => vi.restoreAllMocks());

  it("persists submitted state before execute and keeps a commit tombstone after success", async () => {
    const originalSubmit = pendingMocks.submit.getMockImplementation();
    let finishSubmit: (() => void) | undefined;
    pendingMocks.submit.mockImplementationOnce((...args: unknown[]) => new Promise((resolve) => {
      finishSubmit = () => resolve(originalSubmit?.(...args));
    }));
    const execute = vi.fn().mockResolvedValue({ ok: true });

    const result = performMutation(execute);
    await vi.waitFor(() => expect(pendingMocks.submit).toHaveBeenCalledOnce());
    expect(execute).not.toHaveBeenCalled();
    expect(records.get(operationKey)?.phase).toBe("preparing");

    finishSubmit?.();
    await expect(result).resolves.toEqual({ ok: true });
    expect(execute).toHaveBeenCalledWith(
      expect.objectContaining({ intentId: "intent-1" }),
      authToken,
    );
    expect(records.get(operationKey)).toMatchObject({
      phase: "committed",
      intentId: "intent-1",
      expiresAt: 1_800_000_300,
    });
  });

  it("lets only one cross-tab reservation reach intent and execute, then blocks on tombstone", async () => {
    let releaseWallet: (() => void) | undefined;
    apiMocks.wallet.mockImplementationOnce(() => new Promise((resolve) => {
      releaseWallet = () => resolve({ accountId, balance: 100, activePublicKey });
    }));
    const winnerExecute = vi.fn().mockResolvedValue({ ok: true });
    const loserExecute = vi.fn().mockResolvedValue({ ok: true });

    const winner = performMutation(winnerExecute);
    await vi.waitFor(() => expect(apiMocks.wallet).toHaveBeenCalledOnce());
    await expect(performMutation(loserExecute)).rejects.toBeInstanceOf(
      WalletMutationUncertainError,
    );
    expect(apiMocks.creditSigningIntent).not.toHaveBeenCalled();
    expect(loserExecute).not.toHaveBeenCalled();

    releaseWallet?.();
    await expect(winner).resolves.toEqual({ ok: true });
    expect(apiMocks.creditSigningIntent).toHaveBeenCalledOnce();
    expect(winnerExecute).toHaveBeenCalledOnce();

    await expect(performMutation(loserExecute)).rejects.toBeInstanceOf(
      WalletMutationCommittedError,
    );
    expect(apiMocks.creditSigningIntent).toHaveBeenCalledOnce();
    expect(loserExecute).not.toHaveBeenCalled();
  });

  it("retains and blocks a submitted claim while the server reports pending", async () => {
    records.set(operationKey, submitted());
    const execute = vi.fn().mockResolvedValue({ ok: true });

    await expect(performMutation(execute)).rejects.toBeInstanceOf(
      WalletMutationUncertainError,
    );
    expect(apiMocks.creditSigningIntentOutcome).toHaveBeenCalledWith(
      "existing-intent",
      authToken,
    );
    expect(records.get(operationKey)).toEqual(submitted());
    expect(apiMocks.creditSigningIntent).not.toHaveBeenCalled();
    expect(execute).not.toHaveBeenCalled();
  });

  it("turns authoritative committed status into a tombstone without resending", async () => {
    records.set(operationKey, submitted());
    apiMocks.creditSigningIntentOutcome.mockResolvedValueOnce({
      intentId: "existing-intent",
      status: "committed",
      expiresAt,
    });
    const execute = vi.fn().mockResolvedValue({ ok: true });

    await expect(performMutation(execute)).rejects.toBeInstanceOf(
      WalletMutationCommittedError,
    );
    expect(records.get(operationKey)).toMatchObject({ phase: "committed" });
    expect(apiMocks.creditSigningIntent).not.toHaveBeenCalled();
    expect(execute).not.toHaveBeenCalled();
  });

  it("releases an authoritatively expired intent and safely retries once", async () => {
    records.set(operationKey, submitted());
    apiMocks.creditSigningIntentOutcome.mockResolvedValueOnce({
      intentId: "existing-intent",
      status: "expired",
      expiresAt,
    });
    const execute = vi.fn().mockResolvedValue({ ok: true });

    await expect(performMutation(execute)).resolves.toEqual({ ok: true });
    expect(pendingMocks.delete).toHaveBeenCalledWith(
      { environmentNamespace, accountId },
      operationKey,
      "existing-claim",
      "submitted",
    );
    expect(apiMocks.creditSigningIntent).toHaveBeenCalledOnce();
    expect(execute).toHaveBeenCalledOnce();
    expect(records.get(operationKey)?.phase).toBe("committed");
  });

  it("does not resolve or delete an A claim after an A to B to A auth switch", async () => {
    records.set(operationKey, submitted());
    apiMocks.creditSigningIntentOutcome.mockImplementationOnce(async () => {
      currentAuthToken = "access-token-b";
      currentAccountId = "account-2";
      authContextVersion += 1;
      currentAuthToken = authToken;
      currentAccountId = accountId;
      authContextVersion += 1;
      return { intentId: "existing-intent", status: "expired", expiresAt };
    });

    await expect(performMutation(vi.fn())).rejects.toBeInstanceOf(
      WalletMutationUncertainError,
    );
    expect(pendingMocks.delete).not.toHaveBeenCalled();
    expect(pendingMocks.commit).not.toHaveBeenCalled();
    expect(records.get(operationKey)).toEqual(submitted());
  });

  it.each([
    ["intent id", { intentId: "another-intent", status: "expired", expiresAt }],
    ["expiry", { intentId: "existing-intent", status: "expired", expiresAt: expiresAt + 1 }],
    ["status", { intentId: "existing-intent", status: "unknown", expiresAt }],
  ])("fails closed on a malformed or mismatched %s outcome", async (_label, outcome) => {
    records.set(operationKey, submitted());
    apiMocks.creditSigningIntentOutcome.mockResolvedValueOnce(outcome);

    await expect(performMutation(vi.fn())).rejects.toBeInstanceOf(
      WalletMutationUncertainError,
    );
    expect(pendingMocks.delete).not.toHaveBeenCalled();
    expect(pendingMocks.commit).not.toHaveBeenCalled();
    expect(records.get(operationKey)).toEqual(submitted());
  });

  it("releases submitted state after a definite 4xx without querying outcome", async () => {
    const rejection = new ApiError(422, "request rejected", "invalid_request");
    const execute = vi.fn().mockRejectedValue(rejection);

    await expect(performMutation(execute)).rejects.toBe(rejection);
    expect(records.has(operationKey)).toBe(false);
    expect(apiMocks.creditSigningIntentOutcome).not.toHaveBeenCalled();
  });

  it("retains submitted state when a network failure remains pending", async () => {
    const execute = vi.fn().mockRejectedValue(new TypeError("network unavailable"));
    apiMocks.creditSigningIntentOutcome.mockImplementationOnce(async (intentId, explicitToken) => ({
      intentId,
      status: "pending",
      expiresAt,
      explicitToken,
    }));

    await expect(performMutation(execute)).rejects.toBeInstanceOf(
      WalletMutationUncertainError,
    );
    expect(records.get(operationKey)?.phase).toBe("submitted");
    expect(pendingMocks.delete).not.toHaveBeenCalled();
  });

  it("fails before intent when the owner wallet or local key cannot be proven", async () => {
    walletMocks.hasLocalWallet.mockResolvedValueOnce(false);

    await expect(performMutation(vi.fn())).rejects.toThrow(
      "当前环境没有与服务端公钥匹配的本地钱包密钥",
    );
    expect(apiMocks.wallet).toHaveBeenCalledWith(authToken);
    expect(apiMocks.creditSigningIntent).not.toHaveBeenCalled();
    expect(records.has(operationKey)).toBe(false);
  });

  it("never signs or executes an envelope that does not match the confirmed request", async () => {
    envelopeMocks.matches.mockResolvedValueOnce(false);
    const execute = vi.fn().mockResolvedValue({ ok: true });

    await expect(performMutation(execute)).rejects.toThrow(
      "钱包签名 intent 与当前账号、请求或页面确认内容不匹配",
    );
    expect(walletMocks.sign).not.toHaveBeenCalled();
    expect(pendingMocks.submit).not.toHaveBeenCalled();
    expect(execute).not.toHaveBeenCalled();
    expect(records.has(operationKey)).toBe(false);
  });

  it("reconciles submitted records only from lock-aware intent status", async () => {
    records.set(operationKey, submitted());
    apiMocks.creditSigningIntentOutcome.mockResolvedValueOnce({
      intentId: "existing-intent",
      status: "committed",
      expiresAt,
    });

    await expect(reconcileWalletMutations(accountId)).resolves.toEqual({
      resolvedCount: 1,
      unresolvedCount: 0,
    });
    expect(records.get(operationKey)?.phase).toBe("committed");
  });
});
