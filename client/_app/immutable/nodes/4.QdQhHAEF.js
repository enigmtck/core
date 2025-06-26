import{s as H,f,a as S,g as v,h as A,u as x,c as C,d as _,r as D,j as h,i as M,v as g,C as F,D as N,F as k,w as O,N as T,p as P}from"../chunks/scheduler.vJi1FF2t.js";import{S as R,i as U}from"../chunks/index.NkgvvCTc.js";import{a as L,e as G}from"../chunks/stores.Q3fukup_.js";import{g as I}from"../chunks/navigation.9JcVx6SN.js";function W(o){let e,n,r="ENIGMATICK",d,t,w=`<form method="dialog" class="svelte-1t5100b"><h3 class="svelte-1t5100b">Authentication Failed</h3> <p class="svelte-1t5100b">Either the information you submitted was incorrect, or there is a problem with the service.
				If you suspect the latter, please try again later.</p> <button class="svelte-1t5100b">Okay</button></form>`,c,s,l=`<label class="svelte-1t5100b">Username
			<input name="username" type="text" placeholder="bob" class="svelte-1t5100b"/></label> <label class="svelte-1t5100b">Password
			<input name="password" type="password" placeholder="Use a password manager" class="svelte-1t5100b"/></label> <button class="svelte-1t5100b">Sign In</button>`,m,a,b=`body {
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
}`,p,y;return{c(){e=f("main"),n=f("h1"),n.textContent=r,d=S(),t=f("dialog"),t.innerHTML=w,c=S(),s=f("form"),s.innerHTML=l,m=S(),a=f("style"),a.textContent=b,this.h()},l(u){e=v(u,"MAIN",{class:!0});var i=A(e);n=v(i,"H1",{class:!0,"data-svelte-h":!0}),x(n)!=="svelte-1bn9c6g"&&(n.textContent=r),d=C(i),t=v(i,"DIALOG",{class:!0,"data-svelte-h":!0}),x(t)!=="svelte-7x3i2c"&&(t.innerHTML=w),c=C(i),s=v(i,"FORM",{id:!0,method:!0,class:!0,"data-svelte-h":!0}),x(s)!=="svelte-1b0v038"&&(s.innerHTML=l),i.forEach(_),m=C(u);const E=D("svelte-skzsmw",document.head);a=v(E,"STYLE",{lang:!0,"data-svelte-h":!0}),x(a)!=="svelte-1m6wti8"&&(a.textContent=b),E.forEach(_),this.h()},h(){h(n,"class","svelte-1t5100b"),h(t,"class","svelte-1t5100b"),h(s,"id","login"),h(s,"method","POST"),h(s,"class","svelte-1t5100b"),h(e,"class","svelte-1t5100b"),h(a,"lang","scss")},m(u,i){M(u,e,i),g(e,n),g(e,d),g(e,t),o[3](t),g(e,c),g(e,s),M(u,m,i),g(document.head,a),p||(y=F(s,"submit",N(o[1])),p=!0)},p:k,i:k,o:k,d(u){u&&(_(e),_(m)),o[3](null),_(a),p=!1,y()}}}function j(o,e,n){let r,d;O(o,G,l=>n(2,d=l));let t=T(L).username;t&&I("/@"+t).then(()=>{console.log("logged in")});async function w(l){let m=new FormData(l.target);console.log("clicked");let a=await r?.authenticate(String(m.get("username")),String(m.get("password")));if(a){let b=await r?.load_instance_information();L.set({username:String(a?.username),display_name:String(a?.display_name),avatar:String(a?.avatar_filename),domain:b?.domain||null,url:b?.url||null}),t=T(L).username;let p=r?.get_state();if(console.debug(p),p){let y=await r?.replenish_mkp();console.debug(`REPLENISH RESULT: ${y}`)}I("/@"+t).then(()=>{console.log("logged in")})}else console.debug("authentication failed"),c.showModal()}let c;function s(l){P[l?"unshift":"push"](()=>{c=l,n(0,c)})}return o.$$.update=()=>{o.$$.dirty&4&&(r=d)},[c,w,d,s]}class B extends R{constructor(e){super(),U(this,e,j,W,H,{})}}export{B as component};
