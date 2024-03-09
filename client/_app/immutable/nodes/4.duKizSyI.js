import{s as I,f,a as w,g as v,h as H,u as y,c as x,d as _,r as A,j as h,i as L,v as g,D,E as O,G as S,w as P,x as T,p as F}from"../chunks/scheduler.tnyNm5U0.js";import{S as G,i as j}from"../chunks/index.R_ElRwcJ.js";import{a as C,w as z,e as N}from"../chunks/stores.bABpd-zC.js";import{g as E}from"../chunks/navigation._3dZ04L8.js";function W(r){let e,l,m="ENIGMATICK",c,t,b=`<form method="dialog" class="svelte-1mrqlli"><h3 class="svelte-1mrqlli">Authentication Failed</h3> <p class="svelte-1mrqlli">Either the information you submitted was incorrect, or there is a problem with the service.
				If you suspect the latter, please try again later.</p> <button class="svelte-1mrqlli">Okay</button></form>`,o,s,n=`<label class="svelte-1mrqlli">Username
			<input name="username" type="text" placeholder="bob" class="svelte-1mrqlli"/></label> <label class="svelte-1mrqlli">Password
			<input name="password" type="password" placeholder="Provides access to the server" class="svelte-1mrqlli"/></label> <button class="svelte-1mrqlli">Sign In</button>`,d,a,p=`body {
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
}`,q,k;return{c(){e=f("main"),l=f("h1"),l.textContent=m,c=w(),t=f("dialog"),t.innerHTML=b,o=w(),s=f("form"),s.innerHTML=n,d=w(),a=f("style"),a.textContent=p,this.h()},l(u){e=v(u,"MAIN",{class:!0});var i=H(e);l=v(i,"H1",{class:!0,"data-svelte-h":!0}),y(l)!=="svelte-1bn9c6g"&&(l.textContent=m),c=x(i),t=v(i,"DIALOG",{class:!0,"data-svelte-h":!0}),y(t)!=="svelte-7x3i2c"&&(t.innerHTML=b),o=x(i),s=v(i,"FORM",{id:!0,method:!0,class:!0,"data-svelte-h":!0}),y(s)!=="svelte-sjfllb"&&(s.innerHTML=n),i.forEach(_),d=x(u);const M=A("svelte-1s0zqhx",document.head);a=v(M,"STYLE",{lang:!0,"data-svelte-h":!0}),y(a)!=="svelte-16pe6kz"&&(a.textContent=p),M.forEach(_),this.h()},h(){h(l,"class","svelte-1mrqlli"),h(t,"class","svelte-1mrqlli"),h(s,"id","login"),h(s,"method","POST"),h(s,"class","svelte-1mrqlli"),h(e,"class","svelte-1mrqlli"),h(a,"lang","scss")},m(u,i){L(u,e,i),g(e,l),g(e,c),g(e,t),r[3](t),g(e,o),g(e,s),L(u,d,i),g(document.head,a),q||(k=D(s,"submit",O(r[1])),q=!0)},p:S,i:S,o:S,d(u){u&&(_(e),_(d)),r[3](null),_(a),q=!1,k()}}}function K(r,e,l){let m,c;P(r,N,n=>l(2,c=n));let t=T(C).username;t&&E("/@"+t).then(()=>{console.log("logged in")});function b(n){let d=new FormData(n.target);console.log("clicked"),m?.authenticate(String(d.get("username")),String(d.get("password"))).then(a=>{a?m?.load_instance_information().then(p=>{C.set({username:String(a?.username),display_name:String(a?.display_name),avatar:String(a?.avatar_filename),domain:p?.domain||null,url:p?.url||null}),t=T(C).username,z.set(String(m?.get_state().export())),E("/@"+t).then(()=>{console.log("logged in")})}):(console.debug("authentication failed"),o.showModal())})}let o;function s(n){F[n?"unshift":"push"](()=>{o=n,l(0,o)})}return r.$$.update=()=>{r.$$.dirty&4&&(m=c)},[o,b,c,s]}class J extends G{constructor(e){super(),j(this,e,K,W,I,{})}}export{J as component};
