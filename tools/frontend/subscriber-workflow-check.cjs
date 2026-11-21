// Use only the isolated browser_fixture. The test never reads a production account or router.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  try {
    const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-subscriber-workflow';fs.mkdirSync(out,{recursive:true});
    const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'dark'});
    const errors=[],writes=[];let recordRequests=0,syntheticSnapshot=null;
    page.on('pageerror',e=>errors.push(e.message));
    page.on('request',r=>{
      if(r.url().includes('/api/v1/topology/observations'))recordRequests++;
      if(r.url().includes('/api/v1/topology')&&r.method()!=='GET')writes.push(r.method()+' '+r.url());
    });
    await page.route('**/api/v1/topology',async route=>{
      const response=await route.fetch();
      if(response.ok()){
        const data=await response.json();if(syntheticSnapshot)data.snapshot_id=syntheticSnapshot;
        await route.fulfill({response,json:data});
      }else await route.fulfill({response});
    });
    // The fixture intentionally has no MAC. Add one only to its synthetic display response.
    // Delay older Offline results to prove that a rapid filter change cannot overwrite Online.
    await page.route('**/api/v1/topology/observations?**',async route=>{
      const response=await route.fetch();
      const state=new URL(route.request().url()).searchParams.get('state');
      if(response.ok()){
        const data=await response.json();
        if(syntheticSnapshot)data.snapshot_id=syntheticSnapshot;
        for(const row of data.items)if(row.name==='account-02')row.mac='02:00:00:00:00:42';
        if(state==='offline')await new Promise(resolve=>setTimeout(resolve,500));
        await route.fulfill({response,json:data});
      }else await route.fulfill({response});
    });
    await page.goto('http://127.0.0.1:19581');
    await page.getByLabel('Username',{exact:true}).fill('browser-test');
    await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
    await page.getByRole('button',{name:'Sign in',exact:true}).click();
    const nav=page.getByRole('navigation',{name:'Main navigation'});
    await nav.getByRole('link',{name:'PPPoE',exact:true}).click();
    const table=page.getByRole('region',{name:'PPPoE identity records'});
    await table.locator('tbody tr').nth(3).waitFor();
    assert.equal(await table.locator('tbody tr').count(),4);
    await table.getByText('02:00:00:00:00:42',{exact:true}).waitFor();

    await page.getByLabel('Search',{exact:true}).fill('192.0.2.12');
    await page.getByRole('button',{name:'Apply',exact:true}).click();
    await page.waitForFunction(()=>document.querySelectorAll('[aria-label="PPPoE identity records"] tbody tr').length===1);
    assert.match(await table.innerText(),/account-02/);
    await page.getByRole('button',{name:'Clear filters',exact:true}).click();
    await page.waitForFunction(()=>document.querySelectorAll('[aria-label="PPPoE identity records"] tbody tr').length===4);
    assert.equal(await page.getByLabel('Search',{exact:true}).inputValue(),'');
    await page.getByLabel('Local plan',{exact:true}).selectOption('plan-access-100');
    await page.waitForFunction(()=>document.querySelectorAll('[aria-label="PPPoE identity records"] tbody tr').length===3);
    await page.getByLabel('Identity link',{exact:true}).selectOption('none');
    await page.getByText('No source records match. Clear filters or reload the source view.',{exact:true}).waitFor();
    await page.getByRole('button',{name:'Clear filters',exact:true}).click();
    await table.locator('tbody tr').nth(3).waitFor();

    const offlineReply=page.waitForResponse(r=>r.url().includes('/api/v1/topology/observations?')&&new URL(r.url()).searchParams.get('state')==='offline');
    const offlineRequest=page.waitForRequest(r=>r.url().includes('/api/v1/topology/observations?')&&new URL(r.url()).searchParams.get('state')==='offline');
    await page.getByLabel('State',{exact:true}).selectOption('offline');await offlineRequest;
    await page.getByLabel('State',{exact:true}).selectOption('online');
    await page.waitForFunction(()=>document.querySelectorAll('[aria-label="PPPoE identity records"] tbody tr').length===2);
    await offlineReply;
    await page.waitForTimeout(150);
    assert.equal(await table.locator('tbody tr').count(),2,'older Offline reply does not overwrite Online');
    assert.doesNotMatch(await table.locator('tbody').innerText(),/Offline/);

    const selected=table.getByRole('button',{name:'tm-sub-0000000000000002',exact:true});
    await selected.click();
    const detail=page.getByRole('region',{name:'Selected subscriber'});
    await detail.getByText('2 verified identities',{exact:false}).waitFor();
    assert.equal(await selected.getAttribute('aria-pressed'),'true');
    assert.equal(await page.evaluate(()=>document.activeElement.id),'source-detail-heading');
    const before=recordRequests;
    await page.getByRole('button',{name:'Reload view',exact:true}).click();
    await page.waitForFunction(()=>document.querySelector('[aria-labelledby="records-title"]').getAttribute('aria-busy')==='false');
    assert.ok(recordRequests>before,'Reload view refreshes the records, not only the summary');
    assert.equal(await selected.getAttribute('aria-pressed'),'true','reload preserves a matching selection');
    await page.getByRole('button',{name:'Close subscriber details',exact:true}).click();
    await detail.waitFor({state:'hidden'});
    assert.equal(await page.evaluate(()=>document.activeElement.textContent),'tm-sub-0000000000000002');
    await selected.click();await detail.getByText('2 verified identities',{exact:false}).waitFor();
    await page.keyboard.press('Escape');await detail.waitFor({state:'hidden'});
    await selected.click();await detail.getByText('2 verified identities',{exact:false}).waitFor();
    await detail.getByRole('button',{name:'Change plan',exact:true}).click();
    const dialog=page.getByRole('dialog');await dialog.waitFor();
    await dialog.getByLabel('Local plan',{exact:true}).selectOption('');
    syntheticSnapshot='r42-source-workflow-refresh';
    const poll=page.waitForResponse(r=>r.url().endsWith('/api/v1/topology'));
    await poll;
    await page.getByText('Details are from a previous snapshot. The current draft is kept. Refresh follows after the draft closes.',{exact:true}).waitFor();
    assert.equal(await dialog.isVisible(),true,'a source snapshot refresh keeps an open policy draft');
    assert.equal(await dialog.getByLabel('Local plan',{exact:true}).inputValue(),'','background refresh preserves draft edits');
    await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden'});
    await page.getByText('Details are from a previous snapshot. The current draft is kept. Refresh follows after the draft closes.',{exact:true}).waitFor({state:'hidden'});
    await page.screenshot({path:`${out}/pppoe-workflow-dark.png`,fullPage:true});

    await nav.getByRole('link',{name:'IPoE / DHCP',exact:true}).click();
    const dhcp=page.getByRole('region',{name:'DHCP client records'});await dhcp.locator('tbody tr').nth(2).waitFor();
    await dhcp.getByRole('button',{name:'tm-sub-0000000000000002',exact:true}).click();
    await detail.getByText('2 verified identities',{exact:false}).waitFor();
    for(const [scheme,width] of [['dark',1440],['light',1440],['dark',768],['light',390],['dark',320]]){
      await page.getByLabel('Color theme').selectOption(scheme);
      await page.setViewportSize({width,height:1000});
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`subscriber view fits ${width}px`);
      const headers=await dhcp.locator('thead th').evaluateAll(cells=>cells.map(cell=>getComputedStyle(cell).display));
      assert.equal(headers.length,6);
      assert.ok(headers.every(display=>display!=='none'),`all subscriber columns remain accessible at ${width}px`);
      await page.screenshot({path:`${out}/ipoe-workflow-${scheme}-${width}.png`,fullPage:true});
    }
    await page.setViewportSize({width:1440,height:1000});
    await page.getByRole('navigation',{name:'IPoE / DHCP views'}).getByRole('link',{name:'Local plans',exact:true}).click();
    await page.getByRole('button',{name:'Create plan',exact:true}).click();
    await page.getByRole('dialog',{name:'Create plan',exact:true}).waitFor();
    await page.keyboard.press('Escape');
    await page.getByRole('button',{name:'Edit plan Access 100',exact:true}).waitFor();
    assert.deepEqual(writes,[],'inspection and draft controls perform no policy or router writes');
    assert.deepEqual(errors,[]);
    console.log('PASS: source search/filter/reset/reload, late reply guard, MAC display, shared identity, selection/focus/close, reviewed actions, local plans, themes and 320–1440px layouts.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
