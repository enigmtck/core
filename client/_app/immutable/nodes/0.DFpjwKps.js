import{_ as He}from"../chunks/C1FmrZbK.js";import"../chunks/CWj6FrbW.js";import{aG as oe,p as ie,k,q as v,u as d,ay as z,v as e,aA as ne,n as m,aD as U,aC as w,m as we,a as de,aB as Q,aw as Z,r as q,J as Te,o as ke,ax as Ae,az as Re,y as Fe,aH as le,aF as Be,aV as ye}from"../chunks/C-PNKZ_s.js";import{i as R}from"../chunks/CPpPu6cz.js";import{h as Ue}from"../chunks/EyKtY1Wo.js";import{s as X,h as _e}from"../chunks/CT8Pszs0.js";import{s as Pe}from"../chunks/BKz2wWfE.js";import{s as ue,a as Y}from"../chunks/ChSDbmkr.js";import{a as Se,l as re,e as he,h as We,b as Me,s as Ce}from"../chunks/n0qzx-_D.js";import{p as Ne}from"../chunks/D7gh-3I6.js";import"../chunks/DBPuqUlq.js";import{s as $}from"../chunks/CGqlG8Jg.js";import{e as Ie,i as De}from"../chunks/gGHH_MYY.js";import{b as me}from"../chunks/C6Vr0pnO.js";import{S as Ve}from"../chunks/D0c0nNae.js";const je=!1,bt=Object.freeze(Object.defineProperty({__proto__:null,ssr:je},Symbol.toStringTag,{value:"Module"}));var ze=k('<a href="#filters" title="Delete" class="svelte-1avybgx"> </a>'),Ge=k('<div class="filters svelte-1avybgx"><div class="svelte-1avybgx"></div> <form class="svelte-1avybgx"><input type="text" name="statement" placeholder="Hashtag to filter by" class="svelte-1avybgx"/></form></div>');function be(P,r){ie(r,!0);let S,W=U(()=>{const i=r.hashtags;return i.size,Array.from(i)});const I=i=>{var f;i.preventDefault();const l=new FormData(S);let s;if(s=(f=l.get("statement"))==null?void 0:f.toString().trim()){s=s.startsWith("#")?s.slice(1):s;const _=s.toString().toLowerCase();r.onHashtagAdd?r.onHashtagAdd(_):r.hashtags.add(_),r.resetData()}S.reset()},b=i=>{i.preventDefault();let l=i.currentTarget||i.target,s=l==null?void 0:l.dataset.tag;s&&(r.onHashtagRemove?r.onHashtagRemove(s):r.hashtags.delete(s),r.resetData())};var u=Ge(),y=v(u);Ie(y,21,()=>e(W),De,(i,l)=>{var s=ze();s.__click=b;var f=v(s);d(s),z(()=>{X(s,"data-tag",e(l)),ne(f,`#${e(l)??""}`)}),m(i,s)}),d(y);var o=w(y,2);me(o,i=>S=i,()=>S),d(u),we("submit",o,I),m(P,u),de()}oe(["click"]);var Je=k('<div class="filters-drawer svelte-vd7ohq"><div><div><!></div>  <div role="button" tabindex="0"><i class="fa-solid fa-caret-down svelte-vd7ohq"></i></div></div></div> <div class="filters-container svelte-vd7ohq"><h1 class="svelte-vd7ohq">Hashtags</h1> <!></div>',1);function Ke(P,r){ie(r,!0);const S=()=>Y(r.hashtags,"$hashtagsStore",W),[W,I]=ue();let b=U(S);const u=g=>{r.onHashtagAdd?r.onHashtagAdd(g):r.hashtags.update(x=>{const O=new Set(x);return O.add(g),O})},y=g=>{r.onHashtagRemove?r.onHashtagRemove(g):r.hashtags.update(x=>{const O=new Set(x);return O.delete(g),O})};let o=Z(!1);function i(){q(o,!e(o))}var l=Je(),s=Q(l),f=v(s);let _;var L=v(f);let H;var G=v(L);be(G,{get hashtags(){return e(b)},get resetData(){return r.resetData},onHashtagAdd:u,onHashtagRemove:y}),d(L);var M=w(L,2);let C;M.__click=i,M.__keydown=g=>{(g.key==="Enter"||g.key===" ")&&(g.preventDefault(),i())},d(f),d(s);var T=w(s,2),F=w(v(T),2);be(F,{get hashtags(){return e(b)},get resetData(){return r.resetData},onHashtagAdd:u,onHashtagRemove:y}),d(T),z(()=>{_=$(f,1,"filters-drawer__content svelte-vd7ohq",null,_,{open:e(o)}),H=$(L,1,"filters-container-wrapper svelte-vd7ohq",null,H,{open:e(o)}),C=$(M,1,"filters-drawer__pull svelte-vd7ohq",null,C,{open:e(o)})}),m(P,l),de(),I()}oe(["click","keydown"]);var Qe=k("<span> </span>"),Xe=k(`<div class="overlay svelte-8ukb9p" role="dialog" aria-modal="true" aria-labelledby="login-title"><div class="modal svelte-8ukb9p"><h1 id="login-title"></h1> <dialog class="svelte-8ukb9p"><form method="dialog" class="svelte-8ukb9p"><h3 class="svelte-8ukb9p">Authentication Failed</h3> <p class="svelte-8ukb9p">Either the information you submitted was incorrect, or there is a problem with the service.
					If you suspect the latter, please try again later.</p> <button class="svelte-8ukb9p">Okay</button></form></dialog> <form id="login" method="POST" class="svelte-8ukb9p"><label class="svelte-8ukb9p">Username <input name="username" type="text" placeholder="bob" autocomplete="username" class="svelte-8ukb9p"/></label> <label class="svelte-8ukb9p">Password <input name="password" type="password" placeholder="Use a password manager" autocomplete="current-password" class="svelte-8ukb9p"/></label> <button type="submit" class="svelte-8ukb9p">Sign In</button></form></div></div>`);function Ye(P,r){ie(r,!0);const S=()=>Y(he,"$enigmatickWasm",W),[W,I]=ue();let b=U(S),u,y;const o="ENIGMATICK";let i=Z(Te(Array(10).fill(0).map(()=>"?").join(""))),l=null,s=null,f=Z(0),_=Z(!1);const L="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";function H(){return L[Math.floor(Math.random()*L.length)]}function G(){l&&clearInterval(l),s&&clearInterval(s),q(i,Array(10).fill(0).map(()=>H()).join(""),!0),q(f,0),q(_,!1),l=setInterval(()=>{if(e(f)<o.length){let t=e(i).split("");for(let c=e(f);c<o.length;c++)t[c]=H();q(i,t.join(""),!0)}},50),s=setInterval(()=>{if(e(f)<o.length){let t=e(i).split("");t[e(f)]=o[e(f)],q(i,t.join(""),!0),Re(f)}else l&&clearInterval(l),s&&clearInterval(s),l=null,s=null,q(i,o),setTimeout(()=>{const t=document.getElementById("login-title");if(t){const c=t.querySelectorAll(".char"),h=[];c.forEach(n=>{const p=n.style.width;n.style.width="auto",h.push(n.offsetWidth),n.style.width=p}),c.forEach((n,p)=>{n.style.width=`${h[p]}px`}),setTimeout(()=>{q(_,!0),c.forEach(n=>{n.style.width=""})},600)}else q(_,!0)},400)},400)}async function M(t){if(t.preventDefault(),!y||!e(b))return;let c=new FormData(y);console.log("clicked");let h=await e(b).authenticate(String(c.get("username")),String(c.get("password")));if(h){let n=await e(b).load_instance_information();Se.set({username:String(h==null?void 0:h.username),display_name:String(h==null?void 0:h.display_name),avatar:String(h==null?void 0:h.avatar_filename),domain:(n==null?void 0:n.domain)||null,url:(n==null?void 0:n.url)||null});let p=e(b).get_state();if(console.debug(p),p){let N=await e(b).replenish_mkp();console.debug(`REPLENISH RESULT: ${N}`)}re.set(!1)}else console.debug("authentication failed"),u.showModal()}function C(t){t.target===t.currentTarget&&re.set(!1)}function T(t){t.key==="Escape"&&re.set(!1)}ke(()=>{document.body.style.overflow="hidden",document.addEventListener("keydown",T),G()}),Ae(()=>{document.body.style.overflow="",document.removeEventListener("keydown",T),l&&clearInterval(l),s&&clearInterval(s)});var F=Xe();F.__click=C;var g=v(F);g.__click=t=>t.stopPropagation();var x=v(g);let O;Ie(x,21,()=>e(i).split(""),De,(t,c,h)=>{var n=Qe();let p;var N=v(n,!0);d(n),z(()=>{p=$(n,1,"char svelte-8ukb9p",null,p,{revealed:h<e(f)}),ne(N,e(c))}),m(t,n)}),d(x);var ee=w(x,2);me(ee,t=>u=t,()=>u);var a=w(ee,2);me(a,t=>y=t,()=>y),d(g),d(F),z(()=>O=$(x,1,"svelte-8ukb9p",null,O,{"animation-complete":e(_)})),we("submit",a,M),m(P,F),de(),I()}oe(["click"]);var Ze=k(`<style>@font-face {
			font-family: 'Open Sans';
			src: URL('/fonts/OpenSans-Light.ttf');
			font-weight: 300;
		}

		@font-face {
			font-family: 'Open Sans';
			src: URL('/fonts/OpenSans-Regular.ttf');
			font-weight: 400;
		}

		@font-face {
			font-family: 'Open Sans';
			src: URL('/fonts/OpenSans-Medium.ttf');
			font-weight: 500;
		}

		@font-face {
			font-family: 'Open Sans';
			src: URL('/fonts/OpenSans-SemiBold.ttf');
			font-weight: 600;
		}

		@font-face {
			font-family: 'Open Sans';
			src: URL('/fonts/OpenSans-Bold.ttf');
			font-weight: 700;
		}

		/* Inter Variable Font - Normal */
		@font-face {
			font-family: 'Inter';
			src: url('/fonts/Inter-VariableFont_opsz,wght.ttf') format('truetype');
			font-weight: 100 900;
			font-style: normal;
			font-display: swap;
		}

		/* Inter Variable Font - Italic */
		@font-face {
			font-family: 'Inter';
			src: url('/fonts/Inter-Italic-VariableFont_opsz,wght.ttf') format('truetype');
			font-weight: 100 900;
			font-style: italic;
			font-display: swap;
		}

		* {
			box-sizing: border-box;
		}

		html {
			margin: 0;
			padding: 0;
			width: 100dvw;
		}

		body {
			margin: 0;
			padding: 0;
			width: 100dvw;
		}</style>`),$e=k('<a class="svelte-12qhfyh"><img alt="Avatar" class="avatar-image svelte-12qhfyh"/></a>'),et=k('<a aria-label="Profile" class="svelte-12qhfyh"><div class="avatar-placeholder svelte-12qhfyh"><i class="fa-solid fa-user"></i></div></a>'),tt=k('<nav class="top svelte-12qhfyh"><div class="user-card svelte-12qhfyh"><div class="banner-container svelte-12qhfyh"><div class="avatar-container svelte-12qhfyh"><!></div></div> <div class="user-info svelte-12qhfyh"><a class="user-name svelte-12qhfyh"> </a> <a class="user-handle svelte-12qhfyh"> </a></div> <div class="toggle svelte-12qhfyh"><label class="svelte-12qhfyh"><input type="checkbox" id="theme" class="svelte-12qhfyh"/> <span class="slider svelte-12qhfyh"></span></label></div></div></nav>'),at=k('<div class="context"><!></div>'),st=k('<div class="top-container svelte-12qhfyh"><!> <!> <!></div>'),lt=k('<div class="app svelte-12qhfyh"><!></div> <!>',1);function wt(P,r){ie(r,!0);const S=()=>Y(Se,"$appData",u),W=()=>Y(he,"$enigmatickWasm",u),I=()=>Y(Ne,"$page",u),b=()=>Y(re,"$loginOverlayOpen",u),[u,y]=ue();let o=U(()=>S().username),i=U(()=>S().display_name),l=U(W),s=Z(null),f=U(()=>{var a,t;return!((t=(a=e(s))==null?void 0:a.image)!=null&&t.url)||!e(l)?null:_e(e(l),String(e(s).image.url))}),_=U(()=>{var a,t;return!((t=(a=e(s))==null?void 0:a.icon)!=null&&t.url)||!e(l)?null:_e(e(l),String(e(s).icon.url))});Fe(()=>{e(l)&&e(o)&&e(l).get_profile_by_username(e(o)).then(a=>{if(a)try{q(s,JSON.parse(a),!0)}catch(t){console.error("Failed to parse profile:",t)}})}),ke(async()=>{const a=localStorage.getItem("theme");if(a&&a==="dark"?H():a&&a==="light"?G():H(),!e(l)){console.log("importing wasm"),q(l,await He(()=>import("../chunks/BCXkyNpO.js"),[],import.meta.url)),await e(l).default();let t=await e(l).load_instance_information();console.log(t==null?void 0:t.domain),console.log(t==null?void 0:t.url),console.log(e(l)),he.set(e(l))}});function L(){let a=document.getElementsByTagName("body")[0];return!!(a&&a.classList.contains("dark"))}function H(){let a=document.getElementsByTagName("body")[0],t=document.documentElement,c=document.getElementById("theme");a&&!a.classList.contains("dark")&&(a.classList.add("dark"),t.classList.add("dark"),localStorage.setItem("theme","dark")),c&&(c.checked=!1)}function G(){let a=document.getElementsByTagName("body")[0],t=document.documentElement,c=document.getElementById("theme");a&&a.classList.contains("dark")&&(a.classList.remove("dark"),t.classList.remove("dark"),localStorage.setItem("theme","light")),c&&(c.checked=!0)}function M(a){L()?G():H()}var C=lt();Ue("12qhfyh",a=>{var t=Ze();m(a,t)});var T=Q(C),F=v(T);{var g=a=>{var t=st(),c=v(t);{var h=D=>{var E=tt(),J=v(E),K=v(J);let te;var A=v(K),ae=v(A);{var ce=B=>{var j=$e(),Le=v(j);d(j),z(()=>{X(j,"href",`/@${e(o)??""}`),X(Le,"src",e(_))}),m(B,j)},ve=B=>{var j=et();z(()=>X(j,"href",`/@${e(o)??""}`)),m(B,j)};R(ae,B=>{e(_)?B(ce):B(ve,!1)})}d(A),d(K);var V=w(K,2),se=v(V),xe=v(se,!0);d(se);var fe=w(se,2),Oe=v(fe);d(fe),d(V);var ge=w(V,2),pe=v(ge),Ee=v(pe);Ee.__change=B=>{B.preventDefault(),M()},Be(2),d(pe),d(ge),d(J),d(E),z(()=>{te=Pe(K,"",te,{"background-image":e(f)?`url(${e(f)})`:void 0}),X(se,"href",`/@${e(o)??""}`),ne(xe,e(i)||e(o)),X(fe,"href",`/@${e(o)??""}`),ne(Oe,`@${e(o)??""}`)}),m(D,E)};R(c,D=>{e(o)&&e(s)&&D(h)})}var n=w(c,2);{var p=D=>{var E=le(),J=Q(E);ye(J,()=>r.children),m(D,E)};R(n,D=>{r.children&&D(p)})}var N=w(n,2);{var qe=D=>{var E=at(),J=v(E);{var K=A=>{Ke(A,{get hashtags(){return We},resetData:async()=>{}})},te=A=>{var ae=le(),ce=Q(ae);{var ve=V=>{Ve(V,{get searchTypes(){return Ce},get searchOrder(){return Me}})};R(ce,V=>{I().url.pathname==="/search"&&V(ve)},!0)}m(A,ae)};R(J,A=>{I().url.pathname==="/timeline"?A(K):A(te,!1)})}d(E),m(D,E)};R(N,D=>{e(o)&&D(qe)})}d(t),m(a,t)},x=a=>{var t=le(),c=Q(t);{var h=n=>{var p=le(),N=Q(p);ye(N,()=>r.children),m(n,p)};R(c,n=>{r.children&&n(h)})}m(a,t)};R(F,a=>{I().url.pathname!=="/"&&I().url.pathname!=="/login"&&I().url.pathname!=="/signup"?a(g):a(x,!1)})}d(T);var O=w(T,2);{var ee=a=>{Ye(a,{})};R(O,a=>{b()&&a(ee)})}m(P,C),de(),y()}oe(["change"]);export{wt as component,bt as universal};
