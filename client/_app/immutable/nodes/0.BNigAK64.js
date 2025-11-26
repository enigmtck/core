import{_ as Le}from"../chunks/C1FmrZbK.js";import"../chunks/CWj6FrbW.js";import{aL as ce,p as ee,w,z as i,D as n,aE as G,E as e,aG as ge,y as f,aJ as E,aI as O,x as _e,a as ae,aH as N,aC as pe,A as ie,o as ye,aD as qe,I as He,aM as Z,aV as me}from"../chunks/CPBLXjHw.js";import{i as I}from"../chunks/DQNDscH3.js";import{h as Ie}from"../chunks/BjeqDZl8.js";import{s as j}from"../chunks/BjYZIULF.js";import{s as Ee,F as Fe}from"../chunks/663Jjc3J.js";import{s as fe,a as V}from"../chunks/DmKqPpxj.js";import{a as be,l as $,e as de,h as Re,b as xe,s as Ae}from"../chunks/CS4B0N_g.js";import{p as Te}from"../chunks/NTu1Tg67.js";import"../chunks/Bp3pig_u.js";import{c as he}from"../chunks/CR2iVpIj.js";import{s as oe}from"../chunks/DSGZP5v2.js";import{e as Ue,i as Pe}from"../chunks/J1mst6rE.js";import{b as ve}from"../chunks/CD7duDpS.js";import{S as We}from"../chunks/xoSPiZpW.js";const ze=!1,ga=Object.freeze(Object.defineProperty({__proto__:null,ssr:ze},Symbol.toStringTag,{value:"Module"}));var Me=w('<a href="#filters" title="Delete" class="svelte-1avybgx"> </a>'),Ne=w('<div class="filters svelte-1avybgx"><div class="svelte-1avybgx"></div> <form class="svelte-1avybgx"><input type="text" name="statement" placeholder="Hashtag to filter by" class="svelte-1avybgx"/></form></div>');function ue(x,s){ee(s,!0);let p,A=E(()=>{const o=s.hashtags;return o.size,Array.from(o)});const g=o=>{var h;o.preventDefault();const v=new FormData(p);let a;if(a=(h=v.get("statement"))==null?void 0:h.toString().trim()){a=a.startsWith("#")?a.slice(1):a;const u=a.toString().toLowerCase();s.onHashtagAdd?s.onHashtagAdd(u):s.hashtags.add(u),s.resetData()}p.reset()},y=o=>{o.preventDefault();let v=o.currentTarget||o.target,a=v==null?void 0:v.dataset.tag;a&&(s.onHashtagRemove?s.onHashtagRemove(a):s.hashtags.delete(a),s.resetData())};var m=Ne(),_=i(m);Ue(_,21,()=>e(A),Pe,(o,v)=>{var a=Me();a.__click=y;var h=i(a);n(a),G(()=>{j(a,"data-tag",e(v)),ge(h,`#${e(v)??""}`)}),f(o,a)}),n(_);var l=O(_,2);ve(l,o=>p=o,()=>p),n(m),_e("submit",l,g),f(x,m),ae()}ce(["click"]);var Ve=w('<div class="filters-drawer svelte-vd7ohq"><div><div><!></div>  <div role="button" tabindex="0"><i class="fa-solid fa-caret-down svelte-vd7ohq"></i></div></div></div> <div class="filters-container svelte-vd7ohq"><h1 class="svelte-vd7ohq">Hashtags</h1> <!></div>',1);function Be(x,s){ee(s,!0);const p=()=>V(s.hashtags,"$hashtagsStore",A),[A,g]=fe();let y=E(p);const m=k=>{s.onHashtagAdd?s.onHashtagAdd(k):s.hashtags.update(C=>{const U=new Set(C);return U.add(k),U})},_=k=>{s.onHashtagRemove?s.onHashtagRemove(k):s.hashtags.update(C=>{const U=new Set(C);return U.delete(k),U})};let l=pe(!1);function o(){ie(l,!e(l))}var v=Ve(),a=N(v),h=i(a);let u;var L=i(h);let d;var T=i(L);ue(T,{get hashtags(){return e(y)},get resetData(){return s.resetData},onHashtagAdd:m,onHashtagRemove:_}),n(L);var c=O(L,2);let b;c.__click=o,c.__keydown=k=>{(k.key==="Enter"||k.key===" ")&&(k.preventDefault(),o())},n(h),n(a);var F=O(a,2),B=O(i(F),2);ue(B,{get hashtags(){return e(y)},get resetData(){return s.resetData},onHashtagAdd:m,onHashtagRemove:_}),n(F),G(()=>{u=oe(h,1,"filters-drawer__content svelte-vd7ohq",null,u,{open:e(l)}),d=oe(L,1,"filters-container-wrapper svelte-vd7ohq",null,d,{open:e(l)}),b=oe(c,1,"filters-drawer__pull svelte-vd7ohq",null,b,{open:e(l)})}),f(x,v),ae(),g()}ce(["click","keydown"]);var Ce=w(`<div class="overlay svelte-8ukb9p" role="dialog" aria-modal="true" aria-labelledby="login-title"><div class="modal svelte-8ukb9p"><h1 id="login-title" class="svelte-8ukb9p">ENIGMATICK</h1> <dialog class="svelte-8ukb9p"><form method="dialog" class="svelte-8ukb9p"><h3 class="svelte-8ukb9p">Authentication Failed</h3> <p class="svelte-8ukb9p">Either the information you submitted was incorrect, or there is a problem with the service.
					If you suspect the latter, please try again later.</p> <button class="svelte-8ukb9p">Okay</button></form></dialog> <form id="login" method="POST" class="svelte-8ukb9p"><label class="svelte-8ukb9p">Username <input name="username" type="text" placeholder="bob" autocomplete="username" class="svelte-8ukb9p"/></label> <label class="svelte-8ukb9p">Password <input name="password" type="password" placeholder="Use a password manager" autocomplete="current-password" class="svelte-8ukb9p"/></label> <button type="submit" class="svelte-8ukb9p">Sign In</button></form></div></div>`);function je(x,s){ee(s,!0);const p=()=>V(de,"$enigmatickWasm",A),[A,g]=fe();let y=E(p),m,_;async function l(d){if(d.preventDefault(),!_||!e(y))return;let T=new FormData(_);console.log("clicked");let c=await e(y).authenticate(String(T.get("username")),String(T.get("password")));if(c){let b=await e(y).load_instance_information();be.set({username:String(c==null?void 0:c.username),display_name:String(c==null?void 0:c.display_name),avatar:String(c==null?void 0:c.avatar_filename),domain:(b==null?void 0:b.domain)||null,url:(b==null?void 0:b.url)||null});let F=e(y).get_state();if(console.debug(F),F){let B=await e(y).replenish_mkp();console.debug(`REPLENISH RESULT: ${B}`)}$.set(!1)}else console.debug("authentication failed"),m.showModal()}function o(d){d.target===d.currentTarget&&$.set(!1)}function v(d){d.key==="Escape"&&$.set(!1)}ye(()=>{document.body.style.overflow="hidden",document.addEventListener("keydown",v)}),qe(()=>{document.body.style.overflow="",document.removeEventListener("keydown",v)});var a=Ce();a.__click=o;var h=i(a);h.__click=d=>d.stopPropagation();var u=O(i(h),2);ve(u,d=>m=d,()=>m);var L=O(u,2);ve(L,d=>_=d,()=>_),n(h),n(a),_e("submit",L,l),f(x,a),ae(),g()}ce(["click"]);var Ge=w(`<style>@font-face {
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
		}</style>`),Je=w('<a class="svelte-12qhfyh"><img alt="Avatar" class="avatar-image svelte-12qhfyh"/></a>'),Ke=w('<a aria-label="Profile" class="svelte-12qhfyh"><div class="avatar-placeholder svelte-12qhfyh"><i class="fa-solid fa-user"></i></div></a>'),Qe=w('<div class="user-handle svelte-12qhfyh"><!></div>'),Xe=w('<nav class="top svelte-12qhfyh"><div class="user-card svelte-12qhfyh"><div class="banner-container svelte-12qhfyh"><div class="avatar-container svelte-12qhfyh"><!></div></div> <div class="user-info svelte-12qhfyh"><a class="user-name svelte-12qhfyh"> </a> <!></div></div></nav>'),Ye=w('<div class="context"><!></div>'),Ze=w('<div class="top-container svelte-12qhfyh"><!> <!> <!></div>'),$e=w('<div class="app svelte-12qhfyh"><!></div> <!>',1);function _a(x,s){ee(s,!0);const p=()=>V(be,"$appData",m),A=()=>V(de,"$enigmatickWasm",m),g=()=>V(Te,"$page",m),y=()=>V($,"$loginOverlayOpen",m),[m,_]=fe();let l=E(()=>p().username),o=E(()=>p().display_name),v=E(()=>p().domain),a=E(A),h=E(()=>e(l)&&e(v)?`@${e(l)}@${e(v)}`:e(l)?`@${e(l)}`:""),u=pe(null),L=E(()=>{var t,r;return!((r=(t=e(u))==null?void 0:t.image)!=null&&r.url)||!e(a)?null:he(e(a),String(e(u).image.url))}),d=E(()=>{var t,r;return!((r=(t=e(u))==null?void 0:t.icon)!=null&&r.url)||!e(a)?null:he(e(a),String(e(u).icon.url))});He(()=>{e(a)&&e(l)&&e(a).get_profile_by_username(e(l)).then(t=>{if(t)try{ie(u,JSON.parse(t),!0)}catch(r){console.error("Failed to parse profile:",r)}})}),ye(async()=>{const t=localStorage.getItem("theme");if(t&&t==="dark"?T():t&&t==="light"?c():T(),!e(a)){console.log("importing wasm"),ie(a,await Le(()=>import("../chunks/CW7yky5l.js"),[],import.meta.url)),await e(a).default();let r=await e(a).load_instance_information();console.log(r==null?void 0:r.domain),console.log(r==null?void 0:r.url),console.log(e(a)),de.set(e(a))}});function T(){let t=document.getElementsByTagName("body")[0],r=document.documentElement;t&&!t.classList.contains("dark")&&(t.classList.add("dark"),r.classList.add("dark"),localStorage.setItem("theme","dark"))}function c(){let t=document.getElementsByTagName("body")[0],r=document.documentElement;t&&t.classList.contains("dark")&&(t.classList.remove("dark"),r.classList.remove("dark"),localStorage.setItem("theme","light"))}var b=$e();Ie("12qhfyh",t=>{var r=Ge();f(t,r)});var F=N(b),B=i(F);{var k=t=>{var r=Ze(),J=i(r);{var te=S=>{var q=Xe(),W=i(q),z=i(W);let Q;var R=i(z),X=i(R);{var re=H=>{var D=Je(),ne=i(D);n(D),G(()=>{j(D,"href",`/@${e(l)??""}`),j(ne,"src",e(d))}),f(H,D)},le=H=>{var D=Ke();G(()=>j(D,"href",`/@${e(l)??""}`)),f(H,D)};I(X,H=>{e(d)?H(re):H(le,!1)})}n(R),n(z);var M=O(z,2),Y=i(M),Se=i(Y,!0);n(Y);var De=O(Y,2);{var Oe=H=>{var D=Qe(),ne=i(D);Fe(ne,{get handle(){return e(h)}}),n(D),f(H,D)};I(De,H=>{e(h)&&H(Oe)})}n(M),n(W),n(q),G(()=>{Q=Ee(z,"",Q,{"background-image":e(L)?`url(${e(L)})`:void 0}),j(Y,"href",`/@${e(l)??""}`),ge(Se,e(o)||e(l))}),f(S,q)};I(J,S=>{e(l)&&e(u)&&g().url.pathname!=="/objects"&&!g().url.pathname.startsWith("/@")&&S(te)})}var P=O(J,2);{var K=S=>{var q=Z(),W=N(q);me(W,()=>s.children),f(S,q)};I(P,S=>{s.children&&S(K)})}var se=O(P,2);{var ke=S=>{var q=Ye(),W=i(q);{var z=R=>{Be(R,{get hashtags(){return Re},resetData:async()=>{}})},Q=R=>{var X=Z(),re=N(X);{var le=M=>{We(M,{get searchTypes(){return Ae},get searchOrder(){return xe}})};I(re,M=>{g().url.pathname==="/search"&&M(le)},!0)}f(R,X)};I(W,R=>{g().url.pathname==="/timeline"?R(z):R(Q,!1)})}n(q),f(S,q)};I(se,S=>{e(l)&&S(ke)})}n(r),f(t,r)},C=t=>{var r=Z(),J=N(r);{var te=P=>{var K=Z(),se=N(K);me(se,()=>s.children),f(P,K)};I(J,P=>{s.children&&P(te)})}f(t,r)};I(B,t=>{g().url.pathname!=="/"&&g().url.pathname!=="/login"&&g().url.pathname!=="/signup"?t(k):t(C,!1)})}n(F);var U=O(F,2);{var we=t=>{je(t,{})};I(U,t=>{y()&&t(we)})}f(x,b),ae(),_()}export{_a as component,ga as universal};
