// Isolated synthetic fixture only. No production service or router is used.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1536,height:1024},colorScheme:'dark'});
  const errors=[],writes=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('request',r=>{if(r.method()!=='GET'&&!r.url().endsWith('/api/v1/session'))writes.push(r.url());});
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-router-layout';fs.mkdirSync(out,{recursive:true});
  await page.goto('http://127.0.0.1:19581/routers');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const inspect=page.getByRole('button',{name:'Inspect Access-01',exact:true});
  await inspect.waitFor();
  const preview=page.getByRole('complementary',{name:'Access-01',exact:true});
  await inspect.focus();await page.keyboard.press('Enter');
  await preview.waitFor();
  await page.waitForFunction(()=>document.activeElement?.id==='router-preview-title');
  assert.equal(await inspect.getAttribute('aria-expanded'),'true');
  assert.equal(await page.locator('.router-table tbody tr.selected').count(),1);
  assert.match(await preview.innerText(),/192\.0\.2\.11:8728[\s\S]*Configured[\s\S]*Last collection[\s\S]*Subscribers\s*3/);
  const left=await page.locator('.router-inventory-column').boundingBox(),right=await preview.boundingBox();
  assert.ok(right.x>=left.x+left.width+20,'details sit beside the inventory');
  assert.ok(right.y<=left.y+2,'the two columns share a top edge');
  await page.getByRole('heading',{name:'Routers',level:1}).click();
  for(const theme of ['dark','light']){
    await page.getByLabel('Color theme').selectOption(theme);
    await page.screenshot({path:`${out}/desktop-${theme}.png`,fullPage:true});
  }
  await preview.getByRole('heading',{name:'Access-01',exact:true}).focus();
  await page.keyboard.press('Escape');await preview.waitFor({state:'detached'});
  await page.waitForFunction(()=>document.activeElement?.id==='inspect-lq3-00000000000000a1');
  assert.equal(await inspect.getAttribute('aria-expanded'),'false');
  await inspect.click();await preview.getByRole('button',{name:'Close router details'}).click();
  await page.waitForFunction(()=>document.activeElement?.id==='inspect-lq3-00000000000000a1');
  await inspect.click();
  const filters=page.getByRole('search',{name:'Filter routers'});
  await filters.getByLabel('Search').fill('Access-02');
  await preview.waitFor({state:'detached'});
  await filters.getByLabel('Search').fill('');
  await page.getByRole('button',{name:'Inspect Access-02',exact:true}).click();
  const failed=page.getByRole('complementary',{name:'Access-02',exact:true});
  assert.match(await failed.innerText(),/Missing[\s\S]*Unreachable[\s\S]*No successful read yet/);
  await failed.getByRole('button',{name:'Close router details'}).click();
  await page.getByRole('button',{name:'Inspect Spare-01',exact:true}).click();
  assert.match(await page.getByRole('complementary',{name:'Spare-01'}).innerText(),/Disabled[\s\S]*no connection/);
  await page.getByRole('button',{name:'Inspect Access-01',exact:true}).click();
  for(const width of [1280,1024,768,390,320]){
    await page.setViewportSize({width,height:900});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`no page overflow at ${width}`);
    const table=page.getByRole('region',{name:'Router table',exact:true});
    assert.equal(await page.locator('.router-table thead th').count(),6);
    assert.ok(await page.locator('.router-table thead th').evaluateAll(cells=>cells.every(c=>getComputedStyle(c).display!=='none')),'all columns remain available');
    if(width<=768)assert.ok(await table.evaluate(e=>e.scrollWidth>e.clientWidth),'table scrolls locally');
    if(width<1200){const l=await page.locator('.router-inventory-column').boundingBox(),r=await preview.boundingBox();assert.ok(r.y>=l.y+l.height,'details stack below inventory');}
    if([1280,390,320].includes(width))await page.screenshot({path:`${out}/layout-${width}.png`,fullPage:true});
  }
  await page.setViewportSize({width:1536,height:1024});
  await preview.getByRole('link',{name:'Connection',exact:true}).click();
  await page.waitForURL('**/routers/lq3-00000000000000a1?tab=connection');
  await page.getByRole('button',{name:'Test connection',exact:true}).waitFor();
  assert.deepEqual(writes,[],'inspection and navigation cause no write or router connection request');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: reference split layout, stable selection, cached facts, keyboard open/close and focus return, filters, disabled/failed states, existing connection route, themes, 320–1536px and no write requests.');
})().catch(e=>{console.error(e);process.exit(1);});
