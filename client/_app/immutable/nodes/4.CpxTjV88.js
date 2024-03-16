import{s as H,f as p,a as x,g as f,h as A,u as b,c as S,d as v,r as D,j as h,i as T,v as g,D as O,E as F,G as C,w as G,x as E,p as P}from"../chunks/scheduler.tnyNm5U0.js";import{S as N,i as U}from"../chunks/index.R_ElRwcJ.js";import{a as k,w as W,e as j}from"../chunks/stores.bABpd-zC.js";import{g as I}from"../chunks/navigation.ZrV5zl5E.js";function q(i){let e,n,u="ENIGMATICK",d,t,_=`<form method="dialog" class="svelte-10uww3e"><h3 class="svelte-10uww3e">Authentication Failed</h3> <p class="svelte-10uww3e">Either the information you submitted was incorrect, or there is a problem with the service.
				If you suspect the latter, please try again later.</p> <button class="svelte-10uww3e">Okay</button></form>`,r,s,l=`<label class="svelte-10uww3e">Username
			<input name="username" type="text" placeholder="bob" class="svelte-10uww3e"/></label> <label class="svelte-10uww3e">Password
			<input name="password" type="password" placeholder="Use a password manager" class="svelte-10uww3e"/></label> <button class="svelte-10uww3e">Sign In</button>`,c,a,w=`body {
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
}`,y,M;return{c(){e=p("main"),n=p("h1"),n.textContent=u,d=x(),t=p("dialog"),t.innerHTML=_,r=x(),s=p("form"),s.innerHTML=l,c=x(),a=p("style"),a.textContent=w,this.h()},l(m){e=f(m,"MAIN",{class:!0});var o=A(e);n=f(o,"H1",{class:!0,"data-svelte-h":!0}),b(n)!=="svelte-1bn9c6g"&&(n.textContent=u),d=S(o),t=f(o,"DIALOG",{class:!0,"data-svelte-h":!0}),b(t)!=="svelte-7x3i2c"&&(t.innerHTML=_),r=S(o),s=f(o,"FORM",{id:!0,method:!0,class:!0,"data-svelte-h":!0}),b(s)!=="svelte-1b0v038"&&(s.innerHTML=l),o.forEach(v),c=S(m);const L=D("svelte-skzsmw",document.head);a=f(L,"STYLE",{lang:!0,"data-svelte-h":!0}),b(a)!=="svelte-1m6wti8"&&(a.textContent=w),L.forEach(v),this.h()},h(){h(n,"class","svelte-10uww3e"),h(t,"class","svelte-10uww3e"),h(s,"id","login"),h(s,"method","POST"),h(s,"class","svelte-10uww3e"),h(e,"class","svelte-10uww3e"),h(a,"lang","scss")},m(m,o){T(m,e,o),g(e,n),g(e,d),g(e,t),i[3](t),g(e,r),g(e,s),T(m,c,o),g(document.head,a),y||(M=O(s,"submit",F(i[1])),y=!0)},p:C,i:C,o:C,d(m){m&&(v(e),v(c)),i[3](null),v(a),y=!1,M()}}}function z(i,e,n){let u,d;G(i,j,l=>n(2,d=l));let t=E(k).username;t&&I("/@"+t).then(()=>{console.log("logged in")});function _(l){let c=new FormData(l.target);console.log("clicked"),u?.authenticate(String(c.get("username")),String(c.get("password"))).then(a=>{a?u?.load_instance_information().then(w=>{k.set({username:String(a?.username),display_name:String(a?.display_name),avatar:String(a?.avatar_filename),domain:w?.domain||null,url:w?.url||null}),t=E(k).username,W.set(String(u?.get_state().export())),I("/@"+t).then(()=>{console.log("logged in")})}):(console.debug("authentication failed"),r.showModal())})}let r;function s(l){P[l?"unshift":"push"](()=>{r=l,n(0,r)})}return i.$$.update=()=>{i.$$.dirty&4&&(u=d)},[r,_,d,s]}class J extends N{constructor(e){super(),U(this,e,z,q,H,{})}}export{J as component};
