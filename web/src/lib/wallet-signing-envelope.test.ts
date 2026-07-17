import { describe, expect, it } from "vitest";

import {
  walletSigningEnvelopeMatches,
  type WalletSigningEnvelopeExpectation,
} from "./wallet-signing-envelope";

const accountId = "101";
const counterpartyId = "202";
const intentId = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const transactionId = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const nonce = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const expiresAt = 1_800_000_300;
const idempotencyKey = "credit:request-1";
const publicKey = "wallet-public-key";
const walletBalance = 100;

type Envelope = Record<string, unknown> & {
  ledgerEntry: Record<string, unknown> | null;
  snapshot: Record<string, unknown>;
};

function canonical(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  const object = value as Record<string, unknown>;
  return `{${Object.keys(object).sort().map((key) => (
    `${JSON.stringify(key)}:${canonical(object[key])}`
  )).join(",")}}`;
}

function hex(bytes: Uint8Array) {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function hash(request: unknown) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonical(request)));
  return hex(new Uint8Array(digest));
}

function baseLedger(overrides: Record<string, unknown>): Record<string, unknown> {
  return {
    amount: 7,
    from_account: accountId,
    metadata: {},
    nonce,
    signer: accountId,
    timestamp: 1_800_000_000,
    to_account: null,
    tx_id: transactionId,
    type: "escrow_hold",
    ...overrides,
  };
}

async function tipFixture() {
  const request = {
    toAccountId: counterpartyId,
    amount: 7,
    targetType: "thread",
    targetId: "42",
  };
  const expectation: WalletSigningEnvelopeExpectation = {
    accountId,
    action: "credit.tip",
    request,
    confirmation: { kind: "tip" },
    expiresAt,
    idempotencyKey,
    intentId,
    publicKey,
    walletBalance,
  };
  const envelope: Envelope = {
    accountId,
    action: "credit.tip",
    expiresAt,
    idempotencyKey,
    intentId,
    ledgerEntry: baseLedger({
      amount: 7,
      metadata: {
        signing_intent_id: intentId,
        target_id: "42",
        target_type: "thread",
      },
      to_account: counterpartyId,
      type: "tip",
    }),
    publicKey,
    requestHash: await hash(request),
    snapshot: { balance: walletBalance },
    version: 1,
  };
  return { envelope, expectation };
}

async function taskCreateFixture() {
  const request = { title: "Campus task", rewardAmount: 12 };
  const normalizedRequest = {
    contactInfo: null,
    description: null,
    rewardAmount: 12,
    title: "Campus task",
  };
  const expectation: WalletSigningEnvelopeExpectation = {
    accountId,
    action: "credit.task.create",
    request,
    confirmation: { kind: "taskCreate" },
    expiresAt,
    idempotencyKey,
    intentId,
    publicKey,
    walletBalance,
  };
  const envelope: Envelope = {
    accountId,
    action: "credit.task.create",
    expiresAt,
    idempotencyKey,
    intentId,
    ledgerEntry: baseLedger({
      amount: 12,
      metadata: { signing_intent_id: intentId },
    }),
    publicKey,
    requestHash: await hash(normalizedRequest),
    snapshot: { balance: walletBalance },
    version: 1,
  };
  return { envelope, expectation };
}

async function productFixture() {
  const request = { productId: "303" };
  const product = {
    id: "303",
    price: 25,
    sellerId: counterpartyId,
    status: "on_sale",
    stock: 2,
  };
  const expectation: WalletSigningEnvelopeExpectation = {
    accountId,
    action: "credit.product.purchase",
    request,
    confirmation: { kind: "productPurchase", product },
    expiresAt,
    idempotencyKey,
    intentId,
    publicKey,
    walletBalance,
  };
  const envelope: Envelope = {
    accountId,
    action: "credit.product.purchase",
    expiresAt,
    idempotencyKey,
    intentId,
    ledgerEntry: baseLedger({
      amount: 25,
      metadata: { product_id: "303", signing_intent_id: intentId },
    }),
    publicKey,
    requestHash: await hash(request),
    snapshot: { price: 25, sellerId: counterpartyId, status: "on_sale", stock: 2 },
    version: 1,
  };
  return { envelope, expectation };
}

