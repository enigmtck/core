import"../chunks/CWj6FrbW.js";import"../chunks/CGsJVIYN.js";import{p as N,i as A,o as D,l as S,j as B,k,m as i,n as h,a as E,aC as t,q as o,r as I,t as P,u as r,aF as F,v as M}from"../chunks/C-PNKZ_s.js";import{h as T}from"../chunks/EyKtY1Wo.js";import{p as d}from"../chunks/CWmzcjye.js";import{i as W}from"../chunks/DWt-1M3a.js";import{s as O,a as U}from"../chunks/ChSDbmkr.js";import{e as V,a as j}from"../chunks/n0qzx-_D.js";import{g as b}from"../chunks/CjNKWMH0.js";var G=k(`<style lang="scss">body {
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
}</style>`),K=k('<main class="svelte-kmqcod"><h1 class="svelte-kmqcod">ENIGMATICK</h1> <h2 class="svelte-kmqcod">Create Account</h2> <form id="signup" method="POST" class="svelte-kmqcod"><label class="svelte-kmqcod">Username <input name="username" type="text" placeholder="bob" class="svelte-kmqcod"/></label> <label class="svelte-kmqcod">Display Name <input name="display_name" type="text" placeholder="Bob Anderson" class="svelte-kmqcod"/></label> <label class="svelte-kmqcod">Password <input name="password" type="password" required minlength="5" placeholder="Use a password manager" class="svelte-kmqcod"/></label> <label class="svelte-kmqcod">Confirm Password <input name="confirm_password" type="password" required minlength="5" placeholder="Confirm your password" class="svelte-kmqcod"/></label> <div class="svelte-kmqcod"><button class="svelte-kmqcod">Create Account</button></div></form></main>');function ae(y,w){N(w,!1);const p=()=>U(V,"$enigmatickWasm",_),[_,q]=O(),u=P();let g=A(j).username;D(()=>{g&&b(`/@${g}`)});function l(){const a=document.getElementsByName("password")[0],s=document.getElementsByName("confirm_password")[0];return a.value==s.value}async function $(a){var v;const s=document.getElementsByTagName("button")[0];if(s.disabled=!0,a.target.checkValidity&&l()){let e=new FormData(a.target);console.log(e),e.get("username")&&e.get("display_name")&&e.get("password")&&((v=M(u))==null||v.create_user(String(e.get("username")),String(e.get("display_name")),String(e.get("password"))).then(L=>{b("/login")}))}else console.error("FORM INVALID")}S(()=>p(),()=>{I(u,p())}),B(),W();var n=K();T("kmqcod",a=>{var s=G();h(a,s)});var m=t(o(n),4),c=t(o(m),4),x=t(o(c));r(c);var f=t(c,2),C=t(o(f));r(f),F(2),r(m),r(n),i("change",x,d(l)),i("change",C,d(l)),i("submit",m,d($)),h(y,n),E(),q()}export{ae as component};
