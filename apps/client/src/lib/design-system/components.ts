// Component-specific design tokens

// Component styles definition
export const components = {
  // LinkButton component - for text-only buttons that look like links
  linkButton: {
    base: 'inline-flex items-center transition-colors focus:outline-none',
    variants: {
      primary: 'text-accent-primary hover:text-primary',
      secondary: 'text-secondary hover:text-primary',
      danger: 'text-status-error hover:text-status-error/80'
    },
    sizes: {
      sm: 'text-sm',
      md: 'text-base',
      lg: 'text-lg'
    }
  },
  
  // Badge component
  badge: {
    base: 'inline-flex items-center font-medium rounded-full',
    variants: {
      primary: 'bg-accent-primary text-inverse',
      secondary: 'bg-surface-secondary text-primary border border-border-subtle',
      success: 'bg-status-online text-inverse',
      warning: 'bg-status-warning text-inverse',
      error: 'bg-status-error text-inverse',
      info: 'bg-status-info text-inverse',
      neutral: 'bg-surface-elevated text-secondary'
    },
    sizes: {
      sm: 'px-2 py-0.5 text-xs',
      md: 'px-2.5 py-1 text-sm',
      lg: 'px-3 py-1.5 text-base'
    }
  },
  
  // Button component
  button: {
    base: 'inline-flex items-center justify-center font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed',
    variants: {
      primary: 'bg-accent-primary text-inverse hover:bg-accent-primary-hover focus:ring-accent-primary',
      secondary: 'bg-surface-secondary border border-border-subtle text-primary hover:bg-surface-elevated focus:ring-border-subtle',
      danger: 'bg-status-error text-inverse hover:opacity-90 focus:ring-status-error'
    },
    sizes: {
      sm: 'px-2.5 py-1.5 text-xs',
      md: 'px-4 py-2 text-sm',
      lg: 'px-6 py-3 text-base'
    }
  },
  
  // Card component
  card: {
    base: 'p-6 bg-surface-secondary border border-border-primary rounded-lg overflow-hidden',
    header: 'px-6 py-4 border-b border-border-primary',
    title: 'text-xl font-semibold text-primary',
    body: 'px-6 py-4'
  },
  
  // Form components
  form: {
    label: 'block text-sm font-medium text-primary mb-2',
    labelInline: 'text-sm text-primary',
    input: 'w-full px-3 py-2 bg-surface-secondary border border-border-subtle rounded-lg text-primary placeholder-text-tertiary focus:outline-none focus:ring-2 focus:ring-border-focus focus:border-transparent transition-colors',
    select: 'w-full px-3 py-2 bg-surface-secondary border border-border-subtle rounded-lg text-primary focus:outline-none focus:ring-2 focus:ring-border-focus focus:border-transparent transition-colors',
    textarea: 'w-full px-3 py-2 bg-surface-secondary border border-border-subtle rounded-lg text-primary placeholder-text-tertiary focus:outline-none focus:ring-2 focus:ring-border-focus focus:border-transparent transition-colors resize-vertical',
    checkbox: 'w-4 h-4 text-border-focus bg-surface-secondary border-border-subtle rounded focus:ring-border-focus focus:ring-2'
  },
  
  // Alert component
  alert: {
    base: 'p-4 rounded-lg border',
    variants: {
      info: 'bg-surface-secondary border-status-info text-primary',
      success: 'bg-surface-secondary border-status-online text-primary',
      warning: 'bg-surface-secondary border-status-warning text-primary',
      error: 'bg-surface-secondary border-status-error text-primary'
    }
  },
  
  // Heading component
  heading: {
    base: 'font-semibold text-primary',
    sizes: {
      1: 'text-4xl',
      2: 'text-3xl',
      3: 'text-2xl',
      4: 'text-xl',
      5: 'text-lg',
      6: 'text-base'
    }
  },
  
  // StatusBadge component
  statusBadge: {
    base: 'inline-flex items-center gap-1.5',
    variants: {
      online: 'text-status-online',
      offline: 'text-status-error',
      warning: 'text-status-warning',
      error: 'text-status-error',
      unknown: 'text-tertiary'
    }
  },
  
  // ProgressBar component
  progressBar: {
    base: 'w-full bg-surface-elevated rounded-full overflow-hidden',
    bar: 'h-2 transition-all duration-300 ease-in-out',
    variants: {
      primary: 'bg-accent-primary',
      success: 'bg-status-online',
      warning: 'bg-status-warning',
      error: 'bg-status-error'
    }
  },
  
  // CircularProgress component
  circularProgress: {
    base: 'rounded-full',
    variants: {
      primary: 'text-accent-primary',
      success: 'text-status-online',
      warning: 'text-status-warning',
      error: 'text-status-error'
    },
    sizes: {
      sm: 'w-4 h-4',
      md: 'w-8 h-8',
      lg: 'w-12 h-12'
    }
  },
  
  // Table component
  table: {
    base: 'w-full border-collapse',
    header: 'bg-surface-elevated',
    headerCell: 'px-4 py-3 text-left text-sm font-medium text-secondary',
    row: 'border-b border-border-subtle',
    cell: 'px-4 py-3 text-sm text-primary',
    footer: 'bg-surface-elevated'
  }
};
