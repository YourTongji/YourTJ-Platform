export function formatUnixTime(value?: number | string | null) {
  if (value === undefined || value === null || value === "") {
    return "未知";
  }
  const date =
    typeof value === "number" ? new Date(value * 1000) : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "未知";
  }
  return new Intl.DateTimeFormat("zh-CN", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatRelativeTime(value?: number | string | null) {
  if (value === undefined || value === null || value === "") {
    return "刚刚";
  }

  const date = typeof value === "number" ? new Date(value * 1000) : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "刚刚";
  }

  const seconds = Math.round((date.getTime() - Date.now()) / 1000);
  const formatter = new Intl.RelativeTimeFormat("zh-CN", { numeric: "auto" });
  const ranges: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ["year", 60 * 60 * 24 * 365],
    ["month", 60 * 60 * 24 * 30],
    ["day", 60 * 60 * 24],
    ["hour", 60 * 60],
    ["minute", 60],
  ];

  for (const [unit, size] of ranges) {
    if (Math.abs(seconds) >= size) {
      return formatter.format(Math.round(seconds / size), unit);
    }
  }

  return "刚刚";
}

export function formatDate(value?: number | string | null) {
  if (value === undefined || value === null || value === "") {
    return "未知";
  }
  const date =
    typeof value === "number" ? new Date(value * 1000) : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "未知";
  }
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(date);
}

export function formatNumber(value?: number | null) {
  return new Intl.NumberFormat("zh-CN").format(value ?? 0);
}

export function formatRating(value?: number | null) {
  if (value === undefined || value === null || Number.isNaN(value)) {
    return "暂无";
  }
  return value.toFixed(1);
}

export function shortHash(value?: string | null, size = 8) {
  if (!value) {
    return "无";
  }
  if (value.length <= size * 2) {
    return value;
  }
  return `${value.slice(0, size)}…${value.slice(-size)}`;
}

export function idempotencyKey(prefix: string) {
  return `${prefix}:${crypto.randomUUID()}`;
}
