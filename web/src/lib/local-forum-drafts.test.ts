import { afterEach, describe, expect, it, vi } from "vitest";

import type { DraftPayload } from "@/lib/api/types";

import {
  isLocalForumDraftStorageAvailable,
  readLocalForumDraft,
  sanitizeLocalDraftPayload,
  writeLocalForumDraft,
} from "./local-forum-drafts";

interface MemoryRecord {
  storageKey: string;
  accountId: string;
  expiresAt: number;
  [key: string]: unknown;
}

class MemoryKeyRange {
  private constructor(
    readonly kind: "only" | "upperBound",
    readonly value: IDBValidKey,
  ) {}

  static only(value: IDBValidKey) {
    return new MemoryKeyRange("only", value);
  }

  static upperBound(value: IDBValidKey) {
    return new MemoryKeyRange("upperBound", value);
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
  private pending = 0;
  private isComplete = false;
  private isCompletionQueued = false;

  constructor(private readonly data: MemoryStoreData) {
    this.queueCompletion();
  }

  objectStore() {
    return new MemoryObjectStore(this.data, this);
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
          : new DOMException("memory IndexedDB request failed");
        request.onerror?.(new Event("error"));
        this.error = request.error;
        this.onerror?.(new Event("error"));
      } finally {
        this.pending -= 1;
        this.queueCompletion();
      }
    });
    return request;
  }

  cursor(indexName: string, range: MemoryKeyRange) {
    const request = new MemoryRequest<IDBCursor | null>();
    const keys = [...this.data.records.entries()]
      .filter(([, record]) => {
        const candidate = record[indexName] as IDBValidKey;
        return range.kind === "only"
          ? candidate === range.value
          : typeof candidate === "number"
            && typeof range.value === "number"
            && candidate <= range.value;
      })
      .map(([key]) => key);
    let position = 0;
    this.pending += 1;
    const advance = () => {
      queueMicrotask(() => {
        const primaryKey = keys[position];
        if (primaryKey === undefined) {
          request.result = null;
          request.onsuccess?.(new Event("success"));
          this.pending -= 1;
          this.queueCompletion();
          return;
        }
        let didContinue = false;
        request.result = {
          primaryKey,
          continue: () => {
            if (didContinue) return;
            didContinue = true;
            position += 1;
            advance();
          },
        } as IDBCursor;
        request.onsuccess?.(new Event("success"));
        if (!didContinue) {
          this.pending -= 1;
          this.queueCompletion();
        }
      });
    };
    advance();
    return request;
  }

  private queueCompletion() {
    if (this.isComplete || this.isCompletionQueued) return;
    this.isCompletionQueued = true;
    queueMicrotask(() => {
      this.isCompletionQueued = false;
      if (this.pending === 0 && !this.isComplete) {
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
  ) {}

  get indexNames() {
    return { contains: (name: string) => this.data.indexes.has(name) };
  }

  createIndex(name: string) {
    this.data.indexes.add(name);
    return {};
  }

  index(name: string) {
    return {
      openKeyCursor: (range: IDBKeyRange) =>
        this.transaction.cursor(name, range as unknown as MemoryKeyRange) as unknown as IDBRequest<IDBCursor | null>,
    };
  }

  get(key: IDBValidKey) {
    return this.transaction.request(() => {
      const record = this.data.records.get(String(key));
      return record ? structuredClone(record) : undefined;
    }) as unknown as IDBRequest<unknown>;
  }

  put(value: MemoryRecord) {
    return this.transaction.request(() => {
      this.data.records.set(value.storageKey, structuredClone(value));
      return value.storageKey;
    }) as unknown as IDBRequest<IDBValidKey>;
  }

  delete(key: IDBValidKey) {
    return this.transaction.request(() => this.data.records.delete(String(key))) as unknown as IDBRequest<undefined>;
  }
}

class MemoryDatabase {
  private readonly stores = new Map<string, MemoryStoreData>();

  get objectStoreNames() {
    return { contains: (name: string) => this.stores.has(name) };
  }

  createObjectStore(name: string) {
    const data = { records: new Map(), indexes: new Set<string>() } satisfies MemoryStoreData;
    this.stores.set(name, data);
    return new MemoryObjectStore(data, new MemoryTransaction(data));
  }

  transaction(name: string) {
    const data = this.stores.get(name);
    if (!data) throw new DOMException("object store not found");
    return new MemoryTransaction(data) as unknown as IDBTransaction;
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

  open() {
    const request = new MemoryOpenRequest();
    queueMicrotask(() => {
      const needsUpgrade = this.database === null;
      this.database ??= new MemoryDatabase();
      request.result = this.database as unknown as IDBDatabase;
      if (needsUpgrade) request.onupgradeneeded?.(new Event("upgradeneeded"));
      request.onsuccess?.(new Event("success"));
    });
    return request as unknown as IDBOpenDBRequest;
  }
}

describe("local forum draft storage", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("copies only canonical fields and retains attachment asset identities", () => {
    const payload = {
      kind: "comment",
      threadId: "42",
      body: "正文",
      contentFormat: "markdown_v1",
      parentId: null,
      attachmentAssetIds: ["7"],
      deliveryUrl: "https://cdn.example/signed-secret",
    } as DraftPayload & { deliveryUrl: string };

    expect(sanitizeLocalDraftPayload(payload)).toEqual({
      kind: "comment",
      threadId: "42",
      body: "正文",
      contentFormat: "markdown_v1",
      parentId: null,
      attachmentAssetIds: ["7"],
    });
  });

  it("fails closed without IndexedDB instead of falling back to localStorage", async () => {
    vi.stubGlobal("indexedDB", undefined);

    expect(isLocalForumDraftStorageAvailable()).toBe(false);
    await expect(readLocalForumDraft("account-1", "comment:42")).resolves.toBeNull();
  });

  it("never extends a local copy beyond seven days from the payload update", async () => {
    vi.stubGlobal("indexedDB", new MemoryIndexedDbFactory() as unknown as IDBFactory);
    vi.stubGlobal("IDBKeyRange", MemoryKeyRange as unknown as typeof IDBKeyRange);
    const updatedAt = 1_700_000_000;
    const expiresAt = updatedAt + 7 * 24 * 60 * 60;
    let now = updatedAt;
    vi.spyOn(Date, "now").mockImplementation(() => now * 1000);
    const payload: DraftPayload = {
      kind: "comment",
      threadId: "42",
      body: "正文",
      contentFormat: "markdown_v1",
      parentId: null,
      attachmentAssetIds: [],
    };

    await writeLocalForumDraft("account-retention", "comment:42", payload, updatedAt);
    now = expiresAt - 1;
    await writeLocalForumDraft("account-retention", "comment:42", payload, updatedAt);
    await expect(readLocalForumDraft("account-retention", "comment:42")).resolves.toMatchObject({
      updatedAt,
      expiresAt,
    });

    now = expiresAt;
    await writeLocalForumDraft("account-retention", "comment:42", payload, updatedAt);
    await expect(readLocalForumDraft("account-retention", "comment:42")).resolves.toBeNull();
  });
});
