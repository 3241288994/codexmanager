export const DEFAULT_APPEARANCE_PRESET = "classic";
export const APPEARANCE_PRESET_STORAGE_KEY =
  "codexmanager.ui.appearance-preset";

export const APPEARANCE_PRESETS = [
  {
    id: "classic",
    name: "默认",
    description: "使用更轻的玻璃效果和更简洁的背景表现。",
  },
  {
    id: "modern",
    name: "渐变版本",
    description: "使用更明显的渐层背景、增强玻璃质感和更强层次感。",
  },
] as const;

/**
 * 函数 `normalizeAppearancePreset`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * - value: 参数 value
 *
 * # 返回
 * 返回函数执行结果
 */
export function normalizeAppearancePreset(
  value: string | null | undefined,
): string {
  return value === "modern" ? "modern" : DEFAULT_APPEARANCE_PRESET;
}

/**
 * 函数 `applyAppearancePreset`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * - value: 参数 value
 *
 * # 返回
 * 返回函数执行结果
 */
export function applyAppearancePreset(
  value: string | null | undefined,
): string {
  const preset = normalizeAppearancePreset(value);

  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("data-appearance", preset);
  }

  if (typeof window !== "undefined") {
    try {
      window.localStorage.setItem(APPEARANCE_PRESET_STORAGE_KEY, preset);
    } catch {}
  }

  return preset;
}

export const appearanceInitScript = `(() => {
  try {
    var raw = window.localStorage.getItem(${JSON.stringify(APPEARANCE_PRESET_STORAGE_KEY)});
    var preset = raw === "modern" ? "modern" : ${JSON.stringify(DEFAULT_APPEARANCE_PRESET)};
    document.documentElement.setAttribute("data-appearance", preset);
  } catch (_error) {
    document.documentElement.setAttribute("data-appearance", ${JSON.stringify(DEFAULT_APPEARANCE_PRESET)});
  }
})();`;
