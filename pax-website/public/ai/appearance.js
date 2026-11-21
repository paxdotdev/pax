// The reader-friendly guide does not mount the website runtime.
try {
  document.documentElement.dataset.paxTheme = localStorage.getItem('pax-site-theme') || 'dark';
} catch { /* Storage is optional. */ }
