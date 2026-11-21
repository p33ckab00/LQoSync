// Isolated browser fixture only. Never use production account credentials.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-workers-evidence';fs.mkdirSync(out,{recursive:true});
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const nav=page.getByRole('navigation',{name:'Main navigation'});
  await nav.getByRole('link',{name:'Workers'}).click();
  await page.waitForURL('**/workers');
  await page.getByRole('heading',{name:'Workers',level:1}).waitFor();
  assert.equal(await nav.getByRole('link',{name:'Workers'}).getAttribute('aria-current'),'page');
  const table=page.locator('.workers-table');
  await table.locator('tbody tr').first().waitFor();
  assert.match(await page.locator('.workers-summary').innerText(),/^2 workers\. Mean busy \d+\.\d%\. Busiest: W\d at \d+\.\d%\.$/);
  const rows=await table.locator('tbody > tr:not(.worker-detail)').allInnerTexts();
  assert.equal(rows.length,2);
  assert.ok(rows.every(r=>/^W\d\s+\d+\.\d%/.test(r)),rows.join('|'));
  // The session trend draws a line after two distinct samples.
  await page.waitForFunction(()=>[...document.querySelectorAll('.sparkline path')].some(p=>(p.getAttribute('d')||'').startsWith('M')),{},{timeout:10000});
  // A disclosure shows the exact phase cycles and says what the shares mean.
  const first=table.locator('tbody th button').first();
  assert.equal(await first.getAttribute('aria-expanded'),'false');
  await first.click();
  assert.equal(await first.getAttribute('aria-expanded'),'true');
  await page.getByText('Cycle shares since the bridge started, not the CPU time now.').waitFor();
  await page.locator('.workers-panel').screenshot({path:`${out}/workers.png`});
  await page.setViewportSize({width:320,height:900});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'fits 320px');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: Workers navigation, summary, one row for each worker, session trend, phase disclosure, and 320px layout.');
})().catch(e=>{console.error(e);process.exit(1);});
