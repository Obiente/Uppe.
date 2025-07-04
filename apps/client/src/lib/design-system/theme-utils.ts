import type { Theme } from './themes';
import { spacing, typography, borderRadius } from './tokens';
import { rosePineTheme } from './themes';
//TODO: some cookie ssr magic perchance?
let activeTheme: Theme = rosePineTheme;

export function setTheme(theme: Theme): void {
  activeTheme = theme;
  applyThemeToDOM(theme);
}

export function getCurrentTheme(): Theme {
  return activeTheme;
}

export function applyThemeToDOM(theme: Theme): void {
  if (typeof document === 'undefined') return; // Skip during SSR

  const root = document.documentElement;
  
  // Background colors
  root.style.setProperty('--color-bg-primary', theme.colors.bg.primary);
  root.style.setProperty('--color-bg-secondary', theme.colors.bg.secondary);
  root.style.setProperty('--color-bg-tertiary', theme.colors.bg.tertiary);
  root.style.setProperty('--color-bg-overlay', theme.colors.bg.overlay);
  root.style.setProperty('--color-bg-elevated', theme.colors.bg.elevated);
  
  // Text colors
  root.style.setProperty('--color-text-primary', theme.colors.text.primary);
  root.style.setProperty('--color-text-secondary', theme.colors.text.secondary);
  root.style.setProperty('--color-text-tertiary', theme.colors.text.tertiary);
  root.style.setProperty('--color-text-inverse', theme.colors.text.inverse);
  
  // Border colors
  root.style.setProperty('--color-border-primary', theme.colors.border.primary);
  root.style.setProperty('--color-border-secondary', theme.colors.border.secondary);
  root.style.setProperty('--color-border-focus', theme.colors.border.focus);
  
  // Interactive colors
  root.style.setProperty('--color-interactive-primary', theme.colors.interactive.primary);
  root.style.setProperty('--color-interactive-primary-hover', theme.colors.interactive.primaryHover);
  root.style.setProperty('--color-interactive-secondary', theme.colors.interactive.secondary);
  root.style.setProperty('--color-interactive-secondary-hover', theme.colors.interactive.secondaryHover);
  root.style.setProperty('--color-interactive-danger', theme.colors.interactive.danger);
  root.style.setProperty('--color-interactive-danger-hover', theme.colors.interactive.dangerHover);
  
  // Status colors
  root.style.setProperty('--color-status-success', theme.colors.status.success);
  root.style.setProperty('--color-status-warning', theme.colors.status.warning);
  root.style.setProperty('--color-status-error', theme.colors.status.error);
  root.style.setProperty('--color-status-info', theme.colors.status.info);
  root.style.setProperty('--color-status-neutral', theme.colors.status.neutral);
  
  // Accent colors
  root.style.setProperty('--color-accent-primary', theme.colors.accent.primary);
  root.style.setProperty('--color-accent-secondary', theme.colors.accent.secondary);
  root.style.setProperty('--color-accent-tertiary', theme.colors.accent.tertiary);

  // Apply spacing
  Object.entries(spacing).forEach(([key, value]) => {
    root.style.setProperty(`--spacing-${key}`, value);
  });
  
  // Apply typography sizes
  Object.entries(typography.sizes).forEach(([key, value]) => {
    root.style.setProperty(`--font-size-${key}`, value);
  });
  
  // Apply border radius
  Object.entries(borderRadius).forEach(([key, value]) => {
    root.style.setProperty(`--border-radius-${key}`, value);
  });
}
