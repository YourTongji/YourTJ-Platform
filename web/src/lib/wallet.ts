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

export function signExactBytes(value: string) {
  const stored = localStorage.getItem(WALLET_SEED_KEY);
  if (!stored) {
    throw new Error("请先在本机生成并绑定钱包公钥");
  }
  const seed = base64ToBytes(stored);
  return bytesToBase64(ed25519.sign(utf8(value), seed));
}
