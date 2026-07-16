import { ed25519 } from "@noble/curves/ed25519";

const LEGACY_WALLET_SEED_KEY = "yourtj.walletSeed";
const WALLET_DATABASE_NAME = "yourtj.wallet.v1";
const WALLET_DATABASE_VERSION = 1;
const WALLET_OBJECT_STORE = "accountKeys";
const WALLET_RECORD_VERSION = 1;
const ED25519_KEY_LENGTH = 32;
const ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);
const KEY_CHECK_BYTES = new TextEncoder().encode("yourtj.wallet.key-check.v1");

export interface LocalWallet {
  publicKey: string;
}

export interface WalletServerKeyState {
  isKnown: boolean;
  activePublicKey: string | null;
}

interface StoredWalletKey {
  version: typeof WALLET_RECORD_VERSION;
  scopeKey: string;
  environment: string;
  accountId: string;
  publicKey: string;
  privateKey: CryptoKey;
  createdAt: number;
}

export class WalletStorageUnavailableError extends Error {
  constructor(cause?: unknown) {
    super("当前浏览器无法安全访问本地钱包密钥", { cause });
    this.name = "WalletStorageUnavailableError";
  }
}

export class WalletKeyUnavailableError extends Error {
  constructor() {
    super("当前账号没有可用的本地钱包密钥");
    this.name = "WalletKeyUnavailableError";
  }
}

export class WalletKeyMismatchError extends Error {
  readonly expectedPublicKey: string;
  readonly actualPublicKey: string | null;

  constructor(expectedPublicKey: string, actualPublicKey: string | null, cause?: unknown) {
    super("服务端绑定的钱包公钥与当前账号的本地密钥不一致", { cause });
    this.name = "WalletKeyMismatchError";
    this.expectedPublicKey = expectedPublicKey;
    this.actualPublicKey = actualPublicKey;
  }
}

export class LegacyWalletMigrationRequiredError extends Error {
  readonly expectedPublicKey: string;
  readonly legacyPublicKey: string | null;

  constructor(expectedPublicKey: string, legacyPublicKey: string | null, cause?: unknown) {
    super("旧版浏览器钱包不属于当前账号，必须先切回原账号迁移或明确丢弃", { cause });
    this.name = "LegacyWalletMigrationRequiredError";
    this.expectedPublicKey = expectedPublicKey;
    this.legacyPublicKey = legacyPublicKey;
  }
}

