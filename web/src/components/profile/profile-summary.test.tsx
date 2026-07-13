import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ProfileSummary } from "./profile-summary";

describe("ProfileSummary verification semantics", () => {
  it("renders role and public verification as separate signals", async () => {
    const view = render(
      <MemoryRouter>
        <ProfileSummary
          profile={{
            id: "42",
            handle: "campus-club",
            displayName: "校园组织",
            bio: null,
            website: null,
            avatarUrl: null,
            bannerUrl: null,
            role: "user",
            trustLevel: 2,
            badges: [{ slug: "first-thread", name: "首次发帖" }],
            verifications: [{
              slug: "official-organization",
              category: "identity",
              label: "官方学生组织",
              description: "已核实的学生组织账号",
              icon: "building-2",
              badgeVariant: "default",
              issuedAt: 1_700_000_000,
              expiresAt: null,
            }],
            threadCount: 3,
            commentCount: 5,
            votesReceived: 8,
            followerCount: 2,
            followingCount: 1,
            canViewActivity: true,
            createdAt: 1_700_000_000,
          }}
          relationship={undefined}
          isAuthenticated={false}
          isSelf={false}
          relationshipLoading={false}
          relationshipPending={false}
          messagePending={false}
          canStartConversation={false}
          canManageUser={false}
          canManageVerifications={false}
          confirmBlockOpen={false}
          onConfirmBlockOpenChange={() => undefined}
          onStartConversation={() => undefined}
          onToggleFollow={() => undefined}
          onToggleMute={() => undefined}
          onToggleBlock={() => undefined}
          onOpenRelationshipList={() => undefined}
          onMediaDeliveryRefresh={() => undefined}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole("heading", { name: "校园组织" })).toBeInTheDocument();
    // Ordinary members do not show a redundant role pill (Figma profile card).
    expect(screen.queryByText("社区成员")).not.toBeInTheDocument();
    expect(screen.getByLabelText("官方学生组织，身份认证")).toBeInTheDocument();
    // Achievement badges live in the profile sidebar, not the summary identity row.
    expect(screen.queryByText("首次发帖")).not.toBeInTheDocument();
    await expectNoAccessibilityViolations(view.container);
  });
});
