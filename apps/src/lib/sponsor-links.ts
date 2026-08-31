export interface SponsorLinkItem {
  key: string;
  name: string;
  description: string;
  href: string;
  actionLabel: string;
  imageSrc?: string;
  imageAlt?: string;
}

export const DEFAULT_AUTHOR_SPONSORS: SponsorLinkItem[] = [];
export const DEFAULT_AUTHOR_SERVER_RECOMMENDATIONS: SponsorLinkItem[] = [];

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function asTrimmedString(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeOptionalText(value: unknown): string | undefined {
  const normalized = asTrimmedString(value);
  return normalized || undefined;
}

export function cloneSponsorLinkItems(
  items: readonly SponsorLinkItem[],
): SponsorLinkItem[] {
  return items.map((item) => ({ ...item }));
}

export function normalizeSponsorLinkItems(
  value: unknown,
  fallback: readonly SponsorLinkItem[] = [],
): SponsorLinkItem[] {
  if (!Array.isArray(value)) {
    return cloneSponsorLinkItems(fallback);
  }

  return value.map((item, index) => {
    const source = asRecord(item) ?? {};
    return {
      key: asTrimmedString(source.key) || `item-${index + 1}`,
      name: asTrimmedString(source.name),
      description: asTrimmedString(source.description),
      href: asTrimmedString(source.href),
      actionLabel: asTrimmedString(source.actionLabel),
      imageSrc: normalizeOptionalText(source.imageSrc),
      imageAlt: normalizeOptionalText(source.imageAlt),
    } satisfies SponsorLinkItem;
  });
}

export function createEmptySponsorLinkItem(prefix: string): SponsorLinkItem {
  return {
    key: `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name: "",
    description: "",
    href: "",
    actionLabel: "",
    imageSrc: "",
    imageAlt: "",
  };
}
