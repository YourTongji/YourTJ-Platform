import {
  api,
  type CreditSigningIntentOutcome,
  type WalletAuthorization,
} from "@/lib/api/endpoints";
import { ApiError } from "@/lib/api/client";
import {
  readAccessToken,
  readAuthContextVersion,
  readStoredAccount,
} from "@/lib/auth-storage";
import {
  claimWalletPendingMutation,
  commitWalletPendingMutation,
  deleteWalletPendingMutation,
  listWalletPendingMutations,
  submitWalletPendingMutation,
  walletOperationKey,
  type WalletPendingMutation,
  type WalletPendingScope,
} from "@/lib/wallet-pending";
import { randomUuid } from "@/lib/random";
import {
  walletSigningEnvelopeMatches,
  type WalletSigningConfirmation,
} from "@/lib/wallet-signing-envelope";
import {
  hasLocalWallet,
  resolveWalletServerKeyState,
  signExactBytes,
  walletEnvironmentNamespace,
} from "@/lib/wallet";

export type CreditSigningAction =
  | "credit.tip"
  | "credit.task.create"
  | "credit.task.action"
  | "credit.product.purchase"
  | "credit.purchase.action";

interface WalletAuthSnapshot {
  accountId: string;
  authToken: string;
  contextVersion: number;
}

interface PreparedWalletMutation {
  scope: WalletPendingScope;
  pending: WalletPendingMutation;
  authorization: WalletAuthorization;
  auth: WalletAuthSnapshot;
}

type SubmittedResolution = "pending" | "committed" | "expired" | "unknown";

const MAX_EXPIRED_RETRIES = 2;

export class WalletMutationUncertainError extends Error {
  constructor(cause?: unknown) {
    super("上次积分操作的结果仍无法确认；已保留待核验记录并阻止重复提交", { cause });
    this.name = "WalletMutationUncertainError";
  }
}

export class WalletMutationCommittedError extends Error {
  constructor(cause?: unknown) {
    super("积分操作已经提交；页面将刷新服务端状态，不会重复发送", { cause });
    this.name = "WalletMutationCommittedError";
  }
}

export class WalletMutationUnavailableError extends Error {
  constructor(message: string, cause?: unknown) {
    super(message, { cause });
    this.name = "WalletMutationUnavailableError";
  }
}

function nowSeconds() {
  return Math.floor(Date.now() / 1000);
}

function captureAuthSnapshot(accountId: string): WalletAuthSnapshot {
  const contextVersion = readAuthContextVersion();
  const authToken = readAccessToken();
  const storedAccountId = readStoredAccount()?.id;
  if (!authToken || storedAccountId !== accountId || readAuthContextVersion() !== contextVersion) {
    throw new WalletMutationUnavailableError("登录账号已变化，已停止钱包操作");
  }
  return { accountId, authToken, contextVersion };
}

function isAuthSnapshotCurrent(auth: WalletAuthSnapshot) {
  const versionBeforeRead = readAuthContextVersion();
  const isCurrent = versionBeforeRead === auth.contextVersion
    && readAccessToken() === auth.authToken
    && readStoredAccount()?.id === auth.accountId;
  return isCurrent && readAuthContextVersion() === versionBeforeRead;
}

function assertAuthSnapshot(auth: WalletAuthSnapshot, hasDurableClaim: boolean) {
  if (isAuthSnapshotCurrent(auth)) return;
  if (hasDurableClaim) throw new WalletMutationUncertainError();
  throw new WalletMutationUnavailableError("登录账号已变化，已停止钱包操作");
}

async function inAuthContext<TResult>(
  auth: WalletAuthSnapshot,
  hasDurableClaim: boolean,
  operation: () => Promise<TResult>,
) {
  assertAuthSnapshot(auth, hasDurableClaim);
  try {
    const result = await operation();
    assertAuthSnapshot(auth, hasDurableClaim);
    return result;
  } catch (error) {
    assertAuthSnapshot(auth, hasDurableClaim);
    throw error;
  }
}

function validateIntentOutcome(
  value: CreditSigningIntentOutcome,
  pending: WalletPendingMutation,
): SubmittedResolution {
  if (pending.phase !== "submitted"
    || !pending.intentId
    || !value
    || typeof value !== "object"
    || value.intentId !== pending.intentId
    || value.expiresAt !== pending.expiresAt
    || !Number.isSafeInteger(value.expiresAt)
    || value.expiresAt <= 0
    || !(value.status === "pending"
      || value.status === "committed"
      || value.status === "expired")) {
    return "unknown";
  }
  return value.status;
}

async function querySubmittedOutcome(
  pending: WalletPendingMutation,
  auth: WalletAuthSnapshot,
): Promise<SubmittedResolution> {
  if (pending.phase !== "submitted" || !pending.intentId) return "unknown";
  try {
    const outcome = await inAuthContext(auth, true, () => (
      api.creditSigningIntentOutcome(pending.intentId as string, auth.authToken)
    ));
    return validateIntentOutcome(outcome, pending);
  } catch (error) {
    assertAuthSnapshot(auth, true);
    if (error instanceof WalletMutationUncertainError) throw error;
    return "unknown";
  }
}

