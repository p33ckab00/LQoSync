// Static design references only. No production service, account, or router is used.
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { pathToFileURL } = require('node:url');
const path = require('node:path');
const fs = require('node:fs');
const assert = require('node:assert/strict');
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH, headless: true, args: ['--no-sandbox'] });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, colorScheme: 'dark' });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  const results = [];
  for (const [file, screenshot, width, height, light] of [
    ['mockup.html','builder-dark.png',1440,1000,false],
    ['mockup.html','builder-light.png',1440,1000,true],
    ['assignments.html','assignments-dark.png',1440,1000,false],
    ['policies.html','policies-dark.png',1440,1000,false],
    ['changes.html','changes-dark.png',1440,1000,false],
    ['review.html','review-dark.png',1440,1000,false],
    ['mockup.html','builder-tablet.png',1024,768,false],
    ['mockup.html','builder-mobile.png',390,844,false],
    ['mockup.html','builder-320.png',320,640,false],
    ['assignments.html','assignments-mobile.png',390,844,false],
    ['policies.html','policies-mobile.png',390,844,false],
    ['changes.html','changes-mobile.png',390,844,false],
    ['review.html','review-mobile.png',390,844,false]
  ]) {
    await page.setViewportSize({width,height});
    await page.goto(pathToFileURL(path.join(__dirname,file)).href);
    await page.evaluate(async light => {
      document.body.classList.toggle('light',light);
      document.querySelector('select[aria-label="Color theme"]').value=light?'Light':'Dark';
      await document.fonts.ready;
    },light);
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true,`${screenshot}: no horizontal page overflow`);
    assert.equal(await page.evaluate(()=>document.fonts.check('15px Plex')),true,'local Plex font loaded');
    assert.equal(await page.getByRole('heading',{name:'Topology Manager',exact:true}).count(),1);
    const minimumFont = await page.locator('main').evaluate(root=>Math.min(...[...root.querySelectorAll('*')].filter(e=>e.getClientRects().length&&e.childNodes.length&&[...e.childNodes].some(n=>n.nodeType===3&&n.textContent.trim())).map(e=>parseFloat(getComputedStyle(e).fontSize))));
    assert.ok(minimumFont>=13,`${screenshot}: readable text floor`);
    const contrast=await page.evaluate(()=>{
      const style=getComputedStyle(document.body);
      const rgb=key=>{
        const e=document.createElement('span');e.style.color=style.getPropertyValue(key);document.body.append(e);
        const value=getComputedStyle(e).color.match(/[\d.]+/g).slice(0,3).map(Number);e.remove();return value;
      };
      const lum=c=>c.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((n,v,i)=>n+v*[.2126,.7152,.0722][i],0);
      const ratio=(a,b)=>{a=lum(rgb(a));b=lum(rgb(b));return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)};
      return {text:Math.min(...['--bg','--surface','--accent-bg'].flatMap(bg=>['--text','--muted'].map(fg=>ratio(fg,bg)))),primary:ratio('--on-accent','--accent'),control:ratio('--control','--surface'),focus:ratio('--accent','--surface')};
    });
    assert.ok(contrast.text>=4.5&&contrast.primary>=4.5,`${screenshot}: text token contrast`);
    assert.ok(contrast.control>=3&&contrast.focus>=3,`${screenshot}: control/focus contrast`);
    await page.screenshot({path:path.join(__dirname,screenshot),fullPage:file!=='review.html'});
    results.push({screenshot,width,height,theme:light?'light':'dark',horizontalPageOverflow:false,minimumFontPx:minimumFont,contrast});
  }
  assert.deepEqual(errors,[]);
  fs.writeFileSync(path.join(__dirname,'capture-results.json'),JSON.stringify({kind:'static-design-checks',notRuntimeAcceptance:true,results},null,2)+'\n');
  await browser.close();
  console.log(`PASS: ${results.length} static captures; local font, headings, text floor, token contrast, and horizontal overflow checked. Runtime behavior remains untested.`);
})().catch(e=>{console.error(e);process.exit(1);});
