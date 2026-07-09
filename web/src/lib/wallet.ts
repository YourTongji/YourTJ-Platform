import { ed25519 } from "@noble/curves/ed25519";

const WALLET_SEED_KEY = "yourtj.walletSeed";

function bytesToBase64(bytes: Uint8Array) {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

function base64ToBytes(value: string) {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function utf8(value: string) {
  return new TextEncoder().encode(value);
}

function sortValue(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sortValue);
  }
  if (value && typeof value === "object") {
    const sorted: Record<string, unknown> = {};
    for (const key of Object.keys(value as Record<string, unknown>).sort()) {
      const item = (value as Record<string, unknown>)[key];
      if (item !== undefined) {
        sorted[key] = sortValue(item);
      }
    }
    return sorted;
  }
  return value;
}

export function canonicalJson(value: unknown) {
  return JSON.stringify(sortValue(value));
}

export function hasLocalWallet() {
  return Boolean(localStorage.getItem(WALLET_SEED_KEY));
}

export function createLocalWallet() {
  const seed = crypto.getRandomValues(new Uint8Array(32));
  localStorage.setItem(WALLET_SEED_KEY, bytesToBase64(seed));
  return getLocalWallet();
}

export function clearLocalWallet() {
  localStorage.removeItem(WALLET_SEED_KEY);
}

export function getLocalWallet() {
  const stored = localStorage.getItem(WALLET_SEED_KEY);
  if (!stored) {
    return null;
  }
  const seed = base64ToBytes(stored);
  const publicKey = ed25519.getPublicKey(seed);
  return {
    publicKey: bytesToBase64(publicKey),
  };
}

export function signPayload(payload: unknown) {
  const stored = localStorage.getItem(WALLET_SEED_KEY);
  if (!stored) {
    throw new Error("请先在本机生成并绑定钱包公钥");
  }
  const seed = base64ToBytes(stored);
  const signature = ed25519.sign(utf8(canonicalJson(payload)), seed);
  return bytesToBase64(signature);
}

export function buildClientSignedPayload(payload: unknown) {
  return {
    payload,
    timestamp: Math.floor(Date.now() / 1000),
    nonce: crypto.randomUUID(),
  };
}
