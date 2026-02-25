(function () {
  'use strict';

  const THEME_KEY = 'archivist_theme';
  const DENSITY_KEY = 'archivist_density';
  const body = document.body;
  if (!body) return;

  const themeButtons = Array.from(document.querySelectorAll('[data-theme-choice]'));
  const densityButtons = Array.from(document.querySelectorAll('[data-density-choice]'));

  function setPressedState(buttons, activeValue, attr) {
    buttons.forEach((button) => {
      const isActive = button.getAttribute(attr) === activeValue;
      button.classList.toggle('is-active', isActive);
      button.setAttribute('aria-pressed', isActive ? 'true' : 'false');
    });
  }

  function applyTheme(theme) {
    const effective = theme === 'light' ? 'light' : 'dark';
    body.classList.toggle('theme-light', effective === 'light');
    setPressedState(themeButtons, effective, 'data-theme-choice');
    try {
      localStorage.setItem(THEME_KEY, effective);
    } catch (_err) {}
  }

  function applyDensity(density) {
    const effective = density === 'compact' ? 'compact' : 'normal';
    body.classList.toggle('compact-mode', effective === 'compact');
    setPressedState(densityButtons, effective, 'data-density-choice');
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

  themeButtons.forEach((button) => {
    button.addEventListener('click', () => {
      applyTheme(button.getAttribute('data-theme-choice'));
    });
  });

  densityButtons.forEach((button) => {
    button.addEventListener('click', () => {
      applyDensity(button.getAttribute('data-density-choice'));
    });
  });
}());
