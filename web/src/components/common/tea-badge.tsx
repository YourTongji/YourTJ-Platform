const teaLevels = [
  { level: 0, name: "茶苗", color: "#6baf6b", bg: "#f0f9f0" },
  { level: 1, name: "绿茶", color: "#3d9970", bg: "#e6f5ee" },
  { level: 2, name: "白茶", color: "#7d919b", bg: "#eef2f5" },
  { level: 3, name: "黄茶", color: "#9c7a10", bg: "#fdf4dc" },
  { level: 4, name: "青茶", color: "#007b6c", bg: "#e2f0ed" },
  { level: 5, name: "红茶", color: "#a3432f", bg: "#faeae3" },
  { level: 6, name: "黑茶", color: "#3b2f2f", bg: "#f0ebe8" },
];

export function TeaBadge({ level = 0 }: { level?: number | null }) {
  const item = teaLevels[Math.max(0, Math.min(level ?? 0, teaLevels.length - 1))] ?? teaLevels[0];
  return (
    <span
      className="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium"
      style={{ backgroundColor: item.bg, color: item.color }}
    >
      Lv.{item.level} {item.name}
    </span>
  );
}
