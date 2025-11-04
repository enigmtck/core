import{_ as ce}from"../chunks/C1FmrZbK.js";import"../chunks/CWj6FrbW.js";import{aE as me,p as de,o as ge,j as c,m as o,n as pe,v as r,ah as x,r as _e,q as i,u as n,ae as k,ad as v,aj as ye,af as p,aF as M,aG as C}from"../chunks/iAwc3qQu.js";import{i as _}from"../chunks/BeCdKdsH.js";import{h as ue}from"../chunks/fnhVM35-.js";import{s as U}from"../chunks/e-kiUM3O.js";import{p as qe,s as f,c as m}from"../chunks/CH6Mh4UC.js";import{s as be,a as j}from"../chunks/CTRaWkmf.js";import{e as F,a as Se}from"../chunks/BjbtEI9d.js";import"../chunks/D2xv9s3J.js";const ke=!1,Ge=Object.freeze(Object.defineProperty({__proto__:null,ssr:ke},Symbol.toStringTag,{value:"Module"}));var we=c(`<style>@font-face {
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

		* {
			box-sizing: border-box;
		}

		html {
			margin: 0;
			padding: 0;
			height: 100dvh;
			width: 100dvw;
		}

		body {
			margin: 0;
			padding: 0;
			height: 100dvh;
			width: 100dvw;
		}</style>`),Le=c('<a aria-label="View profile"><i></i></a>'),Oe=c('<a href="/login" aria-label="Login"><i></i></a>'),Ee=c('<span class="avatar svelte-12qhfyh"><a class="svelte-12qhfyh"><img alt="You" class="svelte-12qhfyh"/></a></span>'),Ie=c('<a href="/search"><i class="fa-solid fa-magnifying-glass svelte-12qhfyh"></i>Search</a>'),Te=c('<a href="/settings"><i class="fa-solid fa-gear svelte-12qhfyh"></i>Settings</a>'),Be=c('<a href="/login"><i class="fa-solid fa-right-to-bracket svelte-12qhfyh"></i>Login</a>'),De=c('<!> <footer class="svelte-12qhfyh"><!> <a href="/timeline" aria-label="Timeline"><i></i></a> <a href="/search" aria-label="Search"><i></i></a></footer> <nav class="top svelte-12qhfyh"><div class="svelte-12qhfyh"><span class="title svelte-12qhfyh"><a href="/" class="svelte-12qhfyh">ENIGMATICK</a></span> <!></div> <div class="svelte-12qhfyh"><a href="/timeline"><i class="fa-solid fa-newspaper svelte-12qhfyh"></i>Timeline</a> <!> <!></div> <div class="toggle svelte-12qhfyh"><label class="svelte-12qhfyh"><input type="checkbox" id="theme" class="svelte-12qhfyh"/> <span class="slider svelte-12qhfyh"></span></label></div></nav>',1),Re=c('<div class="app svelte-12qhfyh"><!></div>');function Ve(K,u){de(u,!0);const N=()=>j(Se,"$appData",w),Y=()=>j(F,"$enigmatickWasm",w),l=()=>j(qe,"$page",w),[w,H]=be();let d=x(()=>N().username),$=x(()=>N().avatar),y=x(Y);ge(async()=>{const t=localStorage.getItem("theme");if(t&&t==="dark"?L():t&&t==="light"?P():L(),!r(y)){console.log("importing wasm"),_e(y,await ce(()=>import("../chunks/CZ-JN-Aj.js"),[],import.meta.url)),await r(y).default();let s=await r(y).load_instance_information();console.log(s==null?void 0:s.domain),console.log(s==null?void 0:s.url),console.log(r(y)),F.set(r(y))}});function J(){let t=document.getElementsByTagName("body")[0];return!!(t&&t.classList.contains("dark"))}function L(){let t=document.getElementsByTagName("body")[0],s=document.getElementById("theme");t&&!t.classList.contains("dark")&&(t.classList.add("dark"),localStorage.setItem("theme","dark")),s&&(s.checked=!1)}function P(){let t=document.getElementsByTagName("body")[0],s=document.getElementById("theme");t&&t.classList.contains("dark")&&(t.classList.remove("dark"),localStorage.setItem("theme","light")),s&&(s.checked=!0)}function Q(t){J()?P():L()}var O=Re();ue("12qhfyh",t=>{var s=we();o(t,s)});var X=i(O);{var Z=t=>{var s=De(),b=k(s);{var E=e=>{var a=M(),h=k(a);C(h,()=>u.children),o(e,a)};_(b,e=>{u.children&&e(E)})}var g=v(b,2),q=i(g);{var I=e=>{var a=Le(),h=i(a);n(a),p(()=>{f(a,1,m(l().url.pathname==`/@${r(d)}`?"selected":""),"svelte-12qhfyh"),U(a,"href",`/@${r(d)}`),f(h,1,`fa-solid fa-user ${l().url.pathname=="/@"+r(d)?"selected":""}`,"svelte-12qhfyh")}),o(e,a)},ae=e=>{var a=Oe(),h=i(a);n(a),p((R,ve)=>{f(a,1,R,"svelte-12qhfyh"),f(h,1,`fa-solid fa-right-to-bracket ${ve??""}`,"svelte-12qhfyh")},[()=>m(String(l().url.pathname)==="/login"?"selected":""),()=>String(l().url.pathname)==="/login"?"selected":""]),o(e,a)};_(q,e=>{r(d)?e(I):e(ae,!1)})}var S=v(q,2),te=i(S);n(S);var T=v(S,2),se=i(T);n(T),n(g);var W=v(g,2),B=i(W),le=v(i(B),2);{var re=e=>{var a=Ee(),h=i(a),R=i(h);n(h),n(a),p(()=>{U(h,"href",`/@${r(d)??""}`),U(R,"src",r($))}),o(e,a)};_(le,e=>{r($)&&e(re)})}n(B);var D=v(B,2),z=i(D),A=v(z,2);{var ie=e=>{var a=Ie();p(()=>f(a,1,m(l().url.pathname=="/search"?"selected":""),"svelte-12qhfyh")),o(e,a)};_(A,e=>{r(d)&&e(ie)})}var ne=v(A,2);{var oe=e=>{var a=Te();p(()=>f(a,1,m(l().url.pathname=="/settings"?"selected":""),"svelte-12qhfyh")),o(e,a)},fe=e=>{var a=Be();p(h=>f(a,1,h,"svelte-12qhfyh"),[()=>m(String(l().url.pathname)==="/login"?"selected":"")]),o(e,a)};_(ne,e=>{r(d)?e(oe):e(fe,!1)})}n(D);var G=v(D,2),V=i(G),he=i(V);he.__change=e=>{e.preventDefault(),Q()},ye(2),n(V),n(G),n(W),p(()=>{f(S,1,m(l().url.pathname=="/timeline"?"selected":""),"svelte-12qhfyh"),f(te,1,`fa-solid fa-newspaper ${l().url.pathname=="/timeline"?"selected":""}`,"svelte-12qhfyh"),f(T,1,m(l().url.pathname=="/search"?"selected":""),"svelte-12qhfyh"),f(se,1,`fa-solid fa-magnifying-glass ${l().url.pathname=="/search"?"selected":""}`,"svelte-12qhfyh"),f(z,1,m(l().url.pathname=="/timeline"?"selected":""),"svelte-12qhfyh")}),o(t,s)},ee=t=>{var s=M(),b=k(s);{var E=g=>{var q=M(),I=k(q);C(I,()=>u.children),o(g,q)};_(b,g=>{u.children&&g(E)})}o(t,s)};_(X,t=>{l().url.pathname!=="/"&&l().url.pathname!=="/login"&&l().url.pathname!=="/signup"?t(Z):t(ee,!1)})}n(O),o(K,O),pe(),H()}me(["change"]);export{Ve as component,Ge as universal};
