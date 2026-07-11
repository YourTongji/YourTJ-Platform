import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { PageTransition } from "./page-transition";

describe("PageTransition", () => {
  it("wraps route content in the reduced-motion-aware animation surface", async () => {
    const view = render(
      <MemoryRouter initialEntries={["/forum"]}>
        <PageTransition>
          <main aria-label="社区内容">内容</main>
        </PageTransition>
      </MemoryRouter>,
    );

    expect(screen.getByRole("main", { name: "社区内容" }).parentElement).toHaveClass("motion-page");
    await expectNoAccessibilityViolations(view.container);
  });
});
