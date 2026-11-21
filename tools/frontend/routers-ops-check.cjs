// Isolated browser fixture only. Never use production account credentials.
// The fixture answers connection tests from a fake service socket; no router is contacted.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:1440,height:1000},colorScheme:'dark'});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const out=process.env.ROUTERS_EVIDENCE_DIR||'/tmp/lqosync-routers-evidence';fs.mkdirSync(out,{recursive:true});
  await page.goto('http://127.0.0.1:19581/routers');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  await page.locator('.router-table tbody tr').first().waitFor();
  // Sync now: one request; the table follows the next cycle.
  const sync=page.getByRole('button',{name:'Sync now'});
  const add=page.getByRole('button',{name:'Add router',exact:true});
  const before=await add.boundingBox();
  await sync.click();
  await page.getByText('Sync started for all enabled routers. The table updates after the cycle.').waitFor();
  assert.deepEqual(await add.boundingBox(),before,'sync feedback does not move the toolbar');
  for(const [theme,width] of [['dark',1440],['light',1440],['dark',390]]){
    await page.getByLabel('Color theme').selectOption(theme);
    await page.setViewportSize({width,height:900});
    await page.screenshot({path:`${out}/toolbar-${theme}-${width}.png`});
    const a=await add.boundingBox(),s=await sync.boundingBox();
    assert.equal(a.y,s.y);
    assert.ok(a.x+a.width<=width);
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  }
  await page.setViewportSize({width:1440,height:1000});
  // Connection test results: a success with the identity, and a failure with its next step.
  const test=async(id,expect)=>{
    await page.goto(`http://127.0.0.1:19581/routers/${id}?tab=connection`);
    const button=page.getByRole('button',{name:'Test connection'});
    const edit=page.getByRole('button',{name:'Edit connection',exact:true});
    const testBox=await button.boundingBox(),editBox=await edit.boundingBox();
    assert.equal(testBox.y,editBox.y,'connection icons share one row');
    assert.ok(editBox.x>testBox.x);
    assert.equal(await button.innerText(),'');
    assert.equal(await edit.innerText(),'');
    const hint=label=>page.locator('.action-tooltip').filter({hasText:label});
    await button.hover();
    await hint('Test connection').waitFor({state:'visible'});
    await hint('Test connection').hover();
    assert.ok(await hint('Test connection').isVisible(),'the hint remains hoverable');
    await button.focus();
    await page.keyboard.press('Escape');
    await hint('Test connection').waitFor({state:'hidden'});
    await edit.focus();
    await hint('Edit connection').waitFor({state:'visible'});
    await page.keyboard.press('Escape');
    await hint('Edit connection').waitFor({state:'hidden'});
    await edit.hover();
    await hint('Edit connection').waitFor({state:'visible'});
    await page.locator('.router-section').screenshot({path:`${out}/connection-icons-${id}.png`});
    await page.keyboard.press('Escape');
    await button.click();
    await page.locator('.test-result').getByText(expect).waitFor();
    assert.match(await page.locator('.test-result').innerText(),/Tested at .* in 42 ms\./);
  };
  await test('lq3-00000000000000c1','Connected and signed in. The router identity is fixture-c1.');
  assert.ok((await page.locator('.test-result').getAttribute('class')).includes('good'));
  await page.locator('.router-section').screenshot({path:`${out}/test-ok.png`});
  await test('lq3-00000000000000a2','No TCP connection to 192.0.2.12:8729. Check the address, the API port, and the firewall.');
  assert.ok((await page.locator('.test-result').getAttribute('class')).includes('bad'));
  // A disabled router has no test button action.
  await page.goto('http://127.0.0.1:19581/routers/lq3-00000000000000a3?tab=connection');
  const off=page.getByRole('button',{name:'Test connection'});
  await off.waitFor();
  assert.ok(await off.isDisabled());
  await off.locator('..').hover();
  await page.locator('.action-tooltip').filter({hasText:'Test connection'}).waitFor({state:'visible'});
  await page.keyboard.press('Escape');
  await page.locator('.action-tooltip').filter({hasText:'Test connection'}).waitFor({state:'hidden'});
  await page.getByText('Enable the router to test it: LQoSync makes no connection to a disabled router.').waitFor();
  await page.setViewportSize({width:320,height:900});
  await page.goto('http://127.0.0.1:19581/routers/lq3-00000000000000c1?tab=connection');
  await page.getByRole('button',{name:'Test connection'}).waitFor();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'fits 320px');
  const compact=page.getByRole('group',{name:'Connection actions'});
  const boxes=await compact.locator('button').evaluateAll(bs=>bs.map(b=>({x:b.getBoundingClientRect().x,y:b.getBoundingClientRect().y,width:b.getBoundingClientRect().width})));
  assert.equal(boxes[0].y,boxes[1].y);
  assert.ok(boxes.every(b=>b.width>=44));
  await page.screenshot({path:`${out}/connection-icons-320.png`});
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: Sync now, connection test success with identity, failure with its next step, no test for a disabled router, and 320px layout.');
})().catch(e=>{console.error(e);process.exit(1);});
