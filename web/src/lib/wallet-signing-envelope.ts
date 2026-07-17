import type { CreditSigningAction } from "@/lib/wallet-mutations";

const ENVELOPE_FIELDS = [
  "accountId",
  "action",
  "expiresAt",
  "idempotencyKey",
  "intentId",
  "ledgerEntry",
  "publicKey",
  "requestHash",
  "snapshot",
  "version",
] as const;
const LEDGER_FIELDS = [
  "amount",
  "from_account",
  "metadata",
  "nonce",
  "signer",
  "timestamp",
  "to_account",
  "tx_id",
  "type",
] as const;
const I64_MAX = 9_223_372_036_854_775_807n;
const UUID_V4_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const SHA256_HEX_PATTERN = /^[0-9a-f]{64}$/;

export type WalletSigningConfirmation =
  | { kind: "tip" }
  | { kind: "taskCreate" }
  | {
    kind: "productPurchase";
    product: {
      id: string;
      price: number;
      sellerId: string;
      status: string;
      stock: number;
    };
  }
  | {
    kind: "taskAction";
    task: {
      id: string;
      creatorId: string;
      acceptorId: string | null;
      rewardAmount: number;
      status: string;
    };
  }
  | {
    kind: "purchaseAction";
    purchase: {
      id: string;
      buyerId: string;
      sellerId: string;
      amount: number;
      status: string;
    };
  };

export interface WalletSigningEnvelopeExpectation {
  accountId: string;
  action: CreditSigningAction;
  request: Record<string, unknown>;
  confirmation: WalletSigningConfirmation;
  expiresAt: number;
  idempotencyKey: string;
  intentId: string;
  publicKey: string;
  walletBalance: number;
}

const OMIT_PROPERTY = Symbol("omit-property");

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function hasExactFields(
  value: unknown,
  fields: readonly string[],
): value is Record<string, unknown> {
  if (!isPlainObject(value)) return false;
  const actual = Object.keys(value).sort();
  const expected = [...fields].sort();
  return actual.length === expected.length
    && actual.every((field, index) => field === expected[index]);
}

function jsonWireValue(value: unknown, ancestors: Set<object>, inArray: boolean): unknown {
  if (value === undefined) return inArray ? null : OMIT_PROPERTY;
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("non-finite JSON number");
    return Object.is(value, -0) ? 0 : value;
  }
  if (typeof value !== "object") throw new Error("non-JSON value");
  if (ancestors.has(value)) throw new Error("cyclic JSON value");
  ancestors.add(value);
  try {
    if (Array.isArray(value)) {
      return value.map((item) => jsonWireValue(item, ancestors, true));
    }
    if (!isPlainObject(value)) throw new Error("non-plain JSON object");
    const result: Record<string, unknown> = {};
    for (const key of Object.keys(value).sort()) {
      const normalized = jsonWireValue(value[key], ancestors, false);
      if (normalized !== OMIT_PROPERTY) result[key] = normalized;
    }
    return result;
  } finally {
    ancestors.delete(value);
  }
}

function canonicalJson(value: unknown) {
  const normalized = jsonWireValue(value, new Set<object>(), false);
  if (normalized === OMIT_PROPERTY) throw new Error("missing JSON root");
  return JSON.stringify(normalized);
}

function hex(bytes: Uint8Array) {
  let result = "";
  for (const byte of bytes) result += byte.toString(16).padStart(2, "0");
  return result;
}

async function requestHash(request: unknown) {
  const bytes = new TextEncoder().encode(canonicalJson(request));
  return hex(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)));
}

function isSafeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value);
}

function isPositiveSafeInteger(value: unknown): value is number {
  return isSafeInteger(value) && value > 0;
}

function isCanonicalI64String(value: unknown, allowZero = false): value is string {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) return false;
  try {
    const parsed = BigInt(value);
    return parsed <= I64_MAX && (allowZero ? parsed >= 0n : parsed > 0n);
  } catch {
    return false;
  }
}

