const DATABASE_NAME = "yourtj-wallet-pending";
const DATABASE_VERSION = 2;
const STORE_NAME = "pendingMutations";
const SCHEMA_VERSION = 2;
const MAX_CANONICAL_REQUEST_BYTES = 64 * 1024;

export const WALLET_PREPARING_LEASE_SECONDS = 2 * 60;
export const WALLET_COMMITTED_TOMBSTONE_SECONDS = 5 * 60;

const ACCOUNT_ID_PATTERN = /^[A-Za-z0-9-]{1,128}$/;
const OPERATION_KEY_PATTERN = /^sha256:[A-Za-z0-9_-]{43}$/;
const CLAIM_ID_PATTERN = /^[A-Za-z0-9-]{1,128}$/;
const ACTION_PATTERN = /^[a-z][a-z0-9._-]{0,127}$/;
const INTENT_ID_PATTERN = /^[A-Za-z0-9-]{1,128}$/;
const STORED_RECORD_KEYS = [
  "accountId",
  "action",
  "claimId",
  "environmentNamespace",
  "expiresAt",
  "intentId",
  "operationKey",
  "phase",
  "schemaVersion",
  "scopeKey",
  "storageKey",
] as const;

export type WalletPendingPhase = "preparing" | "submitted" | "committed";

export interface WalletPendingMutation {
  operationKey: string;
  claimId: string;
  action: string;
  phase: WalletPendingPhase;
  intentId: string | null;
  expiresAt: number;
}

export interface WalletPendingScope {
  environmentNamespace: string;
  accountId: string;
}

interface StoredWalletPendingMutation extends WalletPendingMutation {
  schemaVersion: typeof SCHEMA_VERSION;
  storageKey: string;
  scopeKey: string;
  environmentNamespace: string;
  accountId: string;
}

export interface WalletPendingClaimResult {
  claimed: boolean;
  mutation: WalletPendingMutation;
}

export class WalletPendingStorageError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "WalletPendingStorageError";
  }
}

const scopeOperationTails = new Map<string, Promise<void>>();

function storageError(message: string): WalletPendingStorageError {
  return new WalletPendingStorageError(message);
}

function nowSeconds() {
  return Math.floor(Date.now() / 1000);
}

function validateAccountId(accountId: string) {
  if (!ACCOUNT_ID_PATTERN.test(accountId)) {
    throw storageError("钱包待核验记录的账号范围无效");
  }
}

function hasControlCharacter(value: string) {
  for (const character of value) {
    const characterCode = character.charCodeAt(0);
    if (characterCode <= 31 || characterCode === 127) return true;
  }
  return false;
}

function validateEnvironmentNamespace(environmentNamespace: string) {
  if (environmentNamespace.length === 0
    || environmentNamespace.length > 512
    || environmentNamespace !== environmentNamespace.trim()
    || hasControlCharacter(environmentNamespace)) {
    throw storageError("钱包待核验记录的环境范围无效");
  }
}

function validateScope(scope: WalletPendingScope) {
  validateEnvironmentNamespace(scope.environmentNamespace);
  validateAccountId(scope.accountId);
}

function validateOperationKey(operationKey: string) {
  if (!OPERATION_KEY_PATTERN.test(operationKey)) {
    throw storageError("钱包待核验记录的操作摘要无效");
  }
}

function validateClaimId(claimId: string) {
  if (!CLAIM_ID_PATTERN.test(claimId)) {
    throw storageError("钱包待核验记录的 claim 无效");
  }
}

function validateAction(action: string) {
  if (!ACTION_PATTERN.test(action)) {
    throw storageError("钱包待核验记录的操作类型无效");
  }
}

function validateIntentId(intentId: string) {
  if (!INTENT_ID_PATTERN.test(intentId)) {
    throw storageError("钱包待核验记录的 intent 无效");
  }
}

function validateMutation(mutation: WalletPendingMutation): WalletPendingMutation {
  validateOperationKey(mutation.operationKey);
  validateClaimId(mutation.claimId);
  validateAction(mutation.action);
  if (!(["preparing", "submitted", "committed"] as readonly string[])
    .includes(mutation.phase)) {
    throw storageError("钱包待核验记录的阶段无效");
  }
  if (!Number.isSafeInteger(mutation.expiresAt) || mutation.expiresAt <= 0) {
    throw storageError("钱包待核验记录的期限无效");
  }
  if (mutation.phase === "preparing") {
    if (mutation.intentId !== null) {
      throw storageError("准备中的钱包记录不能包含 intent");
    }
  } else {
    if (typeof mutation.intentId !== "string") {
      throw storageError("已提交的钱包记录缺少 intent");
    }
    validateIntentId(mutation.intentId);
  }
  return {
    operationKey: mutation.operationKey,
    claimId: mutation.claimId,
    action: mutation.action,
    phase: mutation.phase,
    intentId: mutation.intentId,
    expiresAt: mutation.expiresAt,
  };
}

