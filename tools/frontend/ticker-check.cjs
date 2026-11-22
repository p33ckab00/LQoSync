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
  let releaseStartup;
  const startup=new Promise(resolve=>{releaseStartup=resolve;});
  for(const path of ['**/api/v1/overview','**/api/v1/overview/live']){
    await page.route(path,async route=>{await startup;await route.continue();});
  }
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  // The ticker lives in the NOC Command Deck, which renders only after the first source snapshot.
  const section=page.locator('.noc-deck .service-diagnostics');
  await page.getByText('Loading service evidence…',{exact:true}).waitFor();
  assert.equal(await page.locator('.ticker-track .ticker-item').count(),0,
    'do not start a moving queue before the first timestamped snapshot');
  releaseStartup();
  await section.waitFor();

  assert.equal(await section.getByRole('heading').count(),0,'banner has no title');
  assert.equal(await section.getByRole('button').count(),0,'banner has no controls');
  assert.equal(await section.evaluate(n=>n.classList.contains('panel')),false,'banner has no card wrapper');
  const list=page.getByRole('list',{name:'Service and component status'});
  await list.getByText('HEALTHY',{exact:true}).first().waitFor();
  assert.equal(await list.getByRole('listitem').count(),5);
  assert.equal(await list.locator('.ticker-activity').count(),0);
  // Inspect the visible moving copy, not only the accessible static list.
  await page.waitForFunction(()=>document.querySelector('.ticker-track')?.getAnimations().length>0);
  const initialValues=await page.locator('.ticker-track strong').allTextContents();
  assert.deepEqual(initialValues,['HEALTHY','shadow','v3_pair','Applied','482 ms',
    'HEALTHY','shadow','v3_pair','Applied','482 ms'],
    'the first moving queue must use the first source snapshot, not loading placeholders');

  for(const label of ['Service health','Runtime mode','Output target','Publication phase','Last cycle']) assert.equal(await list.getByText(label,{exact:true}).count(),1);
  const bannerBox=await section.boundingBox();
  const footerBox=await page.locator('.noc-deck .noc-footer').boundingBox();
  assert.ok(bannerBox.y>=footerBox.y+footerBox.height,'banner must sit at the bottom of the NOC Command Deck');
  assert.equal(await section.evaluate(n=>n===n.parentElement.lastElementChild),true,'banner is the last deck element');
  assert.equal(await page.locator('.ticker-clone').getAttribute('aria-hidden'),'true');
  await page.mouse.move(0,0);
  const track=page.locator('.ticker-track');
  const position=await track.evaluate(n=>{window.testTrack=n;window.testComponent=n.querySelector('.ticker-item:last-child');return getComputedStyle(n).transform;});
  await page.waitForTimeout(1300);
  assert.notEqual(await track.evaluate(n=>getComputedStyle(n).transform),position);
  assert.equal(await track.evaluate(n=>n===window.testTrack&&n.querySelector('.ticker-item:last-child')===window.testComponent),true);
  assert.ok(bannerBox.height<=60,'rolling banner stays compact');
  await section.hover();
  assert.equal(await track.evaluate(n=>getComputedStyle(n).animationPlayState),'running');
  await page.mouse.move(0,0);
  // A new activity entry must not rebase the position of the visible track.
  await page.evaluate(()=>{
    const source=window.testStreams.at(-1);source.close();
    const track=document.querySelector('.ticker-track');
    const animation=track.getAnimations()[0];
    animation.currentTime=Number(animation.effect.getTiming().duration)*0.4;
  });
  const jump=await page.evaluate(async()=>{
    const track=document.querySelector('.ticker-track');
    const before=track.getBoundingClientRect().x;
    const view=await(await fetch('/api/v1/overview')).json();
    view.components.push({name:'continuity_probe',state:'HEALTHY',observed_at:1});
    window.testActivityView=view;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
    await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
    return Math.abs(track.getBoundingClientRect().x-before);
  });
  assert.ok(jump<12,`activity insertion moved the track by ${jump}px`);
  assert.equal(await track.locator('.ticker-activity').count(),1,'new activity enters the moving queue once');
  assert.match(await track.locator('.ticker-item').last().innerText(),/continuity probe: HEALTHY/);

  const seam=await page.evaluate(async()=>{
    const track=document.querySelector('.ticker-track');
    const animation=track.getAnimations()[0];
    animation.currentTime=Number(animation.effect.getTiming().duration)-20;
    await new Promise(resolve=>requestAnimationFrame(resolve));
    const anchor=track.children[1];
    const before=anchor.getBoundingClientRect().x;
    await new Promise(resolve=>setTimeout(resolve,160));
    return {shift:Math.abs(anchor.getBoundingClientRect().x-before),same:anchor===track.firstElementChild};
  });
  assert.ok(seam.same,'only the outgoing item moves to the tail');
  assert.ok(seam.shift<12,`item handoff jumped by ${seam.shift}px`);
  for(let cycle=0;cycle<14;cycle++){
    const coverage=await page.evaluate(async()=>{
      const track=document.querySelector('.ticker-track');
      const animation=track.getAnimations()[0];
      animation.currentTime=Number(animation.effect.getTiming().duration)-10;
      await new Promise(resolve=>setTimeout(resolve,70));
      const viewport=track.parentElement.getBoundingClientRect();
      const first=track.firstElementChild.getBoundingClientRect();
      const last=track.lastElementChild.getBoundingClientRect();
      return first.left<=viewport.left+2 && last.right>=viewport.right-2;
    });
    assert.ok(coverage,'the handoff must leave no blank tail');
  }
  // Expire the probe so later activity assertions start with an empty feed.
  // Keep fallback polling on the same snapshot during the synthetic clock jump.
  // A fresh fixture snapshot would create new activity instead of testing expiry.
  await page.route('**/api/v1/overview',async route=>route.fulfill({
    status:200,contentType:'application/json',body:JSON.stringify(await page.evaluate(()=>window.testActivityView))
  }));
  await page.evaluate(()=>{window.testOriginalNow=Date.now;Date.now=()=>window.testOriginalNow()+121000;});
  await page.waitForFunction(()=>document.querySelectorAll('.ticker-static .ticker-activity').length===0,{},{timeout:3000});
  await page.evaluate(()=>{Date.now=window.testOriginalNow;});
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
  await page.evaluate(async()=>{
    const view=await(await fetch('/api/v1/overview')).json();
    const source=window.testStreams.at(-1);source.close();
    view.publication='IMPORT_CONFIRMED';
    window.testActivityView=view;
    view.components=[{name:'collector',state:'FAILED',observed_at:1},
      {name:'collector',state:'HEALTHY',observed_at:1},
      {name:'long_component_name_'.repeat(8),state:'UNKNOWN',observed_at:1}];
    source.dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await list.getByText('collector: FAILED',{exact:true}).waitFor();
  assert.equal(await list.getByText('IMPORT_CONFIRMED',{exact:true}).count(),1);
  assert.equal(await list.locator('.ticker-activity').count(),4);
  await page.evaluate(()=>window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(window.testActivityView)})));
  await page.waitForTimeout(200);
  assert.equal(await list.locator('.ticker-activity').count(),4,'repeated snapshots must not replay activity');
  assert.ok(await list.evaluate(n=>n.scrollWidth<=n.clientWidth),'long component names must wrap in reduced motion');
  await page.evaluate(()=>{window.testOriginalNow=Date.now;Date.now=()=>window.testOriginalNow()+121000;});
  await page.waitForFunction(()=>document.querySelectorAll('.ticker-group:not(.ticker-clone) .ticker-activity').length===0,{},{timeout:3000});
  await page.evaluate(()=>{Date.now=window.testOriginalNow;});
  assert.equal(await list.getByRole('listitem').count(),5,'permanent statuses survive activity expiry');
  await page.evaluate(()=>window.testStreams.forEach(s=>{s.close();s.dispatchEvent(new Event('error'));}));
  await list.getByText('UNKNOWN',{exact:true}).first().waitFor();
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: delayed first snapshot, no cached Unknown placeholders, bottom-of-NOC-deck placement, five permanent statuses, activity-only component changes, deduplication, expiry, ticker motion, stable nodes during live refresh, insertion continuity, repeated handoffs without blank tails, compact strip without card/title/buttons, continuous rolling, hidden clone, reduced motion, static details, stale status, Light/Dark, 768/320px.');
})().catch(e=>{console.error(e);process.exit(1);});
