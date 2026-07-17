import { afterEach, describe, expect, it, vi } from "vitest";

import {
  claimWalletPendingMutation,
  commitWalletPendingMutation,
  deleteWalletPendingMutation,
  isWalletPendingStorageAvailable,
  listWalletPendingMutations,
  readWalletPendingMutation,
  submitWalletPendingMutation,
  WALLET_COMMITTED_TOMBSTONE_SECONDS,
  WALLET_PREPARING_LEASE_SECONDS,
  WalletPendingStorageError,
  walletOperationKey,
  type WalletPendingMutation,
  type WalletPendingScope,
} from "./wallet-pending";

interface MemoryRecord {
  storageKey: string;
  scopeKey: string;
  environmentNamespace: string;
  accountId: string;
  [key: string]: unknown;
}

class MemoryKeyRange {
  private constructor(readonly value: IDBValidKey) {}

  static only(value: IDBValidKey) {
    return new MemoryKeyRange(value);
  }
}

class MemoryRequest<TResult> {
  result!: TResult;
  error: DOMException | null = null;
  onsuccess: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
}

interface MemoryStoreData {
  records: Map<string, MemoryRecord>;
  indexes: Set<string>;
}

class MemoryTransaction {
  error: DOMException | null = null;
  oncomplete: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onabort: ((event: Event) => void) | null = null;
  private snapshot = new Map<string, MemoryRecord>();
  private pending = 0;
  private isFinished = false;
  private isCompletionQueued = false;
  private isStarted = false;
  private readonly queuedRequests: Array<() => void> = [];

  constructor(
    private readonly data: MemoryStoreData,
    private readonly failCommit: boolean,
    private readonly onFinished: () => void,
  ) {}

  start() {
    if (this.isStarted || this.isFinished) return;
    this.isStarted = true;
    this.snapshot = new Map(
      [...this.data.records].map(([key, value]) => [key, structuredClone(value)]),
    );
    for (const execute of this.queuedRequests.splice(0)) queueMicrotask(execute);
    this.queueCompletion();
  }

  objectStore() {
    return new MemoryObjectStore(this.data, this);
  }

  abort() {
    this.finish("abort");
  }

  request<TResult>(operation: () => TResult) {
    const request = new MemoryRequest<TResult>();
    this.pending += 1;
    const execute = () => {
      try {
        request.result = operation();
        request.onsuccess?.(new Event("success"));
      } catch (error) {
        request.error = error instanceof DOMException
          ? error
          : new DOMException("memory IndexedDB request failed", "UnknownError");
        this.error = request.error;
        request.onerror?.(new Event("error"));
        this.finish("error");
      } finally {
        this.pending -= 1;
        this.queueCompletion();
      }
    };
    if (this.isStarted) queueMicrotask(execute);
    else this.queuedRequests.push(execute);
    return request;
  }

  private queueCompletion() {
    if (this.isFinished || this.isCompletionQueued) return;
    this.isCompletionQueued = true;
    queueMicrotask(() => {
      this.isCompletionQueued = false;
      if (this.pending !== 0 || this.isFinished) return;
      if (this.failCommit) {
        this.error = new DOMException("memory IndexedDB commit failed", "UnknownError");
        this.finish("abort");
      } else {
        this.finish("complete");
      }
    });
  }

  private finish(outcome: "complete" | "error" | "abort") {
    if (this.isFinished) return;
    this.isFinished = true;
    if (outcome !== "complete") {
      this.data.records.clear();
      for (const [key, value] of this.snapshot) {
        this.data.records.set(key, structuredClone(value));
      }
    }
    this.onFinished();
    if (outcome === "complete") this.oncomplete?.(new Event("complete"));
    if (outcome === "error") this.onerror?.(new Event("error"));
    if (outcome === "abort") this.onabort?.(new Event("abort"));
  }
}

class MemoryObjectStore {
  constructor(
    private readonly data: MemoryStoreData,
    private readonly transaction: MemoryTransaction | null,
  ) {}

  get indexNames() {
    return { contains: (name: string) => this.data.indexes.has(name) };
  }