function bytesToBase64(bytes: Uint8Array) {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

function base64ToBytes(value: string, expectedLength: number): Uint8Array<ArrayBuffer> {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  if (bytes.length !== expectedLength || bytesToBase64(bytes) !== value) {
    bytes.fill(0);
    throw new Error("non-canonical Ed25519 key material");
  }
  return bytes;
}

/** Accept only an owner response for the current account with an explicit canonical key field. */
export function resolveWalletServerKeyState(
  value: unknown,
  expectedAccountId: string | null,
): WalletServerKeyState {
  if (
    !expectedAccountId
    || !value
    || typeof value !== "object"
    || Array.isArray(value)
  ) {
    return { isKnown: false, activePublicKey: null };
  }
  const wallet = value as Record<string, unknown>;
  if (
    wallet.accountId !== expectedAccountId
    || !Object.prototype.hasOwnProperty.call(wallet, "activePublicKey")
  ) {
    return { isKnown: false, activePublicKey: null };
  }
  if (wallet.activePublicKey === null) {
    return { isKnown: true, activePublicKey: null };
  }
  if (typeof wallet.activePublicKey !== "string") {
    return { isKnown: false, activePublicKey: null };
  }
  try {
    const bytes = base64ToBytes(wallet.activePublicKey, ED25519_KEY_LENGTH);
    bytes.fill(0);
    return { isKnown: true, activePublicKey: wallet.activePublicKey };
  } catch {
    return { isKnown: false, activePublicKey: null };
  }
}

function normalizeAccountId(accountId: string) {
  if (typeof accountId !== "string" || !accountId.trim()) {
    throw new WalletStorageUnavailableError(new Error("missing wallet account scope"));
  }
  return accountId;
}

export function walletEnvironmentNamespace() {
  if (typeof globalThis.location === "undefined") {
    throw new WalletStorageUnavailableError(new Error("wallet environment is unavailable"));
  }
  const apiBaseUrl = (import.meta.env.VITE_API_BASE_URL ?? "/api/v2").replace(/\/$/, "");
  const url = new URL(apiBaseUrl, globalThis.location.origin);
  url.hash = "";
  url.search = "";
  const pathname = url.pathname.replace(/\/+$/, "") || "/";
  return `${url.origin}${pathname}`;
}

function walletScopeKey(environment: string, accountId: string) {
  return `${encodeURIComponent(environment)}:${encodeURIComponent(accountId)}`;
}

function validateExpectedPublicKey(expectedPublicKey: string | null) {
  if (expectedPublicKey === null) return;
  try {
    const bytes = base64ToBytes(expectedPublicKey, ED25519_KEY_LENGTH);
    bytes.fill(0);
  } catch (error) {
    throw new WalletKeyMismatchError(expectedPublicKey, null, error);
  }
}

function cryptoProvider() {
  const provider = globalThis.crypto;
  if (!provider?.subtle || typeof provider.getRandomValues !== "function") {
    throw new WalletStorageUnavailableError(new Error("WebCrypto Ed25519 is unavailable"));
  }
  return provider;
}

function isWalletBoundaryError(error: unknown) {
  return error instanceof WalletStorageUnavailableError
    || error instanceof WalletKeyUnavailableError
    || error instanceof WalletKeyMismatchError
    || error instanceof LegacyWalletMigrationRequiredError;
}

function storageFailure(error: unknown): never {
  if (isWalletBoundaryError(error)) throw error;
  throw new WalletStorageUnavailableError(error);
}

function requestResult<TResult>(request: IDBRequest<TResult>) {
  return new Promise<TResult>((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("IndexedDB request failed"));
  });
}

function transactionCompletion(transaction: IDBTransaction) {
  return new Promise<void>((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(
      transaction.error ?? new Error("IndexedDB transaction failed"),
    );
    transaction.onabort = () => reject(
      transaction.error ?? new Error("IndexedDB transaction aborted"),
    );
  });
}

async function openWalletDatabase() {
  if (typeof globalThis.indexedDB === "undefined") {
    throw new WalletStorageUnavailableError(new Error("IndexedDB is unavailable"));
  }
  const request = globalThis.indexedDB.open(WALLET_DATABASE_NAME, WALLET_DATABASE_VERSION);
  request.onupgradeneeded = () => {
    if (!request.result.objectStoreNames.contains(WALLET_OBJECT_STORE)) {
      request.result.createObjectStore(WALLET_OBJECT_STORE, { keyPath: "scopeKey" });
    }
  };
  const database = await new Promise<IDBDatabase>((resolve, reject) => {
    let isSettled = false;
    request.onsuccess = () => {
      if (isSettled) {
        request.result.close();
        return;
      }
      isSettled = true;
      resolve(request.result);
    };
    request.onerror = () => {
      if (isSettled) return;
      isSettled = true;
      reject(request.error ?? new Error("wallet IndexedDB open failed"));
    };
    request.onblocked = () => {
      if (isSettled) return;
      isSettled = true;
      reject(new Error("wallet IndexedDB open was blocked"));
    };
  });
  if (!database.objectStoreNames.contains(WALLET_OBJECT_STORE)) {
    database.close();
    throw new Error("wallet key object store is missing");
  }
  database.onversionchange = () => database.close();
  return database;
}

async function readStoredWalletKey(scopeKey: string) {
  const database = await openWalletDatabase();
  try {
    const transaction = database.transaction(WALLET_OBJECT_STORE, "readonly");
    const request = transaction.objectStore(WALLET_OBJECT_STORE).get(scopeKey);
    const [record] = await Promise.all([
      requestResult(request),
      transactionCompletion(transaction),
    ]);
    return record;
  } finally {
    database.close();
  }
}

