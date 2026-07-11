import axe from "axe-core";
import { expect } from "vitest";

export async function expectNoAccessibilityViolations(container: Element) {
  const result = await axe.run(container);
  expect(result.violations).toEqual([]);
}
