import { AlertCircle, LoaderCircle } from "lucide-react";
import * as React from "react";
import { Link } from "react-router";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { ActivityCalendar, ActivityDay } from "@/lib/api/types";
import { cn } from "@/lib/utils";

const DAY_IN_MILLISECONDS = 86_400_000;
const GRID_DAY_COUNT = 20 * 7;
const WEEKDAY_LABELS = ["一", "", "三", "", "五", "", "日"];
const intensityClasses = [
  "border-border/50 bg-muted",
  "border-primary/10 bg-primary/20",
  "border-primary/15 bg-primary/40",
  "border-primary/20 bg-primary/65",
  "border-primary/25 bg-primary",
] as const;

interface ActivityHeatmapProps {
  isAuthenticated: boolean;
  calendar?: ActivityCalendar;
  isLoading: boolean;
  error?: unknown;
  onRetry: () => void;
}

interface CalendarSlot {
  date: string;
  day?: ActivityDay;
  isFuture: boolean;
}

const displayDate = new Intl.DateTimeFormat("zh-CN", {
  timeZone: "UTC",
  month: "long",
  day: "numeric",
  weekday: "short",
});

function parseIsoDate(date: string) {
  return Date.parse(`${date}T00:00:00Z`);
}

function addDays(date: string, days: number) {
  return new Date(parseIsoDate(date) + days * DAY_IN_MILLISECONDS).toISOString().slice(0, 10);
}

function formatActivityDate(date: string) {
  return displayDate.format(new Date(parseIsoDate(date)));
}

function buildCalendarSlots(calendar: ActivityCalendar) {
  const daysByDate = new Map(calendar.days.map((day) => [day.date, day]));
  return Array.from({ length: GRID_DAY_COUNT }, (_, index): CalendarSlot => {
    const date = addDays(calendar.from, index);
    return {
      date,
      day: daysByDate.get(date),
      isFuture: date > calendar.to,
    };
  });
}

function buildIntensityMap(days: ActivityDay[]) {
  const sortedScores = days
    .map((day) => day.score)
    .filter((score) => score > 0)
    .sort((left, right) => left - right);
  const levels = new Map<number, number>();

  for (let index = 0; index < sortedScores.length; index += 1) {
    const score = sortedScores[index];
    const nextScore = sortedScores[index + 1];
    if (score !== nextScore) {
      levels.set(score, Math.max(1, Math.ceil(((index + 1) / sortedScores.length) * 4)));
    }
  }

  return levels;
}

function activityLabel(day: ActivityDay, intensity: number) {
  return `${formatActivityDate(day.date)}：活跃度 ${day.score} 分；发帖 ${day.threads}，评论 ${day.comments}，点赞 ${day.likes}，签到 ${day.checkIns}；强度 ${intensity} / 4`;
}

