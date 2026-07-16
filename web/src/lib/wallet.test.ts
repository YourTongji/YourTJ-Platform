import { webcrypto } from "node:crypto";

import { ed25519 } from "@noble/curves/ed25519";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import walletSigningFixture from "../../../contract/fixtures/wallet-signing-v1.json";
import {
  clearLocalWallet,
  createLocalWallet,
  discardLegacyWallet,
  getLocalWallet,
  inspectLegacyWallet,
  LegacyWalletMigrationRequiredError,
  resolveWalletServerKeyState,
  signExactBytes,
  WalletKeyMismatchError,
  WalletStorageUnavailableError,
} from "./wallet";

const legacyWalletSeedKey = "yourtj.walletSeed";

class MemoryRequest<TResult> {
  result!: TResult;
  error: DOMException | null = null;
  onsuccess: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
}

interface MemoryStoreData {
  records: Map<string, unknown>;
}

interface MemoryFailureState {
  failNextPut: boolean;
}

class MemoryTransaction {
  error: DOMException | null = null;
  oncomplete: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onabort: ((event: Event) => void) | null = null;
  private pending = 0;
  private isComplete = false;
  private isCompletionQueued = false;
  private hasFailed = false;

  constructor(
    private readonly data: MemoryStoreData,
    private readonly failures: MemoryFailureState,
  ) {
    this.queueCompletion();
  }

  objectStore() {
    return new MemoryObjectStore(this.data, this, this.failures);
  }

  request<TResult>(operation: () => TResult) {
    const request = new MemoryRequest<TResult>();
    this.pending += 1;
    queueMicrotask(() => {
      try {
        request.result = operation();
        request.onsuccess?.(new Event("success"));
      } catch (error) {
        request.error = error instanceof DOMException
          ? error
          : new DOMException("memory IndexedDB request failed", "UnknownError");
        this.error = request.error;
        this.hasFailed = true;
        request.onerror?.(new Event("error"));
        this.onerror?.(new Event("error"));
      } finally {
        this.pending -= 1;
        this.queueCompletion();
      }
    });
    return request;
  }

  private queueCompletion() {
    if (this.isComplete || this.isCompletionQueued || this.hasFailed) return;
    this.isCompletionQueued = true;
    queueMicrotask(() => {
      this.isCompletionQueued = false;
      if (this.pending === 0 && !this.isComplete && !this.hasFailed) {
        this.isComplete = true;
        this.oncomplete?.(new Event("complete"));
      }
    });
  }
}

class MemoryObjectStore {
  constructor(
    private readonly data: MemoryStoreData,
    private readonly transaction: MemoryTransaction,
    private readonly failures: MemoryFailureState,
  ) {}

  get(key: IDBValidKey) {
    return this.transaction.request(() => {
      const record = this.data.records.get(String(key));
      return record === undefined ? undefined : structuredClone(record);
    }) as unknown as IDBRequest<unknown>;
  }

  put(value: { scopeKey: string }) {
    return this.transaction.request(() => {
      if (this.failures.failNextPut) {
        this.failures.failNextPut = false;
        throw new DOMException("injected put failure", "QuotaExceededError");
      }
      this.data.records.set(value.scopeKey, structuredClone(value));
      return value.scopeKey;
    }) as unknown as IDBRequest<IDBValidKey>;
  }

  delete(key: IDBValidKey) {
    return this.transaction.request(
      () => this.data.records.delete(String(key)),
    ) as unknown as IDBRequest<undefined>;
  }
}

class MemoryDatabase {
  onversionchange: ((event: Event) => void) | null = null;
  private readonly stores = new Map<string, MemoryStoreData>();

  constructor(private readonly failures: MemoryFailureState) {}

  get objectStoreNames() {
    return { contains: (name: string) => this.stores.has(name) };
  }

  createObjectStore(name: string) {
    const data = { records: new Map<string, unknown>() } satisfies MemoryStoreData;
    this.stores.set(name, data);
    return new MemoryObjectStore(data, new MemoryTransaction(data, this.failures), this.failures);
  }

  transaction(name: string) {
    const data = this.stores.get(name);
    if (!data) throw new DOMException("object store not found", "NotFoundError");
    return new MemoryTransaction(data, this.failures) as unknown as IDBTransaction;
  }

