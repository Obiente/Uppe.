// Theme initialization script for server-side rendering

import { rosePineTheme } from './design-system';

//NOTE: we probably want to use a more dynamic theme system in the future
export function getThemeCSS(): string {
  const theme = rosePineTheme;
  
  return `
    :root {
      /* Background colors */
      --color-bg-primary: ${theme.colors.bg.primary};
      --color-bg-secondary: ${theme.colors.bg.secondary};
      --color-bg-tertiary: ${theme.colors.bg.tertiary};
      --color-bg-overlay: ${theme.colors.bg.overlay};
      --color-bg-elevated: ${theme.colors.bg.elevated};
      
      /* Text colors */
      --color-text-primary: ${theme.colors.text.primary};
      --color-text-secondary: ${theme.colors.text.secondary};
      --color-text-tertiary: ${theme.colors.text.tertiary};
      --color-text-inverse: ${theme.colors.text.inverse};
      
      /* Border colors */
      --color-border-primary: ${theme.colors.border.primary};
      --color-border-secondary: ${theme.colors.border.secondary};
      --color-border-focus: ${theme.colors.border.focus};
      
      /* Interactive colors */
      --color-interactive-primary: ${theme.colors.interactive.primary};
      --color-interactive-primary-hover: ${theme.colors.interactive.primaryHover};
      --color-interactive-secondary: ${theme.colors.interactive.secondary};
      --color-interactive-secondary-hover: ${theme.colors.interactive.secondaryHover};
      --color-interactive-danger: ${theme.colors.interactive.danger};
      --color-interactive-danger-hover: ${theme.colors.interactive.dangerHover};
      
      /* Status colors */
      --color-status-success: ${theme.colors.status.success};
      --color-status-warning: ${theme.colors.status.warning};
      --color-status-error: ${theme.colors.status.error};
      --color-status-info: ${theme.colors.status.info};
      --color-status-neutral: ${theme.colors.status.neutral};
      
      /* Accent colors */
      --color-accent-primary: ${theme.colors.accent.primary};
      --color-accent-secondary: ${theme.colors.accent.secondary};
      --color-accent-tertiary: ${theme.colors.accent.tertiary};
    }
  `;
}

// This file is strictly for server-side rendering of theme CSS
// For client-side theme functionality, see theme-client.ts