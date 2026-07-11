import {
  clearAuth,
  readAccessToken,
  readRefreshToken,
  writeAuth,
} from "@/lib/auth-storage";
import type { Account, ApiErrorBody } from "./types";

export const API_BASE_URL =
  (import.meta.env.VITE_API_BASE_URL ?? "/api/v2").replace(/\/$/, "");

export class ApiError extends Error {
  readonly status: number;
  readonly code?: string;
  readonly details?: Record<string, unknown>;

  constructor(status: number, message: string, code?: string, details?: Record<string, unknown>) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.details = details;
  }
}

interface RequestOptions {
  method?: string;
  query?: Record<string, string | number | boolean | null | undefined>;
  body?: unknown;
  headers?: HeadersInit;
  auth?: boolean | "optional";
  signal?: AbortSignal;
}

let refreshPromise: Promise<boolean> | null = null;

function buildUrl(path: string, query?: RequestOptions["query"]) {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const url = new URL(`${API_BASE_URL}${normalizedPath}`, window.location.origin);
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value !== undefined && value !== null && value !== "") {
      url.searchParams.set(key, String(value));
    }
  }
  return url;
}

async function parseError(response: Response) {
  const fallback = response.statusText || "请求失败";
  try {
    const body = (await response.json()) as ApiErrorBody;
    return new ApiError(
      response.status,
      body.error?.message ?? fallback,
      body.error?.code,
      body.error?.details,
    );
  } catch {
    return new ApiError(response.status, fallback);
  }
}

async function refreshTokens() {
  if (refreshPromise) {
    return refreshPromise;
  }
  refreshPromise = (async () => {
    const refreshToken = readRefreshToken();
    if (!refreshToken) {
      return false;
    }
    const response = await fetch(`${API_BASE_URL}/auth/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refreshToken }),
    });
    if (!response.ok) {
      clearAuth();
      return false;
    }
    const data = (await response.json()) as {
      accessToken: string;
      refreshToken: string;
      account: Account;
    };
    writeAuth(data);
    return true;
  })().finally(() => {
    refreshPromise = null;
  });
  return refreshPromise;
}

async function fetchOnce<T>(path: string, options: RequestOptions) {
  const headers = new Headers(options.headers);
  const token = options.auth === false ? null : readAccessToken();
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }
  if (options.body !== undefined && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  const response = await fetch(buildUrl(path, options.query), {
    method: options.method ?? "GET",
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
    signal: options.signal,
  });

  if (response.status === 204 || response.status === 202) {
    return undefined as T;
  }
  if (!response.ok) {
    throw await parseError(response);
  }
  const contentType = response.headers.get("content-type") ?? "";
  if (!contentType.includes("application/json")) {
    return undefined as T;
  }
  return (await response.json()) as T;
}

export async function apiRequest<T>(path: string, options: RequestOptions = {}) {
  try {
    return await fetchOnce<T>(path, options);
  } catch (error) {
    if (error instanceof ApiError && error.status === 401 && options.auth !== false) {
      const refreshed = await refreshTokens();
      if (refreshed) {
        return fetchOnce<T>(path, options);
      }
      if (options.auth === "optional") {
        return fetchOnce<T>(path, { ...options, auth: false });
      }
    }
    throw error;
  }
}
