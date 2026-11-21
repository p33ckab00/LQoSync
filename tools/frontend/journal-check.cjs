// Isolated browser fixture only. Never use production account credentials.
// The fixture answers journal reads with synthetic lines; the journal of this machine is never read.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const writes=[];page.on('request',r=>{if(r.method()!=='GET'&&!r.url().endsWith('/api/v1/session'))writes.push(r.url());});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-journal-evidence';fs.mkdirSync(out,{recursive:true});
  await page.goto('http://127.0.0.1:19581/journal');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  await page.getByRole('heading',{name:'Journal',level:1}).waitFor();
  const lines=page.locator('.journal-line');
  await lines.first().waitFor();
  assert.equal(await lines.count(),200,'one page of lines');
  assert.match(await lines.first().innerText(),/cycle 599 done/,'newest first');
  const count=page.locator('.router-count');
  // Older lines: a second page joins below, with no repeat.
  await page.getByRole('button',{name:'Load older lines'}).click();
  await page.waitForFunction(()=>document.querySelectorAll('.journal-line').length===400);
  const texts=await lines.allInnerTexts();
  assert.equal(new Set(texts).size,texts.length,'no repeated line');
  // Level and search filters.
  const filters=page.getByRole('search',{name:'Filter the log'});
  await filters.getByLabel('Level').selectOption('3');
  await page.waitForFunction(()=>[...document.querySelectorAll('.journal-line')].every(l=>l.innerText.includes('Error')));
  await filters.getByLabel('Search in the loaded lines').fill('line 557');
  await page.waitForFunction(()=>document.querySelectorAll('.journal-line').length===1);
  assert.match(await count.innerText(),/^1 of \d+ lines$/);
  await page.locator('.journal-panel').screenshot({path:`${out}/journal.png`});
  await filters.getByLabel('Search in the loaded lines').fill('');
  await filters.getByLabel('Service').selectOption('lqosync3-web.service');
  await page.getByText('No lines at this level for this service.').waitFor();
  await filters.getByLabel('Follow new lines').uncheck();
  await page.locator('.freshness').getByText('Paused').waitFor();
  await page.setViewportSize({width:320,height:900});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'fits 320px');
  assert.deepEqual(writes,[],'the Journal sends no write request');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: Journal newest first, one bounded page, older lines without repeats, level and search filters, service switch, Follow pause, no writes, and 320px layout.');
})().catch(e=>{console.error(e);process.exit(1);});
