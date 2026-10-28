// SPDX-FileCopyrightText: 2026 The OpenDAP Project
// SPDX-License-Identifier: CC-BY-SA-4.0
// Shared components of the T1 interface mockups (Engineered Matter tokens).
const sv=d=>`<svg class="i" viewBox="0 0 24 24">${d}</svg>`;
const I={
  back:sv('<path d="M15 5l-7 7 7 7"/>'), down:sv('<path d="M6 9l6 6 6-6"/>'),
  search:sv('<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>'),
  heart:sv('<path d="M12 20s-7.5-4.6-7.5-10A4.3 4.3 0 0 1 12 7.6 4.3 4.3 0 0 1 19.5 10c0 5.4-7.5 10-7.5 10z"/>'),
  queue:sv('<path d="M4 6h16M4 12h10M4 18h10"/><path d="M18 14v6l3-3z" fill="currentColor"/>'),
  devices:sv('<rect x="3" y="5" width="12" height="9"/><path d="M1.5 17h15"/><rect x="17" y="8" width="5.5" height="11"/>'),
  shuffle:sv('<path d="M3 7h3.5l7 10H21M3 17h3.5L9 13.5M13.5 10.5L16 7h5"/><path d="M18 4l3 3-3 3M18 14l3 3-3 3"/>'),
  repeat:sv('<path d="M4 11V6h16"/><path d="M17 3l3 3-3 3"/><path d="M20 13v5H4"/><path d="M7 21l-3-3 3-3"/>'),
  plus:sv('<path d="M12 5v14M5 12h14"/>'), check:sv('<path d="M5 12.5l4.5 4.5L19 7.5"/>'),
  sun:sv('<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M21.5 12h-2M4.5 12h-2"/>'),
  handle:sv('<path d="M5 9h14M5 15h14"/>'),
  chev:'<svg viewBox="0 0 16 16" style="width:16px;height:16px;stroke:var(--muted);fill:none;stroke-width:2;flex:none"><path d="M6 3l5 5-5 5"/></svg>',
  rew:sv('<path d="M11 6.5a7.5 7.5 0 1 1-6.8 4.3"/><path d="M4 5v5h5"/>'),
  fwd:sv('<path d="M13 6.5a7.5 7.5 0 1 0 6.8 4.3"/><path d="M20 5v5h-5"/>'),
};
const F={
  prev:'<svg viewBox="0 0 24 24"><path d="M6 5h2v14H6zM19 5v14L9 12z"/></svg>',
  next:'<svg viewBox="0 0 24 24"><path d="M16 5h2v14h-2zM5 5v14l10-7z"/></svg>',
  play:'<svg viewBox="0 0 24 24"><path d="M7 4.5v15l13-7.5z"/></svg>',
  pause:'<svg viewBox="0 0 24 24"><path d="M6.5 4.5h4v15h-4zM13.5 4.5h4v15h-4z"/></svg>',
};
const SB=`<div class="sb">
    <div class="l"><span>10:42</span>
      <svg viewBox="0 0 14 14" aria-label="microSD card"><path d="M4 1.5h5l2.5 2.5v8.5h-7.5z"/><path d="M6 1.5v2M8 1.5v2"/></svg>
      <span class="vol" style="display:inline-flex;gap:3px;align-items:center" aria-label="Volume 38"><svg viewBox="0 0 14 14"><path d="M2 5.5h2.5L8 2.5v9L4.5 8.5H2z"/><path d="M10 5a3 3 0 0 1 0 4"/></svg>38</span>
    </div>
    <div class="r">
      <svg viewBox="0 0 14 14" aria-label="Bluetooth"><path d="M4 4l6 5-3 3V2l3 3-6 5"/></svg>
      <svg viewBox="0 0 14 14" aria-label="Wi-Fi"><path d="M1.5 5.5a8 8 0 0 1 11 0M3.5 7.8a5 5 0 0 1 7 0M5.6 10a2 2 0 0 1 2.8 0"/></svg>
      <span>82%</span>
      <svg viewBox="0 0 20 14" style="width:21px" aria-label="Battery 82 percent"><rect x="1" y="3" width="16" height="8" rx="2"/><rect x="3" y="5" width="11" height="4" rx="1" fill="currentColor" stroke="none"/><path d="M19 6v2"/></svg>
    </div>
  </div>`;
