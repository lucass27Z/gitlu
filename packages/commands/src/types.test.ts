import { describe, test, expect } from "bun:test";
import { StashEntrySchema } from "./types";

// Tests for the StashEntrySchema focusing on the renamed `is_gitlu` field
// (previously `is_Gitlu`).

describe("StashEntrySchema - is_gitlu field", () => {
  const baseEntry = {
    index: 0,
    reference: "stash@{0}",
    message: "!!Gitlu<main> -> <feature>",
    branch: undefined,
  };

  test("parses a valid gitlu stash entry with is_gitlu=true", () => {
    const result = StashEntrySchema.parse({
      ...baseEntry,
      is_gitlu: true,
    });
    expect(result.is_gitlu).toBe(true);
  });

  test("parses a valid non-gitlu stash entry with is_gitlu=false", () => {
    const result = StashEntrySchema.parse({
      ...baseEntry,
      message: "On main: WIP changes",
      is_gitlu: false,
    });
    expect(result.is_gitlu).toBe(false);
  });

  test("coerces numeric 1 to true for is_gitlu", () => {
    const result = StashEntrySchema.parse({
      ...baseEntry,
      is_gitlu: 1,
    });
    expect(result.is_gitlu).toBe(true);
  });

  test("coerces numeric 0 to false for is_gitlu", () => {
    const result = StashEntrySchema.parse({
      ...baseEntry,
      is_gitlu: 0,
    });
    expect(result.is_gitlu).toBe(false);
  });

  test("coerces string 'true' to true for is_gitlu", () => {
    const result = StashEntrySchema.parse({
      ...baseEntry,
      is_gitlu: "true",
    });
    expect(result.is_gitlu).toBe(true);
  });

  test("coerces string 'false' to false for is_gitlu", () => {
    const result = StashEntrySchema.parse({
      ...baseEntry,
      is_gitlu: "false",
    });
    expect(result.is_gitlu).toBe(false);
  });

  test("schema uses is_gitlu key (not is_Gitlu)", () => {
    // Verify the field is named is_gitlu (snake_case, all lowercase)
    const result = StashEntrySchema.parse({
      ...baseEntry,
      is_gitlu: true,
    });
    expect(Object.prototype.hasOwnProperty.call(result, "is_gitlu")).toBe(true);
    expect(Object.prototype.hasOwnProperty.call(result, "is_Gitlu")).toBe(false);
  });

  test("rejects input missing is_gitlu", () => {
    expect(() =>
      StashEntrySchema.parse({
        ...baseEntry,
        // is_gitlu not provided
      })
    ).toThrow();
  });

  test("parses full stash entry with optional branch present", () => {
    const result = StashEntrySchema.parse({
      index: 1,
      reference: "stash@{1}",
      message: "On feature: WIP",
      branch: "feature",
      is_gitlu: false,
    });
    expect(result.branch).toBe("feature");
    expect(result.is_gitlu).toBe(false);
    expect(result.index).toBe(1);
    expect(result.reference).toBe("stash@{1}");
  });

  test("parses full stash entry with branch absent", () => {
    const result = StashEntrySchema.parse({
      index: 0,
      reference: "stash@{0}",
      message: "!!Gitlu<dev> -> <main>",
      is_gitlu: true,
    });
    expect(result.branch).toBeUndefined();
    expect(result.is_gitlu).toBe(true);
  });

  test("coerces string index to number", () => {
    const result = StashEntrySchema.parse({
      index: "3",
      reference: "stash@{3}",
      message: "test",
      is_gitlu: false,
    });
    expect(result.index).toBe(3);
  });
});