  createIndex(name: string) {
    this.data.indexes.add(name);
    return {};
  }

  index(name: string) {
    if (!this.data.indexes.has(name)) throw new DOMException("index not found", "NotFoundError");
    return {
      getAll: (range: IDBKeyRange) => this.requireTransaction().request(() => {
        const expected = (range as unknown as MemoryKeyRange).value;
        return [...this.data.records.values()]
          .filter((record) => record[name] === expected)
          .map((record) => structuredClone(record));
      }) as unknown as IDBRequest<MemoryRecord[]>,
    };
  }

  get(key: IDBValidKey) {
    return this.requireTransaction().request(() => {
      const record = this.data.records.get(String(key));
      return record === undefined ? undefined : structuredClone(record);
    }) as unknown as IDBRequest<MemoryRecord | undefined>;
  }

  put(value: MemoryRecord) {
    return this.requireTransaction().request(() => {
      this.data.records.set(value.storageKey, structuredClone(value));
      return value.storageKey;
    }) as unknown as IDBRequest<IDBValidKey>;
  }

  add(value: MemoryRecord) {
    return this.requireTransaction().request(() => {
      if (this.data.records.has(value.storageKey)) {
        throw new DOMException("record already exists", "ConstraintError");
      }
      this.data.records.set(value.storageKey, structuredClone(value));
      return value.storageKey;
    }) as unknown as IDBRequest<IDBValidKey>;
  }

  delete(key: IDBValidKey) {
    return this.requireTransaction().request(() => {
      this.data.records.delete(String(key));
      return undefined;
    }) as unknown as IDBRequest<undefined>;
  }

  private requireTransaction() {
    if (!this.transaction) throw new DOMException("upgrade store has no requests", "InvalidStateError");
    return this.transaction;
  }
}

class MemoryDatabase {
  onversionchange: ((event: Event) => void) | null = null;

  constructor(
    private readonly factory: MemoryIndexedDbFactory,
    private readonly stores: Map<string, MemoryStoreData>,
  ) {}

  get objectStoreNames() {
    return { contains: (name: string) => this.stores.has(name) };
  }

  createObjectStore(name: string) {
    const data = { records: new Map(), indexes: new Set<string>() } satisfies MemoryStoreData;
    this.stores.set(name, data);
    return new MemoryObjectStore(data, null);
  }

  transaction(name: string, mode: IDBTransactionMode = "readonly") {
    const data = this.stores.get(name);
    if (!data) throw new DOMException("object store not found", "NotFoundError");
    return this.factory.createTransaction(data, mode) as unknown as IDBTransaction;
  }

  records() {
    const data = this.stores.get("pendingMutations");
    return data ? [...data.records.values()] : [];
  }

  close() {}
}

class MemoryOpenRequest extends MemoryRequest<IDBDatabase> {
  transaction: IDBTransaction | null = null;
  onupgradeneeded: ((event: Event) => void) | null = null;
  onblocked: ((event: Event) => void) | null = null;
}

class MemoryIndexedDbFactory {
  private readonly stores = new Map<string, MemoryStoreData>();
  private hasOpenedDatabase = false;
  private failCommit = false;
  private activeTransactions = 0;
  private isReadwriteActive = false;
  private readonly readwriteQueue: MemoryTransaction[] = [];
  maximumActiveTransactions = 0;
  openedConnections = 0;

  open() {
    const request = new MemoryOpenRequest();
    queueMicrotask(() => {
      const needsUpgrade = !this.hasOpenedDatabase;
      this.hasOpenedDatabase = true;
      const database = new MemoryDatabase(this, this.stores);
      this.openedConnections += 1;
      request.result = database as unknown as IDBDatabase;
      if (needsUpgrade) request.onupgradeneeded?.(new Event("upgradeneeded"));
      request.onsuccess?.(new Event("success"));
    });
    return request as unknown as IDBOpenDBRequest;
  }

