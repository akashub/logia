// Native IPC/ASR double; real React controllers run in two independent webviews.
// Native focus and delivery are exercised separately by the owned macOS fixture.
exports.openFixture = async (browser, options = {}) => {
  const context = await browser.newContext();
  const pages = {};
  const state = { generation: 0, starts: 0, warmups: 0, captures: 0, discarded: [], calls: [], conflict: false, overlayVisible: false, hidden: true };
  state.microphone = options.microphone ?? 'authorized'; state.accessibility = options.accessibility ?? true;
  const emit = async (name, payload, target = 'main') => pages[target]?.evaluate(({ name, payload }) => window.emitTest(name, payload), { name, payload });
  const recognition = event => emit('recognition', { generation: state.generation, event });
  await context.exposeBinding('nativeCommand', async ({ page }, command, args = {}) => {
    state.calls.push(command);
    if (command === 'plugin:event|emit_to') {
      if (args.event === 'overlay-state') state.lastOverlayState = args.payload;
      if (state.dropOverlayEvents || state.dropApplied && args.event === 'overlay-applied') return;
      await emit(args.event, args.payload, args.target.label); return;
    }
    if (command === 'plugin:event|emit') { await Promise.all(Object.keys(pages).map(label => emit(args.event, args.payload, label))); return; }
    if (command === 'model_ready') return true;
    // claude 2026-09-14: the model catalogue. Mirrors the Rust shape, including
    // an entry that is listed but not yet pinned for download.
    if (command === 'list_models') return state.models ?? (state.models = [
      { id: 'moonshine-streaming-small', name: 'Moonshine Streaming Small', detail: 'Live captions while you speak.',
        languages: 'English', bytes: 198506848, family: 'moonshine_streaming', installed: true, available: true, streams: true },
      { id: 'parakeet-v3', name: 'Parakeet v3', detail: 'Stronger on names and technical words.',
        languages: 'English and 24 European languages', bytes: 0, family: 'parakeet_stream', installed: false, available: false, streams: true }
    ]);
    if (command === 'remove_model') {
      const entry = (state.models || []).find(m => m.id === args.id);
      if (!entry) throw Error('Unknown model');
      if (entry.id === 'moonshine-streaming-small') throw Error('The default model cannot be removed while it is the only one.');
      entry.installed = false; return;
    }
    if (command === 'register_shortcut') {
      if (state.delayShortcut) await new Promise(resolve => { state.finishShortcut = resolve; });
      if (state.conflict) throw Error('Shortcut is already in use.'); return '⌃ ⌥ Space';
    }
    if (command === 'warmup_recognizer') { state.warmups++; return state.generation; }
    if (command === 'show_main_window') { state.hidden = false; return; }
    if (command === 'hide_main_window') {
      if (state.delayHide) await new Promise(resolve => { state.finishHide = resolve; });
      state.hidden = true; return;
    }
    if (command === 'hide_overlay') { state.overlayVisible = false; return; }
    if (command === 'resize_overlay') { state.size = args; return; }
    if (command === 'show_overlay') {
      state.capturesBeforeReveal = state.captures;
      if (state.failWindow) throw Error('Could not position the dictation overlay.');
      if (state.delayWindow) await new Promise(resolve => { state.finishWindow = resolve; });
      state.overlayVisible = true; return;
    }
    if (command === 'capture_target') return { token: String(++state.captures), status: 'armed' };
    if (command === 'discard_target') { state.discarded.push(args.token); return; }
    if (command === 'accessibility_permission') {
      if (args.prompt) state.typingRequests = (state.typingRequests || 0) + 1;
      return state.accessibility;
    }
    if (command === 'microphone_status') {
      if (state.delayMicStatus) await new Promise(resolve => (state.micWaiters ??= []).push(resolve));
      if (state.micStatusError) throw Error('Permission status unavailable');
      return state.microphone;
    }
    if (command === 'request_microphone') {
      state.micRequests = (state.micRequests || 0) + 1;
      if (state.micRequestError) throw Error('Microphone request failed');
      state.microphone = state.micRequestResult ?? 'authorized'; return state.microphone;
    }
    if (command === 'open_permission_settings') {
      if (state.settingsError) throw Error('Could not open System Settings');
      state.openedPermission = args.kind; return;
    }
    if (command === 'overlay_editing_token') return 1;
    if (command === 'set_overlay_editing') { state.editing = args.editing; return; }
    if (command === 'start_recording') {
      state.panelAtStart = await pages.overlay.evaluate(() => {
        const card = document.querySelector('.overlay-card');
        return card && { phase: card.dataset.phase, rung: card.dataset.rung };
      });
      state.visibleAtStart = state.overlayVisible;
      state.starts++; state.target = args.target; state.generation++;
      setTimeout(() => void recognition({ type: 'listening' }), 25); return state.generation;
    }
    if (command === 'stop_recording') { state.stops = (state.stops || 0) + 1; return; }
    if (command === 'cancel_recording') {
      if (state.delayCancel) return new Promise(resolve => { state.finishCancel = async () => {
        await recognition({ type: 'stopped' }); resolve({ status: 'sent', text: 'The authoritative delivered paragraph.' });
      }; });
      state.generation++; await recognition({ type: 'stopped' }); return { status: 'canceled' };
    }
    if (command === 'plugin:clipboard-manager|write_text') { state.copied = args.text; return; }
    throw Error(`Unexpected native command ${command} from ${page.url()}`);
  });
  await context.addInitScript(() => {
    let id = 0;
    const callbacks = new Map(), events = new Map();
    window.isTauri = true;
    window.emitTest = (name, payload) => { for (const handler of events.get(name) || []) callbacks.get(handler)?.({ payload }); };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      transformCallback(fn) { callbacks.set(++id, fn); return id; },
      async invoke(command, args) {
        if (command === 'plugin:event|listen') {
          if (location.search.includes('fail-listener') && args.event === 'dictation-shortcut') throw 'Listener unavailable';
          events.set(args.event, [...(events.get(args.event) || []), args.handler]); return args.handler;
        }
        if (command === 'plugin:event|unlisten') { events.set(args.event, (events.get(args.event) || []).filter(id => id !== args.eventId)); return; }
        return window.nativeCommand(command, args);
      }
    };
  });
  pages.main = await context.newPage();
  pages.overlay = await context.newPage();
  await pages.main.setViewportSize({ width: 940, height: 760 });
  await pages.overlay.setViewportSize({ width: 464, height: 280 });
  const url = process.env.LOGIA_UI_URL || 'http://127.0.0.1:1420';
  await pages.main.goto(url);
  if (options.connectOverlay !== false) await pages.overlay.goto(`${url}?overlay`);
  return { context, ...pages, state, emit, recognition, url };
};