async function retainCommittedTombstone(
  scope: WalletPendingScope,
  pending: WalletPendingMutation,
  auth: WalletAuthSnapshot,
) {
  if (pending.phase !== "submitted" || !pending.intentId) {
    throw new WalletMutationUncertainError();
  }
  const committed = await inAuthContext(auth, true, () => commitWalletPendingMutation(
    scope,
    pending.operationKey,
    pending.claimId,
    pending.intentId as string,
    nowSeconds(),
  ));
  if (!committed) throw new WalletMutationUncertainError();
  return committed;
}

async function releasePending(
  scope: WalletPendingScope,
  pending: WalletPendingMutation,
  auth: WalletAuthSnapshot,
) {
  const deleted = await inAuthContext(auth, true, () => deleteWalletPendingMutation(
    scope,
    pending.operationKey,
    pending.claimId,
    pending.phase,
  ));
  if (!deleted) throw new WalletMutationUncertainError();
}

async function activePublicKeyFor(auth: WalletAuthSnapshot) {
  const wallet = await inAuthContext(auth, true, () => api.wallet(auth.authToken));
  const serverKeyState = resolveWalletServerKeyState(wallet, auth.accountId);
  if (!serverKeyState.isKnown) {
    throw new WalletMutationUnavailableError("后端尚未提供钱包公钥匹配能力，已停止签名");
  }
  if (!serverKeyState.activePublicKey) {
    throw new WalletMutationUnavailableError("请先生成本地钱包并绑定公钥，再发起积分操作");
  }
  if (!Number.isSafeInteger(wallet.balance)) {
    throw new WalletMutationUnavailableError("服务端钱包余额快照无效，已停止签名");
  }
  return { publicKey: serverKeyState.activePublicKey, balance: wallet.balance };
}

async function resolveExistingClaim(
  scope: WalletPendingScope,
  pending: WalletPendingMutation,
  action: CreditSigningAction,
  auth: WalletAuthSnapshot,
) {
  if (pending.action !== action) throw new WalletMutationUncertainError();
  if (pending.phase === "committed") throw new WalletMutationCommittedError();
  if (pending.phase === "preparing") throw new WalletMutationUncertainError();

  const outcome = await querySubmittedOutcome(pending, auth);
  if (outcome === "committed") {
    await retainCommittedTombstone(scope, pending, auth);
    throw new WalletMutationCommittedError();
  }
  if (outcome === "expired") {
    await releasePending(scope, pending, auth);
    return "retry" as const;
  }
  throw new WalletMutationUncertainError();
}

async function prepareClaim(
  scope: WalletPendingScope,
  pending: WalletPendingMutation,
  accountId: string,
  action: CreditSigningAction,
  request: Record<string, unknown>,
  confirmation: WalletSigningConfirmation,
  auth: WalletAuthSnapshot,
): Promise<PreparedWalletMutation> {
  try {
    const walletSnapshot = await activePublicKeyFor(auth);
    const hasKey = await inAuthContext(auth, true, () => (
      hasLocalWallet(accountId, walletSnapshot.publicKey)
    ));
    if (!hasKey) {
      throw new WalletMutationUnavailableError(
        "当前环境没有与服务端公钥匹配的本地钱包密钥",
      );
    }

    const idempotencyKey = `credit:${randomUuid()}`;
    const intent = await inAuthContext(auth, true, () => (
      api.creditSigningIntent(action, request, idempotencyKey, auth.authToken)
    ));
    if (!intent.intentId
      || !intent.signingBytes
      || !Number.isSafeInteger(intent.expiresAt)
      || intent.expiresAt <= nowSeconds()) {
      throw new WalletMutationUnavailableError("后端没有返回完整且有效的钱包签名 intent");
    }
    const envelopeMatches = await inAuthContext(auth, true, () => walletSigningEnvelopeMatches(
      intent.signingBytes,
      {
        accountId,
        action,
        request,
        confirmation,
        expiresAt: intent.expiresAt,
        idempotencyKey,
        intentId: intent.intentId,
        publicKey: walletSnapshot.publicKey,
        walletBalance: walletSnapshot.balance,
      },
    ));
    if (!envelopeMatches) {
      throw new WalletMutationUnavailableError(
        "钱包签名 intent 与当前账号、请求或页面确认内容不匹配",
      );
    }

    const signature = await inAuthContext(auth, true, () => (
      signExactBytes(accountId, walletSnapshot.publicKey, intent.signingBytes)
    ));
    const submitted = await inAuthContext(auth, true, () => submitWalletPendingMutation(
      scope,
      pending.operationKey,
      pending.claimId,
      intent.intentId,
      intent.expiresAt,
    ));
    if (!submitted) throw new WalletMutationUncertainError();
    return {
      scope,
      pending: submitted,
      authorization: { idempotencyKey, intentId: intent.intentId, signature },
      auth,
    };
  } catch (error) {
    if (error instanceof WalletMutationUncertainError) throw error;
    try {
      await releasePending(scope, pending, auth);
    } catch (cleanupError) {
      throw new WalletMutationUncertainError(cleanupError);
    }
    throw error;
  }
}

