// Client-side theme functionality
// This file is intentionally separate from theme-init.ts to avoid SSR hydration issues
import { rosePineTheme } from './design-system/themes';
import { setTheme as applyTheme } from './design-system/theme-utils';

let isInitialized = false;

// Initialize theme on the client side safely
// This function will apply any client-side theme logic after hydration
export function initTheme(): void {
  // Skip if already initialized or not in browser environment
  if (isInitialized || typeof window === 'undefined') return;
  
  try {
    // Apply theme from preferences if available, otherwise use default
    const savedTheme = localStorage.getItem('peerup-theme');
    
    // Currently only rosePineTheme is fully implemented
    // In the future, this will be extended to support multiple themes
    
    // Mark as initialized to prevent duplicate initialization
    isInitialized = true;
  } catch (error) {
    // Fail silently in case of localStorage errors
    console.error('Theme initialization error:', error);
  }
}

// Function to change theme (placeholder for future implementation)
export function setTheme(themeName: string): void {
  if (typeof window === 'undefined') return;
  
  try {
    // For now just log the theme change request
    console.log(`Theme switching to ${themeName} (not yet implemented)`);
    
    // Store preference for future visits
    localStorage.setItem('peerup-theme', themeName);
    
    // Future implementation will load the appropriate theme
    // and update DOM variables accordingly
  } catch (error) {
    console.error('Theme switching error:', error);
  }
}
