/* Every sample in the book is the real thing: edit it, run it, and the
   toolchain that answers is the one in your tab. The engine is a megabyte, so
   nothing loads until a reader touches a panel — someone who only reads still
   gets a static page.

   A panel is upgraded only when the program can actually run here. The tab has
   no filesystem, no argv and no stdin, and it can hold one file plus the entry
   that runs it, so a sample that reads a fixture or imports a sibling keeps the
   output the book recorded from a machine that had those things. */
'use strict';
(function () {
  const engine = window.KansoEngine;
  if (!engine) return;
  const { ready, playSource, runLibrary, highlight } = engine;

  /* what the tab cannot give a program */
  const NEEDS_A_MACHINE = /\bio\/(read_file|write_file|args|stdin|list_dir|run|make_dir|exists|is_dir)\b/;

  const localImport = (src) =>
    src.split('\n').some((line) => {
      const bare = line.trimStart();
      return bare.startsWith('import ') && !bare.startsWith('import "std/');
    });

  const runnable = (src) => !NEEDS_A_MACHINE.test(src) && !localImport(src);

  const exportsPlay = (src) => src.startsWith('pub play') || src.includes('\npub play');

  /* The panel title carries the file name; the sample is the text of the
     first code block, which is highlighted markup rather than source. */
  function sampleOf(panel) {
    const title = panel.querySelector('.code-panel-title');
    const code = panel.querySelector(':scope > pre > code');
    if (!title || !code) return null;
    const name = title.textContent.trim();
    if (!name.endsWith('.kso')) return null;
    /* a panel's markup ends at the last character of the program; a file ends
       with exactly one newline, and the grammar says so */
    const source = code.textContent.replace(/\n*$/, '\n');
    return { name, stem: name.slice(0, -4), source, title, code };
  }

  function upgrade(panel) {
    const sample = sampleOf(panel);
    if (!sample || !runnable(sample.source)) return;

    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'book-run';
    button.textContent = 'run';
    sample.title.appendChild(button);

    const edit = document.createElement('div');
    edit.className = 'book-edit';
    const mirrorScroll = document.createElement('pre');
    mirrorScroll.setAttribute('aria-hidden', 'true');
    const mirror = document.createElement('code');
    mirrorScroll.appendChild(mirror);
    const editor = document.createElement('textarea');
    editor.spellcheck = false;
    editor.autocomplete = 'off';
    editor.setAttribute('aria-label', `editable ${sample.name}`);
    editor.value = sample.source;
    edit.appendChild(mirrorScroll);
    edit.appendChild(editor);
    sample.code.parentElement.replaceWith(edit);

    /* the <pre> is the scroll container and the mirror inside it cannot
       scroll, so the highlight layer moves by its parent or it lags the caret */
    const track = () => {
      mirrorScroll.scrollTop = editor.scrollTop;
      mirrorScroll.scrollLeft = editor.scrollLeft;
    };
    const paint = () => {
      mirror.innerHTML = highlight(editor.value) + '\n';
      track();
    };
    paint();
    edit.style.height = `${editor.scrollHeight}px`;

    const output = panel.querySelector('.code-output pre code');
    const recorded = output ? output.innerHTML : '';

    let woken = false;
    const wake = () => {
      if (woken) return;
      woken = true;
      button.textContent = 'waking the engine…';
      ready().then(() => {
        button.textContent = 'run';
        button.disabled = false;
      });
    };

    const run = async () => {
      wake();
      button.disabled = true;
      const source = editor.value.replace(/\n*$/, '\n');
      const answer = exportsPlay(source)
        ? await runLibrary(sample.stem, source)
        : await playSource(source);
      button.disabled = false;
      if (!output) return;
      output.textContent = answer.text === '' ? '(no output)' : answer.text;
      panel.classList.add('book-ran');
    };

    const reset = () => {
      editor.value = sample.source;
      paint();
      if (output) output.innerHTML = recorded;
      panel.classList.remove('book-ran');
    };

    button.addEventListener('click', run);
    editor.addEventListener('input', paint);
    editor.addEventListener('scroll', track);
    editor.addEventListener('focus', wake, { once: true });
    panel.addEventListener('dblclick', (e) => {
      if (e.target === sample.title) reset();
    });
  }

  document.querySelectorAll('.code-panel').forEach(upgrade);
})();