function HeatmapGrid({ calendar }: { calendar: ActivityCalendar }) {
  const slots = React.useMemo(() => buildCalendarSlots(calendar), [calendar]);
  const activityDays = React.useMemo(
    () => slots.flatMap((slot) => (slot.day ? [slot.day] : [])),
    [slots],
  );
  const intensityByScore = React.useMemo(() => buildIntensityMap(activityDays), [activityDays]);
  const interactiveIndices = React.useMemo(
    () => slots.flatMap((slot, index) => (slot.day ? [index] : [])),
    [slots],
  );
  const lastInteractiveIndex = interactiveIndices.at(-1) ?? 0;
  const [focusedIndex, setFocusedIndex] = React.useState(lastInteractiveIndex);
  const cellRefs = React.useRef(new Map<number, HTMLButtonElement>());

  React.useEffect(() => {
    if (!slots[focusedIndex]?.day) {
      setFocusedIndex(lastInteractiveIndex);
    }
  }, [focusedIndex, lastInteractiveIndex, slots]);

  function focusCell(index: number) {
    if (!slots[index]?.day) {
      return;
    }
    setFocusedIndex(index);
    cellRefs.current.get(index)?.focus();
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLButtonElement>, index: number) {
    const moves: Record<string, number> = {
      ArrowUp: -1,
      ArrowDown: 1,
      ArrowLeft: -7,
      ArrowRight: 7,
    };
    const move = moves[event.key];
    if (move !== undefined) {
      event.preventDefault();
      focusCell(index + move);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      focusCell(interactiveIndices[0] ?? 0);
    } else if (event.key === "End") {
      event.preventDefault();
      focusCell(lastInteractiveIndex);
    }
  }

  const totalScore = activityDays.reduce((sum, day) => sum + day.score, 0);
  const hasActivity = activityDays.some((day) => day.score > 0);

  return (
    <div>
      <div className="flex items-center justify-between gap-2 text-xs">
        <span className="font-medium">活跃度</span>
        <span className="text-[10px] text-muted-foreground">近 20 周 · {totalScore} 分</span>
      </div>

      <div className="mt-3 grid grid-cols-[12px_minmax(0,1fr)] gap-1">
        <div className="grid grid-rows-7 gap-0.5 text-[8px] leading-none text-muted-foreground" aria-hidden="true">
          {WEEKDAY_LABELS.map((label, index) => (
            <span key={index} className="flex items-center">{label}</span>
          ))}
        </div>
        <div
          className="grid min-w-0 grid-flow-col grid-cols-20 grid-rows-7 gap-0.5"
          role="grid"
          aria-label="近 20 周每日活跃度。进入图表后可使用方向键按日期浏览。"
          aria-rowcount={7}
          aria-colcount={20}
        >
          {slots.map((slot, index) => {
            if (!slot.day) {
              return (
                <span
                  key={slot.date}
                  className={cn(
                    "aspect-square rounded-[2px]",
                    !slot.isFuture && "border border-dashed border-border/60",
                  )}
                  aria-hidden="true"
                />
              );
            }

            const intensity = slot.day.score === 0 ? 0 : (intensityByScore.get(slot.day.score) ?? 1);
            const label = activityLabel(slot.day, intensity);
            return (
              <Tooltip key={slot.date} delayDuration={120}>
                <TooltipTrigger asChild>
                  <button
                    ref={(element) => {
                      if (element) {
                        cellRefs.current.set(index, element);
                      } else {
                        cellRefs.current.delete(index);
                      }
                    }}
                    type="button"
                    role="gridcell"
                    aria-colindex={Math.floor(index / 7) + 1}
                    aria-rowindex={(index % 7) + 1}
                    aria-label={label}
                    tabIndex={focusedIndex === index ? 0 : -1}
                    onFocus={() => setFocusedIndex(index)}
                    onKeyDown={(event) => handleKeyDown(event, index)}
                    className={cn(
                      "aspect-square min-w-0 rounded-[2px] border transition-transform hover:scale-110 focus-visible:z-10 focus-visible:scale-125 focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-card",
                      intensityClasses[intensity],
                    )}
                  />
                </TooltipTrigger>
                <TooltipContent side="top" className="space-y-0.5 text-center">
                  <p className="font-medium">{formatActivityDate(slot.day.date)} · {slot.day.score} 分</p>
                  <p>发帖 {slot.day.threads} · 评论 {slot.day.comments} · 点赞 {slot.day.likes} · 签到 {slot.day.checkIns}</p>
                </TooltipContent>
              </Tooltip>
            );
          })}
        </div>
      </div>

      <div className="mt-2 flex items-center justify-between gap-2 text-[9px] text-muted-foreground">
        <span className="truncate">
          发帖 ×{calendar.weights.thread} · 评论 ×{calendar.weights.comment} · 点赞 ×{calendar.weights.like}（每日最多 {calendar.likeDailyCap} 分）· 签到 ×{calendar.weights.checkIn}
        </span>
        <span className="flex shrink-0 items-center gap-1" aria-label="活跃度强度从少到多">
          <span>少</span>
          {intensityClasses.map((className, index) => (
            <span key={index} className={cn("size-2.5 rounded-[1px] border", className)} aria-hidden="true" />
          ))}
          <span>多</span>
        </span>
      </div>
      {!hasActivity ? <p className="mt-2 text-[10px] text-muted-foreground">近 20 周还没有公开活跃记录。</p> : null}
    </div>
  );
}

export function ActivityHeatmap({
  isAuthenticated,
  calendar,
  isLoading,
  error,
  onRetry,
}: ActivityHeatmapProps) {
  if (!isAuthenticated) {
    return (
      <div>
        <p className="text-xs font-medium">活跃度</p>
        <div className="flex min-h-24 flex-col items-center justify-center gap-2 text-center">
          <p className="max-w-56 text-[10px] leading-4 text-muted-foreground">
            登录后查看近 20 周每天的发帖、评论、点赞和签到活跃度。
          </p>
          <Button asChild variant="link" size="sm" className="h-auto p-0 text-xs">
            <Link to="/login">登录查看</Link>
          </Button>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div>
        <p className="text-xs font-medium">活跃度</p>
        <div className="flex min-h-24 items-center justify-center gap-2 text-[10px] text-muted-foreground">
          <LoaderCircle className="size-3.5 animate-spin" aria-hidden="true" />
          <span>正在加载每日活跃度</span>
        </div>
      </div>
    );
  }

  if (error) {
    const message = error instanceof Error ? error.message : "请稍后重试";
    return (
      <div>
        <p className="text-xs font-medium">活跃度</p>
        <div className="flex min-h-24 flex-col items-center justify-center gap-2 text-center">
          <AlertCircle className="size-4 text-destructive" aria-hidden="true" />
          <p className="line-clamp-2 max-w-56 text-[10px] leading-4 text-muted-foreground">活跃度加载失败：{message}</p>
          <Button type="button" variant="outline" size="sm" className="h-7" onClick={onRetry}>重试</Button>
        </div>
      </div>
    );
  }

  if (!calendar || calendar.days.length === 0) {
    return (
      <div>
        <p className="text-xs font-medium">活跃度</p>
        <div className="flex min-h-24 items-center justify-center text-center">
          <p className="max-w-56 text-[10px] leading-4 text-muted-foreground">暂时没有可展示的每日活跃度数据。</p>
        </div>
      </div>
    );
  }

  return <HeatmapGrid calendar={calendar} />;
}