  createTransaction(data: MemoryStoreData, mode: IDBTransactionMode) {
    const shouldFail = this.failCommit;
    this.failCommit = false;
    const transaction = new MemoryTransaction(data, shouldFail, () => {
      this.activeTransactions -= 1;
      if (mode === "readwrite") {
        this.isReadwriteActive = false;
        this.startNextReadwrite();
      }
    });
    if (mode === "readwrite") {
      this.readwriteQueue.push(transaction);
      this.startNextReadwrite();
    } else {
      this.startTransaction(transaction);
    }
    return transaction;
  }

  private startTransaction(transaction: MemoryTransaction) {
    this.activeTransactions += 1;
    this.maximumActiveTransactions = Math.max(
      this.maximumActiveTransactions,
      this.activeTransactions,
    );
    transaction.start();
  }

  private startNextReadwrite() {
    if (this.isReadwriteActive) return;
    const transaction = this.readwriteQueue.shift();
    if (!transaction) return;
    this.isReadwriteActive = true;
    this.startTransaction(transaction);
  }

  failNextTransactionCommit() {
    this.failCommit = true;
  }

  rawRecords() {
    const data = this.stores.get("pendingMutations");
    return data ? [...data.records.values()] : [];
  }
}

const operationKeyA = "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const operationKeyB = "sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
const scopeA: WalletPendingScope = {
  environmentNamespace: "/api/v2",
  accountId: "account-a",
};
const scopeB: WalletPendingScope = {
  environmentNamespace: "/api/v2",
  accountId: "account-b",
};
const previewScopeA: WalletPendingScope = {
  environmentNamespace: "https://preview.example/api/v2",
  accountId: "account-a",
};

function pending(
  operationKey = operationKeyA,
  overrides: Partial<WalletPendingMutation> = {},
): WalletPendingMutation {
  return {
    operationKey,
    claimId: "claim-a",
    action: "credit.tip",
    phase: "preparing",
    intentId: null,
    expiresAt: 1_000 + WALLET_PREPARING_LEASE_SECONDS,
    ...overrides,
  };
}

function installMemoryIndexedDb() {
  const factory = new MemoryIndexedDbFactory();
  vi.stubGlobal("indexedDB", factory as unknown as IDBFactory);
  vi.stubGlobal("IDBKeyRange", MemoryKeyRange as unknown as typeof IDBKeyRange);
  return factory;
}

