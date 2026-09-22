// Rebuild native icons from the same vector used by the interface.
// Requires the pinned desktop development dependencies; no network or AI service.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { chromium } = require('playwright');
const desktop = path.resolve(__dirname, '..');

(async () => {
  const mark = fs.readFileSync(path.join(desktop, 'src/brand/mark.svg'), 'utf8');
  const paths = mark.replace(/<svg[^>]*>/, '').replace('</svg>', '').trim();
  const colored = paths.replace('currentColor', '#f7f6f2').replace('currentColor', '#f2a03d');
  const icon = `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <rect x="64" y="64" width="896" height="896" rx="202" fill="#171a2e"/>
  <rect x="65" y="65" width="894" height="894" rx="201" fill="none" stroke="#ffffff" stroke-opacity=".08" stroke-width="2"/>
  <g transform="translate(128 96) scale(9.6)">${colored}</g>
</svg>\n`;
  const output = path.join(desktop, 'src-tauri/icons');
  fs.mkdirSync(output, { recursive: true });
  const source = path.join(output, 'icon.svg');
  fs.writeFileSync(source, icon);
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'logia-icon-export-'));
  try {
    execFileSync('pnpm', ['tauri', 'icon', source, '--output', temporary], { cwd: desktop, stdio: 'pipe' });
    for (const file of ['32x32.png', '128x128.png', '128x128@2x.png', 'icon.png', 'icon.icns', 'icon.ico']) {
      fs.copyFileSync(path.join(temporary, file), path.join(output, file));
    }
    const browser = await chromium.launch({ headless: true });
    try {
      const page = await browser.newPage();
      const rgba = await page.evaluate(async svg => {
        const image = new Image();
        image.src = 'data:image/svg+xml;base64,' + btoa(svg);
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = canvas.height = 44;
        const context = canvas.getContext('2d');
        context.drawImage(image, 0, 0, 44, 44);
        return [...context.getImageData(0, 0, 44, 44).data];
      }, mark.replace('0 0 80 80', '8 8 64 64').replaceAll('currentColor', '#000000'));
      fs.writeFileSync(path.join(output, 'tray.rgba'), Buffer.from(rgba));
    } finally { await browser.close(); }
  } finally { fs.rmSync(temporary, { recursive: true, force: true }); }
  console.log('Exported Logia app icons and 44px monochrome menu-bar template.');
})().catch(error => { console.error(error.message); process.exitCode = 1; });
