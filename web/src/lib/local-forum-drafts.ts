import type { DraftPayload } from "@/lib/api/types";

const DATABASE_NAME = "yourtj-forum-drafts";
const DATABASE_VERSION = 2;
const STORE_NAME = "drafts";
const LOCAL_DRAFT_TTL_SECONDS = 7 * 24 * 60 * 60;
const ASSET_ID_PATTERN = /^[1-9][0-9]*$/;
const blockedAccountIds = new Set<string>();
const accountOperationTails = new Map<string, Promise<void>>();

export interface LocalForumDraft {
  accountId: string;
  draftKey: string;
  payload: DraftPayload;
  updatedAt: number;
  expiresAt: number;
}

interface StoredLocalForumDraft extends LocalForumDraft {
  storageKey: string;
  schemaVersion: 1;
}

function localDraftKey(accountId: string, draftKey: string) {
  return `${accountId}\u0000${draftKey}`;
}

function enqueueAccountOperation<TResult>(accountId: string, operation: () => Promise<TResult>) {
  const previous = accountOperationTails.get(accountId) ?? Promise.resolve();
  const result = previous.catch(() => undefined).then(operation);
  const tail = result.then(() => undefined, () => undefined);
  accountOperationTails.set(accountId, tail);
  void tail.then(() => {
    if (accountOperationTails.get(accountId) === tail) accountOperationTails.delete(accountId);
  });
  return result;
}

export function allowLocalForumDraftsForAccount(accountId: string) {
  blockedAccountIds.delete(accountId);
}

function validatedAssetIds(assetIds: string[], maximum: number) {
  if (
    assetIds.length > maximum
    || new Set(assetIds).size !== assetIds.length
    || assetIds.some((assetId) => !ASSET_ID_PATTERN.test(assetId))
  ) {
    throw new Error("draft contains invalid attachment asset identities");
  }
  return [...assetIds];
}

/** Keep only canonical draft fields so Delivery URLs or future response fields cannot enter IndexedDB. */
export function sanitizeLocalDraftPayload(payload: DraftPayload): DraftPayload {
  if (payload.kind === "thread") {
    return {
      kind: "thread",
      boardId: payload.boardId,
      title: payload.title,
      body: payload.body,
      contentFormat: payload.contentFormat,
      tags: [...payload.tags],
      pollQuestion: payload.pollQuestion,
      pollOptions: [...payload.pollOptions],
      attachmentAssetIds: validatedAssetIds(payload.attachmentAssetIds, 8),
    };
  }
  return {
    kind: "comment",
    threadId: payload.threadId,
    body: payload.body,
    contentFormat: payload.contentFormat,
    parentId: payload.parentId,
    attachmentAssetIds: validatedAssetIds(payload.attachmentAssetIds, 4),
  };
}

export function isLocalForumDraftStorageAvailable() {
  return typeof indexedDB !== "undefined";
}

function openDatabase() {
  if (!isLocalForumDraftStorageAvailable()) {
    return Promise.resolve<IDBDatabase | null>(null);
  }
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(DATABASE_NAME, DATABASE_VERSION);
    request.onupgradeneeded = () => {
      const database = request.result;
      const store = database.objectStoreNames.contains(STORE_NAME)
        ? request.transaction?.objectStore(STORE_NAME)
        : database.createObjectStore(STORE_NAME, { keyPath: "storageKey" });
      if (store && !store.indexNames.contains("accountId")) {
        store.createIndex("accountId", "accountId", { unique: false });
      }
      if (store && !store.indexNames.contains("expiresAt")) {
        store.createIndex("expiresAt", "expiresAt", { unique: false });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("cannot open local draft storage"));
    request.onblocked = () => reject(new Error("local draft storage upgrade is blocked"));
  });
}

function completeTransaction(transaction: IDBTransaction) {
  return new Promise<void>((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error ?? new Error("local draft transaction failed"));
    transaction.onabort = () => reject(transaction.error ?? new Error("local draft transaction aborted"));
  });
}

