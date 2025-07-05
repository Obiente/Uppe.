/**
 * Centralized icon definitions for the PeerUP application
 *
 * This file contains all the icons used throughout the application,
 * making it easy to switch icon libraries or update icon names consistently.
 */
export type IconPack = "heroicons";
export type Icon = `${IconPack}:${string}`;
export const ICONS = {
  // Navigation & Actions
  NAVIGATE_FORWARD: "heroicons:arrow-right",
  REFRESH: "heroicons:arrow-path",
  EXPAND: "heroicons:chevron-down",
  COLLAPSE: "heroicons:chevron-right",
  ADD: "heroicons:plus",
  CONFIRM: "heroicons:check",
  CLOSE: "heroicons:x-mark",
  STOP: "heroicons:pause",

  // Status & Indicators
  SUCCESS: "heroicons:check-circle",
  WARNING: "heroicons:exclamation-triangle",
  HELP: "heroicons:question-mark-circle",
  INFO: "heroicons:information-circle",
  PENDING: "heroicons:clock",

  // Charts & Analytics
  ANALYTICS: "heroicons:chart-bar",
  DASHBOARD: "heroicons:chart-bar-square",
  TRENDING_UP: "heroicons:arrow-trending-up",
  TRENDING_DOWN: "heroicons:arrow-trending-down",
  DECREASE: "heroicons:minus",

  // Network & Connectivity
  NETWORK: "heroicons:wifi",
  CONNECTION_STRENGTH: "heroicons:signal",
  GLOBAL: "heroicons:globe-americas",

  // Documents & Content
  DOCUMENT: "heroicons:document-text",
  COPY: "heroicons:clipboard-document",

  // System & Performance
  PERFORMANCE: "heroicons:bolt",
  EXTERNAL_LINK: "heroicons:arrow-top-right-on-square",
} as const;

export function getIcon(iconKey: keyof typeof ICONS): Icon {
  return ICONS[iconKey];
}
export const ICON_CATEGORIES: Record<
  Uppercase<string>,
  Record<Uppercase<string>, Icon>
> = {
  NAVIGATION: {
    NAVIGATE_FORWARD: ICONS.NAVIGATE_FORWARD,
    REFRESH: ICONS.REFRESH,
    EXPAND: ICONS.EXPAND,
    COLLAPSE: ICONS.COLLAPSE,
  },
  ACTIONS: {
    ADD: ICONS.ADD,
    CONFIRM: ICONS.CONFIRM,
    CLOSE: ICONS.CLOSE,
    STOP: ICONS.STOP,
  },
  STATUS: {
    SUCCESS: ICONS.SUCCESS,
    WARNING: ICONS.WARNING,
    HELP: ICONS.HELP,
    INFO: ICONS.INFO,
    PENDING: ICONS.PENDING,
  },
  CHARTS: {
    ANALYTICS: ICONS.ANALYTICS,
    DASHBOARD: ICONS.DASHBOARD,
    TRENDING_UP: ICONS.TRENDING_UP,
    TRENDING_DOWN: ICONS.TRENDING_DOWN,
    DECREASE: ICONS.DECREASE,
  },
  NETWORK: {
    NETWORK: ICONS.NETWORK,
    CONNECTION_STRENGTH: ICONS.CONNECTION_STRENGTH,
    GLOBAL: ICONS.GLOBAL,
  },
  DOCUMENTS: {
    DOCUMENT: ICONS.DOCUMENT,
    COPY: ICONS.COPY,
  },
  SYSTEM: {
    PERFORMANCE: ICONS.PERFORMANCE,
    EXTERNAL_LINK: ICONS.EXTERNAL_LINK,
  },
} as const;