const sb=()=>SB;
const ib=(icon,label,style='')=>`<span class="ib" role="button" aria-label="${label}" style="${style}">${icon}</span>`;
const head=(title,back=true,right='',cls='')=>`<div class="head${back?' back':''} ${cls}">${back?ib(I.back,'Back'):''}<h2>${title}</h2>${right}</div>`;
const ln=(t1,t2,v='',extra='')=>`<div class="ln${t2?' two':''}" ${extra}><span class="t"><div class="t1">${t1}</div>${t2?`<div class="t2">${t2}</div>`:''}</span>${v}</div>`;
const PAL=[['#4E46BE','#2896BE'],['#3C5A78','#8A6BC8'],['#2E6E5E','#5B8FD0'],['#6A4FA0','#C07AA0'],['#22506E','#4FB0A8'],['#5A5F8E','#9AA6D6']];
const art=(k,w,h=w,r=6)=>{const [a,b]=PAL[k%PAL.length];return `<span class="art" style="width:${w}px;height:${h}px;border-radius:${r}px;background:radial-gradient(${w*0.9}px ${h*0.9}px at 30% 30%,${a},transparent 70%),radial-gradient(${w*0.8}px ${h*0.8}px at 75% 75%,${b},transparent 70%),#191C1F" aria-hidden="true"></span>`};
const strip=(t='You Only Live Once',a='The Strokes')=>`<div class="strip" role="button" aria-label="Now playing: ${t}, ${a}. Open player"><span class="prog"><i></i></span>${art(0,40)}<span class="t"><div class="t1">${t}</div><div class="t2">${a}</div></span><span class="ppw" role="button" aria-label="Pause">${F.pause}</span></div>`;
const actionLabel=t=>`<div class="elabel" style="top:var(--y-action)" aria-label="Action button: ${t}"><i></i></div>`;
const player=(o={})=>`<div class="ambient" aria-hidden="true"></div>${sb()}<div class="head" style="justify-content:space-between;padding:0">${ib(I.down,'Library')}<span style="display:flex">${ib(I.devices,'Play on')}${ib(I.heart,'Like')}${ib(I.queue,'Queue')}</span></div>
  <div style="position:absolute;left:var(--m);right:var(--m);top:262px;z-index:1">
   <div style="font-size:var(--fs-title);line-height:var(--lh-title);font-weight:600">You Only Live Once</div>
   <div style="color:var(--text2);font-size:var(--fs-body);line-height:var(--lh-body)">The Strokes · First Impressions of Earth</div>
   <div class="mono" style="font-size:var(--fs-cap);line-height:var(--lh-cap);color:var(--muted);margin-top:var(--unit);display:flex;justify-content:space-between;white-space:nowrap;gap:8px"><span>${o.line1||'FLAC 16/44.1 · 4.4 mm'}</span><span style="color:var(--text2)">${o.line2===undefined?'Direct':o.line2}</span></div>
   <div style="margin-top:var(--group);height:3px;border-radius:2px;background:var(--active);position:relative"><div style="width:42%;height:3px;background:var(--text);border-radius:2px"></div><div style="position:absolute;left:42%;top:50%;width:12px;height:12px;margin:-6px 0 0 -6px;border-radius:50%;background:var(--text)"></div></div>
   <div class="mono" style="display:flex;justify-content:space-between;font-size:var(--fs-cap);line-height:var(--lh-cap);color:var(--muted);margin-top:6px"><span>1:19</span><span>−1:50</span></div>
  </div>
  <div style="position:absolute;left:0;right:0;bottom:18px;display:flex;align-items:center;justify-content:space-between;padding:0 6px;z-index:1"><span class="tg" role="button" aria-label="Shuffle off">${I.shuffle}</span><span class="sk" role="button" aria-label="Previous">${F.prev}</span><span class="pp" role="button" aria-label="${o.paused?'Play':'Pause'}">${o.paused?F.play:F.pause}</span><span class="sk" role="button" aria-label="Next">${F.next}</span><span class="tg on" role="button" aria-label="Repeat album, on">${I.repeat}</span></div>`;

// Renders groups of screens: [title, intro, [ {title, cap, laws, html} ]] into #groups.
function renderGroups(G){
  const root=document.getElementById('groups');let n=0;
  G.forEach(([title,intro,screens])=>{
    const h=document.createElement('h2');h.className='group';h.textContent=title;root.appendChild(h);
    const p=document.createElement('p');p.className='group';p.textContent=intro;root.appendChild(p);
    const grid=document.createElement('div');grid.className='grid';root.appendChild(grid);
    screens.forEach(s=>{n++;
      const f=document.createElement('figure');
      f.innerHTML=`<div class="dev"><span class="key action" title="Action"></span><span class="key vp" title="Volume +"></span><span class="key vm" title="Volume −"></span><span class="key main" title="Main"></span><div class="screen" role="img" aria-label="${s.title}">${s.html}</div></div><figcaption><b>${n}. ${s.title}</b>${s.cap}<span class="laws"><em>Applies:</em> ${s.laws}</span></figcaption>`;
      grid.appendChild(f);
    });
  });
}
const S=(title,cap,laws,html)=>({title,cap,laws,html});