function normalizeRequest(
  action: CreditSigningAction,
  request: Record<string, unknown>,
): Record<string, unknown> | null {
  const wire = jsonWireValue(request, new Set<object>(), false);
  if (!isPlainObject(wire)) return null;
  if (action === "credit.tip") {
    if (!hasExactFields(wire, ["amount", "targetId", "targetType", "toAccountId"])
      || !isCanonicalI64String(wire.toAccountId)
      || !isPositiveSafeInteger(wire.amount)
      || !(wire.targetType === "review"
        || wire.targetType === "thread"
        || wire.targetType === "comment")
      || !isCanonicalI64String(wire.targetId)) return null;
    return wire;
  }
  if (action === "credit.task.create") {
    if (!hasExactFields(wire, ["title", "rewardAmount"])
      && !hasExactFields(wire, ["description", "title", "rewardAmount"])
      && !hasExactFields(wire, ["contactInfo", "title", "rewardAmount"])
      && !hasExactFields(wire, ["contactInfo", "description", "title", "rewardAmount"])) {
      return null;
    }
    if (typeof wire.title !== "string"
      || wire.title.length === 0
      || !isPositiveSafeInteger(wire.rewardAmount)
      || !(wire.description === undefined
        || wire.description === null
        || typeof wire.description === "string")
      || !(wire.contactInfo === undefined
        || wire.contactInfo === null
        || typeof wire.contactInfo === "string")) return null;
    return {
      contactInfo: wire.contactInfo ?? null,
      description: wire.description ?? null,
      rewardAmount: wire.rewardAmount,
      title: wire.title,
    };
  }
  if (action === "credit.product.purchase") {
    if (!hasExactFields(wire, ["productId"])
      || !isCanonicalI64String(wire.productId)) return null;
    return wire;
  }
  if (!hasExactFields(wire, ["action", "id"])
    || !isCanonicalI64String(wire.id)) return null;
  if (action === "credit.task.action") {
    if (!(wire.action === "confirm"
      || wire.action === "cancel"
      || wire.action === "reject"
      || wire.action === "delete")) return null;
  } else if (!(wire.action === "confirm" || wire.action === "cancel")) {
    return null;
  }
  return wire;
}

function validateBalanceSnapshot(snapshot: unknown, amount: number, walletBalance: number) {
  return hasExactFields(snapshot, ["balance"])
    && isSafeInteger(snapshot.balance)
    && snapshot.balance === walletBalance
    && walletBalance >= amount;
}

function validateProductSnapshot(
  snapshot: unknown,
  accountId: string,
  confirmation: Extract<WalletSigningConfirmation, { kind: "productPurchase" }>,
) {
  const product = confirmation.product;
  return hasExactFields(snapshot, ["price", "sellerId", "status", "stock"])
    && isCanonicalI64String(product.id)
    && isCanonicalI64String(product.sellerId)
    && product.sellerId !== accountId
    && isPositiveSafeInteger(product.price)
    && isSafeInteger(product.stock)
    && product.stock > 0
    && product.status === "on_sale"
    && snapshot.price === product.price
    && snapshot.sellerId === product.sellerId
    && snapshot.status === product.status
    && snapshot.stock === product.stock;
}

function validateTaskActionSnapshot(
  snapshot: unknown,
  accountId: string,
  request: Record<string, unknown>,
  confirmation: Extract<WalletSigningConfirmation, { kind: "taskAction" }>,
) {
  const task = confirmation.task;
  if (!hasExactFields(snapshot, ["actorId", "amount", "partyA", "partyB", "status"])
    || request.id !== task.id
    || !isCanonicalI64String(task.id)
    || !isCanonicalI64String(task.creatorId)
    || !(task.acceptorId === null || isCanonicalI64String(task.acceptorId))
    || !isPositiveSafeInteger(task.rewardAmount)
    || snapshot.actorId !== accountId
    || snapshot.partyA !== task.creatorId
    || snapshot.partyB !== (task.acceptorId ?? "0")
    || snapshot.amount !== task.rewardAmount
    || snapshot.status !== task.status) return false;
  if (request.action === "confirm") {
    return accountId === task.creatorId
      && task.acceptorId !== null
      && task.status === "submitted";
  }
  if (request.action === "cancel") {
    return accountId === task.creatorId
      && (task.status === "open"
        || task.status === "in_progress"
        || task.status === "submitted");
  }
  if (request.action === "reject") {
    return accountId === task.acceptorId
      && (task.status === "in_progress" || task.status === "submitted");
  }
  return request.action === "delete"
    && accountId === task.creatorId
    && task.status === "open";
}

function validatePurchaseActionSnapshot(
  snapshot: unknown,
  accountId: string,
  request: Record<string, unknown>,
  confirmation: Extract<WalletSigningConfirmation, { kind: "purchaseAction" }>,
) {
  const purchase = confirmation.purchase;
  if (!hasExactFields(snapshot, ["actorId", "amount", "partyA", "partyB", "status"])
    || request.id !== purchase.id
    || !isCanonicalI64String(purchase.id)
    || !isCanonicalI64String(purchase.buyerId)
    || !isCanonicalI64String(purchase.sellerId)
    || !isPositiveSafeInteger(purchase.amount)
    || snapshot.actorId !== accountId
    || snapshot.partyA !== purchase.buyerId
    || snapshot.partyB !== purchase.sellerId
    || snapshot.amount !== purchase.amount
    || snapshot.status !== purchase.status
    || accountId !== purchase.buyerId) return false;
  if (request.action === "confirm") return purchase.status === "delivered";
  return request.action === "cancel"
    && (purchase.status === "pending" || purchase.status === "accepted");
}

