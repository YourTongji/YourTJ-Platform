const DAY_IN_MILLISECONDS = 86_400_000;
const WEEKS_TO_SHOW = 20;

function formatIsoDate(timestamp: number) {
  return new Date(timestamp).toISOString().slice(0, 10);
}

/** Returns twenty calendar columns ending in the current Asia/Shanghai week. */
export function getTwentyWeekActivityRange(now = new Date()) {
  const dateParts = new Intl.DateTimeFormat("en-US", {
    timeZone: "Asia/Shanghai",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(now);
  const values = Object.fromEntries(dateParts.map((part) => [part.type, part.value]));
  const today = Date.UTC(Number(values.year), Number(values.month) - 1, Number(values.day));
  const daysSinceMonday = (new Date(today).getUTCDay() + 6) % 7;
  const firstMonday = today - (daysSinceMonday + (WEEKS_TO_SHOW - 1) * 7) * DAY_IN_MILLISECONDS;

  return {
    from: formatIsoDate(firstMonday),
    to: formatIsoDate(today),
  };
}