function scopeKey(scope: WalletPendingScope) {
  return `${scope.environmentNamespace}\u0000${scope.accountId}`;
}

function storageKey(scope: WalletPendingScope, operationKey: string) {
  return `${scopeKey(scope)}\u0000${operationKey}`;
}

function enqueueScopeOperation<TResult>(
  scope: WalletPendingScope,
  operation: () => Promise<TResult>,
): Promise<TResult> {
  const operationScopeKey = scopeKey(scope);
  const previous = scopeOperationTails.get(operationScopeKey) ?? Promise.resolve();
  const result = previous.catch(() => undefined).then(operation);
  const tail = result.then(() => undefined, () => undefined);
  scopeOperationTails.set(operationScopeKey, tail);
  void tail.then(() => {
    if (scopeOperationTails.get(operationScopeKey) === tail) {
      scopeOperationTails.delete(operationScopeKey);
    }
  });
  return result;
}

export function isWalletPendingStorageAvailable() {
  return typeof indexedDB !== "undefined" && typeof IDBKeyRange !== "undefined";
}

function openDatabase(): Promise<IDBDatabase> {
  if (!isWalletPendingStorageAvailable()) {
    return Promise.reject(storageError("浏览器不支持安全的待核验钱包记录"));
  }
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(DATABASE_NAME, DATABASE_VERSION);
    request.onupgradeneeded = () => {
      const database = request.result;
      const store = database.objectStoreNames.contains(STORE_NAME)
        ? request.transaction?.objectStore(STORE_NAME)
        : database.createObjectStore(STORE_NAME, { keyPath: "storageKey" });
      if (store && !store.indexNames.contains("scopeKey")) {
        store.createIndex("scopeKey", "scopeKey", { unique: false });
      }
    };
    request.onsuccess = () => {
      request.result.onversionchange = () => request.result.close();
      resolve(request.result);
    };
    request.onerror = () => reject(storageError("无法打开待核验钱包记录"));
    request.onblocked = () => reject(storageError("待核验钱包记录升级被其他页面阻止"));
  });
}

function completeTransaction(transaction: IDBTransaction): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(storageError("待核验钱包记录事务失败"));
    transaction.onabort = () => reject(storageError("待核验钱包记录事务未持久化"));
  });
}

function requestResult<TResult>(request: IDBRequest<TResult>, message: string): Promise<TResult> {
  return new Promise<TResult>((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(storageError(message));
  });
}

function hasExactStoredKeys(value: Record<string, unknown>) {
  const keys = Object.keys(value).sort();
  return keys.length === STORED_RECORD_KEYS.length
    && keys.every((key, index) => key === STORED_RECORD_KEYS[index]);
}

function parseStoredMutation(
  value: unknown,
  expectedScope: WalletPendingScope,
  expectedOperationKey?: string,
): WalletPendingMutation {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw storageError("待核验钱包记录已损坏");
  }
  const stored = value as Record<string, unknown>;
  if (!hasExactStoredKeys(stored)
    || stored.schemaVersion !== SCHEMA_VERSION
    || stored.environmentNamespace !== expectedScope.environmentNamespace
    || stored.accountId !== expectedScope.accountId
    || stored.scopeKey !== scopeKey(expectedScope)
    || typeof stored.operationKey !== "string"
    || (expectedOperationKey !== undefined && stored.operationKey !== expectedOperationKey)
    || stored.storageKey !== storageKey(expectedScope, stored.operationKey)) {
    throw storageError("待核验钱包记录的环境、账号或版本不匹配");
  }
  return validateMutation({
    operationKey: stored.operationKey,
    claimId: stored.claimId as string,
    action: stored.action as string,
    phase: stored.phase as WalletPendingPhase,
    intentId: stored.intentId as string | null,
    expiresAt: stored.expiresAt as number,
  });
}

