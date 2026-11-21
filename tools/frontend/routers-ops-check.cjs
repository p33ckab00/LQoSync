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
  await page.getByRole('button',{name:'Sync now'}).click();
  await page.getByText('Sync started for all enabled routers. The table updates after the cycle.').waitFor();
  // Connection test results: a success with the identity, and a failure with its next step.
  const test=async(id,expect)=>{
    await page.goto(`http://127.0.0.1:19581/routers/${id}?tab=connection`);
    const button=page.getByRole('button',{name:'Test connection'});
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
  await page.getByText('Enable the router to test it: LQoSync makes no connection to a disabled router.').waitFor();
  await page.setViewportSize({width:320,height:900});
  await page.goto('http://127.0.0.1:19581/routers/lq3-00000000000000c1?tab=connection');
  await page.getByRole('button',{name:'Test connection'}).waitFor();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'fits 320px');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: Sync now, connection test success with identity, failure with its next step, no test for a disabled router, and 320px layout.');
})().catch(e=>{console.error(e);process.exit(1);});
