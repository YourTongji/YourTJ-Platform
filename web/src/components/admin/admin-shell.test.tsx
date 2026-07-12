import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LayoutDashboard, Tags } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { AdminShell } from "./admin-shell";

describe("AdminShell", () => {
  it("constrains the mobile navigation scroller to the grid and switches sections", async () => {
    const onActiveChange = vi.fn();
    const user = userEvent.setup();
    render(
      <AdminShell
        active="overview"
        items={[
          {
            id: "overview",
            label: "概览",
            description: "队列与社区状态",
            icon: LayoutDashboard,
          },
          {
            id: "resources",
            label: "内容资源",
            description: "媒体、课程与社区结构",
            icon: Tags,
          },
        ]}
        onActiveChange={onActiveChange}
      >
        <p>当前面板</p>
      </AdminShell>,
    );

    const navigation = screen.getByRole("navigation", { name: "管理后台功能" });
    expect(navigation).toHaveClass("w-full", "min-w-0", "overflow-x-auto");
    expect(navigation.parentElement).toHaveClass("min-w-0", "max-w-full", "overflow-hidden");
    expect(screen.getByRole("button", { name: "概览队列与社区状态" })).toHaveAttribute(
      "aria-current",
      "page",
    );

    await user.click(screen.getByRole("button", { name: "内容资源媒体、课程与社区结构" }));
    expect(onActiveChange).toHaveBeenCalledWith("resources");
  });
});