function storedMutation(
  scope: WalletPendingScope,
  mutation: WalletPendingMutation,
): StoredWalletPendingMutation {
  const validated = validateMutation(mutation);
  return {
    schemaVersion: SCHEMA_VERSION,
    storageKey: storageKey(scope, validated.operationKey),
    scopeKey: scopeKey(scope),
    environmentNamespace: scope.environmentNamespace,
    accountId: scope.accountId,
    ...validated,
  };
}

async function withDatabase<TResult>(operation: (database: IDBDatabase) => Promise<TResult>) {
  const database = await openDatabase();
  try {
    return await operation(database);
  } finally {
    database.close();
  }
}

function mutateRecord<TResult>(
  database: IDBDatabase,
  scope: WalletPendingScope,
  operationKey: string,
  mutation: (
    current: WalletPendingMutation | null,
    store: IDBObjectStore,
  ) => TResult,
): Promise<TResult> {
  const transaction = database.transaction(STORE_NAME, "readwrite");
  const completion = completeTransaction(transaction);
  const store = transaction.objectStore(STORE_NAME);
  const request = store.get(storageKey(scope, operationKey));
  let result: TResult;
  let callbackError: unknown;
  request.onsuccess = () => {
    try {
      const current = request.result === undefined
        ? null
        : parseStoredMutation(request.result, scope, operationKey);
      result = mutation(current, store);
    } catch (error) {
      callbackError = error;
      transaction.abort();
    }
  };
  request.onerror = () => {
    callbackError = storageError("无法读取待核验钱包记录");
  };
  return completion.then(
    () => result,
    (error) => Promise.reject(callbackError ?? error),
  );
}

export function readWalletPendingMutation(scope: WalletPendingScope, operationKey: string) {
  validateScope(scope);
  validateOperationKey(operationKey);
  return enqueueScopeOperation(scope, () => withDatabase(async (database) => {
    const transaction = database.transaction(STORE_NAME, "readonly");
    const completion = completeTransaction(transaction);
    const request = transaction.objectStore(STORE_NAME).get(storageKey(scope, operationKey));
    const [value] = await Promise.all([
      requestResult(request, "无法读取待核验钱包记录"),
      completion,
    ]);
    return value === undefined ? null : parseStoredMutation(value, scope, operationKey);
  }));
}

export function listWalletPendingMutations(scope: WalletPendingScope) {
  validateScope(scope);
  return enqueueScopeOperation(scope, () => withDatabase(async (database) => {
    const transaction = database.transaction(STORE_NAME, "readonly");
    const completion = completeTransaction(transaction);
    const request = transaction
      .objectStore(STORE_NAME)
      .index("scopeKey")
      .getAll(IDBKeyRange.only(scopeKey(scope)));
    const [values] = await Promise.all([
      requestResult(request, "无法列出待核验钱包记录"),
      completion,
    ]);
    return values
      .map((value) => parseStoredMutation(value, scope))
      .sort((left, right) => left.operationKey.localeCompare(right.operationKey));
  }));
}

/** Atomically reserve an operation before any server preflight or intent request. */
export function claimWalletPendingMutation(
  scope: WalletPendingScope,
  operationKey: string,
  claimId: string,
  action: string,
  claimedAt = nowSeconds(),
) {
  validateScope(scope);
  validateOperationKey(operationKey);
  validateClaimId(claimId);
  validateAction(action);
  if (!Number.isSafeInteger(claimedAt) || claimedAt <= 0) {
    throw storageError("钱包 claim 时间无效");
  }
  const preparing = storedMutation(scope, {
    operationKey,
    claimId,
    action,
    phase: "preparing",
    intentId: null,
    expiresAt: claimedAt + WALLET_PREPARING_LEASE_SECONDS,
  });
  return enqueueScopeOperation(scope, () => withDatabase((database) => mutateRecord(
    database,
    scope,
    operationKey,
    (current, store): WalletPendingClaimResult => {
      if (current) {
        if (current.action !== action) {
          throw storageError("钱包待核验记录的操作上下文不匹配");
        }
        const canRecover = current.phase !== "submitted" && current.expiresAt <= claimedAt;
        if (!canRecover) return { claimed: false, mutation: current };
        store.delete(storageKey(scope, operationKey));
      }
      store.add(preparing);
      return { claimed: true, mutation: validateMutation(preparing) };
    },
  )));
}

