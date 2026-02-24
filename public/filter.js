(function () {
  'use strict';

  const form = document.getElementById('filter-form');
  const threadsEl = document.getElementById('threads');
  if (!form || !threadsEl) return;

  // Highlight search term in already-rendered HTML, skipping over tags.
  function highlight(html, term) {
    if (!term) return html;
    const escaped = term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    return html.replace(/(<[^>]+>)|([^<]+)/g, (_, tag, text) =>
      tag ? tag : text.replace(
        new RegExp(escaped, 'gi'),
        m => `<mark class="search-highlight">${m}</mark>`
      )
    );
  }

  function applyFilters() {
    const fd      = new FormData(form);
    const sort    = fd.get('sort')    || 'score';
    const period  = fd.get('period')  || 'all';
    const user    = fd.get('user')    || '';
    const channel = fd.get('channel') || '';
    const search  = (fd.get('search') || '').trim();
    const sl      = search.toLowerCase();

    const now    = Date.now() / 1000;
    const cutoff = period === '7d'  ? now - 7  * 86400
                 : period === '30d' ? now - 30 * 86400
                 : 0;

    const all = Array.from(threadsEl.querySelectorAll('.thread-card'));

    // Restore all previews to their original server-rendered HTML before filtering.
    all.forEach(c => { c.querySelector('.thread-preview').innerHTML = c.dataset.preview; });

    // Filter
    const visible = all.filter(c => {
      if (cutoff > 0 && parseFloat(c.dataset.ts) < cutoff) return false;
      if (user    && c.dataset.user        !== user)    return false;
      if (channel && c.dataset.channelName !== channel) return false;
      if (sl      && !c.dataset.text.includes(sl))      return false;
      return true;
    });

    // Sort
    visible.sort((a, b) => {
      if (sort === 'date')      return parseFloat(b.dataset.ts)          - parseFloat(a.dataset.ts);
      if (sort === 'reactions') return parseInt(b.dataset.reactions, 10) - parseInt(a.dataset.reactions, 10);
      if (sort === 'replies')   return parseInt(b.dataset.replies,   10) - parseInt(a.dataset.replies,   10);
      return parseInt(b.dataset.score, 10) - parseInt(a.dataset.score, 10);
    });

    // Apply client-side search highlight to matching previews.
    if (sl) {
      visible.forEach(c => {
        c.querySelector('.thread-preview').innerHTML = highlight(c.dataset.preview, search);
      });
    }

    // Hide all, then show and reorder the first 50 matches.
    all.forEach(c => { c.hidden = true; });
    visible.slice(0, 50).forEach(c => {
      c.hidden = false;
      threadsEl.appendChild(c); // moves to end — this is how reordering is done
    });

    // Empty state message
    const existing = document.getElementById('filter-empty');
    if (visible.length === 0) {
      if (!existing) {
        const p = document.createElement('p');
        p.id        = 'filter-empty';
        p.className = 'empty';
        p.textContent = 'No threads match your filters.';
        threadsEl.appendChild(p);
      }
    } else {
      existing?.remove();
    }
  }

  // Initial render: hide threads beyond the first 50 (all 200 are in the DOM).
  applyFilters();

  let debounceTimer;

  form.addEventListener('change', e => {
    if (e.target.tagName === 'SELECT') { applyFilters(); e.target.blur(); }
  });


  form.addEventListener('input', e => {
    if (e.target.id !== 'search-input') return;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(applyFilters, 100);
  });

  form.addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); applyFilters(); }
  });
}());
