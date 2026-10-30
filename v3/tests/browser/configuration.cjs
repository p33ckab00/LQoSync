const {chromium}=require(process.env.LQOSYNC_PLAYWRIGHT || 'playwright');
const assert=require('node:assert/strict');
const origin = 'http://127.0.0.1:19304'; // Synthetic preview only. Never point this script at live data.
(async()=>{
 const browser=await chromium.launch({headless:true});
 try {
 const page=await browser.newPage({viewport:{width:1440,height:900}});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const routes=['/','/routers','/topology','/wans','/sources','/assignments','/subscribers','/speed-rules','/core-lists','/core-review','/validation','/settings'];
 for(const width of [1440,390,320]){
  await page.setViewportSize({width,height:900});
  for(const route of routes){const response=await page.goto(origin+route);assert.equal(response.status(),200,route);assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`overflow ${width} ${route}`);}
 }
 await page.setViewportSize({width:1440,height:900});await page.goto(origin+'/topology');
 const nodeCog=page.locator('.configcog').filter({hasText:'⚙'});
 assert(await nodeCog.count()>0);
 await page.locator('a[href="#add-node"]').first().click();
 let dialog=page.locator('dialog[open]');await dialog.waitFor();
 assert.equal(await page.locator('dialog[open]').count(),1);
 assert.equal(await page.evaluate(()=>document.body.style.overflow),'hidden');
 assert(await dialog.evaluate(d=>{const b=d.querySelector('header button').getBoundingClientRect();return document.elementFromPoint(b.x+b.width/2,b.y+b.height/2).closest('dialog')===d}));
 await dialog.locator('input[name="name"]').fill('Unsaved draft');
 await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden'});
 await page.waitForFunction(()=>document.activeElement.classList.contains('configcog'));
 await page.locator('a[href="#add-node"]').first().click();dialog=page.locator('dialog[open]');
 assert.equal(await dialog.locator('input[name="name"]').inputValue(),'Unsaved draft');
 for(let i=0;i<25;i++){await page.keyboard.press('Tab');assert(await page.evaluate(()=>!!document.activeElement.closest('dialog[open]')),'focus escaped');}
 await page.keyboard.press('Escape');
 await page.goto(origin+'/settings');
 const cog=page.locator('.configcog').first();await cog.click();dialog=page.locator('dialog[open]');
 const interval=dialog.locator('input[name="interval_seconds"]');const original=await interval.inputValue();
 await interval.fill('0');await dialog.locator('footer button[type="submit"], footer button:not([type])').last().click();
 await dialog.locator('[role="alert"]').waitFor({state:'visible'});assert.equal(await interval.inputValue(),'0');assert(await dialog.isVisible());
 assert.equal(await interval.getAttribute('aria-invalid'),'true');
 const errorId=await interval.getAttribute('aria-describedby');assert(await dialog.locator('#'+errorId).isVisible());
 const token=dialog.locator('input[name=token]');const savedToken=await token.inputValue();
 await token.evaluate(input=>input.value='invalid');await interval.fill(original);
 await dialog.locator('footer button[type="submit"], footer button:not([type])').last().click();
 await page.waitForFunction(()=>document.querySelector('dialog[open] [role=alert]')?.textContent.includes('Invalid form token'));
 await token.evaluate((input,value)=>input.value=value,savedToken);
 await interval.fill(original);await dialog.locator('footer button[type="submit"], footer button:not([type])').last().click();await page.waitForURL('**/settings');await page.locator('dialog[open]').waitFor({state:'hidden'});
 for(const width of [720,390,320]){
  await page.setViewportSize({width,height:640});await page.goto(origin+'/settings');
  await page.locator('#menuBtn').click();
  await page.locator('.configcog').first().evaluate(button=>button.click());dialog=page.locator('dialog[open]');await dialog.waitFor();
  assert(!await page.locator('#sidebar').evaluate(s=>s.classList.contains('open')));
  const bounds=await dialog.boundingBox();assert(bounds.x>=0&&bounds.y>=0&&bounds.x+bounds.width<=width+1&&bounds.y+bounds.height<=641);
  assert(await dialog.evaluate(d=>{const b=d.querySelector('header button').getBoundingClientRect();return document.elementFromPoint(b.x+b.width/2,b.y+b.height/2).closest('dialog')===d}));
  await page.keyboard.press('Escape');
 }
 assert.deepEqual(errors,[]);console.log('PASS: 12 routes x 3 widths; cog dialogs, top layer, focus/Tab/Escape, drafts, invalid/save/retry, field associations, rejected token, 720px zoom-equivalent/mobile menu and bounds.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e.stack);process.exit(1)});
