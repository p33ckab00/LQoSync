// Isolated browser fixture only. Never use production account credentials.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const writes=[];page.on('request',r=>{if(r.method()!=='GET'&&!r.url().endsWith('/api/v1/session'))writes.push(`${r.method()} ${r.url()}`);});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-routers-evidence';fs.mkdirSync(out,{recursive:true});
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  // The Dashboard counts every configured router, including the WAN-only core.
  const metric=page.locator('.metric').filter({has:page.getByRole('heading',{name:'Routers',exact:true})});
  await metric.getByText('3',{exact:true}).waitFor();

  const nav=page.getByRole('navigation',{name:'Main navigation'});
  await nav.getByRole('link',{name:'Routers'}).click();
  await page.waitForURL('**/routers');
  await page.getByRole('heading',{name:'Routers',level:1}).waitFor();
  assert.equal(await nav.getByRole('link',{name:'Routers'}).getAttribute('aria-current'),'page');
  assert.equal(await nav.getByRole('link',{name:'Dashboard'}).getAttribute('aria-current'),null);
  const table=page.locator('.router-table');
  await table.locator('tbody tr').first().waitFor();
  assert.equal(await table.locator('tbody tr').count(),3);
  const row=name=>table.locator('tbody tr').filter({has:page.getByRole('link',{name,exact:true})});
  assert.match(await row('Core-01').innerText(),/WAN[\s\S]*Not read[\s\S]*No subscriber role[\s\S]*TLS, checked with its CA/);
  assert.match(await row('Access-02').innerText(),/Unreachable[\s\S]*No successful read yet[\s\S]*certificate not checked[\s\S]*Credential missing/);
  assert.match(await row('Access-01').innerText(),/PPPoE[\s\S]*DHCP[\s\S]*Online/);
  for(const theme of ['dark','light']){
    await page.getByLabel('Color theme').selectOption(theme);
    await page.locator('.routers-panel').screenshot({path:`${out}/inventory-${theme}.png`});
  }

  // Filters: role, read state, and text combine. Clear filters restores all rows.
  const filters=page.getByRole('search',{name:'Filter routers'});
  await filters.getByLabel('Role').selectOption('wan');
  assert.equal(await table.locator('tbody tr').count(),1);
  await filters.getByLabel('Role').selectOption('');
  await filters.getByLabel('Read state').selectOption('problem');
  assert.deepEqual(await table.locator('tbody th a').allTextContents(),['Access-02']);
  await filters.getByLabel('Read state').selectOption('');
  await filters.getByLabel('Search').fill('nothing-matches');
  await page.getByText('No router matches these filters.').waitFor();
  await page.getByRole('button',{name:'Clear filters'}).click();
  assert.equal(await table.locator('tbody tr').count(),3);

  // Detail: tabs are links; Back restores the inventory filters.
  await filters.getByLabel('Role').selectOption('dhcp');
  await row('Access-01').getByRole('link',{name:'Access-01'}).click();
  await page.waitForURL('**/routers/lq3-00000000000000a1');
  const h1=page.getByRole('heading',{name:'Access-01',level:1});
  await h1.waitFor();
  assert.ok(await h1.evaluate(n=>document.activeElement===n),'focus moves to the page heading');
  const facts=page.locator('.router-facts');
  assert.match(await facts.innerText(),/Subscribers\s*3/);
  assert.match(await facts.innerText(),/14\.59950, 120\.98420/);
  const tabs=page.getByRole('navigation',{name:'Router sections'});
  await tabs.getByRole('link',{name:'Sources'}).click();
  await page.waitForURL('**/routers/lq3-00000000000000a1?tab=sources');
  assert.equal(await tabs.getByRole('link',{name:'Sources'}).getAttribute('aria-current'),'page');
  assert.match(await page.locator('.router-section').innerText(),/PPPoE\s*Yes\s*3\s*2\s*1/);
  assert.match(await page.locator('.router-section').innerText(),/Router exclusions\s*3/);
  await page.locator('.router-section').screenshot({path:`${out}/detail-sources.png`});
  await page.goBack();await page.goBack();
  await page.waitForURL('**/routers');
  assert.equal(await filters.getByLabel('Role').inputValue(),'dhcp','Back keeps the filters');
  assert.equal(await table.locator('tbody tr').count(),1);

  // Deep links work after a reload; an unknown router and an unknown page say so.
  await page.goto('http://127.0.0.1:19581/routers/lq3-00000000000000c1?tab=connection');
  await page.getByRole('heading',{name:'Core-01',level:1}).waitFor();
  assert.match(await page.locator('.router-section').innerText(),/TLS, checked with its CA[\s\S]*CA certificate\s*Configured[\s\S]*Credential\s*Configured/);
  assert.ok(!(await page.content()).includes('.password'),'no file path reaches the page');
  await page.goto('http://127.0.0.1:19581/routers/lq3-ffffffffffffffff');
  await page.getByRole('heading',{name:'Router not found',level:1}).waitFor();

  // Narrow screens: no horizontal page scroll; the name and state stay visible.
  await page.goto('http://127.0.0.1:19581/routers');
  await table.locator('tbody tr').first().waitFor();
  for(const width of [768,320]){
    await page.setViewportSize({width,height:900});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`fits ${width}px`);
    assert.ok(await row('Access-02').getByText('Unreachable').isVisible());
    if(width===320)await page.locator('.routers-panel').screenshot({path:`${out}/inventory-320.png`});
  }
  assert.deepEqual(writes,[],'the Routers screen sends no write request');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: Dashboard router count, navigation and aria-current, inventory states and transport labels, combined filters, detail tabs as links, heading focus, Back keeps filters, deep links, unknown router, no file paths, no write requests, Light/Dark, and 768/320px layouts.');
})().catch(e=>{console.error(e);process.exit(1);});
