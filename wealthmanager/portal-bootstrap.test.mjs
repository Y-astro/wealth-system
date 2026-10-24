import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const source = fs.readFileSync(new URL('./portal-bootstrap.js', import.meta.url), 'utf8');
function client() {
  const calls = [], redirects = [], events = {}, documentEvents = {}, buttons = [], beacons = [];
  let now = 1000, status = 200, interval;
  const location = {href:'https://home:8444/wealth/app/statements', origin:'https://home:8444', replace:url=>redirects.push(url)};
  const document = {
    documentElement:{style:{}},
    addEventListener:(name,callback)=>documentEvents[name]=callback,
    createElement:()=>({style:{},setAttribute(){},addEventListener(name,callback){this[name]=callback;}}),
    body:{append:button=>buttons.push(button)},
  };
  const window = {
    fetch:async(input,init)=>{calls.push([input,init]);return {status};},
    EventSource:class {constructor(url,options){this.url=String(url);this.options=options;}},
    addEventListener:(name,callback)=>events[name]=callback,
  };
  vm.runInNewContext(source,{window,document,location,URL,Request,
    navigator:{sendBeacon:(url,body)=>beacons.push([url,body])},
    Date:{now:()=>now},setInterval:callback=>interval=callback});
  return {window,document,calls,redirects,events,documentEvents,buttons,beacons,
    setStatus:value=>status=value, advance:value=>now+=value, tick:()=>interval()};
}
{
  const c=client();
  await c.window.fetch('/api/v1/statements/library',{credentials:'same-origin'});
  assert.equal(String(c.calls[0][0]),'https://home:8444/wealth/api/v1/statements/library');
  assert.equal(c.calls[0][1].credentials,'same-origin');
  await c.window.fetch(new Request('https://home:8444/api/v1/settings',{headers:{'X-Test':'preserved'}}));
  assert.equal(c.calls[1][0].url,'https://home:8444/wealth/api/v1/settings');
  assert.equal(c.calls[1][0].headers.get('X-Test'),'preserved');
  await c.window.fetch('https://other/api/v1/settings');
  assert.equal(c.calls[2][0],'https://other/api/v1/settings');
  assert.equal(new c.window.EventSource('/api/v1/events',{withCredentials:true}).url,'https://home:8444/wealth/api/v1/events');
}
{
  const c=client(); c.setStatus(401);
  await c.window.fetch('/api/v1/statements/library');
  assert.deepEqual(c.redirects,['/wealth/']);
}
{
  const c=client(); c.documentEvents.DOMContentLoaded();
  assert.equal(c.buttons[0].textContent,'Lock Wealthfolio');
  await c.buttons[0].click();
  assert.equal(c.document.documentElement.style.visibility,'hidden');
  assert.equal(c.calls[0][0],'/wealth/logout');
  assert.equal(c.calls[0][1].method,'POST');
  assert.deepEqual(c.redirects,['/wealth/']);
}
{
  const c=client(); c.documentEvents.DOMContentLoaded();
  c.advance(15*60*1000); c.tick(); await new Promise(setImmediate);
  assert.equal(c.document.documentElement.style.visibility,'hidden');
  assert.deepEqual(c.redirects,['/wealth/']);
}
{
  const c=client(); c.documentEvents.DOMContentLoaded();
  c.events.pageshow({persisted:true}); await new Promise(setImmediate);
  assert.deepEqual(c.redirects,['/wealth/']);
  c.events.pagehide();
  assert.equal(c.beacons[0][0],'/wealth/logout');
}
console.log('PASS portal API/event routing, cookie options, session rejection, manual lock, inactivity and back-cache lock');
