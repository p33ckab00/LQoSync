// Browser fixture only. Never use production account credentials here.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1100},colorScheme:'dark',reducedMotion:'no-preference'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    const Native=window.EventSource;window.testStreams=[];
    window.EventSource=class extends Native{constructor(...args){super(...args);window.testStreams.push(this);}};
  });
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  const section=page.locator('.service-diagnostics');
  await section.getByRole('heading',{name:'Service diagnostics',exact:true}).waitFor();
  const list=page.getByRole('list',{name:'Service and component status'});
  await list.getByText('HEALTHY',{exact:true}).first().waitFor();
  assert.equal(await list.getByRole('listitem').count(),8);
  assert.equal(await page.locator('.ticker-clone').getAttribute('aria-hidden'),'true');
  await page.mouse.move(0,0);
  const track=page.locator('.ticker-track');
  const position=await track.evaluate(n=>{window.testTrack=n;window.testComponent=n.querySelector('.ticker-item:last-child');return getComputedStyle(n).transform;});
  await page.waitForTimeout(1300);
  assert.notEqual(await track.evaluate(n=>getComputedStyle(n).transform),position);
  assert.equal(await track.evaluate(n=>n===window.testTrack&&n.querySelector('.ticker-item:last-child')===window.testComponent),true);
  await section.getByRole('button',{name:'Pause banner',exact:true}).focus();
  await page.keyboard.press('Enter');
  assert.equal(await page.getByRole('button',{name:'Resume banner',exact:true}).count(),1);
  await page.evaluate(()=>document.activeElement.blur());await page.mouse.move(0,0);
  const stopped=await track.evaluate(n=>getComputedStyle(n).transform);
  await page.waitForTimeout(1200);
  assert.equal(await track.evaluate(n=>getComputedStyle(n).transform),stopped);
  await section.getByRole('button',{name:'Resume banner',exact:true}).click();
  await page.evaluate(()=>document.activeElement.blur());await page.mouse.move(0,0);
  assert.equal(await track.evaluate(n=>getComputedStyle(n).animationPlayState),'running');
  await section.hover();
  assert.equal(await track.evaluate(n=>getComputedStyle(n).animationPlayState),'paused');
  await page.mouse.move(0,0);
  const out=process.env.TICKER_EVIDENCE_DIR || '/tmp/lqosync-ticker-evidence';fs.mkdirSync(out,{recursive:true});
  await page.getByLabel('Color theme').selectOption('dark');
  await page.evaluate(()=>{document.activeElement.blur();window.scrollTo(0,0);});
  await page.screenshot({path:`${out}/dark.png`,fullPage:true});
  await page.getByLabel('Color theme').selectOption('light');
  await page.evaluate(()=>{document.activeElement.blur();window.scrollTo(0,0);});
  await page.screenshot({path:`${out}/light.png`,fullPage:true});
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await track.evaluate(n=>getComputedStyle(n).animationName),'none');
  assert.equal(await page.locator('.ticker-clone').evaluate(n=>getComputedStyle(n).display),'none');
  for(const width of [768,320]){
    await page.setViewportSize({width,height:1000});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    assert.ok(await list.evaluate(n=>n.scrollWidth<=n.clientWidth));
    await page.evaluate(()=>{document.activeElement.blur();window.scrollTo(0,0);});
    if(width===320)await page.screenshot({path:`${out}/mobile-static.png`,fullPage:true});
  }
  await page.locator('details.diagnostics summary').click();
  await section.getByText('Publication phase',{exact:true}).waitFor();
  await page.evaluate(async()=>{
    const view=await(await fetch('/api/v1/overview')).json();
    const source=window.testStreams.at(-1);source.close();
    view.components=[{name:'collector',state:'FAILED',observed_at:1},
      {name:'collector',state:'HEALTHY',observed_at:1},
      {name:'long_component_name_'.repeat(8),state:'UNKNOWN',observed_at:1}];
    source.dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await list.getByText('FAILED',{exact:true}).waitFor();
  assert.equal(await list.getByRole('listitem').count(),9,'component identities must include position when names repeat');
  assert.ok(await list.evaluate(n=>n.scrollWidth<=n.clientWidth),'long component names must wrap in reduced motion');
  await page.evaluate(()=>window.testStreams.forEach(s=>{s.close();s.dispatchEvent(new Event('error'));}));
  await list.getByText('UNKNOWN',{exact:true}).first().waitFor();
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: ticker motion, stable nodes during live refresh, keyboard pause/resume, hover pause, hidden clone, reduced motion, static details, stale status, Light/Dark, 768/320px.');
})().catch(e=>{console.error(e);process.exit(1);});
