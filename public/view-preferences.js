(function () {
  'use strict';

  const THEME_KEY = 'archivist_theme';
  const DENSITY_KEY = 'archivist_density';
  const body = document.body;
  if (!body) return;

  const themeToggle = document.querySelector('[data-theme-toggle]');
  const densityToggle = document.querySelector('[data-density-toggle]');

  function syncThemeToggle(theme) {
    if (!themeToggle) return;
    const isDark = theme === 'dark';
    themeToggle.classList.toggle('is-active', isDark);
    themeToggle.setAttribute('aria-pressed', isDark ? 'true' : 'false');
    themeToggle.setAttribute('aria-label', isDark ? 'Switch to light mode' : 'Switch to dark mode');
    themeToggle.setAttribute('title', isDark ? 'Switch to light mode' : 'Switch to dark mode');
  }

  function syncDensityToggle(density) {
    if (!densityToggle) return;
    const isCompact = density === 'compact';
    densityToggle.classList.toggle('is-active', isCompact);
    densityToggle.setAttribute('aria-pressed', isCompact ? 'true' : 'false');
    densityToggle.setAttribute('aria-label', isCompact ? 'Switch to normal density' : 'Switch to compact density');
    densityToggle.setAttribute('title', isCompact ? 'Switch to normal density' : 'Switch to compact density');
  }

  function applyTheme(theme) {
    const effective = theme === 'light' ? 'light' : 'dark';
    body.classList.toggle('theme-light', effective === 'light');
    syncThemeToggle(effective);
    try {
      localStorage.setItem(THEME_KEY, effective);
    } catch (_err) {}
  }

  function applyDensity(density) {
    const effective = density === 'compact' ? 'compact' : 'normal';
    body.classList.toggle('compact-mode', effective === 'compact');
    syncDensityToggle(effective);
    try {
      localStorage.setItem(DENSITY_KEY, effective);
    } catch (_err) {}
  }

  const savedTheme = (() => {
    try {
      return localStorage.getItem(THEME_KEY);
    } catch (_err) {
      return null;
    }
  })();
  const savedDensity = (() => {
    try {
      return localStorage.getItem(DENSITY_KEY);
    } catch (_err) {
      return null;
    }
  })();

  applyTheme(savedTheme || 'dark');
  applyDensity(savedDensity || 'normal');

  if (themeToggle) {
    themeToggle.addEventListener('click', () => {
      const currentTheme = body.classList.contains('theme-light') ? 'light' : 'dark';
      applyTheme(currentTheme === 'dark' ? 'light' : 'dark');
    });
  }

  if (densityToggle) {
    densityToggle.addEventListener('click', () => {
      const currentDensity = body.classList.contains('compact-mode') ? 'compact' : 'normal';
      applyDensity(currentDensity === 'compact' ? 'normal' : 'compact');
    });
  }
}());
