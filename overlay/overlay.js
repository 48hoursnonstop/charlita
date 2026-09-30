const stage = document.querySelector('#stage');
const [, , key, kind, id] = location.pathname.split('/');
const nodes = new Map();
let socket, reconnect, lastSnapshot, animating = false;
const reduced = matchMedia('(prefers-reduced-motion: reduce)');

function mediaElement(guest, pauseAnimation) {
  const art = document.createElement('div');
  art.className = 'art';
  if (!guest.media) {
    const fallback = document.createElement('div');
    fallback.className = 'fallback';
    fallback.textContent = guest.name.slice(0, 2);
    art.append(fallback);
    return art;
  }
  const still = pauseAnimation && ['gif', 'webp', 'webm'].includes(guest.media.kind)
    && guest.media.url.startsWith('/o/');
  const video = guest.media.kind === 'webm' && !still;
  const el = document.createElement(video ? 'video' : 'img');
  if (video) {
    el.muted = true; el.loop = true; el.playsInline = true;
    el.autoplay = true; el.preload = 'auto';
  } else {
    el.alt = ''; el.draggable = false;
  }
  el.src = guest.media.url + (still ? '/still' : '');
  art.append(el);
  return art;
}
function fitMedia(node, guest, frame = 0) {
  const el = node.querySelector('img,video');
  if (!el) return;
  const crop = guest.pose?.crop || [0, 0, 1, 1];
  const sprite = guest.pose?.sprite;
  const cols = sprite?.columns || 1, rows = sprite?.rows || 1;
  const cellW = guest.media.width / cols, cellH = guest.media.height / rows;
  const scale = Math.min(guest.size / (cellW * crop[2]), guest.size / (cellH * crop[3]));
  const viewW = cellW * crop[2] * scale, viewH = cellH * crop[3] * scale;
  Object.assign(node.querySelector('.art').style, {
    width: `${viewW}px`, height: `${viewH}px`,
    left: `${(guest.size - viewW) / 2}px`, top: `${(guest.size - viewH) / 2}px`,
  });
  Object.assign(el.style, {
    width: `${guest.media.width * scale}px`, height: `${guest.media.height * scale}px`,
    left: `${-(frame % cols * cellW + crop[0] * cellW) * scale}px`,
    top: `${-(Math.floor(frame / cols) * cellH + crop[1] * cellH) * scale}px`,
  });
}
function render(data) {
  lastSnapshot = data;
  stage.style.width = `${data.width}px`; stage.style.height = `${data.height}px`;
  stage.style.transform = `scale(${Math.min(innerWidth / data.width, innerHeight / data.height)})`;
  stage.classList.toggle('reduced', data.reduced_motion || reduced.matches);
  const active = new Set();
  for (const g of data.guests) {
    active.add(g.id);
    let n = nodes.get(g.id);
    if (!n) {
      n = document.createElement('section'); n.className = 'guest';
      const motion = document.createElement('div'); motion.className = 'motion';
      const label = document.createElement('span'); label.className = 'name';
      n.append(motion, label); stage.append(n); nodes.set(g.id, n);
    }
    const pause = data.reduced_motion || reduced.matches || document.hidden || !g.visible;
    const signature = JSON.stringify([g.media?.url, g.pose, pause]);
    if (n.dataset.media !== signature) {
      n.querySelector('video')?.pause();
      n.querySelector('.motion').replaceChildren(mediaElement(g, pause));
      n.dataset.media = signature; n.dataset.start = String(performance.now());
      n.dataset.frame = '0';
    }
    const wasSpeaking = n.dataset.speaking === 'true';
    const changed = n.dataset.state !== g.state;
    n.className = `guest ${g.state}${g.effects.glow ? ' glow' : ''}`;
    n.style.display = g.visible ? 'block' : 'none';
    Object.assign(n.style, {left: `${g.x}px`, top: `${g.y}px`, width: `${g.size}px`, height: `${g.size}px`});
    if (g.state === 'speaking' && data.connected) {
      clearTimeout(n.release); n.classList.add('speaking'); n.dataset.speaking = 'true';
    } else if (wasSpeaking && data.connected && g.visible) {
      n.classList.add('speaking');
      if (changed) {
        clearTimeout(n.release);
        n.release = setTimeout(() => {
          n.classList.remove('speaking'); n.dataset.speaking = 'false';
        }, g.effects.release_ms);
      }
    } else {
      clearTimeout(n.release); n.classList.remove('speaking'); n.dataset.speaking = 'false';
    }
    n.dataset.state = g.state;
    n.style.setProperty('--jump', `${g.effects.jump}px`);
    n.style.setProperty('--brightness', g.effects.brightness);
    n.style.setProperty('--idle', g.effects.dim_idle);
    n.style.setProperty('--duration', `${g.effects.duration_ms}ms`);
    n.querySelector('.art').classList.toggle('mirrored', g.mirror);
    const label = n.querySelector('.name');
    label.textContent = g.name; label.title = g.name; label.hidden = !data.labels;
    fitMedia(n, g, pause ? 0 : Number(n.dataset.frame));
    const video = n.querySelector('video');
    if (video) { if (pause) video.pause(); else video.play().catch(() => {}); }
  }
  for (const [uid, n] of nodes) if (!active.has(uid)) {
    clearTimeout(n.release); n.querySelector('video')?.pause(); n.remove(); nodes.delete(uid);
  }
  scheduleSprites();
}
function scheduleSprites() {
  if (animating || document.hidden || !lastSnapshot || lastSnapshot.reduced_motion || reduced.matches
      || !lastSnapshot.guests.some(g => g.visible && g.pose?.sprite)) return;
  animating = true; requestAnimationFrame(animateSprites);
}
function animateSprites(now) {
  animating = false;
  if (document.hidden || !lastSnapshot || lastSnapshot.reduced_motion || reduced.matches) return;
  for (const g of lastSnapshot.guests) {
    const s = g.pose?.sprite, n = nodes.get(g.id);
    if (g.visible && s && n) {
      const frame = Math.floor((now - Number(n.dataset.start)) / 1000 * s.fps) % s.frames;
      if (n.dataset.frame !== String(frame)) { fitMedia(n, g, frame); n.dataset.frame = String(frame); }
    }
  }
  scheduleSprites();
}
function connect() {
  clearTimeout(reconnect);
  socket = new WebSocket(`${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/o/${key}/ws/${kind}/${id}`);
  socket.onmessage = e => { try { render(JSON.parse(e.data)); } catch (err) { console.error('Charlita: invalid overlay state', err); } };
  socket.onclose = () => {
    if (lastSnapshot) render({...lastSnapshot, connected: false, guests: lastSnapshot.guests.map(g =>
      g.state === 'speaking' ? {...g, state: 'idle', pose: g.idle_pose, media: g.idle_media} : g)});
    reconnect = setTimeout(connect, 1500);
  };
  socket.onerror = () => socket.close();
}
addEventListener('resize', () => lastSnapshot && render(lastSnapshot));
document.addEventListener('visibilitychange', () => lastSnapshot && render(lastSnapshot));
reduced.addEventListener('change', () => lastSnapshot && render(lastSnapshot));
connect();
