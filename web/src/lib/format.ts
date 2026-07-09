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
