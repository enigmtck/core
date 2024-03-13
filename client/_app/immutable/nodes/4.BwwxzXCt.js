import{s as I,f,a as y,g as v,h as H,u as b,c as q,d as x,r as A,j as h,i as L,v as g,D,E as O,G as S,w as F,x as T,p as G}from"../chunks/scheduler.tnyNm5U0.js";import{S as P,i as z}from"../chunks/index.R_ElRwcJ.js";import{a as C,w as N,e as U}from"../chunks/stores.bABpd-zC.js";import{g as E}from"../chunks/navigation.g998M0x9.js";function W(i){let e,n,d="ENIGMATICK",c,t,_=`<form method="dialog" class="svelte-sxe9sq"><h3 class="svelte-sxe9sq">Authentication Failed</h3> <p class="svelte-sxe9sq">Either the information you submitted was incorrect, or there is a problem with the service.
				If you suspect the latter, please try again later.</p> <button class="svelte-sxe9sq">Okay</button></form>`,r,a,l=`<label class="svelte-sxe9sq">Username
			<input name="username" type="text" placeholder="bob" class="svelte-sxe9sq"/></label> <label class="svelte-sxe9sq">Password
			<input name="password" type="password" placeholder="Use a password manager" class="svelte-sxe9sq"/></label> <button class="svelte-sxe9sq">Sign In</button>`,m,s,p=`body {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
  display: block;
  background: white;
}
@media screen and (max-width: 600px) {
  body {
    background: #fafafa;
  }
}`,w,k;return{c(){e=f("main"),n=f("h1"),n.textContent=d,c=y(),t=f("dialog"),t.innerHTML=_,r=y(),a=f("form"),a.innerHTML=l,m=y(),s=f("style"),s.textContent=p,this.h()},l(u){e=v(u,"MAIN",{class:!0});var o=H(e);n=v(o,"H1",{class:!0,"data-svelte-h":!0}),b(n)!=="svelte-1bn9c6g"&&(n.textContent=d),c=q(o),t=v(o,"DIALOG",{class:!0,"data-svelte-h":!0}),b(t)!=="svelte-7x3i2c"&&(t.innerHTML=_),r=q(o),a=v(o,"FORM",{id:!0,method:!0,class:!0,"data-svelte-h":!0}),b(a)!=="svelte-1b0v038"&&(a.innerHTML=l),o.forEach(x),m=q(u);const M=A("svelte-1s0zqhx",document.head);s=v(M,"STYLE",{lang:!0,"data-svelte-h":!0}),b(s)!=="svelte-16pe6kz"&&(s.textContent=p),M.forEach(x),this.h()},h(){h(n,"class","svelte-sxe9sq"),h(t,"class","svelte-sxe9sq"),h(a,"id","login"),h(a,"method","POST"),h(a,"class","svelte-sxe9sq"),h(e,"class","svelte-sxe9sq"),h(s,"lang","scss")},m(u,o){L(u,e,o),g(e,n),g(e,c),g(e,t),i[3](t),g(e,r),g(e,a),L(u,m,o),g(document.head,s),w||(k=D(a,"submit",O(i[1])),w=!0)},p:S,i:S,o:S,d(u){u&&(x(e),x(m)),i[3](null),x(s),w=!1,k()}}}function j(i,e,n){let d,c;F(i,U,l=>n(2,c=l));let t=T(C).username;t&&E("/@"+t).then(()=>{console.log("logged in")});function _(l){let m=new FormData(l.target);console.log("clicked"),d?.authenticate(String(m.get("username")),String(m.get("password"))).then(s=>{s?d?.load_instance_information().then(p=>{C.set({username:String(s?.username),display_name:String(s?.display_name),avatar:String(s?.avatar_filename),domain:p?.domain||null,url:p?.url||null}),t=T(C).username,N.set(String(d?.get_state().export())),E("/@"+t).then(()=>{console.log("logged in")})}):(console.debug("authentication failed"),r.showModal())})}let r;function a(l){G[l?"unshift":"push"](()=>{r=l,n(0,r)})}return i.$$.update=()=>{i.$$.dirty&4&&(d=c)},[r,_,c,a]}class J extends P{constructor(e){super(),z(this,e,j,W,I,{})}}export{J as component};