async function taskActionFixture() {
  const request = { id: "404", action: "confirm" };
  const expectation: WalletSigningEnvelopeExpectation = {
    accountId,
    action: "credit.task.action",
    request,
    confirmation: {
      kind: "taskAction",
      task: {
        id: "404",
        creatorId: accountId,
        acceptorId: counterpartyId,
        rewardAmount: 20,
        status: "submitted",
      },
    },
    expiresAt,
    idempotencyKey,
    intentId,
    publicKey,
    walletBalance,
  };
  const envelope: Envelope = {
    accountId,
    action: "credit.task.action",
    expiresAt,
    idempotencyKey,
    intentId,
    ledgerEntry: null,
    publicKey,
    requestHash: await hash(request),
    snapshot: {
      actorId: accountId,
      amount: 20,
      partyA: accountId,
      partyB: counterpartyId,
      status: "submitted",
    },
    version: 1,
  };
  return { envelope, expectation };
}

async function purchaseActionFixture() {
  const request = { id: "505", action: "confirm" };
  const expectation: WalletSigningEnvelopeExpectation = {
    accountId,
    action: "credit.purchase.action",
    request,
    confirmation: {
      kind: "purchaseAction",
      purchase: {
        id: "505",
        buyerId: accountId,
        sellerId: counterpartyId,
        amount: 30,
        status: "delivered",
      },
    },
    expiresAt,
    idempotencyKey,
    intentId,
    publicKey,
    walletBalance,
  };
  const envelope: Envelope = {
    accountId,
    action: "credit.purchase.action",
    expiresAt,
    idempotencyKey,
    intentId,
    ledgerEntry: null,
    publicKey,
    requestHash: await hash(request),
    snapshot: {
      actorId: accountId,
      amount: 30,
      partyA: accountId,
      partyB: counterpartyId,
      status: "delivered",
    },
    version: 1,
  };
  return { envelope, expectation };
}

function signingBytes(envelope: Envelope) {
  return canonical(envelope);
}

function cloneEnvelope(envelope: Envelope): Envelope {
  return structuredClone(envelope);
}

