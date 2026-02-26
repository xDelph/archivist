// File modal — open/close, gallery nav (←/→), keyboard Esc
// Expand/collapse is handled natively by <details>/<summary> + HTMX

let modalFiles = [];
let modalIndex = 0;

function renderModalContent(url, name, mimetype) {
  let content = '';
  if (mimetype.startsWith('image/')) {
    content = `<img src="${url}" alt="${name}">`;
  } else if (mimetype.startsWith('video/')) {
    content = `<video src="${url}" controls autoplay></video>`;
  } else {
    content = `<iframe src="${url}" title="${name}"></iframe>`;
  }
  document.getElementById('modal-filename').textContent = name;
  document.getElementById('modal-download').href = url;
  document.getElementById('modal-body').innerHTML = content;
  const counter = document.getElementById('modal-counter');
  counter.textContent = modalFiles.length > 1 ? `${modalIndex + 1} / ${modalFiles.length}` : '';
  document.getElementById('modal-prev').disabled = modalIndex === 0;
  document.getElementById('modal-next').disabled = modalIndex === modalFiles.length - 1;
}

function openFileModal(files, index) {
  modalFiles = files;
  modalIndex = index;
  renderModalContent(files[index].url, files[index].name, files[index].mimetype);
  document.getElementById('file-modal').style.display = 'flex';
}

function closeFileModal() {
  document.getElementById('file-modal').style.display = 'none';
  document.getElementById('modal-body').innerHTML = '';
  modalFiles = [];
}

function modalNav(delta) {
  const next = modalIndex + delta;
  if (next < 0 || next >= modalFiles.length) return;
  modalIndex = next;
  renderModalContent(modalFiles[next].url, modalFiles[next].name, modalFiles[next].mimetype);
}

document.getElementById('modal-close').addEventListener('click', closeFileModal);
document.getElementById('modal-prev').addEventListener('click', () => modalNav(-1));
document.getElementById('modal-next').addEventListener('click', () => modalNav(1));
document.getElementById('file-modal').addEventListener('click', e => {
  if (e.target === document.getElementById('file-modal')) closeFileModal();
});
document.addEventListener('keydown', e => {
  if (e.key === 'Escape') closeFileModal();
  if (e.key === 'ArrowLeft') modalNav(-1);
  if (e.key === 'ArrowRight') modalNav(1);
});

// Delegated click: collect sibling files in the same thread for gallery
document.addEventListener('click', e => {
  const target = e.target.closest('[data-file-url]');
  if (!target) return;
  e.preventDefault();
  const container = target.closest('.thread-messages') || document;
  const allTargets = [...container.querySelectorAll('[data-file-url]')];
  const files = allTargets.map(el => ({
    url: el.dataset.fileUrl,
    name: el.dataset.fileName,
    mimetype: el.dataset.fileMime,
  }));
  const index = allTargets.indexOf(target);
  openFileModal(files, index >= 0 ? index : 0);
});
