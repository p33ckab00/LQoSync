const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'/tmp/lqosync-web-tools/node_modules/playwright');
const assert=require('node:assert/strict');
const path=require('node:path');
const {pathToFileURL}=require('node:url');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'/home/pi/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome',args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:960}});
  page.setDefaultTimeout(5000);
  const errors=[],network=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('request',r=>{if(/^https?:/.test(r.url()))network.push(r.url())});
  await page.goto(pathToFileURL(path.join(__dirname,'mockup.html')).href);
  await page.screenshot({path:path.join(__dirname,'canvas-dark.png'),fullPage:true});
  const settings=page.getByRole('button',{name:'Topology settings',exact:true});
  await settings.focus();await page.keyboard.press('Enter');
  let dialog=page.getByRole('dialog',{name:'Topology settings',exact:true});await dialog.waitFor();
  await dialog.getByLabel('Show capacities').uncheck();
  assert.equal(await dialog.getByRole('status').innerText(),'Local example updated');
  await page.screenshot({path:path.join(__dirname,'settings-dark.png'),fullPage:true});
  await page.keyboard.press('Escape');
  assert.equal(await settings.evaluate(el=>document.activeElement===el),true);
  await page.getByRole('button',{name:'Configure Site A',exact:true}).click();
  dialog=page.getByRole('dialog',{name:'Site A',exact:true});
  assert.equal(await dialog.getByLabel('Latitude',{exact:true}).inputValue(),'14.599512345678901');
  await dialog.getByLabel('Longitude',{exact:true}).fill('120.9842123456789012345');
  assert.equal(await dialog.getByLabel('Longitude',{exact:true}).inputValue(),'120.9842123456789012345');
  await page.screenshot({path:path.join(__dirname,'node-location.png'),fullPage:true});
  await page.keyboard.press('Escape');
  await page.getByLabel('Theme',{exact:true}).selectOption('light');
  await page.screenshot({path:path.join(__dirname,'canvas-light.png'),fullPage:true});
  for(const name of ['Router inventory','Subscriber allocation','Optional topology preview']){
   await page.getByRole('button',{name,exact:true}).click();
   assert.equal(await page.locator('dialog[open]').count(),1);
   assert.equal(await page.getByRole('button',{name:/^(Save|Approve|Publish|Verify)$/}).count(),0);
   await page.keyboard.press('Escape');
  }
  for(const width of [375,768]){
   await page.setViewportSize({width,height:812});await settings.click();
   dialog=page.getByRole('dialog',{name:'Topology settings',exact:true});
   await page.screenshot({path:path.join(__dirname,`settings-${width}.png`),fullPage:true});
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
   assert.equal(await dialog.evaluate(el=>el.scrollWidth<=el.clientWidth),true);
   await page.keyboard.press('Escape');
  }
  assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
  console.log('PASS: keyboard modal access, focus return, local feedback, long decimal input, optional preview, themes, 375/768px overflow, and no network requests.');
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