describe("wallet signing envelope verification", () => {
  it("accepts the five exact server envelope shapes", async () => {
    for (const fixture of [
      await tipFixture(),
      await taskCreateFixture(),
      await productFixture(),
      await taskActionFixture(),
      await purchaseActionFixture(),
    ]) {
      await expect(walletSigningEnvelopeMatches(
        signingBytes(fixture.envelope),
        fixture.expectation,
      )).resolves.toBe(true);
    }
  });

  it("normalizes omitted task optionals to null when recomputing requestHash", async () => {
    const fixture = await taskCreateFixture();
    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      {
        ...fixture.expectation,
        request: {
          title: "Campus task",
          rewardAmount: 12,
          description: undefined,
          contactInfo: undefined,
        },
      },
    )).resolves.toBe(true);
  });

  it("rejects non-canonical bytes and extra or missing envelope fields", async () => {
    const fixture = await tipFixture();
    const nonCanonical = signingBytes(fixture.envelope).replace("{", "{ ");
    expect(nonCanonical).not.toBe(signingBytes(fixture.envelope));
    await expect(walletSigningEnvelopeMatches(nonCanonical, fixture.expectation)).resolves
      .toBe(false);

    const extra = cloneEnvelope(fixture.envelope);
    extra.untrusted = true;
    await expect(walletSigningEnvelopeMatches(signingBytes(extra), fixture.expectation)).resolves
      .toBe(false);

    const missing = cloneEnvelope(fixture.envelope);
    delete (missing as Record<string, unknown>).snapshot;
    await expect(walletSigningEnvelopeMatches(signingBytes(missing), fixture.expectation)).resolves
      .toBe(false);
  });

  it("rejects a recomputed canonical envelope with a tampered requestHash", async () => {
    const fixture = await taskCreateFixture();
    fixture.envelope.requestHash = "0".repeat(64);

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it("rejects a self-tip even when its request and ledger proof agree", async () => {
    const fixture = await tipFixture();
    const request = { ...fixture.expectation.request, toAccountId: accountId };
    fixture.expectation.request = request;
    fixture.envelope.requestHash = await hash(request);
    (fixture.envelope.ledgerEntry as Record<string, unknown>).to_account = accountId;

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it.each([
    ["amount", (entry: Record<string, unknown>) => { entry.amount = 70; }],
    ["recipient", (entry: Record<string, unknown>) => { entry.to_account = "303"; }],
    ["target metadata", (entry: Record<string, unknown>) => {
      (entry.metadata as Record<string, unknown>).target_id = "43";
    }],
    ["intent metadata", (entry: Record<string, unknown>) => {
      (entry.metadata as Record<string, unknown>).signing_intent_id = transactionId;
    }],
    ["extra ledger field", (entry: Record<string, unknown>) => { entry.extra = true; }],
    ["missing ledger field", (entry: Record<string, unknown>) => { delete entry.nonce; }],
  ])("rejects canonical tip ledger tampering: %s", async (_label, mutate) => {
    const fixture = await tipFixture();
    mutate(fixture.envelope.ledgerEntry as Record<string, unknown>);

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it.each([
    ["price", "price", 26],
    ["seller", "sellerId", "303"],
    ["status", "status", "sold_out"],
    ["stock", "stock", 1],
  ])("rejects a product %s snapshot that differs from the displayed card", async (
    _label,
    field,
    value,
  ) => {
    const fixture = await productFixture();
    fixture.envelope.snapshot[field] = value;

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it("rejects a product purchase when the owner wallet cannot cover the signed price", async () => {
    const fixture = await productFixture();
    fixture.expectation.walletBalance = 24;

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it("does not treat cancelled-task deletion as a signed value action", async () => {
    const fixture = await taskActionFixture();
    const request = { id: "404", action: "delete" };
    fixture.expectation.request = request;
    fixture.expectation.confirmation = {
      kind: "taskAction",
      task: {
        id: "404",
        creatorId: accountId,
        acceptorId: counterpartyId,
        rewardAmount: 20,
        status: "cancelled",
      },
    };
    fixture.envelope.requestHash = await hash(request);
    fixture.envelope.snapshot.status = "cancelled";

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it.each(["actorId", "amount", "partyA", "partyB", "status"])(
    "rejects a task action %s snapshot mismatch",
    async (field) => {
      const fixture = await taskActionFixture();
      fixture.envelope.snapshot[field] = field === "amount" ? 21 : "tampered";

      await expect(walletSigningEnvelopeMatches(
        signingBytes(fixture.envelope),
        fixture.expectation,
      )).resolves.toBe(false);
    },
  );

  it("rejects a task cancellation from an unknown future status", async () => {
    const fixture = await taskActionFixture();
    const request = { id: "404", action: "cancel" };
    fixture.expectation.request = request;
    fixture.expectation.confirmation = {
      kind: "taskAction",
      task: {
        id: "404",
        creatorId: accountId,
        acceptorId: counterpartyId,
        rewardAmount: 20,
        status: "future_state",
      },
    };
    fixture.envelope.requestHash = await hash(request);
    fixture.envelope.snapshot.status = "future_state";

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it("requires a null ledgerEntry for task and purchase actions", async () => {
    for (const fixture of [await taskActionFixture(), await purchaseActionFixture()]) {
      fixture.envelope.ledgerEntry = baseLedger({ amount: 1 });
      await expect(walletSigningEnvelopeMatches(
        signingBytes(fixture.envelope),
        fixture.expectation,
      )).resolves.toBe(false);
    }
  });

  it("binds the balance snapshot to the same owner-wallet response", async () => {
    const fixture = await tipFixture();
    fixture.envelope.snapshot.balance = walletBalance + 1;

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it("rejects invalid UUID and integer types even when other semantics match", async () => {
    const fixture = await tipFixture();
    (fixture.envelope.ledgerEntry as Record<string, unknown>).nonce = "not-a-uuid";
    (fixture.envelope.ledgerEntry as Record<string, unknown>).timestamp = 1.5;

    await expect(walletSigningEnvelopeMatches(
      signingBytes(fixture.envelope),
      fixture.expectation,
    )).resolves.toBe(false);
  });

  it.each([0, -1, expiresAt + 1])(
    "rejects ledger timestamp %s outside the signed intent lifetime",
    async (timestamp) => {
      const fixture = await tipFixture();
      (fixture.envelope.ledgerEntry as Record<string, unknown>).timestamp = timestamp;

      await expect(walletSigningEnvelopeMatches(
        signingBytes(fixture.envelope),
        fixture.expectation,
      )).resolves.toBe(false);
    },
  );
});