async function storeWalletKeyIfAbsent(record: StoredWalletKey) {
  const database = await openWalletDatabase();
  try {
    const transaction = database.transaction(WALLET_OBJECT_STORE, "readwrite");
    const completion = transactionCompletion(transaction);
    const objectStore = transaction.objectStore(WALLET_OBJECT_STORE);
    const existingRequest = objectStore.get(record.scopeKey);
    let persistedRecord: unknown;
    existingRequest.onsuccess = () => {
      if (existingRequest.result !== undefined) {
        persistedRecord = existingRequest.result;
        return;
      }
      const putRequest = objectStore.put(record);
      putRequest.onsuccess = () => {
        persistedRecord = record;
      };
    };
    await completion;
    if (persistedRecord === undefined) {
      throw transaction.error ?? new Error("wallet key write did not complete");
    }
    return persistedRecord;
  } finally {
    database.close();
  }
}

async function deleteStoredWalletKey(scopeKey: string) {
  const database = await openWalletDatabase();
  try {
    const transaction = database.transaction(WALLET_OBJECT_STORE, "readwrite");
    const request = transaction.objectStore(WALLET_OBJECT_STORE).delete(scopeKey);
    await Promise.all([
      requestResult(request),
      transactionCompletion(transaction),
    ]);
  } finally {
    database.close();
  }
}

function isStoredWalletKey(value: unknown): value is StoredWalletKey {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<StoredWalletKey>;
  const algorithmName = candidate.privateKey?.algorithm?.name;
  return candidate.version === WALLET_RECORD_VERSION
    && typeof candidate.scopeKey === "string"
    && typeof candidate.environment === "string"
    && typeof candidate.accountId === "string"
    && typeof candidate.publicKey === "string"
    && typeof candidate.createdAt === "number"
    && Number.isFinite(candidate.createdAt)
    && candidate.privateKey?.type === "private"
    && candidate.privateKey.extractable === false
    && algorithmName === "Ed25519"
    && candidate.privateKey.usages.length === 1
    && candidate.privateKey.usages[0] === "sign";
}

async function validateStoredWalletKey(
  value: unknown,
  environment: string,
  accountId: string,
  expectedPublicKey: string | null,
) {
  if (
    !isStoredWalletKey(value)
    || value.environment !== environment
    || value.accountId !== accountId
    || value.scopeKey !== walletScopeKey(environment, accountId)
  ) {
    throw new WalletStorageUnavailableError(new Error("invalid wallet key record"));
  }
  let publicKeyBytes: Uint8Array<ArrayBuffer>;
  try {
    publicKeyBytes = base64ToBytes(value.publicKey, ED25519_KEY_LENGTH);
  } catch (error) {
    throw new WalletStorageUnavailableError(error);
  }
  if (expectedPublicKey !== null && value.publicKey !== expectedPublicKey) {
    publicKeyBytes.fill(0);
    throw new WalletKeyMismatchError(expectedPublicKey, value.publicKey);
  }
  try {
    const provider = cryptoProvider();
    const publicKey = await provider.subtle.importKey(
      "raw",
      publicKeyBytes,
      { name: "Ed25519" },
      false,
      ["verify"],
    );
    const signature = await provider.subtle.sign(
      { name: "Ed25519" },
      value.privateKey,
      KEY_CHECK_BYTES,
    );
    const matches = await provider.subtle.verify(
      { name: "Ed25519" },
      publicKey,
      signature,
      KEY_CHECK_BYTES,
    );
    if (!matches) {
      throw new WalletStorageUnavailableError(new Error("wallet key record is inconsistent"));
    }
  } finally {
    publicKeyBytes.fill(0);
  }
  return value;
}

