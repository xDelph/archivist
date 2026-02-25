(function () {
  'use strict';

  const loader = document.getElementById('page-loader');
  if (!loader) return;

  function showLoader() {
    loader.classList.add('is-active');
  }

  document.addEventListener('click', (e) => {
    const tab = e.target.closest('a.header-tab');
    if (!tab) return;
    if (e.defaultPrevented) return;
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    if (tab.target && tab.target !== '_self') return;
    const href = tab.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    showLoader();
  });

  window.addEventListener('beforeunload', showLoader);
}());
