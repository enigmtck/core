import"../chunks/CWj6FrbW.js";import"../chunks/BfK2I2Fg.js";import{p as P,f,l as T,i as U,j as z,k as W,m as b,n as A,r as v,t as _,ad as y,q as F,v as t,u as M}from"../chunks/iAwc3qQu.js";import{h as N}from"../chunks/fnhVM35-.js";import{b as O}from"../chunks/CsyQ7Zbw.js";import{p as R}from"../chunks/CWmzcjye.js";import{i as j}from"../chunks/CmZ756AE.js";import{s as q,a as C}from"../chunks/CTRaWkmf.js";import{a as m,e as G}from"../chunks/BjbtEI9d.js";import{g as w}from"../chunks/BZzrGudY.js";var H=z(`<style lang="scss">body {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
  display: block;
  background: white;
}
@media screen and (max-width: 700px) {
  body {
    background: #fafafa;
  }
}</style>`),K=z(`<main class="svelte-1x05zx6"><h1 class="svelte-1x05zx6">ENIGMATICK</h1> <dialog class="svelte-1x05zx6"><form method="dialog" class="svelte-1x05zx6"><h3 class="svelte-1x05zx6">Authentication Failed</h3> <p class="svelte-1x05zx6">Either the information you submitted was incorrect, or there is a problem with the service.
				If you suspect the latter, please try again later.</p> <button class="svelte-1x05zx6">Okay</button></form></dialog> <form id="login" method="POST" class="svelte-1x05zx6"><label class="svelte-1x05zx6">Username <input name="username" type="text" placeholder="bob" class="svelte-1x05zx6"/></label> <label class="svelte-1x05zx6">Password <input name="password" type="password" placeholder="Use a password manager" class="svelte-1x05zx6"/></label> <button class="svelte-1x05zx6">Sign In</button></form></main>`);function se(k,S){P(S,!1);const c=()=>C(G,"$enigmatickWasm",$),[$,E]=q(),s=_();let l=f(m).username;l&&w("/@"+l).then(()=>{console.log("logged in")});async function I(o){var g,p,u,h;let n=new FormData(o.target);console.log("clicked");let e=await((g=t(s))==null?void 0:g.authenticate(String(n.get("username")),String(n.get("password"))));if(e){let a=await((p=t(s))==null?void 0:p.load_instance_information());m.set({username:String(e==null?void 0:e.username),display_name:String(e==null?void 0:e.display_name),avatar:String(e==null?void 0:e.avatar_filename),domain:(a==null?void 0:a.domain)||null,url:(a==null?void 0:a.url)||null}),l=f(m).username;let x=(u=t(s))==null?void 0:u.get_state();if(console.debug(x),x){let L=await((h=t(s))==null?void 0:h.replenish_mkp());console.debug(`REPLENISH RESULT: ${L}`)}w("/@"+l).then(()=>{console.log("logged in")})}else console.debug("authentication failed"),t(r).showModal()}let r=_();T(()=>c(),()=>{v(s,c())}),U(),j();var i=K();N("1x05zx6",o=>{var n=H();b(o,n)});var d=y(F(i),2);O(d,o=>v(r,o),()=>t(r));var D=y(d,2);M(i),W("submit",D,R(I)),b(k,i),A(),E()}export{se as component};