  read(accountId: string) {
    for (const store of this.stores.values()) {
      for (const record of store.records.values()) {
        if (
          record
          && typeof record === "object"
          && "accountId" in record
          && record.accountId === accountId
        ) {
          return structuredClone(record);
        }
      }
    }
    return undefined;
  }

  close() {}
}

class MemoryOpenRequest extends MemoryRequest<IDBDatabase> {
  transaction: IDBTransaction | null = null;
  onupgradeneeded: ((event: Event) => void) | null = null;
  onblocked: ((event: Event) => void) | null = null;
}

class MemoryIndexedDbFactory {
  private database: MemoryDatabase | null = null;
  private readonly failures: MemoryFailureState = { failNextPut: false };
  private shouldBlockNextOpen = false;

  open() {
    const request = new MemoryOpenRequest();
    queueMicrotask(() => {
      if (this.shouldBlockNextOpen) {
        this.shouldBlockNextOpen = false;
        request.onblocked?.(new Event("blocked"));
        return;
      }
      const needsUpgrade = this.database === null;
      this.database ??= new MemoryDatabase(this.failures);
      request.result = this.database as unknown as IDBDatabase;
      if (needsUpgrade) request.onupgradeneeded?.(new Event("upgradeneeded"));
      request.onsuccess?.(new Event("success"));
    });
    return request as unknown as IDBOpenDBRequest;
  }

  failNextPut() {
    this.failures.failNextPut = true;
  }

  blockNextOpen() {
    this.shouldBlockNextOpen = true;
  }

  read(accountId: string) {
    return this.database?.read(accountId);
  }
}

interface WalletSigningVector {
  id: string;
  seedHex: string;
  publicKeyBase64: string;
  signingBytes: string;
  signatureBase64: string;
}