/** Move a claim to submitted only if the caller still owns the preparing lease. */
export function submitWalletPendingMutation(
  scope: WalletPendingScope,
  operationKey: string,
  claimId: string,
  intentId: string,
  expiresAt: number,
) {
  validateScope(scope);
  validateOperationKey(operationKey);
  validateClaimId(claimId);
  validateIntentId(intentId);
  if (!Number.isSafeInteger(expiresAt) || expiresAt <= 0) {
    throw storageError("钱包 intent 期限无效");
  }
  return enqueueScopeOperation(scope, () => withDatabase((database) => mutateRecord(
    database,
    scope,
    operationKey,
    (current, store) => {
      if (!current || current.claimId !== claimId || current.phase !== "preparing") return null;
      const submitted = storedMutation(scope, {
        ...current,
        phase: "submitted",
        intentId,
        expiresAt,
      });
      store.put(submitted);
      return validateMutation(submitted);
    },
  )));
}

/** Keep authoritative commit proof as a cooldown tombstone against slower tabs. */
export function commitWalletPendingMutation(
  scope: WalletPendingScope,
  operationKey: string,
  claimId: string,
  intentId: string,
  provedAt = nowSeconds(),
) {
  validateScope(scope);
  validateOperationKey(operationKey);
  validateClaimId(claimId);
  validateIntentId(intentId);
  if (!Number.isSafeInteger(provedAt) || provedAt <= 0) {
    throw storageError("钱包提交证明时间无效");
  }
  return enqueueScopeOperation(scope, () => withDatabase((database) => mutateRecord(
    database,
    scope,
    operationKey,
    (current, store) => {
      if (!current
        || current.claimId !== claimId
        || current.phase !== "submitted"
        || current.intentId !== intentId) return null;
      const committed = storedMutation(scope, {
        ...current,
        phase: "committed",
        expiresAt: provedAt + WALLET_COMMITTED_TOMBSTONE_SECONDS,
      });
      store.put(committed);
      return validateMutation(committed);
    },
  )));
}

/** Delete only the exact claim and phase observed by the caller. */
export function deleteWalletPendingMutation(
  scope: WalletPendingScope,
  operationKey: string,
  claimId: string,
  expectedPhase: WalletPendingPhase,
) {
  validateScope(scope);
  validateOperationKey(operationKey);
  validateClaimId(claimId);
  return enqueueScopeOperation(scope, () => withDatabase((database) => mutateRecord(
    database,
    scope,
    operationKey,
    (current, store) => {
      if (!current || current.claimId !== claimId || current.phase !== expectedPhase) return false;
      store.delete(storageKey(scope, operationKey));
      return true;
    },
  )));
}

const OMIT_OBJECT_PROPERTY = Symbol("omit-object-property");

function canonicalJsonValue(value: unknown, ancestors: Set<object>, inArray: boolean): unknown {
  if (value === undefined) return inArray ? null : OMIT_OBJECT_PROPERTY;
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw storageError("钱包操作请求含有非有限数字");
    return Object.is(value, -0) ? 0 : value;
  }
  if (typeof value !== "object") {
    throw storageError("钱包操作请求不是可序列化的 JSON");
  }
  if (ancestors.has(value)) throw storageError("钱包操作请求不能包含循环引用");
  ancestors.add(value);
  try {
    if (Array.isArray(value)) {
      return value.map((item) => canonicalJsonValue(item, ancestors, true));
    }
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) {
      throw storageError("钱包操作请求只能包含普通 JSON 对象");
    }
    const result: Record<string, unknown> = {};
    for (const key of Object.keys(value).sort()) {
      const normalized = canonicalJsonValue((value as Record<string, unknown>)[key], ancestors, false);
      if (normalized !== OMIT_OBJECT_PROPERTY) result[key] = normalized;
    }
    return result;
  } finally {
    ancestors.delete(value);
  }
}

function digestBase64Url(bytes: Uint8Array) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

export async function walletOperationKey(
  accountId: string,
  action: string,
  request: Record<string, unknown>,
) {
  validateAccountId(accountId);
  validateAction(action);
  if (!request || typeof request !== "object" || Array.isArray(request)) {
    throw storageError("钱包操作请求必须是 JSON 对象");
  }
  const canonical = JSON.stringify({
    accountId,
    action,
    request: canonicalJsonValue(request, new Set<object>(), false),
  });
  if (new TextEncoder().encode(canonical).byteLength > MAX_CANONICAL_REQUEST_BYTES) {
    throw storageError("钱包操作请求超过本地核验摘要上限");
  }
  if (typeof crypto === "undefined" || !crypto.subtle) {
    throw storageError("浏览器不支持钱包操作摘要");
  }
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonical));
  return `sha256:${digestBase64Url(new Uint8Array(digest))}`;
}
