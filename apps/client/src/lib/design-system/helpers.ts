import { components } from './components';

type ComponentKey = keyof typeof components;

// Extract the structure of each component for better type inference
type ComponentConfig = typeof components;

// Get the variants for a specific component
type GetVariants<T extends ComponentKey> = ComponentConfig[T] extends { variants: infer V }
  ? V extends Record<string, string>
    ? keyof V
    : never
  : never;

// Get the sizes for a specific component
type GetSizes<T extends ComponentKey> = ComponentConfig[T] extends { sizes: infer S }
  ? S extends Record<string | number, string>
    ? keyof S
    : never
  : never;

// Get all possible option keys for a component
type GetOptionKeys<T extends ComponentKey> = ComponentConfig[T] extends Record<string, any>
  ? Exclude<keyof ComponentConfig[T], 'base' | 'variants' | 'sizes'>
  : never;

// Component-specific options type
type ComponentOptions<T extends ComponentKey> = {
  variant?: GetVariants<T>;
  size?: GetSizes<T>;
  inline?: boolean;
} & {
  [K in GetOptionKeys<T>]?: boolean;
};

interface ComponentStyles {
  base?: string;
  variants?: Record<string, string>;
  sizes?: Record<string | number, string>;
  labelInline?: string;
  [key: string]: any;
}

/**
 * Get component classes from the design system with full type inference
 * 
 * @param component - The component name (e.g., 'badge', 'button')
 * @param options - Component options (variant, size, etc) - fully typed based on component
 * @param className - Additional class names
 * @returns Combined class names
 */
export function getClasses<T extends ComponentKey>(
  component: T,
  options?: ComponentOptions<T>,
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