function isDefinitiveRejection(error: unknown) {
  return error instanceof ApiError && error.status >= 400 && error.status < 500;
}

async function executePrepared<TResult>(
  prepared: PreparedWalletMutation,
  execute: (authorization: WalletAuthorization, authToken: string) => Promise<TResult>,
) {
  try {
    const result = await inAuthContext(prepared.auth, true, () => (
      execute(prepared.authorization, prepared.auth.authToken)
    ));
    try {
      await retainCommittedTombstone(prepared.scope, prepared.pending, prepared.auth);
    } catch (error) {
      throw new WalletMutationCommittedError(error);
    }
    return { kind: "success" as const, result };
  } catch (error) {
    if (error instanceof WalletMutationCommittedError
      || error instanceof WalletMutationUncertainError) throw error;
    if (isDefinitiveRejection(error)) {
      await releasePending(prepared.scope, prepared.pending, prepared.auth);
      throw error;
    }

    const outcome = await querySubmittedOutcome(prepared.pending, prepared.auth);
    if (outcome === "committed") {
      await retainCommittedTombstone(prepared.scope, prepared.pending, prepared.auth);
      throw new WalletMutationCommittedError(error);
    }
    if (outcome === "expired") {
      await releasePending(prepared.scope, prepared.pending, prepared.auth);
      return { kind: "retry" as const, cause: error };
    }
    throw new WalletMutationUncertainError(error);
  }
}

/**
 * Reserve one operation before network access, then retain it until an
 * owner-authenticated intent outcome authoritatively proves the result.
 */
export async function performWalletMutation<TResult>(
  accountId: string,
  action: CreditSigningAction,
  request: Record<string, unknown>,
  confirmation: WalletSigningConfirmation,
  execute: (authorization: WalletAuthorization, authToken: string) => Promise<TResult>,
) {
  const auth = captureAuthSnapshot(accountId);
  const scope = {
    environmentNamespace: walletEnvironmentNamespace(),
    accountId,
  } satisfies WalletPendingScope;
  const operationKey = await inAuthContext(auth, false, () => (
    walletOperationKey(accountId, action, request)
  ));

  let expiredRetries = 0;
  let lastExpiredCause: unknown;
  while (expiredRetries <= MAX_EXPIRED_RETRIES) {
    const claimId = randomUuid();
    const claim = await inAuthContext(auth, true, () => claimWalletPendingMutation(
      scope,
      operationKey,
      claimId,
      action,
      nowSeconds(),
    ));
    if (!claim.claimed) {
      const resolution = await resolveExistingClaim(scope, claim.mutation, action, auth);
      if (resolution === "retry") {
        expiredRetries += 1;
        continue;
      }
    }

    const prepared = await prepareClaim(
      scope,
      claim.mutation,
      accountId,
      action,
      request,
      confirmation,
      auth,
    );
    const execution = await executePrepared(prepared, execute);
    if (execution.kind === "success") return execution.result;
    lastExpiredCause = execution.cause;
    expiredRetries += 1;
  }
  throw new WalletMutationUnavailableError(
    "服务端签名 intent 反复过期，请稍后重新提交",
    lastExpiredCause,
  );
}

/** Reconcile durable claims for the active environment/account after reload. */
export async function reconcileWalletMutations(accountId: string) {
  const auth = captureAuthSnapshot(accountId);
  const scope = {
    environmentNamespace: walletEnvironmentNamespace(),
    accountId,
  } satisfies WalletPendingScope;
  const pendingMutations = await inAuthContext(auth, false, () => (
    listWalletPendingMutations(scope)
  ));
  let resolvedCount = 0;
  let unresolvedCount = 0;

  for (const pending of pendingMutations) {
    assertAuthSnapshot(auth, true);
    if (pending.phase === "preparing") {
      if (pending.expiresAt <= nowSeconds()) {
        const deleted = await inAuthContext(auth, true, () => deleteWalletPendingMutation(
          scope,
          pending.operationKey,
          pending.claimId,
          "preparing",
        ));
        if (deleted) resolvedCount += 1;
        else unresolvedCount += 1;
      } else {
        unresolvedCount += 1;
      }
      continue;
    }
    if (pending.phase === "committed") {
      if (pending.expiresAt <= nowSeconds()) {
        const deleted = await inAuthContext(auth, true, () => deleteWalletPendingMutation(
          scope,
          pending.operationKey,
          pending.claimId,
          "committed",
        ));
        if (deleted) resolvedCount += 1;
        else unresolvedCount += 1;
      } else {
        resolvedCount += 1;
      }
      continue;
    }

    const outcome = await querySubmittedOutcome(pending, auth);
    if (outcome === "committed") {
      await retainCommittedTombstone(scope, pending, auth);
      resolvedCount += 1;
    } else if (outcome === "expired") {
      await releasePending(scope, pending, auth);
      resolvedCount += 1;
    } else {
      unresolvedCount += 1;
    }
  }
  return { resolvedCount, unresolvedCount };
}
