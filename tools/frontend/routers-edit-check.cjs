// Isolated browser fixture only. Never use production account credentials.
// The fixture answers router edits on its synthetic inventory; no config file or router is touched.
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

  // Toolbar controls stay aligned; the native modal contains focus and restores the trigger.
  const add=page.getByRole('button',{name:'Add router',exact:true});
  const sync=page.getByRole('button',{name:'Sync now',exact:true});
  assert.equal(await add.innerText(),'');
  assert.equal(await sync.innerText(),'');
  const addBox=await add.boundingBox(),syncBox=await sync.boundingBox();
  assert.equal(addBox.y,syncBox.y);
  assert.ok(addBox.width>=44&&syncBox.width>=44);
  await page.getByLabel('Search',{exact:true}).fill('Core');
  await add.click();
  const registration=page.getByRole('dialog',{name:'Add router',exact:true});
  await registration.waitFor();
  assert.equal(await registration.evaluate(d=>d.matches(':modal')),true);
  assert.equal(await page.evaluate(()=>document.activeElement.id),'add-router-title');
  for(let i=0;i<18;i++){
    await page.keyboard.press('Tab');
    assert.equal(await registration.evaluate(d=>d.contains(document.activeElement)||document.activeElement===document.body),true,'Tab cannot focus a background control');
  }
  for(const width of [1440,768,390]){
    await page.setViewportSize({width,height:900});
    const bounds=await registration.boundingBox();
    assert.ok(bounds.x>=0&&bounds.x+bounds.width<=width);
    assert.equal(await registration.evaluate(d=>d.scrollWidth<=d.clientWidth),true);
    await page.screenshot({path:`${out}/add-modal-${width}.png`});
  }
  await page.keyboard.press('Escape');
  await registration.waitFor({state:'hidden'});
  await page.waitForURL('**/routers');
  await page.waitForFunction(()=>document.activeElement.id==='add-router');
  assert.equal(await page.getByLabel('Search',{exact:true}).inputValue(),'Core');
  await page.getByLabel('Search',{exact:true}).fill('');
  await add.click();
  await registration.getByRole('button',{name:'Cancel',exact:true}).click();
  await registration.waitFor({state:'hidden'});
  await page.setViewportSize({width:1440,height:1000});
  await page.getByLabel('Color theme').selectOption('light');
  await add.click();
  await registration.waitFor();
  await page.screenshot({path:`${out}/add-modal-light-1440.png`});
  await registration.getByRole('button',{name:'Close Add router',exact:true}).click();
  await registration.waitFor({state:'hidden'});
  await page.getByLabel('Color theme').selectOption('dark');

  // Add router: step checks, focus on the first invalid field, review, then a disabled router.
  await page.getByRole('button',{name:'Add router',exact:true}).click();
  await page.waitForURL('**/routers/new');
  await page.getByRole('heading',{name:'Add router',level:2}).waitFor();
  const steps=page.getByRole('list',{name:'Steps'});
  assert.match(await steps.locator('[aria-current=step]').innerText(),/Connection/);
  await page.getByRole('button',{name:'Next'}).click();
  await page.getByText('Use 1 to 64 letters, digits, dots, underscores, or hyphens.').waitFor();
  await page.waitForFunction(()=>document.activeElement.id==='name');
  assert.equal(await page.locator('#name').getAttribute('aria-invalid'),'true');
  await page.getByLabel('Router name').fill('Core-02');
  await page.getByLabel('Address').fill('192.0.2.20');
  await page.getByLabel('RouterOS user').fill('api');
  await page.getByLabel('Credential',{exact:true}).fill('fixture-credential');
  assert.equal(await page.locator('.field-error:not(:empty)').count(),0,'fixed fields lose their errors while the operator types');
  assert.equal(await page.getByLabel('API port').inputValue(),'8729');
  await page.locator('.router-form').screenshot({path:`${out}/add-connection.png`});
  await page.getByRole('button',{name:'Next'}).click();
  assert.match(await steps.locator('[aria-current=step]').innerText(),/Sources/);
  await page.locator('#pppoe').check();
  await page.getByRole('button',{name:'Next'}).click();
  const review=page.locator('.router-form');
  assert.match(await review.innerText(),/Core-02[\s\S]*192\.0\.2\.20:8729[\s\S]*Entered \(not shown\)[\s\S]*PPPoE[\s\S]*Disabled after the save/);
  assert.ok(!(await review.innerText()).includes('fixture-credential'),'the review never shows the credential');
  await review.screenshot({path:`${out}/add-review.png`});
  await registration.getByRole('button',{name:'Add router',exact:true}).click();
  await page.waitForURL(/\/routers\/lq3-[0-9a-f]{16}$/);
  const notice=page.locator('.saved-notice');
  await notice.getByText('Router added and disabled.').waitFor();
  await notice.getByText('The service loaded the change.').waitFor({timeout:15000});
  await page.getByRole('heading',{name:'Core-02',level:1}).waitFor();
  assert.match(await page.locator('.page-heading').innerText(),/Disabled/);
  const tabs=page.getByRole('navigation',{name:'Router sections'});
  await tabs.getByRole('link',{name:'Activity'}).click();
  await page.locator('.activity li').first().waitFor();
  assert.match(await page.locator('.activity').innerText(),/Added by browser-test/);

  // Connection edit: only the changed field is sent; then a credential replacement asks for the password.
  const sent=[];page.on('request',r=>{if(r.method()==='PATCH')sent.push(JSON.parse(r.postData()));});
  await tabs.getByRole('link',{name:'Connection'}).click();
  await page.getByRole('button',{name:'Edit connection'}).click();
  await page.getByLabel('API port').fill('8730');
  await page.getByRole('button',{name:'Save changes'}).click();
  await notice.getByText('Changes saved.').waitFor();
  await notice.getByText('The service loaded the change.').waitFor({timeout:15000});
  assert.deepEqual(Object.keys(sent[0].edit).filter(k=>sent[0].edit[k]!==null&&sent[0].edit[k]!==false),['expected_digest','port']);
  await page.locator('.router-facts').getByText('8730',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Edit connection'}).click();
  await page.getByLabel(/^Replace the credential/).check();
  await page.getByLabel('Credential',{exact:true}).fill('replacement');
  await page.getByRole('button',{name:'Save changes'}).click();
  const dialog=page.getByRole('dialog',{name:'Confirm with your password'});
  await dialog.waitFor();
  assert.equal(await page.evaluate(()=>document.activeElement.id),'current-password','focus moves into the dialog');
  await dialog.getByLabel('Your password').fill('not-the-password');
  await dialog.getByRole('button',{name:'Confirm and save'}).click();
  await dialog.getByText(/The password is not correct/).waitFor();
  await page.screenshot({path:`${out}/reauth.png`});
  await dialog.getByLabel('Your password').fill(process.env.BROWSER_TEST_PASSWORD);
  await dialog.getByRole('button',{name:'Confirm and save'}).click();
  await dialog.waitFor({state:'hidden'});
  await notice.getByText('Changes saved.').waitFor();
  assert.ok(!(await page.content()).includes('replacement'),'the credential is not kept in the page');

  // A change after another edit is a conflict; the message says what to do.
  await page.route('**/api/v1/routers/lq3-*',async route=>{
    if(route.request().method()!=='PATCH')return route.continue();
    const body=JSON.parse(route.request().postData());body.edit.expected_digest='ffffffffffffffff';
    await route.continue({postData:JSON.stringify(body)});
  });
  await page.getByRole('button',{name:'Edit connection'}).click();
  await page.getByLabel('API port').fill('8731');
  await page.getByRole('button',{name:'Save changes'}).click();
  await page.getByRole('alert').getByText(/The configuration changed after this page loaded/).waitFor();
  await page.unroute('**/api/v1/routers/lq3-*');
  await page.getByRole('button',{name:'Cancel'}).click();

  // Sources of an enabled router: turning a source off shows the removal warning before the save.
  await page.goto('http://127.0.0.1:19581/routers/lq3-00000000000000a1?tab=sources');
  await page.getByRole('button',{name:'Edit sources'}).click();
  await page.locator('#pppoe').uncheck();
  await page.getByText(/A source that you turn off loses its circuits/).waitFor();
  await page.locator('.router-section').screenshot({path:`${out}/edit-sources.png`});
  await page.getByRole('button',{name:'Cancel'}).click();

  // Disable: the impact first, an acknowledgement for the circuits, the password, then two result tracks.
  await page.goto('http://127.0.0.1:19581/routers/lq3-00000000000000a1');
  await page.getByRole('heading',{name:'Access-01',level:1}).waitFor();
  await page.getByRole('button',{name:'Disable router'}).click();
  const admin=page.getByRole('dialog',{name:'Disable Access-01?'});
  await admin.waitFor();
  assert.match(await admin.innerText(),/3 circuits of this router leave the shaping after the grace time \(10 min\)/);
  const confirm=admin.getByRole('button',{name:'Disable router'});
  assert.ok(await confirm.isDisabled(),'the circuits need an acknowledgement first');
  await admin.getByLabel('I understand that 3 circuits leave the shaping.').check();
  await admin.getByLabel('Your password').fill('not-the-password');
  await confirm.click();
  await admin.getByText(/The password is not correct/).waitFor();
  await page.screenshot({path:`${out}/disable-dialog.png`});
  await admin.getByLabel('Your password').fill(process.env.BROWSER_TEST_PASSWORD);
  await confirm.click();
  await admin.waitFor({state:'hidden'});
  await notice.getByText('Router disabled.').waitFor();
  await notice.getByText('Stopped: LQoSync makes no connection to this router.').waitFor({timeout:15000});
  assert.match(await notice.innerText(),/Shaping removal\s*Pending: 3 circuits leave after the grace time \(10 min\)\./);
  await notice.screenshot({path:`${out}/disable-tracks.png`});
  assert.match(await page.locator('.page-heading').innerText(),/Disabled/);
  // Enable: a plain confirmation without a password.
  await page.getByRole('button',{name:'Enable router'}).click();
  const enable=page.getByRole('dialog',{name:'Enable Access-01?'});
  assert.match(await enable.innerText(),/Old circuits do not come back from a cache/);
  await enable.getByRole('button',{name:'Enable router'}).click();
  await notice.getByText('Router enabled.').waitFor();
  await page.getByRole('button',{name:'Disable router'}).waitFor({timeout:15000});
  await tabs.getByRole('link',{name:'Activity'}).click();
  await page.locator('.activity li').first().waitFor();
  assert.match(await page.locator('.activity').innerText(),/Enabled by browser-test[\s\S]*Disabled by browser-test/);

  // Narrow screens: the form fits without a horizontal page scroll.
  await page.setViewportSize({width:320,height:900});
  await page.goto('http://127.0.0.1:19581/routers/new');
  await page.getByRole('heading',{name:'Add router',level:2}).waitFor();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'the form fits 320px');
  assert.ok(await registration.evaluate(d=>d.scrollWidth<=d.clientWidth),'the modal content fits 320px');
  await page.screenshot({path:`${out}/add-320.png`,fullPage:true});
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: Add router steps with field errors and focus, review without the credential, saved and loaded stages, disabled new router, activity, changed-field-only edits, password dialog for a credential replacement, conflict message, source removal warning, disable review with acknowledgement, password and two result tracks, enable without a password, and 320px layout.');
})().catch(e=>{console.error(e);process.exit(1);});
