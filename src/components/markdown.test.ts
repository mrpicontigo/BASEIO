import { describe, expect, it } from "vitest";
import { tidy } from "./Markdown";

describe("tidy", () => {
  it("collapses the blank gaps models sometimes produce", () => {
    expect(tidy("Here is your image:\n\n   \n\n\n\n\nWant another?")).toBe("Here is your image:\n\nWant another?");
  });
});
