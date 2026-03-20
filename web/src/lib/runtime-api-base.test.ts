import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  clearApiBaseUrl,
  getApiBaseUrl,
  getDefaultApiBaseUrl,
  getEffectiveApiBaseUrl,
  normalizeApiBaseUrl,
  setApiBaseUrl,
} from "./runtime-api-base";

describe("runtime-api-base", () => {
  beforeEach(() => {
    window.sessionStorage.clear();
    vi.unstubAllEnvs();
  });

  afterEach(() => {
    window.sessionStorage.clear();
    vi.unstubAllEnvs();
  });

  it("prefers session storage over env default", () => {
    vi.stubEnv("VITE_API_BASE_URL", "https://env.example.com");
    setApiBaseUrl("https://session.example.com");

    expect(getApiBaseUrl()).toBe("https://session.example.com/api");
    expect(getEffectiveApiBaseUrl()).toBe("https://session.example.com/api");
  });

  it("falls back to env default when session storage is empty", () => {
    vi.stubEnv("VITE_API_BASE_URL", "https://env.example.com/base");

    expect(getDefaultApiBaseUrl()).toBe("https://env.example.com/base/api");
    expect(getEffectiveApiBaseUrl()).toBe("https://env.example.com/base/api");
  });

  it("falls back to localhost default when env is missing", () => {
    expect(getDefaultApiBaseUrl()).toBe("http://localhost:3000/api");
  });

  it("normalizes root URLs to /api", () => {
    expect(normalizeApiBaseUrl("https://a.com")).toBe("https://a.com/api");
  });

  it("keeps /api suffix and trims trailing slash", () => {
    expect(normalizeApiBaseUrl("https://a.com/api")).toBe("https://a.com/api");
    expect(normalizeApiBaseUrl("https://a.com/api/")).toBe("https://a.com/api");
  });

  it("rejects invalid protocols", () => {
    expect(() => normalizeApiBaseUrl("ftp://a.com")).toThrow("backend.endpoint.error.protocol");
  });

  it("clears stored session value", () => {
    setApiBaseUrl("https://session.example.com");
    clearApiBaseUrl();

    expect(getApiBaseUrl()).toBeNull();
  });
});
