const SESSION_KEY = "clawlegion.apiBaseUrl";
const DEFAULT_API_BASE_URL = "http://localhost:3000/api";

function getSessionStorage(): Storage | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

function normalizePathname(pathname: string) {
  if (pathname === "/") {
    return "/api";
  }

  const trimmed = pathname.replace(/\/+$/, "");
  return trimmed.endsWith("/api") ? trimmed : `${trimmed}/api`;
}

export function normalizeApiBaseUrl(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error("backend.endpoint.error.required");
  }

  const url = new URL(trimmed);
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("backend.endpoint.error.protocol");
  }

  url.pathname = normalizePathname(url.pathname);
  url.hash = "";

  return url.toString().replace(/\/+$/, "");
}

export function getDefaultApiBaseUrl(): string {
  return normalizeApiBaseUrl(import.meta.env.VITE_API_BASE_URL ?? DEFAULT_API_BASE_URL);
}

export function getApiBaseUrl(): string | null {
  const storage = getSessionStorage();
  const value = storage?.getItem(SESSION_KEY)?.trim();
  return value ? value : null;
}

export function hasStoredApiBaseUrl(): boolean {
  return getApiBaseUrl() !== null;
}

export function getEffectiveApiBaseUrl(): string {
  return getApiBaseUrl() ?? getDefaultApiBaseUrl();
}

export function setApiBaseUrl(value: string): string {
  const normalized = normalizeApiBaseUrl(value);
  const storage = getSessionStorage();
  storage?.setItem(SESSION_KEY, normalized);
  return normalized;
}

export function clearApiBaseUrl(): void {
  const storage = getSessionStorage();
  storage?.removeItem(SESSION_KEY);
}