function hexToBytes(value: string) {
  const bytes = new Uint8Array(value.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(value.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

function bytesToBase64(bytes: Uint8Array) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function fixtureVectors() {
  return walletSigningFixture.vectors as WalletSigningVector[];
}

describe("account-scoped WebCrypto wallet", () => {
  let indexedDb: MemoryIndexedDbFactory;

  beforeEach(() => {
    indexedDb = new MemoryIndexedDbFactory();
    vi.stubGlobal("crypto", webcrypto as unknown as Crypto);
    vi.stubGlobal("indexedDB", indexedDb as unknown as IDBFactory);
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
  });

  it("migrates a matching legacy seed and matches every shared exact-byte vector", async () => {
    const vectors = fixtureVectors();
    const first = vectors[0]!;
    localStorage.setItem(legacyWalletSeedKey, bytesToBase64(hexToBytes(first.seedHex)));

    await expect(getLocalWallet("fixture-account", first.publicKeyBase64)).resolves.toEqual({
      publicKey: first.publicKeyBase64,
    });
    expect(localStorage.getItem(legacyWalletSeedKey)).toBeNull();

    for (const vector of vectors) {
      await expect(signExactBytes(
        "fixture-account",
        vector.publicKeyBase64,
        vector.signingBytes,
      )).resolves.toBe(vector.signatureBase64);
    }
  });

  it("persists only a non-extractable private key and zeroizes the generated seed", async () => {
    const generatedSeeds: Uint8Array[] = [];
    const deterministicCrypto = {
      subtle: webcrypto.subtle,
      getRandomValues<TArray extends ArrayBufferView | null>(array: TArray) {
        if (!(array instanceof Uint8Array)) throw new TypeError("expected Uint8Array");
        array.fill(0x5a);
        generatedSeeds.push(array);
        return array;
      },
    } as unknown as Crypto;
    vi.stubGlobal("crypto", deterministicCrypto);

    const wallet = await createLocalWallet("account-private", null);
    const record = indexedDb.read("account-private") as unknown as { privateKey: CryptoKey };

    expect(wallet.publicKey).toBeTruthy();
    expect(record.privateKey.extractable).toBe(false);
    await expect(webcrypto.subtle.exportKey("pkcs8", record.privateKey)).rejects.toThrow();
    expect(generatedSeeds).toHaveLength(1);
    expect([...generatedSeeds[0]!]).toEqual(new Array(32).fill(0));
    expect(localStorage.getItem(legacyWalletSeedKey)).toBeNull();
  });

  it("keeps records isolated between accounts and rejects a server-key mismatch", async () => {
    const accountOne = await createLocalWallet("account-one", null);
    const accountTwo = await createLocalWallet("account-two", null);

    expect(accountOne.publicKey).not.toBe(accountTwo.publicKey);
    await expect(getLocalWallet("account-one", accountOne.publicKey)).resolves.toEqual(accountOne);
    await expect(getLocalWallet("account-two", accountTwo.publicKey)).resolves.toEqual(accountTwo);
    await expect(getLocalWallet("account-two", accountOne.publicKey)).rejects.toBeInstanceOf(
      WalletKeyMismatchError,
    );

    await clearLocalWallet("account-one");
    await expect(getLocalWallet("account-one", null)).resolves.toBeNull();
    await expect(getLocalWallet("account-two", accountTwo.publicKey)).resolves.toEqual(accountTwo);
  });

  it("keeps the same account isolated between API environments", async () => {
    vi.stubEnv("VITE_API_BASE_URL", "https://api-one.example/api/v2");
    const firstEnvironmentWallet = await createLocalWallet("shared-account", null);

    vi.stubEnv("VITE_API_BASE_URL", "https://api-two.example/api/v2");
    await expect(getLocalWallet("shared-account", null)).resolves.toBeNull();
    const secondEnvironmentWallet = await createLocalWallet("shared-account", null);
    expect(secondEnvironmentWallet.publicKey).not.toBe(firstEnvironmentWallet.publicKey);

    vi.stubEnv("VITE_API_BASE_URL", "https://api-one.example/api/v2");
    await expect(
      getLocalWallet("shared-account", firstEnvironmentWallet.publicKey),
    ).resolves.toEqual(firstEnvironmentWallet);
  });

  it("never assigns a mismatched legacy seed to the current account", async () => {
    const vector = fixtureVectors()[0]!;
    const otherSeed = new Uint8Array(32).fill(0x7b);
    const otherPublicKey = bytesToBase64(ed25519.getPublicKey(otherSeed));
    const encodedLegacySeed = bytesToBase64(hexToBytes(vector.seedHex));
    localStorage.setItem(legacyWalletSeedKey, encodedLegacySeed);

    const error = await getLocalWallet("other-account", otherPublicKey).catch(
      (caught: unknown) => caught,
    );

    expect(error).toBeInstanceOf(LegacyWalletMigrationRequiredError);
    expect(error).toMatchObject({
      expectedPublicKey: otherPublicKey,
      legacyPublicKey: vector.publicKeyBase64,
    });
    expect(indexedDb.read("other-account")).toBeUndefined();
    expect(localStorage.getItem(legacyWalletSeedKey)).toBe(encodedLegacySeed);
  });

  it("retains the legacy seed when the durable IndexedDB put fails", async () => {
    const vector = fixtureVectors()[0]!;
    const encodedLegacySeed = bytesToBase64(hexToBytes(vector.seedHex));
    localStorage.setItem(legacyWalletSeedKey, encodedLegacySeed);
    indexedDb.failNextPut();

    await expect(getLocalWallet("account-put-failure", vector.publicKeyBase64)).rejects
      .toBeInstanceOf(WalletStorageUnavailableError);
    expect(indexedDb.read("account-put-failure")).toBeUndefined();
    expect(localStorage.getItem(legacyWalletSeedKey)).toBe(encodedLegacySeed);
  });

  it("retries legacy seed removal after the matching key was durably migrated", async () => {
    const vector = fixtureVectors()[0]!;
    const encodedLegacySeed = bytesToBase64(hexToBytes(vector.seedHex));
    localStorage.setItem(legacyWalletSeedKey, encodedLegacySeed);
    const removeItem = vi.spyOn(Storage.prototype, "removeItem")
      .mockImplementationOnce(() => {
        throw new DOMException("storage busy", "UnknownError");
      });

    await expect(getLocalWallet("account-remove-retry", vector.publicKeyBase64)).rejects
      .toBeInstanceOf(WalletStorageUnavailableError);
    expect(indexedDb.read("account-remove-retry")).toBeDefined();
    expect(localStorage.getItem(legacyWalletSeedKey)).toBe(encodedLegacySeed);

    removeItem.mockRestore();
    await expect(getLocalWallet("account-remove-retry", vector.publicKeyBase64)).resolves.toEqual({
      publicKey: vector.publicKeyBase64,
    });
    expect(localStorage.getItem(legacyWalletSeedKey)).toBeNull();
  });

  it("fails closed without IndexedDB and never falls back to the raw legacy seed", async () => {
    const vector = fixtureVectors()[0]!;
    const encodedLegacySeed = bytesToBase64(hexToBytes(vector.seedHex));
    localStorage.setItem(legacyWalletSeedKey, encodedLegacySeed);
    vi.stubGlobal("indexedDB", undefined);

    await expect(getLocalWallet("account-no-idb", vector.publicKeyBase64)).rejects
      .toBeInstanceOf(WalletStorageUnavailableError);
    await expect(createLocalWallet("new-account-no-idb", null)).rejects
      .toBeInstanceOf(WalletStorageUnavailableError);
    expect(localStorage.getItem(legacyWalletSeedKey)).toBe(encodedLegacySeed);
  });

  it("rejects a blocked IndexedDB open instead of leaving wallet access pending", async () => {
    indexedDb.blockNextOpen();

    await expect(getLocalWallet("account-blocked", null)).rejects.toBeInstanceOf(
      WalletStorageUnavailableError,
    );
  });

  it("fails closed and zeroizes the seed when WebCrypto import fails", async () => {
    const generatedSeeds: Uint8Array[] = [];
    vi.stubGlobal("crypto", {
      subtle: {
        importKey: vi.fn().mockRejectedValue(new DOMException("unsupported", "NotSupportedError")),
      },
      getRandomValues(array: Uint8Array) {
        array.fill(0x4d);
        generatedSeeds.push(array);
        return array;
      },
    } as unknown as Crypto);

    await expect(createLocalWallet("account-import-failure", null)).rejects.toBeInstanceOf(
      WalletStorageUnavailableError,
    );
    expect(indexedDb.read("account-import-failure")).toBeUndefined();
    expect([...generatedSeeds[0]!]).toEqual(new Array(32).fill(0));
    expect(localStorage.getItem(legacyWalletSeedKey)).toBeNull();
  });

  it("lets a new unbound account generate without deleting an unknown legacy seed", async () => {
    const vector = fixtureVectors()[0]!;
    const encodedLegacySeed = bytesToBase64(hexToBytes(vector.seedHex));
    localStorage.setItem(legacyWalletSeedKey, encodedLegacySeed);

    expect(inspectLegacyWallet()).toEqual({ publicKey: vector.publicKeyBase64 });
    expect(indexedDb.read("new-account")).toBeUndefined();

    const wallet = await createLocalWallet("new-account", null);
    expect(wallet.publicKey).not.toBe(vector.publicKeyBase64);
    expect(localStorage.getItem(legacyWalletSeedKey)).toBe(encodedLegacySeed);

    await clearLocalWallet("new-account");
    expect(localStorage.getItem(legacyWalletSeedKey)).toBe(encodedLegacySeed);
    discardLegacyWallet();
    expect(localStorage.getItem(legacyWalletSeedKey)).toBeNull();
  });

  it("does not generate a replacement when the server already has a public key", async () => {
    const expectedPublicKey = bytesToBase64(ed25519.getPublicKey(new Uint8Array(32).fill(0x31)));

    await expect(createLocalWallet("server-bound-account", expectedPublicKey)).rejects.toMatchObject({
      name: "WalletKeyMismatchError",
      expectedPublicKey,
      actualPublicKey: null,
    });
    expect(indexedDb.read("server-bound-account")).toBeUndefined();
  });

  it("accepts only an explicit canonical owner key state for the current account", () => {
    const publicKey = fixtureVectors()[0]!.publicKeyBase64;

    expect(resolveWalletServerKeyState(
      { accountId: "account-one", balance: 0 },
      "account-one",
    )).toEqual({ isKnown: false, activePublicKey: null });
    expect(resolveWalletServerKeyState(
      { accountId: "account-two", activePublicKey: publicKey },
      "account-one",
    )).toEqual({ isKnown: false, activePublicKey: null });
    expect(resolveWalletServerKeyState(
      { accountId: "account-one", activePublicKey: null },
      "account-one",
    )).toEqual({ isKnown: true, activePublicKey: null });
    expect(resolveWalletServerKeyState(
      { accountId: "account-one", activePublicKey: "not-a-key" },
      "account-one",
    )).toEqual({ isKnown: false, activePublicKey: null });
    expect(resolveWalletServerKeyState(
      { accountId: "account-one", activePublicKey: publicKey },
      "account-one",
    )).toEqual({ isKnown: true, activePublicKey: publicKey });
  });
});
