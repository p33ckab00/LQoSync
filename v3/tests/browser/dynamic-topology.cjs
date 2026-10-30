'use strict';
// Run only against the synthetic preview fixture described in FLEXIBLE_TOPOLOGY.md.
const {chromium}=require(process.env.LQOSYNC_PLAYWRIGHT || 'playwright');
const assert=require('node:assert/strict');
const origin='http://127.0.0.1:19304';
(async()=>{
 const browser=await chromium.launch({headless:true});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  async function open(title){await page.getByRole('button',{name:`Configure ${title}`,exact:true}).click();const d=page.locator('dialog[open]');await d.waitFor();return d;}
  async function submit(d,text){await d.getByRole('button',{name:text,exact:true}).click();await d.waitFor({state:'hidden'});}
  await page.goto(origin+'/topology?selected=branch');
  assert(await page.locator('form[action^="/dynamic/"]').evaluateAll(forms=>forms.every(form=>form.closest('dialog'))),'all dynamic actions must become dialogs');
  let d=await open('Promote Branch');await submit(d,'Promote node');
  await page.goto(origin+'/topology?selected=branch');
  d=await open('Move / demote Branch');assert.equal(await d.locator('select[name=parent]').inputValue(),'');
  await d.locator('select[name=parent]').selectOption('root');await submit(d,'Move / demote node');
  await page.goto(origin+'/topology?selected=branch');
  d=await open('Remove Branch');assert((await d.textContent()).includes('committed: unavailable'));
  await d.locator('select[name=replacement]').selectOption('root');await submit(d,'Remove node with selected replacements');
  await page.goto(origin+'/assignments');
  assert(!await page.locator('main').textContent().then(t=>t.includes('Branch · WAN A')));
  d=await open('Assignment mode and default');await d.locator('input[name=name]').fill('Shared clients');
  await d.locator('select[name=balance]').selectOption('true');await d.locator('select[name=metric]').selectOption('online_clients');
  await submit(d,'Save assignment policy');
  assert(await page.getByRole('heading',{name:'Shared clients',exact:true}).isVisible());
  await page.goto(origin+'/topology');await page.locator('a[href="#add-node"]').first().click();d=page.locator('dialog[open]');
  await d.locator('input[name=id]').fill('extra');await d.locator('input[name=name]').fill('Extra root');
  await d.locator('input[name=download]').fill('100');await d.locator('input[name=upload]').fill('100');await submit(d,'Save node');
  await page.goto(origin+'/topology?selected=extra');d=await open('Move / demote Extra root');assert.equal(await d.locator('select[name=parent]').inputValue(),'');await page.keyboard.press('Escape');
  d=await open('Remove Extra root');await submit(d,'Remove node with selected replacements');
  assert.deepEqual(errors,[]);
  console.log('PASS: synthetic promote/demote/remove with replacement, explicit impact labels, policy edit and independent root creation/removal.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e.stack);process.exit(1)});
