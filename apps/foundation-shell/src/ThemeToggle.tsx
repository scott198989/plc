export type AppTheme = "dark" | "light";

const themeStorageKey = "plc-engineering-simulator.theme";

export const readInitialTheme = (): AppTheme => {
  try {
    const stored = window.localStorage.getItem(themeStorageKey);
    if (stored === "dark" || stored === "light") {
      return stored;
    }
  } catch {
    // A restricted browser may deny storage; the system preference still works.
  }
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches === true ? "dark" : "light";
};

export const applyTheme = (theme: AppTheme): void => {
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
  try {
    window.localStorage.setItem(themeStorageKey, theme);
  } catch {
    // Theme remains active for this session even when persistence is unavailable.
  }
};

export const ThemeToggle = ({
  onToggle,
  theme,
}: Readonly<{
  onToggle: () => void;
  theme: AppTheme;
}>): React.JSX.Element => {
  const nextTheme = theme === "dark" ? "light" : "dark";
  return (
    <button
      aria-label={`Use ${nextTheme} theme`}
      aria-pressed={theme === "dark"}
      className="theme-toggle"
      onClick={onToggle}
      title={`Use ${nextTheme} theme`}
      type="button"
    >
      <span aria-hidden="true" className="theme-toggle__icon">{theme === "dark" ? "☼" : "◐"}</span>
      <span>{theme === "dark" ? "Light" : "Dark"}</span>
    </button>
  );
};
