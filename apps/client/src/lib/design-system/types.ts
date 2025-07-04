import type { typography, borderRadius, spacing } from "./tokens";

// Common component types
export type ComponentSize = 'sm' | 'md' | 'lg';
export type TextAlign = 'left' | 'center' | 'right';

// Component-specific variants
export type BadgeVariant = 'primary' | 'secondary' | 'success' | 'warning' | 'error' | 'info' | 'neutral';
export type ButtonVariant = 'primary' | 'secondary' | 'danger';
export type AlertVariant = 'info' | 'success' | 'warning' | 'error';
export type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;
export type StatusType = 'online' | 'offline' | 'warning' | 'error' | 'unknown';
export type ProgressVariant = 'primary' | 'success' | 'warning' | 'error';

export type TypographySize = keyof typeof typography.sizes;
export type TypographyWeight = keyof typeof typography.weights;
export type TypographyLineHeight = keyof typeof typography.lineHeight;
export type BorderRadius = keyof typeof borderRadius;
export type Spacing = keyof typeof spacing;