async function importPrivateKey(seed: Uint8Array) {
  const pkcs8 = new Uint8Array(ED25519_PKCS8_PREFIX.length + seed.length);
  pkcs8.set(ED25519_PKCS8_PREFIX);
  pkcs8.set(seed, ED25519_PKCS8_PREFIX.length);
  try {
    const privateKey = await cryptoProvider().subtle.importKey(
      "pkcs8",
      pkcs8,
      { name: "Ed25519" },
      false,
      ["sign"],
    );
    if (privateKey.extractable || privateKey.type !== "private") {
      throw new Error("WebCrypto returned an exportable wallet key");
    }
    return privateKey;
  } finally {
    pkcs8.fill(0);
  }
}

function readLegacySeed() {
  if (typeof globalThis.localStorage === "undefined") {
    throw new WalletStorageUnavailableError(new Error("legacy wallet storage is unavailable"));
  }
  try {
    return globalThis.localStorage.getItem(LEGACY_WALLET_SEED_KEY);
  } catch (error) {
    throw new WalletStorageUnavailableError(error);
  }
}

function removeMigratedLegacySeed(originalValue: string) {
  try {
    if (globalThis.localStorage.getItem(LEGACY_WALLET_SEED_KEY) !== originalValue) {
      throw new Error("legacy wallet changed during migration");
    }
    globalThis.localStorage.removeItem(LEGACY_WALLET_SEED_KEY);
  } catch (error) {
    throw new WalletStorageUnavailableError(error);
  }
}

function removeMatchingLegacySeed(expectedPublicKey: string) {
  const encodedSeed = readLegacySeed();
  if (encodedSeed === null) return;
  let seed: Uint8Array | null = null;
  try {
    try {
      seed = base64ToBytes(encodedSeed, ED25519_KEY_LENGTH);
    } catch {
      return;
    }
    if (bytesToBase64(ed25519.getPublicKey(seed)) === expectedPublicKey) {
      removeMigratedLegacySeed(encodedSeed);
    }
  } finally {
    seed?.fill(0);
  }
}

async function migrateLegacyWallet(
  environment: string,
  accountId: string,
  expectedPublicKey: string,
) {
  const encodedSeed = readLegacySeed();
  if (encodedSeed === null) return null;
  let seed: Uint8Array | null = null;
  let legacyPublicKey: string | null = null;
  try {
    try {
      seed = base64ToBytes(encodedSeed, ED25519_KEY_LENGTH);
      legacyPublicKey = bytesToBase64(ed25519.getPublicKey(seed));
    } catch (error) {
      throw new LegacyWalletMigrationRequiredError(expectedPublicKey, null, error);
    }
    if (legacyPublicKey !== expectedPublicKey) {
      throw new LegacyWalletMigrationRequiredError(expectedPublicKey, legacyPublicKey);
    }
    const privateKey = await importPrivateKey(seed);
    const candidate: StoredWalletKey = {
      version: WALLET_RECORD_VERSION,
      scopeKey: walletScopeKey(environment, accountId),
      environment,
      accountId,
      publicKey: legacyPublicKey,
      privateKey,
      createdAt: Date.now(),
    };
    const persisted = await storeWalletKeyIfAbsent(candidate);
    const validated = await validateStoredWalletKey(
      persisted,
      environment,
      accountId,
      expectedPublicKey,
    );
    removeMigratedLegacySeed(encodedSeed);
    return validated;
  } finally {
    seed?.fill(0);
  }
}

async function getLocalWalletKey(
  environment: string,
  accountId: string,
  expectedPublicKey: string | null,
) {
  const stored = await readStoredWalletKey(walletScopeKey(environment, accountId));
  if (stored !== undefined) {
    const validated = await validateStoredWalletKey(
      stored,
      environment,
      accountId,
      expectedPublicKey,
    );
    if (expectedPublicKey !== null) removeMatchingLegacySeed(expectedPublicKey);
    return validated;
  }
  if (expectedPublicKey === null) return null;
  return migrateLegacyWallet(environment, accountId, expectedPublicKey);
}

