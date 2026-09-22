const { chromium } = require('playwright');
const { spawn } = require('node:child_process');
const { createInterface } = require('node:readline');
const assert = require('node:assert/strict');
const { pathToFileURL } = require('node:url');
const { resolve } = require('node:path');

(async () => {
  const browser = await chromium.launch({ executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    headless: false, ignoreDefaultArgs: ['--enable-automation', '--export-tagged-pdf'] });
  let helper;
  try {
    const session = await browser.newBrowserCDPSession();
    const { processInfo } = await session.send('SystemInfo.getProcessInfo');
    const pid = processInfo.find(p => p.type === 'browser').id;
    helper = spawn(process.argv[2], [String(pid)], { stdio: ['pipe', 'pipe', 'inherit'] });
    const lines = createInterface({ input: helper.stdout })[Symbol.asyncIterator]();
    async function read() {
      let timer;
      try {
        const result = await Promise.race([lines.next(), new Promise((_, reject) => { timer = setTimeout(() => reject(Error('native helper timeout')), 5000); })]);
        assert(!result.done, 'helper exited'); return JSON.parse(result.value);
      } finally { clearTimeout(timer); }
    }
    async function command(value) { helper.stdin.write(value + '\n'); const result = await read(); assert(!result.error, result.error); return result; }
    assert((await read()).ready);
    const page = await browser.newPage();
    await page.goto(pathToFileURL(resolve(__dirname, '../../../spikes/target-identity/fixtures/fields.html')).href);
    await page.bringToFront(); await command('activate');
    async function focus(selector) {
      const point = await page.locator(selector).evaluate(e => {
        const r = e.getBoundingClientRect();
        return { x: screenX + r.x + r.width / 2, y: screenY + outerHeight - innerHeight + r.y + r.height / 2 };
      });
      await command(`click ${point.x} ${point.y}`); await page.locator(selector).click();
      assert(await page.evaluate(() => document.hasFocus()));
    }
    await focus('#first');
    // Repeated permission reads must not restart Chrome's two-second activation.
    const started = Date.now();
    for (let i = 0; i < 6; i++) { assert((await command('watch')).trusted); await page.waitForTimeout(150); }
    await page.waitForTimeout(Math.max(0, 2500 - (Date.now() - started)));
    const expected = 'Before Logia café — delivery check. after';
    async function selectOld() {
      await page.evaluate(() => {
        const e = document.querySelector('#delivery');
        if (e.setSelectionRange) e.setSelectionRange(7, 10);
        else { const r = document.createRange(); r.setStart(e.firstChild, 7); r.setEnd(e.firstChild, 10); getSelection().removeAllRanges(); getSelection().addRange(r); }
      });
      for (let i = 0; i < 30; i++) {
        const range = await command('selection');
        if (range.location === 7 && range.length === 3) return;
        await page.waitForTimeout(50);
      }
      throw Error('selected range not exposed by fixture');
    }
    for (const kind of ['input', 'textarea', 'contenteditable']) {
      await page.evaluate(kind => {
        document.querySelector('#delivery')?.remove();
        const e = document.createElement(kind === 'contenteditable' ? 'div' : kind);
        e.id = 'delivery'; e.style.cssText = 'border:1px solid;padding:12px;min-height:36px';
        if (kind === 'contenteditable') { e.contentEditable = 'true'; e.textContent = 'Before OLD after'; }
        else e.value = 'Before OLD after';
        document.body.prepend(e);
      }, kind);
      await focus('#delivery'); await selectOld();
      assert((await command('arm')).armed, `${kind} must arm`);
      assert.equal((await command('send')).result, 5, 'keyboard dispatch is distinct from a receipt');
      await page.waitForFunction(expected => { const e = document.querySelector('#delivery'); return (e.value ?? e.textContent) === expected; }, expected);
      assert.equal((await command('send')).result, 3, 'duplicate attempt consumed');
      console.log(`PASS: production bridge inserts exact Unicode selection in Chrome ${kind}`);
    }
    async function reset() {
      await page.evaluate(() => {
        document.querySelector('#delivery')?.remove();
        const e = document.createElement('textarea'); e.id = 'delivery'; e.value = 'Before OLD after'; document.body.prepend(e);
      });
      await focus('#delivery'); await selectOld(); assert((await command('arm')).armed);
    }
    for (const change of ['field', 'caret', 'recreated', 'cancel', 'clipboard']) {
      await reset();
      if (change === 'field') {
        await page.locator('#second').evaluate(e => { e.value = ''; });
        await focus('#second');
      }
      if (change === 'caret') { await page.locator('#delivery').evaluate(e => e.setSelectionRange(0, 0)); await page.waitForTimeout(150); }
      if (change === 'recreated') { await page.locator('#delivery').evaluate(e => { const n = e.cloneNode(true); e.replaceWith(n); n.focus(); }); await page.waitForTimeout(150); }
      if (change === 'cancel') await command('cancel');
      if (change === 'clipboard') await command('clipboard-change');
      const result = (await command('send')).result;
      assert.equal(result, change === 'cancel' ? 3 : change === 'clipboard' ? 6 : 5, `${change} delivery outcome`);
      await page.waitForTimeout(100);
      if (change === 'field') {
        await page.waitForFunction(() => document.querySelector('#second').value === 'Logia café — delivery check.');
        assert.equal(await page.locator('#delivery').inputValue(), 'Before OLD after');
      } else if (change === 'caret' || change === 'recreated') {
        await page.waitForFunction(() => document.querySelector('#delivery').value === 'Logia café — delivery check.Before OLD after');
      } else assert.equal(await page.locator('#delivery').inputValue(), 'Before OLD after');
      if (change === 'clipboard') assert((await command('clipboard-check')).preserved);
      console.log(`PASS: ${change} follows current-cursor/cancel/clipboard policy`);
    }
    for (const kind of ['password', 'readonly']) {
      await page.evaluate(kind => {
        document.querySelector('#delivery')?.remove(); const e = document.createElement('input');
        e.id = 'delivery'; e.value = 'Synthetic only'; if (kind === 'password') e.type = 'password'; else e.readOnly = true;
        document.body.prepend(e);
      }, kind);
      await focus('#delivery'); assert((await command('arm')).armed, 'arming is session-only');
      const result = (await command('send')).result;
      assert.equal(result, kind === 'password' ? 7 : 5);
      await page.waitForTimeout(100);
      assert.equal(await page.locator('#delivery').inputValue(), 'Synthetic only', `${kind} content unchanged`);
      assert((await command('clipboard-transcript')).copied);
      console.log(`PASS: ${kind} remains unchanged, with transcript on clipboard`);
    }
  } finally {
    if (helper && helper.exitCode === null) {
      helper.stdin.end('quit\n');
      await Promise.race([new Promise(resolve => helper.once('exit', resolve)), new Promise(resolve => setTimeout(resolve, 2000))]);
      if (helper.exitCode === null) helper.kill();
    }
    await browser.close();
  }
})().catch(e => { console.error(e); process.exitCode = 1; });
