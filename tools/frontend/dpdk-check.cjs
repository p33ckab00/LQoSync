// Isolated browser fixture only. Never use production account credentials.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1100},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    window.testRealNow=Date.now;
    Date.now=()=>window.testRealNow()+3600000; // One-hour client clock skew.
    const Native=window.EventSource;window.testStreams=[];
    window.EventSource=class extends Native{constructor(...args){super(...args);window.testStreams.push(this);}};
  });
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const card=page.getByRole('region',{name:'DPDK stats & health',exact:true});
  const nic=page.locator('.nic-summary');
  await nic.getByText('0000:01:00.0',{exact:true}).first().waitFor();
  assert.equal(await card.getByRole('meter',{name:'Worker 1 busy percent',exact:true}).getAttribute('value'),'75');
  assert.match(await card.locator('.dpdk-value').innerText(),/50.0%/);
  assert.equal(await card.locator('.panel-title .chip').innerText(),'ok');
  await card.getByRole('img',{name:'Mean worker busy percent during this browser session',exact:true}).waitFor();
  const details=card.locator('details');
  await details.locator('summary').focus();
  await page.keyboard.press('Enter');
  await card.getByLabel('DPDK worker details',{exact:true}).selectOption('0');
  await page.waitForTimeout(2100);
  assert.equal(await details.getAttribute('open'),'');
  assert.equal(await card.getByLabel('DPDK worker details',{exact:true}).inputValue(),'0');
  await card.getByText('RX cycles (total)',{exact:true}).waitFor();
  const workerTable=card.getByRole('region',{name:'DPDK worker measurements',exact:true});
  assert.match(await workerTable.innerText(),/25.0%/);
  await details.locator('summary').click();
  const out=process.env.DPDK_EVIDENCE_DIR||'/tmp/lqosync-dpdk-evidence';fs.mkdirSync(out,{recursive:true});
  const picture=async path=>{
    const viewport=page.viewportSize();
    await page.setViewportSize({width:viewport.width,height:6000});
    await page.evaluate(()=>{document.activeElement.blur();window.scrollTo(0,0);});
    await card.screenshot({path});
    await page.setViewportSize(viewport);
  };

  for(const theme of ['dark','light']){
    await page.getByLabel('Color theme').selectOption(theme);
    await page.evaluate(()=>document.activeElement.blur());
    await picture(`${out}/${theme}.png`);
  }
  for(const width of [768,320]){
    await page.setViewportSize({width,height:1000});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    assert.ok(await card.evaluate(n=>n.scrollWidth<=n.clientWidth),'card must fit narrow screens');
    if(width===320)await picture(`${out}/mobile.png`);
  }
  await page.emulateMedia({reducedMotion:'reduce',forcedColors:'active'});
  assert.equal(await card.locator('.dpdk-busy-line').evaluate(n=>getComputedStyle(n).animationName),'none');
  await page.emulateMedia({forcedColors:'none'});
  // Stop the fixture stream. Keep fallback reads consistent with each synthetic update.
  await page.evaluate(async()=>{
    window.testStreams.forEach(s=>s.close());
    window.dpdkView=await(await fetch('/api/v1/overview')).json();
  });
  await page.route('**/api/v1/overview',async route=>{
    const view=await page.evaluate(()=>window.dpdkView);
    await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(view)});
  });
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    view.dpdk.analytics.updated=at;
    view.dpdk.mean_busy_ppm=900000;
    view.dpdk.workers[0].busy_ppm=950000;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await page.waitForFunction(()=>document.querySelector('.dpdk-value').textContent.includes('90.0%'));
  assert.equal(await card.getByRole('meter',{name:'Worker 1 busy percent',exact:true}).getAttribute('value'),'95');
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    view.dpdk.analytics.updated=at-20;view.dpdk.analytics.stale=false;
    view.dpdk.health.updated=at;view.dpdk.health.state='degraded';
    view.dpdk.nic.updated=at;view.dpdk.ports[0].state='down';
    view.dpdk.ports[0].name='long_port_name_'.repeat(8);
    view.dpdk.ports[0].reset=true;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await card.getByText('degraded',{exact:true}).first().waitFor();
  await nic.locator('.nic-physical-port.link-down').first().waitFor();
  assert.match(await card.locator('.dpdk-sources').innerText(),/WorkersStale|Workers\s*Stale/);
  assert.match(await nic.textContent(),/Counter reset/);
  assert.ok(await card.evaluate(n=>n.scrollWidth<=n.clientWidth),'long port names must wrap');
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    view.dpdk.health.updated=at-30;view.dpdk.nic.updated=at-30;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await card.getByText('Unknown health',{exact:true}).waitFor();
  await nic.getByText('Link unknown · stale',{exact:true}).waitFor();
  await page.evaluate(()=>{
    const view=window.dpdkView;const at=Math.floor(window.testRealNow()/1000);
    view.dpdk.sampled_at=at;
    for(const key of ['analytics','health','nic']){view.dpdk[key].updated=at;view.dpdk[key].stale=false;}
    view.dpdk.health.state='healthy';
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await card.locator('.panel-title').getByText('healthy',{exact:true}).waitFor();
  await page.evaluate(()=>window.testStreams.forEach(s=>s.dispatchEvent(new Event('error'))));
  await card.getByText('Unknown health',{exact:true}).waitFor();
  assert.match(await card.locator('.dpdk-sources').innerText(),/WorkersStale|Workers\s*Stale/);
  await page.evaluate(()=>{
    window.dpdkView.dpdk=null;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(window.dpdkView)}));
  });
  await card.getByText('Worker telemetry unavailable',{exact:true}).waitFor();
  assert.equal(await card.locator('.dpdk-value').innerText(),'— mean busy');
  assert.equal(await card.getByRole('meter').count(),0);
  await page.evaluate(()=>{
    window.testStreams.forEach(s=>s.dispatchEvent(new Event('error')));
  });
  await card.getByText('Unknown health',{exact:true}).waitFor();
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: DPDK source data with one-hour client clock skew, worker bars and mean, session chart, keyboard details and persistent selection, independent stale/degraded/link states, reset label, missing data, Light/Dark, reduced motion, forced colors, and 768/320px layouts.');
})().catch(e=>{console.error(e);process.exit(1);});