describe("wallet pending reconciliation storage", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    localStorage.clear();
  });

  it("derives a stable SHA-256 key from canonical account, action, and JSON request", async () => {
    const requestA = {
      z: 1,
      list: [3, undefined, { b: true, a: null }],
      omitted: undefined,
      a: { y: 2, x: "ok" },
    };
    const requestB = {
      a: { x: "ok", y: 2 },
      list: [3, null, { a: null, b: true }],
      z: 1,
    };

    await expect(walletOperationKey("account-a", "credit.tip", requestA)).resolves.toBe(
      "sha256:qroBjYnFWjakUPzVGNOHNRp6F4X-FXdqpIsbScwNE8I",
    );
    await expect(walletOperationKey("account-a", "credit.tip", requestB)).resolves.toBe(
      "sha256:qroBjYnFWjakUPzVGNOHNRp6F4X-FXdqpIsbScwNE8I",
    );
    await expect(walletOperationKey("account-b", "credit.tip", requestB)).resolves.not.toBe(
      "sha256:qroBjYnFWjakUPzVGNOHNRp6F4X-FXdqpIsbScwNE8I",
    );
    await expect(walletOperationKey("account-a", "credit.task.create", requestB)).resolves.not.toBe(
      "sha256:qroBjYnFWjakUPzVGNOHNRp6F4X-FXdqpIsbScwNE8I",
    );
  });

  it("fails closed without IndexedDB and never falls back to localStorage", async () => {
    vi.stubGlobal("indexedDB", undefined);
    vi.stubGlobal("IDBKeyRange", undefined);
    localStorage.setItem("sentinel", "unchanged");

    expect(isWalletPendingStorageAvailable()).toBe(false);
    await expect(claimWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "credit.tip",
      1_000,
    )).rejects
      .toBeInstanceOf(WalletPendingStorageError);
    await expect(listWalletPendingMutations(scopeA)).rejects
      .toBeInstanceOf(WalletPendingStorageError);
    expect(localStorage.getItem("sentinel")).toBe("unchanged");
    expect(localStorage.length).toBe(1);
  });

  it("allows only one atomic add to win a competing operation claim", async () => {
    installMemoryIndexedDb();
    const [first, competing] = await Promise.all([
      claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000),
      claimWalletPendingMutation(scopeA, operationKeyA, "claim-b", "credit.tip", 1_000),
    ]);

    expect(first).toEqual({ claimed: true, mutation: pending() });
    expect(competing).toEqual({ claimed: false, mutation: pending() });
    await expect(readWalletPendingMutation(scopeA, operationKeyA)).resolves.toEqual(pending());
  });

  it("allows one claim across independent module instances and database connections", async () => {
    const factory = installMemoryIndexedDb();
    vi.resetModules();
    const secondTab = await import("./wallet-pending");
    expect(secondTab.claimWalletPendingMutation).not.toBe(claimWalletPendingMutation);

    const results = await Promise.all([
      claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000),
      secondTab.claimWalletPendingMutation(
        scopeA,
        operationKeyA,
        "claim-b",
        "credit.tip",
        1_000,
      ),
    ]);

    expect(results.filter((result) => result.claimed)).toHaveLength(1);
    expect(results.filter((result) => !result.claimed)).toHaveLength(1);
    expect(factory.openedConnections).toBeGreaterThanOrEqual(2);
    expect(factory.maximumActiveTransactions).toBe(1);
    await expect(readWalletPendingMutation(scopeA, operationKeyA)).resolves.toMatchObject({
      claimId: results.find((result) => result.claimed)?.mutation.claimId,
      phase: "preparing",
    });
  });

  it("uses claim-and-phase CAS for every transition and delete", async () => {
    installMemoryIndexedDb();
    await claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000);

    await expect(submitWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-b",
      "intent-1",
      1_300,
    )).resolves.toBeNull();
    await expect(deleteWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-b",
      "preparing",
    )).resolves.toBe(false);

    const submitted = pending(operationKeyA, {
      phase: "submitted",
      intentId: "intent-1",
      expiresAt: 1_300,
    });
    await expect(submitWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "intent-1",
      1_300,
    )).resolves.toEqual(submitted);
    await expect(deleteWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "preparing",
    )).resolves.toBe(false);
    await expect(commitWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "another-intent",
      1_200,
    )).resolves.toBeNull();

    const committed = pending(operationKeyA, {
      phase: "committed",
      intentId: "intent-1",
      expiresAt: 1_200 + WALLET_COMMITTED_TOMBSTONE_SECONDS,
    });
    await expect(commitWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "intent-1",
      1_200,
    )).resolves.toEqual(committed);
    await expect(deleteWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-b",
      "committed",
    )).resolves.toBe(false);
    await expect(deleteWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "committed",
    )).resolves.toBe(true);
  });

  it("recovers an expired preparing lease without letting the stale claim resume", async () => {
    installMemoryIndexedDb();
    await claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000);

    await expect(claimWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-b",
      "credit.tip",
      1_000 + WALLET_PREPARING_LEASE_SECONDS - 1,
    )).resolves.toMatchObject({ claimed: false });
    await expect(claimWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-b",
      "credit.tip",
      1_000 + WALLET_PREPARING_LEASE_SECONDS,
    )).resolves.toEqual({
      claimed: true,
      mutation: pending(operationKeyA, {
        claimId: "claim-b",
        expiresAt: 1_000 + (2 * WALLET_PREPARING_LEASE_SECONDS),
      }),
    });
    await expect(submitWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "intent-stale",
      1_500,
    )).resolves.toBeNull();
  });

  it("retains a commit tombstone for five minutes before allowing a fresh claim", async () => {
    installMemoryIndexedDb();
    await claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000);
    await submitWalletPendingMutation(scopeA, operationKeyA, "claim-a", "intent-1", 1_300);
    await commitWalletPendingMutation(scopeA, operationKeyA, "claim-a", "intent-1", 1_200);

    await expect(claimWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-b",
      "credit.tip",
      1_200 + WALLET_COMMITTED_TOMBSTONE_SECONDS - 1,
    )).resolves.toMatchObject({ claimed: false, mutation: { phase: "committed" } });
    await expect(claimWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-b",
      "credit.tip",
      1_200 + WALLET_COMMITTED_TOMBSTONE_SECONDS,
    )).resolves.toMatchObject({ claimed: true, mutation: { phase: "preparing" } });
  });

  it("keeps records environment-and-account-scoped and persists only the allowlist", async () => {
    const factory = installMemoryIndexedDb();
    await claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000);
    await claimWalletPendingMutation(scopeB, operationKeyA, "claim-b", "credit.tip", 1_000);
    await claimWalletPendingMutation(
      previewScopeA,
      operationKeyA,
      "claim-preview",
      "credit.tip",
      1_000,
    );

    await expect(readWalletPendingMutation(scopeA, operationKeyA)).resolves.toEqual(pending());
    await expect(readWalletPendingMutation(scopeB, operationKeyA)).resolves.toEqual(
      pending(operationKeyA, { claimId: "claim-b" }),
    );
    await expect(readWalletPendingMutation(previewScopeA, operationKeyA)).resolves.toEqual(
      pending(operationKeyA, { claimId: "claim-preview" }),
    );
    await expect(listWalletPendingMutations(scopeA)).resolves.toEqual([pending()]);
    await expect(listWalletPendingMutations(previewScopeA)).resolves.toEqual([
      pending(operationKeyA, { claimId: "claim-preview" }),
    ]);
    const encoded = JSON.stringify(factory.rawRecords());
    expect(encoded).not.toContain("rawRequest");
    expect(encoded).not.toContain("idempotencyKey");
    expect(encoded).not.toContain("signature-secret");
    expect(encoded).not.toContain("signing-secret");
  });

  it("serializes same-scope read, write, delete, and list operations", async () => {
    const factory = installMemoryIndexedDb();

    const [, , records] = await Promise.all([
      claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000),
      claimWalletPendingMutation(scopeA, operationKeyB, "claim-b", "credit.tip", 1_000),
      listWalletPendingMutations(scopeA),
    ]);

    expect(records).toEqual([pending(operationKeyA), pending(operationKeyB, { claimId: "claim-b" })]);
    expect(factory.maximumActiveTransactions).toBe(1);
  });

  it("rejects corrupted or secret-bearing storage instead of deleting or ignoring it", async () => {
    const factory = installMemoryIndexedDb();
    await claimWalletPendingMutation(scopeA, operationKeyA, "claim-a", "credit.tip", 1_000);
    const record = factory.rawRecords()[0];
    expect(record).toBeDefined();
    if (record) record.signature = "unexpected-secret";

    await expect(readWalletPendingMutation(scopeA, operationKeyA)).rejects
      .toBeInstanceOf(WalletPendingStorageError);
    await expect(listWalletPendingMutations(scopeA)).rejects
      .toBeInstanceOf(WalletPendingStorageError);
    expect(factory.rawRecords()).toHaveLength(1);
  });

  it("waits for durable transaction completion and continues the serial queue after failure", async () => {
    const factory = installMemoryIndexedDb();
    factory.failNextTransactionCommit();

    await expect(claimWalletPendingMutation(
      scopeA,
      operationKeyA,
      "claim-a",
      "credit.tip",
      1_000,
    )).rejects
      .toBeInstanceOf(WalletPendingStorageError);
    expect(factory.rawRecords()).toEqual([]);

    await expect(claimWalletPendingMutation(
      scopeA,
      operationKeyB,
      "claim-b",
      "credit.tip",
      1_000,
    )).resolves.toMatchObject({ claimed: true });
    await expect(listWalletPendingMutations(scopeA)).resolves.toEqual([
      pending(operationKeyB, { claimId: "claim-b" }),
    ]);
  });
});
