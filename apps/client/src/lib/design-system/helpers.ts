import { components } from './components';

type ComponentKey = keyof typeof components;

interface ComponentStyles {
  base?: string;
  variants?: Record<string, string>;
  sizes?: Record<string | number, string>;
  labelInline?: string;
  [key: string]: any;
}

/**
 * Get component classes from the design system
 * 
 * @param component - The component name (e.g., 'badge', 'button')
 * @param options - Component options (variant, size, etc)
 * @param className - Additional class names
 * @returns Combined class names
 */
export function getClasses(
  component: ComponentKey,
  options?: {
    variant?: string;
    size?: string | number;
    inline?: boolean;
    [key: string]: any;
  },
  className: string = ''
): string {
  const componentStyles = components[component] as ComponentStyles;
  
  if (!componentStyles) {
    console.warn(`Component "${component}" not found in design system`);
    return className;
  }
  
  let classes = componentStyles.base || '';
  
  // Add variant-specific classes
  if (options?.variant && componentStyles.variants && componentStyles.variants[options.variant]) {
    classes += ` ${componentStyles.variants[options.variant]}`;
  }
  
  // Add size-specific classes
  if (options?.size !== undefined && componentStyles.sizes && componentStyles.sizes[options.size]) {
    classes += ` ${componentStyles.sizes[options.size]}`;
  }
  
  // Special case for inline form labels
  if (component === 'form' && options?.inline && componentStyles.labelInline) {
    classes = componentStyles.labelInline;
  }
  
  // Add custom class names
  if (className) {
    classes += ` ${className}`;
  }
  
  return classes;
}
