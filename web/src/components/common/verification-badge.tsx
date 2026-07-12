import { BadgeCheck, Building2, ShieldCheck, Sparkles } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import type { PublicVerification, VerificationType } from "@/lib/api/types";

type VerificationDisplay = Pick<
  PublicVerification | VerificationType,
  "slug" | "category" | "label" | "description" | "icon" | "badgeVariant"
>;

const icons = {
  "badge-check": BadgeCheck,
  "building-2": Building2,
  "shield-check": ShieldCheck,
  sparkles: Sparkles,
} as const;

export function VerificationBadge({ verification }: { verification: VerificationDisplay }) {
  const Icon = icons[verification.icon];
  const category = verification.category === "identity" ? "身份认证" : "特殊认证";
  const description = verification.description
    ? `${category}：${verification.description}`
    : category;

  return (
    <Badge variant={verification.badgeVariant} title={description} aria-label={`${verification.label}，${category}`}>
      <Icon className="size-3.5" aria-hidden="true" />
      {verification.label}
    </Badge>
  );
}