function deleteExpiredRecords(store: IDBObjectStore, now: number) {
  const request = store.index("expiresAt").openKeyCursor(IDBKeyRange.upperBound(now));
  return new Promise<void>((resolve, reject) => {
    request.onsuccess = () => {
      const cursor = request.result;
      if (!cursor) {
        resolve();
        return;
      }
      store.delete(cursor.primaryKey);
      cursor.continue();
    };
    request.onerror = () => reject(request.error ?? new Error("cannot expire local drafts"));
  });
}

export function readLocalForumDraft(accountId: string, draftKey: string) {
  return enqueueAccountOperation(accountId, async () => {
    if (blockedAccountIds.has(accountId)) return null;
    const database = await openDatabase();
    if (!database) return null;
    try {
      const transaction = database.transaction(STORE_NAME, "readwrite");
      const completion = completeTransaction(transaction);
      const store = transaction.objectStore(STORE_NAME);
      const now = Math.floor(Date.now() / 1000);
      const expiration = deleteExpiredRecords(store, now);
      const request = store.get(localDraftKey(accountId, draftKey));
      const record = await new Promise<StoredLocalForumDraft | undefined>((resolve, reject) => {
        request.onsuccess = () => resolve(request.result as StoredLocalForumDraft | undefined);
        request.onerror = () => reject(request.error ?? new Error("cannot read local draft"));
      });
      await expiration;
      await completion;
      if (
        !record
        || record.expiresAt <= now
        || record.schemaVersion !== 1
        || record.accountId !== accountId
        || record.draftKey !== draftKey
      ) {
        return null;
      }
      return {
        accountId: record.accountId,
        draftKey: record.draftKey,
        payload: sanitizeLocalDraftPayload(record.payload),
        updatedAt: record.updatedAt,
        expiresAt: record.expiresAt,
      } satisfies LocalForumDraft;
    } finally {
      database.close();
    }
  });
}

export function writeLocalForumDraft(
  accountId: string,
  draftKey: string,
  payload: DraftPayload,
  updatedAt = Math.floor(Date.now() / 1000),
) {
  if (blockedAccountIds.has(accountId)) return Promise.resolve();
  return enqueueAccountOperation(accountId, async () => {
    const database = await openDatabase();
    if (!database) return;
    try {
      const transaction = database.transaction(STORE_NAME, "readwrite");
      const completion = completeTransaction(transaction);
      const store = transaction.objectStore(STORE_NAME);
      const now = Math.floor(Date.now() / 1000);
      const expiration = deleteExpiredRecords(store, now);
      const storageKey = localDraftKey(accountId, draftKey);
      const expiresAt = updatedAt + LOCAL_DRAFT_TTL_SECONDS;
      if (expiresAt <= now) {
        store.delete(storageKey);
      } else {
        store.put({
          storageKey,
          schemaVersion: 1,
          accountId,
          draftKey,
          payload: sanitizeLocalDraftPayload(payload),
          updatedAt,
          expiresAt,
        } satisfies StoredLocalForumDraft);
      }
      await expiration;
      await completion;
    } finally {
      database.close();
    }
  });
}

export function deleteLocalForumDraft(accountId: string, draftKey: string) {
  return enqueueAccountOperation(accountId, async () => {
    const database = await openDatabase();
    if (!database) return;
    try {
      const transaction = database.transaction(STORE_NAME, "readwrite");
      const completion = completeTransaction(transaction);
      transaction.objectStore(STORE_NAME).delete(localDraftKey(accountId, draftKey));
      await completion;
    } finally {
      database.close();
    }
  });
}

export function clearLocalForumDraftsForAccount(accountId: string) {
  blockedAccountIds.add(accountId);
  return enqueueAccountOperation(accountId, async () => {
    const database = await openDatabase();
    if (!database) return;
    try {
      const transaction = database.transaction(STORE_NAME, "readwrite");
      const completion = completeTransaction(transaction);
      const store = transaction.objectStore(STORE_NAME);
      const request = store.index("accountId").openKeyCursor(IDBKeyRange.only(accountId));
      await new Promise<void>((resolve, reject) => {
        request.onsuccess = () => {
          const cursor = request.result;
          if (!cursor) {
            resolve();
            return;
          }
          store.delete(cursor.primaryKey);
          cursor.continue();
        };
        request.onerror = () => reject(request.error ?? new Error("cannot clear local drafts"));
      });
      await completion;
    } finally {
      database.close();
    }
  });
}