function validateSnapshot(
  snapshot: unknown,
  expected: WalletSigningEnvelopeExpectation,
  request: Record<string, unknown>,
) {
  const { confirmation } = expected;
  if (expected.action === "credit.tip" && confirmation.kind === "tip") {
    return validateBalanceSnapshot(snapshot, request.amount as number, expected.walletBalance);
  }
  if (expected.action === "credit.task.create" && confirmation.kind === "taskCreate") {
    return validateBalanceSnapshot(
      snapshot,
      request.rewardAmount as number,
      expected.walletBalance,
    );
  }
  if (expected.action === "credit.product.purchase"
    && confirmation.kind === "productPurchase"
    && request.productId === confirmation.product.id) {
    return expected.walletBalance >= confirmation.product.price
      && validateProductSnapshot(snapshot, expected.accountId, confirmation);
  }
  if (expected.action === "credit.task.action" && confirmation.kind === "taskAction") {
    return validateTaskActionSnapshot(snapshot, expected.accountId, request, confirmation);
  }
  if (expected.action === "credit.purchase.action" && confirmation.kind === "purchaseAction") {
    return validatePurchaseActionSnapshot(snapshot, expected.accountId, request, confirmation);
  }
  return false;
}

function validateLedgerBase(
  ledgerEntry: unknown,
  accountId: string,
  expiresAt: number,
): ledgerEntry is Record<string, unknown> {
  return hasExactFields(ledgerEntry, LEDGER_FIELDS)
    && UUID_V4_PATTERN.test(String(ledgerEntry.tx_id))
    && UUID_V4_PATTERN.test(String(ledgerEntry.nonce))
    && ledgerEntry.from_account === accountId
    && ledgerEntry.signer === accountId
    && isPositiveSafeInteger(ledgerEntry.amount)
    && isPositiveSafeInteger(ledgerEntry.timestamp)
    && ledgerEntry.timestamp <= expiresAt;
}

function validateLedgerEntry(
  ledgerEntry: unknown,
  expected: WalletSigningEnvelopeExpectation,
  request: Record<string, unknown>,
  snapshot: Record<string, unknown>,
) {
  if (expected.action === "credit.task.action" || expected.action === "credit.purchase.action") {
    return ledgerEntry === null;
  }
  if (!validateLedgerBase(ledgerEntry, expected.accountId, expected.expiresAt)) return false;
  if (expected.action === "credit.tip") {
    return ledgerEntry.type === "tip"
      && ledgerEntry.to_account === request.toAccountId
      && ledgerEntry.amount === request.amount
      && hasExactFields(
        ledgerEntry.metadata,
        ["signing_intent_id", "target_id", "target_type"],
      )
      && ledgerEntry.metadata.signing_intent_id === expected.intentId
      && ledgerEntry.metadata.target_id === request.targetId
      && ledgerEntry.metadata.target_type === request.targetType;
  }
  if (ledgerEntry.type !== "escrow_hold" || ledgerEntry.to_account !== null) return false;
  if (expected.action === "credit.task.create") {
    return ledgerEntry.amount === request.rewardAmount
      && hasExactFields(ledgerEntry.metadata, ["signing_intent_id"])
      && ledgerEntry.metadata.signing_intent_id === expected.intentId;
  }
  return ledgerEntry.amount === snapshot.price
    && hasExactFields(ledgerEntry.metadata, ["product_id", "signing_intent_id"])
    && ledgerEntry.metadata.product_id === request.productId
    && ledgerEntry.metadata.signing_intent_id === expected.intentId;
}

/** Verify exact server bytes against the request and UI state the user is confirming. */
export async function walletSigningEnvelopeMatches(
  signingBytes: string,
  expected: WalletSigningEnvelopeExpectation,
) {
  try {
    const decoded: unknown = JSON.parse(signingBytes);
    if (!hasExactFields(decoded, ENVELOPE_FIELDS)
      || canonicalJson(decoded) !== signingBytes
      || decoded.version !== 1
      || decoded.intentId !== expected.intentId
      || !UUID_V4_PATTERN.test(expected.intentId)
      || decoded.accountId !== expected.accountId
      || !isCanonicalI64String(expected.accountId)
      || decoded.publicKey !== expected.publicKey
      || !isSafeInteger(expected.walletBalance)
      || decoded.action !== expected.action
      || decoded.expiresAt !== expected.expiresAt
      || !isSafeInteger(decoded.expiresAt)
      || decoded.idempotencyKey !== expected.idempotencyKey
      || typeof decoded.requestHash !== "string"
      || !SHA256_HEX_PATTERN.test(decoded.requestHash)) return false;

    const normalizedRequest = normalizeRequest(expected.action, expected.request);
    if (!normalizedRequest
      || (expected.action === "credit.tip"
        && normalizedRequest.toAccountId === expected.accountId)
      || decoded.requestHash !== await requestHash(normalizedRequest)
      || !validateSnapshot(decoded.snapshot, expected, normalizedRequest)
      || !isPlainObject(decoded.snapshot)
      || !validateLedgerEntry(decoded.ledgerEntry, expected, normalizedRequest, decoded.snapshot)) {
      return false;
    }
    return true;
  } catch {
    return false;
  }
}
