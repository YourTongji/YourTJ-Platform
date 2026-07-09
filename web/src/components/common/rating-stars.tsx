import { Star } from "lucide-react";

import { cn } from "@/lib/utils";

export function RatingStars({
  value,
  onChange,
  size = "sm",
}: {
  value?: number | null;
  onChange?: (value: number) => void;
  size?: "sm" | "md";
}) {
  const rounded = Math.round(value ?? 0);
  const iconSize = size === "md" ? "h-5 w-5" : "h-4 w-4";
  return (
    <div className="inline-flex items-center gap-0.5">
      {[1, 2, 3, 4, 5].map((star) => (
        <button
          key={star}
          type="button"
          disabled={!onChange}
          onClick={() => onChange?.(star)}
          className={cn("rounded-sm text-muted-foreground disabled:cursor-default", onChange && "hover:text-primary")}
          aria-label={`${star} 星`}
        >
          <Star
            className={cn(
              iconSize,
              star <= rounded && "fill-[var(--chart-2)] text-[var(--chart-2)]",
            )}
          />
        </button>
      ))}
    </div>
  );
}
