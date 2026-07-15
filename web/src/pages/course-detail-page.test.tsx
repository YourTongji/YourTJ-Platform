import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { CourseDetailPage } from "./course-detail-page";

const apiMocks = vi.hoisted(() => ({
  course: vi.fn(),
  courseAiSummary: vi.fn(),
  courseReviews: vi.fn(),
  createReview: vi.fn(),
  editReview: vi.fn(),
  likeReview: vi.fn(),
  relatedCourses: vi.fn(),
  reportReview: vi.fn(),
  review: vi.fn(),
  unlikeReview: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ isAuthenticated: true }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: apiMocks,
}));

vi.mock("@/components/reviews/review-report-dialog", () => ({
  ReviewReportDialog: ({
    open,
    onSubmit,
  }: {
    open: boolean;
    onSubmit: (reason: string, captchaToken: string) => void;
  }) => open ? (
    <button type="button" onClick={() => onSubmit("垃圾或推广信息：重复推广", "captcha-token")}>
      确认举报
    </button>
  ) : null,
}));

const review = {
  id: "review-1",
  courseId: "course-1",
  rating: 4,
  comment: "作业反馈及时",
  score: "A",
  semester: "2026 春",
  authorHandle: "alice",
  authorAvatar: null,
  approveCount: 8,
  viewerLiked: false,
  canEdit: false,
  canReport: true,
  status: "visible" as const,
  createdAt: 1_700_000_000,
};

function renderPage(initialEntry = "/courses/course-1") {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <Routes>
          <Route path="/courses/:id" element={<CourseDetailPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("CourseDetailPage review interactions", () => {
  beforeEach(() => {
    apiMocks.course.mockReset().mockResolvedValue({
      id: "course-1",
      code: "CS101",
      name: "程序设计",
      department: "计算机系",
      credit: 3,
      reviewAvg: 4,
      reviewCount: 1,
      aliases: [],
      teachers: [],
    });
    apiMocks.courseAiSummary.mockReset().mockResolvedValue({});
    apiMocks.courseReviews.mockReset().mockResolvedValue({
      items: [review],
      nextCursor: null,
      hasMore: false,
    });
    apiMocks.createReview.mockReset();
    apiMocks.editReview.mockReset().mockImplementation(async (_id, input) => ({
      ...review,
      ...input,
      canEdit: true,
      canReport: false,
    }));
    apiMocks.likeReview.mockReset().mockResolvedValue(undefined);
    apiMocks.relatedCourses.mockReset().mockResolvedValue([]);
    apiMocks.reportReview.mockReset().mockResolvedValue(undefined);
    apiMocks.review.mockReset().mockResolvedValue(review);
    apiMocks.unlikeReview.mockReset().mockResolvedValue(undefined);
  });

  it("uses the server viewerLiked state to unlike a review", async () => {
    apiMocks.courseReviews.mockResolvedValue({
      items: [{ ...review, viewerLiked: true }],
      nextCursor: null,
      hasMore: false,
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "取消点赞" }));

    await waitFor(() => expect(apiMocks.unlikeReview).toHaveBeenCalledWith("review-1"));
    expect(apiMocks.likeReview).not.toHaveBeenCalled();
  });

  it("lets the owner edit without exposing like or report actions", async () => {
    apiMocks.courseReviews.mockResolvedValue({
      items: [{ ...review, canEdit: true, canReport: false }],
      nextCursor: null,
      hasMore: false,
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "编辑" }));
    expect(screen.queryByRole("button", { name: "点赞" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "举报" })).not.toBeInTheDocument();
    const comment = screen.getByLabelText("正文");
    await user.clear(comment);
    await user.type(comment, "更新后的课程体验");
    await user.click(screen.getByRole("button", { name: "保存修改" }));

    await waitFor(() => expect(apiMocks.editReview).toHaveBeenCalledWith("review-1", {
      rating: 4,
      semester: "2026 春",
      score: "A",
      comment: "更新后的课程体验",
    }));
  });

  it("removes the report action after the server accepts a report", async () => {
    const serverPage = {
      items: [{ ...review, canReport: false }],
      nextCursor: null,
      hasMore: false,
    };
    let resolveReconciliation!: (page: typeof serverPage) => void;
    const reconciliation = new Promise<typeof serverPage>((resolve) => {
      resolveReconciliation = resolve;
    });
    apiMocks.courseReviews
      .mockResolvedValueOnce({ ...serverPage, items: [review] })
      .mockReturnValueOnce(reconciliation);
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "举报" }));
    await user.click(screen.getByRole("button", { name: "确认举报" }));

    await waitFor(() => expect(apiMocks.reportReview).toHaveBeenCalledWith(
      "review-1",
      "垃圾或推广信息：重复推广",
      "captcha-token",
    ));
    await waitFor(() => expect(screen.queryByRole("button", { name: "举报" }))
      .not.toBeInTheDocument());
    expect(apiMocks.courseReviews).toHaveBeenCalledTimes(2);
    resolveReconciliation(serverPage);
  });

  it("loads an exact review target instead of relying on the hot first page", async () => {
    const targetReview = { ...review, id: "review-target", comment: "只在精确查询中返回" };
    apiMocks.courseReviews.mockResolvedValue({ items: [], nextCursor: null, hasMore: false });
    apiMocks.review.mockResolvedValue(targetReview);

    renderPage("/courses/course-1?review=review-target#review-review-target");

    expect(await screen.findByText("只在精确查询中返回")).toBeVisible();
    expect(apiMocks.review).toHaveBeenCalledWith("review-target");
  });
});
