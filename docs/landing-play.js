/* The landing page's sample is the real thing: edit it, run it, and the
   toolchain that answers is the one in your tab. The engine is a megabyte, so
   nothing loads until you touch the panel — a visitor who only reads still
   gets a static page. */
'use strict';
(function () {
  const panel = document.getElementById('hero-play');
  if (!panel) return;
  const editor = document.getElementById('hero-editor');
  const mirror = document.getElementById('hero-mirror');
  const output = document.getElementById('hero-output');
  const runButton = document.getElementById('hero-run');
  const { ready, playSource, highlight } = window.KansoEngine;

  /* the <pre> is the scroll container; the <code> mirror inside it is inline
     and cannot scroll, so the highlight layer has to be moved by its parent
     or it lags behind the textarea and reads as a ghost copy */
  const mirrorScroll = mirror.parentElement;
  const track = () => {
    mirrorScroll.scrollTop = editor.scrollTop;
    mirrorScroll.scrollLeft = editor.scrollLeft;
  };
  const paint = () => {
    mirror.innerHTML = highlight(editor.value) + '\n';
    track();
  };

  let woken = false;
  const wake = () => {
    if (woken) return;
    woken = true;
    runButton.textContent = 'waking the engine…';
    ready().then(() => {
      runButton.textContent = 'run';
      runButton.disabled = false;
    });
  };

  const run = async () => {
    wake();
    output.textContent = 'running…';
    const result = await playSource(editor.value);
    output.textContent = result.text || '(no output)';
    output.classList.toggle('play-error', result.code !== 0);
    panel.classList.add('has-run');
  };

  editor.addEventListener('input', () => {
    paint();
    wake();
  });
  editor.addEventListener('scroll', track);
  editor.addEventListener('focus', wake, { once: true });
  editor.addEventListener('keydown', (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
      event.preventDefault();
      run();
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      const at = editor.selectionStart;
      editor.setRangeText('  ', at, editor.selectionEnd, 'end');
      paint();
    }
  });
  runButton.addEventListener('click', run);

  // carry whatever the visitor typed into the full playground
  document.getElementById('hero-open').addEventListener('click', () => {
    try {
      sessionStorage.setItem('kanso-carry', editor.value);
    } catch (e) {
      /* private mode: the playground just opens on its own example */
    }
  });

  paint();
})();
