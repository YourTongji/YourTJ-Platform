import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { RouteLoadingState } from "./route-loading-state";

describe("RouteLoadingState", () => {
  it("announces lazy route loading without exposing decorative skeletons", async () => {
    const view = render(<RouteLoadingState />);

    expect(screen.getByRole("status")).toHaveTextContent("页面加载中");
    await expectNoAccessibilityViolations(view.container);
  });
});
