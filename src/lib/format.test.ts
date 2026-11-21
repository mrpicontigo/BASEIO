import { describe, expect, it } from "vitest";
import { formatBytes, formatParams } from "./format";
import { toAppError } from "./api";

describe("formatBytes", () => {
  it("handles small and large values", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(4.7 * 1024 ** 3)).toBe("4.7 GB");
    expect(formatBytes(null)).toBe("—");
  });
});

describe("formatParams", () => {
  it("formats millions and billions", () => {
    expect(formatParams(0.6)).toBe("600M");
    expect(formatParams(8)).toBe("8B");
    expect(formatParams(3.21)).toBe("3.2B");
  });
});

describe("toAppError", () => {
  it("passes through structured errors and wraps strings", () => {
    expect(toAppError({ kind: "hardware", message: "x" })).toEqual({ kind: "hardware", message: "x" });
    expect(toAppError("boom")).toEqual({ kind: "unknown", message: "boom" });
  });
});
