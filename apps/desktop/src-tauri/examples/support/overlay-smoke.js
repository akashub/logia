(() => {
  document.documentElement.innerHTML = `<head><style>
    *{box-sizing:border-box}html,body{margin:0;width:100%;height:100%;background:transparent}
    #card{position:absolute;inset:12px;background:#faf9f5;border-radius:14px;padding:8px 12px}
    #transcript{display:block;width:100%;height:42px;overflow-y:auto;line-height:20px;font-size:16px;border:0;padding:0;resize:none}
    button{position:absolute;top:64px;width:90px;height:28px}
    #stop{left:12px}#cancel{left:122px}#copy{left:232px}
    </style></head><body><div id="card"><textarea id="transcript">${'Synthetic transcript line.\n'.repeat(30)}</textarea>
    <button id="stop">Stop</button><button id="cancel">Cancel</button><button id="copy">Copy</button></div></body>`;
  const report = payload => window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {event:'overlay-smoke',payload});
  for(const action of ['stop','cancel','copy']) document.getElementById(action).onclick=()=>report(action);
  document.getElementById('transcript').onscroll=()=>report('scroll');
  document.getElementById('transcript').oninput=()=>report('edit');
  report('ready');
})();
