export interface Theme {
  colors: {
    bg: {
      primary: string;
      secondary: string;
      tertiary: string;
      overlay: string;
      elevated: string;
    };

    text: {
      primary: string;
      secondary: string;
      tertiary: string;
      inverse: string;
    };

    border: {
      primary: string;
      secondary: string;
      focus: string;
    };

    interactive: {
      primary: string;
      primaryHover: string;
      secondary: string;
      secondaryHover: string;
      danger: string;
      dangerHover: string;
    };

    status: {
      success: string;
      warning: string;
      error: string;
      info: string;
      neutral: string;
    };

    accent: {
      primary: string;
      secondary: string;
      tertiary: string;
    };
  };
}

// Rose Pine Theme (default)
export const rosePineTheme: Theme = {
  colors: {
    bg: {
      primary: "#191724", // rose-pine-base
      secondary: "#1f1d2e", // rose-pine-surface
      tertiary: "#26233a", // rose-pine-overlay
      overlay: "#6e6a86", // rose-pine-muted
      elevated: "#393552", // rose-pine-highlight-med
    },
    text: {
      primary: "#e0def4", // rose-pine-text
      secondary: "#908caa", // rose-pine-subtle
      tertiary: "#6e6a86", // rose-pine-muted
      inverse: "#191724", // rose-pine-base
    },
    border: {
      primary: "#393552", // rose-pine-highlight-med
      secondary: "#26233a", // rose-pine-overlay
      focus: "#c4a7e7", // rose-pine-iris
    },
    interactive: {
      primary: "#eb6f92", // rose-pine-rose
      primaryHover: "#f6c177", // rose-pine-gold
      secondary: "#393552", // rose-pine-highlight-med
      secondaryHover: "#524f67", // rose-pine-highlight-high
      danger: "#eb6f92", // rose-pine-love
      dangerHover: "#f6c177", // rose-pine-gold
    },
    status: {
      success: "#9ccfd8", // rose-pine-foam
      warning: "#f6c177", // rose-pine-gold
      error: "#eb6f92", // rose-pine-love
      info: "#c4a7e7", // rose-pine-iris
      neutral: "#908caa", // rose-pine-subtle
    },
    accent: {
      primary: "#c4a7e7", // rose-pine-iris
      secondary: "#9ccfd8", // rose-pine-foam
      tertiary: "#31748f", // rose-pine-pine
    },
  },
};
