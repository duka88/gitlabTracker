// Label helpers (pure).

/** Splits on the LAST "::", as GitLab does: `a::b::c` → { scope: "a::b", value: "c" }. */
export function splitScopedLabel(title: string): { scope: string | null; value: string } {
  const i = title.lastIndexOf("::");
  if (i <= 0 || i + 2 >= title.length) return { scope: null, value: title };
  return { scope: title.slice(0, i), value: title.slice(i + 2) };
}

function parseHex(color: string): [number, number, number] | null {
  let h = color.trim().replace(/^#/, "");
  if (h.length === 3) h = h.split("").map((c) => c + c).join("");
  if (!/^[0-9a-fA-F]{6}$/.test(h)) return null;
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];
}

function luminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** GitLab's text colour when given; otherwise dark or white text, whichever contrasts more. */
export function labelTextColor(bgColor: string, textColor?: string | null): string {
  if (textColor && parseHex(textColor)) return textColor;
  const rgb = parseHex(bgColor);
  if (!rgb) return "#1f1e24";
  return luminance(rgb) > 0.4 ? "#1f1e24" : "#ffffff";
}