export async function hasLocalWallet(accountId: string, expectedPublicKey: string | null) {
  try {
    const normalizedAccountId = normalizeAccountId(accountId);
    validateExpectedPublicKey(expectedPublicKey);
    return Boolean(await getLocalWalletKey(
      walletEnvironmentNamespace(),
      normalizedAccountId,
      expectedPublicKey,
    ));
  } catch (error) {
    return storageFailure(error);
  }
}

export async function createLocalWallet(accountId: string, expectedPublicKey: string | null) {
  try {
    const normalizedAccountId = normalizeAccountId(accountId);
    const environment = walletEnvironmentNamespace();
    validateExpectedPublicKey(expectedPublicKey);
    const existing = await getLocalWalletKey(environment, normalizedAccountId, expectedPublicKey);
    if (existing) return { publicKey: existing.publicKey };
    if (expectedPublicKey !== null) {
      throw new WalletKeyMismatchError(expectedPublicKey, null);
    }

    const seed = cryptoProvider().getRandomValues(new Uint8Array(ED25519_KEY_LENGTH));
    let publicKey: string;
    let privateKey: CryptoKey;
    try {
      publicKey = bytesToBase64(ed25519.getPublicKey(seed));
      privateKey = await importPrivateKey(seed);
    } finally {
      seed.fill(0);
    }
    const candidate: StoredWalletKey = {
      version: WALLET_RECORD_VERSION,
      scopeKey: walletScopeKey(environment, normalizedAccountId),
      environment,
      accountId: normalizedAccountId,
      publicKey,
      privateKey,
      createdAt: Date.now(),
    };
    const persisted = await storeWalletKeyIfAbsent(candidate);
    const validated = await validateStoredWalletKey(
      persisted,
      environment,
      normalizedAccountId,
      publicKey,
    );
    return { publicKey: validated.publicKey };
  } catch (error) {
    return storageFailure(error);
  }
}

export async function clearLocalWallet(accountId: string) {
  try {
    const normalizedAccountId = normalizeAccountId(accountId);
    await deleteStoredWalletKey(
      walletScopeKey(walletEnvironmentNamespace(), normalizedAccountId),
    );
  } catch (error) {
    return storageFailure(error);
  }
}

export function discardLegacyWallet() {
  try {
    if (typeof globalThis.localStorage === "undefined") {
      throw new Error("legacy wallet storage is unavailable");
    }
    globalThis.localStorage.removeItem(LEGACY_WALLET_SEED_KEY);
  } catch (error) {
    return storageFailure(error);
  }
}

export function inspectLegacyWallet() {
  let seed: Uint8Array | null = null;
  try {
    const encodedSeed = readLegacySeed();
    if (encodedSeed === null) return null;
    seed = base64ToBytes(encodedSeed, ED25519_KEY_LENGTH);
    return { publicKey: bytesToBase64(ed25519.getPublicKey(seed)) };
  } catch (error) {
    return storageFailure(error);
  } finally {
    seed?.fill(0);
  }
}

export async function getLocalWallet(accountId: string, expectedPublicKey: string | null) {
  try {
    const normalizedAccountId = normalizeAccountId(accountId);
    validateExpectedPublicKey(expectedPublicKey);
    const stored = await getLocalWalletKey(
      walletEnvironmentNamespace(),
      normalizedAccountId,
      expectedPublicKey,
    );
    return stored ? { publicKey: stored.publicKey } : null;
  } catch (error) {
    return storageFailure(error);
  }
}

export async function signExactBytes(
  accountId: string,
  expectedPublicKey: string,
  value: string,
) {
  try {
    const normalizedAccountId = normalizeAccountId(accountId);
    validateExpectedPublicKey(expectedPublicKey);
    const stored = await getLocalWalletKey(
      walletEnvironmentNamespace(),
      normalizedAccountId,
      expectedPublicKey,
    );
    if (!stored) throw new WalletKeyUnavailableError();
    const signature = await cryptoProvider().subtle.sign(
      { name: "Ed25519" },
      stored.privateKey,
      new TextEncoder().encode(value),
    );
    return bytesToBase64(new Uint8Array(signature));
  } catch (error) {
    return storageFailure(error);
  }
}
