// Synthetic loopback fixture only. No production account, router, or shaping write.
// Checks the collapsible navigation rail: default per page, names, tooltips, reload, phone width.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  try{
    const page=await browser.newPage({viewport:{width:1440,height:1000}});
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto('http://127.0.0.1:19581');
    await page.getByLabel('Username',{exact:true}).fill('browser-test');
    await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
    await page.getByRole('button',{name:'Sign in',exact:true}).click();
    const aside=page.locator('aside.sidebar');
    const nav=page.getByRole('navigation',{name:'Main navigation'});
    await nav.getByRole('link',{name:'Topology',exact:true}).waitFor();
    assert.equal(await aside.getAttribute('data-collapsed'),'false','other pages start with the full navigation');
    await nav.getByRole('link',{name:'Topology',exact:true}).click();
    await page.waitForFunction(()=>document.querySelector('aside.sidebar')?.dataset.collapsed==='true');
    // The rail: narrow, and every link keeps its name and shows it on focus.
    assert.ok((await aside.boundingBox()).width<=80,'the rail is narrow');
    for(const name of ['Dashboard','Routers','Topology','PPPoE','IPoE / DHCP','Workers','Journal'])
      await nav.getByRole('link',{name,exact:true}).waitFor();
    const routers=nav.getByRole('link',{name:'Routers',exact:true});
    await page.keyboard.press('Tab');await routers.focus();
    await routers.evaluate(e=>new Promise((ok,no)=>{const t=Date.now();(function f(){getComputedStyle(e,'::after').opacity==='1'?ok():Date.now()-t>2000?no(new Error('focus shows the name')):requestAnimationFrame(f)})()}));
    const expand=page.getByRole('button',{name:'Expand navigation',exact:true});
    assert.equal(await expand.getAttribute('aria-expanded'),'false');
    await expand.click();
    assert.equal(await aside.getAttribute('data-collapsed'),'false');
    const collapse=page.getByRole('button',{name:'Collapse navigation',exact:true});
    assert.equal(await collapse.getAttribute('aria-expanded'),'true');
    assert.ok((await aside.boundingBox()).width>=200,'the full navigation is back');
    // The choice of the operator survives a reload, also on the Topology page.
    await page.reload();await nav.getByRole('link',{name:'Topology',exact:true}).waitFor();
    assert.equal(await aside.getAttribute('data-collapsed'),'false','the choice survives a reload');
    await collapse.click();
    assert.equal(await aside.getAttribute('data-collapsed'),'true');
    // A phone keeps its top navigation and has no sideways scroll.
    await page.setViewportSize({width:390,height:800});
    assert.ok(await page.evaluate(()=>document.scrollingElement.scrollWidth<=390),'no sideways scroll at 390 px');
    assert.deepEqual(errors,[]);
    console.log('PASS: navigation rail default, names, focus tooltip, reload memory, phone width.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});
