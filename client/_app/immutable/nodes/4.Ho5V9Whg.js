import{s as T,f as p,a as x,g as f,h as F,u as b,c as S,d as v,r as H,j as h,i as M,v as g,D as O,E as D,G as C,w as P,x as A,p as G}from"../chunks/scheduler.tnyNm5U0.js";import{S as N,i as R}from"../chunks/index.R_ElRwcJ.js";import{a as k,w as U,e as W}from"../chunks/stores.bABpd-zC.js";import{g as I}from"../chunks/navigation.xQTcNFtT.js";function j(i){let e,n,u="ENIGMATICK",c,a,_=`<form method="dialog" class="svelte-10uww3e"><h3 class="svelte-10uww3e">Authentication Failed</h3> <p class="svelte-10uww3e">Either the information you submitted was incorrect, or there is a problem with the service.
				If you suspect the latter, please try again later.</p> <button class="svelte-10uww3e">Okay</button></form>`,r,s,l=`<label class="svelte-10uww3e">Username
			<input name="username" type="text" placeholder="bob" class="svelte-10uww3e"/></label> <label class="svelte-10uww3e">Password
			<input name="password" type="password" placeholder="Use a password manager" class="svelte-10uww3e"/></label> <button class="svelte-10uww3e">Sign In</button>`,d,t,w=`body {
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
}`,y,E;return{c(){e=p("main"),n=p("h1"),n.textContent=u,c=x(),a=p("dialog"),a.innerHTML=_,r=x(),s=p("form"),s.innerHTML=l,d=x(),t=p("style"),t.textContent=w,this.h()},l(m){e=f(m,"MAIN",{class:!0});var o=F(e);n=f(o,"H1",{class:!0,"data-svelte-h":!0}),b(n)!=="svelte-1bn9c6g"&&(n.textContent=u),c=S(o),a=f(o,"DIALOG",{class:!0,"data-svelte-h":!0}),b(a)!=="svelte-7x3i2c"&&(a.innerHTML=_),r=S(o),s=f(o,"FORM",{id:!0,method:!0,class:!0,"data-svelte-h":!0}),b(s)!=="svelte-1b0v038"&&(s.innerHTML=l),o.forEach(v),d=S(m);const L=H("svelte-skzsmw",document.head);t=f(L,"STYLE",{lang:!0,"data-svelte-h":!0}),b(t)!=="svelte-1m6wti8"&&(t.textContent=w),L.forEach(v),this.h()},h(){h(n,"class","svelte-10uww3e"),h(a,"class","svelte-10uww3e"),h(s,"id","login"),h(s,"method","POST"),h(s,"class","svelte-10uww3e"),h(e,"class","svelte-10uww3e"),h(t,"lang","scss")},m(m,o){M(m,e,o),g(e,n),g(e,c),g(e,a),i[3](a),g(e,r),g(e,s),M(m,d,o),g(document.head,t),y||(E=O(s,"submit",D(i[1])),y=!0)},p:C,i:C,o:C,d(m){m&&(v(e),v(d)),i[3](null),v(t),y=!1,E()}}}function q(i,e,n){let u,c;P(i,W,l=>n(2,c=l));let a=A(k).username;a&&I("/@"+a).then(()=>{console.log("logged in")});function _(l){let d=new FormData(l.target);console.log("clicked"),u?.authenticate(String(d.get("username")),String(d.get("password"))).then(t=>{console.log("PROFILE AVATAR FILENAME"),console.log(t.avatar_filename),t?u?.load_instance_information().then(w=>{k.set({username:String(t?.username),display_name:String(t?.display_name),avatar:String(t?.avatar_filename),domain:w?.domain||null,url:w?.url||null}),a=A(k).username,U.set(String(u?.get_state().export())),I("/@"+a).then(()=>{console.log("logged in")})}):(console.debug("authentication failed"),r.showModal())})}let r;function s(l){G[l?"unshift":"push"](()=>{r=l,n(0,r)})}return i.$$.update=()=>{i.$$.dirty&4&&(u=c)},[r,_,c,s]}class B extends N{constructor(e){super(),R(this,e,q,j,T,{})}}export{B as component};
