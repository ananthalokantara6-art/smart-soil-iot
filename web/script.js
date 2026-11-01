const API_BASE = ""; 
const SLOTS = 24;

const PARAM = {
  moisture:    { label: "Kelembapan tanah", unit: "%",     dec: 0, field: "moisture_pct",  color: "#5f8f4a", lowWarn: 55,   lowBad: 40,   highWarn: 80, highBad: 90 },
  temperature: { label: "Suhu tanah",       unit: "°C",    dec: 1, field: "temperature_c", color: "#c47f1a", lowWarn: null, lowBad: null, highWarn: 31, highBad: 35 },
  ph:          { label: "pH tanah",         unit: "",      dec: 1, field: "ph",            color: "#7a5a44", lowWarn: null, lowBad: null, highWarn: 5,  highBad: 6 },
  ec:          { label: "EC tanah",         unit: "mS/cm", dec: 2, field: "ec_ms_cm",      color: "#2c6e8f", lowWarn: null, lowBad: null, highWarn: 1.5, highBad: 3 },
};
const LEVEL = ["Aman", "Waspada", "Bahaya"];
const ALERT_NAME = ["Info", "Waspada", "Bahaya"];
const ALERT_LEVEL = { info: 0, warning: 1, critical: 2 };
const LC = ["#5f8f4a", "#d49a1f", "#c0392b"];
const PC = ["#2f5d3a", "#c47f1a", "#2c6e8f", "#7a5a44"];
const STATUS = {
  online:  { cls: "l0", text: "Online" },
  stale:   { cls: "l1", text: "Tertunda" },
  offline: { cls: "l2", text: "Offline" },
  no_data: { cls: "ln", text: "Belum ada data" },
};

// Fungsi murni: nilai + batas -> 0 aman, 1 waspada, 2 bahaya.
function levelOf(k, v) {
  const c = PARAM[k];
  if ((c.highBad != null && v > c.highBad) || (c.lowBad != null && v < c.lowBad)) return 2;
  if ((c.highWarn != null && v > c.highWarn) || (c.lowWarn != null && v < c.lowWarn)) return 1;
  return 0;
}

/* ===== DATA ===== */
async function ambilData(jam) {
  const r = await fetch(`${API_BASE}/api/v1/overview?hours=${jam}&slots=${SLOTS}`);
  if (!r.ok) throw new Error("Gagal memuat data (kode " + r.status + ")");
  return r.json();
}

/* ===== UTIL ===== */
const $ = (id) => document.getElementById(id);
const state = { data: null, petak: "all", param: "moisture", done: new Set() };
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const has = (v) => v !== null && v !== undefined;
const fmt = (k, v) => (has(v) ? v.toFixed(PARAM[k].dec).replace(".", ",") + (PARAM[k].unit ? " " + PARAM[k].unit : "") : "-");
const tag = (cls, text) => `<span class="tag ${cls}">${esc(text)}</span>`;
const levelTag = (l) => tag("l" + l, LEVEL[l]);
const firstNN = (a) => a.find(has);
const lastNN = (a) => [...a].reverse().find(has);
const nama = (p) => p.plot.name;
const aktif = () => (state.petak === "all" ? state.data.plots : state.data.plots.filter((p) => p.plot.plot_id === state.petak));
const warnaPetak = (p) => PC[state.data.plots.indexOf(p) % PC.length];
const nilaiTerbaru = (p, k) => (p.latest ? p.latest[PARAM[k].field] : null);
const rataList = (xs) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : null);

// Rata-rata deret beberapa petak per slot waktu; slot tanpa data satu pun tetap null.
const rataSeri = (list, k) =>
  Array.from({ length: SLOTS }, (_, i) => rataList(list.map((p) => p.series[k][i]).filter(has)));

const lalu = (iso) => {
  if (!iso) return "belum pernah";
  const m = Math.max(0, Math.round((Date.now() - new Date(iso)) / 60000));
  if (m < 1) return "baru saja";
  if (m < 60) return m + " menit lalu";
  if (m < 48 * 60) return Math.round(m / 60) + " jam lalu";
  return Math.round(m / 1440) + " hari lalu";
};
const waktu = (iso) => new Date(iso).toLocaleString("id-ID", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" });
const kunciAlert = (a) => a.ts + "|" + a.plot_id + "|" + a.message; // penanda tetap meski data dimuat ulang

function spark(arr, color) {
  const pts = arr.map((v, i) => [i, v]).filter(([, v]) => has(v));
  if (pts.length < 2) return `<svg viewBox="0 0 120 36" width="120" height="36" aria-hidden="true"></svg>`;
  const vals = pts.map(([, v]) => v), min = Math.min(...vals), max = Math.max(...vals), W = 120, H = 36, r = max - min || 1;
  const line = pts.map(([i, v]) => `${(i / (arr.length - 1)) * W},${H - 4 - ((v - min) / r) * (H - 8)}`).join(" ");
  return `<svg viewBox="0 0 ${W} ${H}" width="120" height="36" aria-hidden="true"><polyline points="${line}" fill="none" stroke="${color}" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round"/></svg>`;
}

function gauge(v) {
  if (!has(v)) return `<svg class="gauge" viewBox="0 0 120 76" role="img" aria-label="Belum ada data kelembapan"><text x="60" y="48" text-anchor="middle" font-size="12" fill="#5f6b60" style="fill:#5f6b60">Belum ada data</text></svg>`;
  const lvl = levelOf("moisture", v), f = Math.min(1, Math.max(0, v / 100)), L = Math.PI * 50;
  return `<svg class="gauge" viewBox="0 0 120 76" role="img" aria-label="Kelembapan ${fmt("moisture", v)}, ${LEVEL[lvl]}">
    <path d="M10 62 A50 50 0 0 1 110 62" fill="none" stroke="#dde1d6" stroke-width="12" stroke-linecap="round"/>
    <path d="M10 62 A50 50 0 0 1 110 62" fill="none" stroke="${LC[lvl]}" stroke-width="12" stroke-linecap="round" stroke-dasharray="${f * L} ${L}"/>
    <text x="60" y="56" text-anchor="middle" font-size="18" font-weight="700">${v.toFixed(0)}</text>
    <text x="60" y="71" text-anchor="middle" font-size="9" fill="#5f6b60" style="fill:#5f6b60">% kelembapan</text></svg>`;
}

function tampilkanBanner(teks, galat) {
  const b = $("banner");
  b.hidden = !teks;
  b.className = "banner" + (galat ? " err" : "");
  b.textContent = teks || "";
}

/* ===== RENDER ===== */
function renderPilihan() {
  if (state.petak !== "all" && !state.data.plots.some((p) => p.plot.plot_id === state.petak)) state.petak = "all";
  $("selPetak").innerHTML = `<option value="all">Semua petak</option>` + state.data.plots.map((p) => `<option value="${esc(p.plot.plot_id)}">${esc(nama(p))}</option>`).join("");
  $("selPetak").value = state.petak;
  const opts = Object.entries(PARAM).map(([k, v]) => `<option value="${k}">${v.label}</option>`).join("");
  if (!$("selParam").options.length) $("selParam").innerHTML = opts;
  renderChips();
  $("batas").textContent = "Batas status sementara: " + Object.values(PARAM).map((c) => {
    const t = [];
    if (c.lowBad != null) t.push(`< ${c.lowBad} bahaya`);
    if (c.lowWarn != null) t.push(`< ${c.lowWarn} waspada`);
    if (c.highWarn != null) t.push(`> ${c.highWarn} waspada`);
    if (c.highBad != null) t.push(`> ${c.highBad} bahaya`);
    return `${c.label} (${t.join(", ")})`;
  }).join("; ") + ". Akan diganti aturan Modul 5.";
}

function renderChips() {
  const h = Object.entries(PARAM).map(([k, v]) => `<button type="button" role="tab" data-p="${k}" aria-selected="${k === state.param}">${v.label}</button>`).join("");
  document.querySelectorAll(".chips").forEach((c) => (c.innerHTML = h));
}

function renderKpi() {
  const list = aktif();
  $("kpi").innerHTML = Object.keys(PARAM).map((k) => {
    const s = rataSeri(list, k), now = rataList(list.map((p) => nilaiTerbaru(p, k)).filter(has));
    if (!has(now)) return `<div class="k" style="--c:#b8bdb0"><small>${PARAM[k].label}</small><div class="big">-</div>${spark(s, PARAM[k].color)}<div class="foot"><span>Belum ada data</span>${tag("ln", "Tidak ada data")}</div></div>`;
    const lv = levelOf(k, now), a = firstNN(s), b = lastNN(s), d = has(a) && has(b) ? b - a : null;
    const sel = d === null ? "Perubahan belum bisa dihitung" : `${d >= 0 ? "▲" : "▼"} ${Math.abs(d).toFixed(PARAM[k].dec).replace(".", ",")} sejak awal rentang`;
    return `<div class="k" style="--c:${LC[lv]}"><small>${PARAM[k].label}</small><div class="big">${fmt(k, now)}</div>${spark(s, PARAM[k].color)}<div class="foot"><span>${sel}</span>${levelTag(lv)}</div></div>`;
  }).join("");
}

function renderPetak() {
  $("petakList").innerHTML = state.data.plots.map((p, i) => {
    const m = p.plot, h = p.health, st = STATUS[h.status] || STATUS.no_data, skor = Math.round(h.score * 100);
    const bc = skor < 40 ? LC[2] : skor < 70 ? LC[1] : LC[0];
    const lok = has(m.latitude) && has(m.longitude) ? `${m.latitude}, ${m.longitude}` : "lokasi belum diisi";
    const info = [m.plant ? "tanaman: " + m.plant : null, has(m.area_ha) ? m.area_ha + " ha" : null].filter(Boolean).join(" · ");
    return `<div class="pk"><div class="head"><span class="num">${i + 1}</span><b>${esc(m.name)}</b>${tag(st.cls, st.text)}</div>
      <div class="loc">${esc(lok)}${info ? " · " + esc(info) : ""}</div>
      ${gauge(nilaiTerbaru(p, "moisture"))}
      <div class="meta"><span>Data terakhir ${esc(lalu(h.last_seen))} · ${h.sample_count} sampel · ${Math.round(h.unusable_ratio * 100)}% bermasalah · ${h.gap_count} jeda</span>
      <div class="batt" role="img" aria-label="Skor kesehatan sensor ${skor}%"><i style="width:${skor}%;background:${bc}"></i></div><span>Skor kesehatan sensor ${skor}%</span></div></div>`;
  }).join("");
}

function renderGrafik() {
  const cv = $("chart"), dpr = window.devicePixelRatio || 1, W = cv.clientWidth, H = 280, k = state.param, c = PARAM[k];
  cv.width = W * dpr; cv.height = H * dpr;
  const g = cv.getContext("2d"); g.scale(dpr, dpr); g.clearRect(0, 0, W, H);
  const list = aktif(), batas = [[c.lowBad, LC[2], "Bahaya"], [c.lowWarn, LC[1], "Waspada"], [c.highWarn, LC[1], "Waspada"], [c.highBad, LC[2], "Bahaya"]].filter(([v]) => v != null);
  const data = list.flatMap((p) => p.series[k]).filter(has);
  $("legend").innerHTML = list.map((p) => `<span><i style="background:${warnaPetak(p)}"></i>${esc(nama(p))}</span>`).join("") + `<span>${c.label}${c.unit ? " (" + c.unit + ")" : ""}</span>`;
  if (!data.length) { g.fillStyle = "#5f6b60"; g.font = "14px system-ui"; g.textAlign = "center"; g.fillText("Belum ada data pada rentang ini", W / 2, H / 2); return; }
  const all = data.concat(batas.map(([v]) => v));
  let min = Math.min(...all), max = Math.max(...all); const pad = (max - min) * 0.1 || 1; min -= pad; max += pad;
  const L = 48, R = 12, T = 12, B = 28, w = W - L - R, h = H - T - B;
  const X = (i) => L + (i / (SLOTS - 1)) * w, Y = (v) => T + (1 - (v - min) / (max - min)) * h;
  g.font = "12px system-ui"; g.lineWidth = 1;
  for (let i = 0; i <= 3; i++) {
    const v = min + ((max - min) * i) / 3, y = Y(v);
    g.strokeStyle = "#dde1d6"; g.beginPath(); g.moveTo(L, y); g.lineTo(W - R, y); g.stroke();
    g.fillStyle = "#5f6b60"; g.textAlign = "right"; g.fillText(v.toFixed(c.dec), L - 6, y + 4);
  }
  batas.forEach(([v, col, t]) => {
    g.setLineDash([6, 4]); g.strokeStyle = col; g.beginPath(); g.moveTo(L, Y(v)); g.lineTo(W - R, Y(v)); g.stroke();
    g.setLineDash([]); g.fillStyle = col; g.textAlign = "left"; g.fillText(t, L + 6, Y(v) - 5);
  });
  g.fillStyle = "#5f6b60"; g.textAlign = "center";
  [0, 0.5, 1].forEach((f) => { const t = Math.round(state.data.hours * (1 - f)); g.fillText(f === 1 ? "Kini" : "-" + (t >= 48 ? Math.round(t / 24) + " hr" : t + " j"), L + f * w, H - 8); });
  list.forEach((p) => {
    g.strokeStyle = warnaPetak(p); g.fillStyle = warnaPetak(p); g.lineWidth = 2.5; g.lineJoin = "round";
    let pen = false; // garis putus di slot yang tidak punya data
    p.series[k].forEach((v, i) => {
      if (!has(v)) { pen = false; return; }
      if (pen) g.lineTo(X(i), Y(v)); else { g.beginPath(); g.moveTo(X(i), Y(v)); pen = true; }
      g.stroke(); g.beginPath(); g.moveTo(X(i), Y(v));
    });
    p.series[k].forEach((v, i) => { if (has(v)) { g.beginPath(); g.arc(X(i), Y(v), 2.5, 0, Math.PI * 2); g.fill(); } });
  });
}

function renderHeat() {
  const k = state.param, jam = state.data.hours;
  $("heat").innerHTML = aktif().map((p) => `<div class="rl" title="${esc(nama(p))}">${esc(nama(p))}</div>` + p.series[k].map((v, i, a) => {
    const t = Math.round(jam * (1 - i / (a.length - 1)));
    if (!has(v)) return `<div class="cell n" title="${esc(nama(p))}, ${t ? "-" + t + " j" : "kini"}: tidak ada data"></div>`;
    const l = levelOf(k, v);
    return `<div class="cell l${l}" title="${esc(nama(p))}, ${t ? "-" + t + " j" : "kini"}: ${fmt(k, v)} (${LEVEL[l]})"></div>`;
  }).join("")).join("");
}

function renderBanding() {
  const k = $("selParam").value || "moisture", vals = state.data.plots.map((p) => ({ n: nama(p), v: nilaiTerbaru(p, k) }));
  const mx = Math.max(1e-9, ...vals.filter((x) => has(x.v)).map((x) => Math.abs(x.v)));
  $("bars").innerHTML = vals.map((x) => `<div class="b"><span class="n" title="${esc(x.n)}">${esc(x.n)}</span><div class="t"><div class="f" style="width:${has(x.v) ? (Math.abs(x.v) / mx) * 100 : 0}%;background:${has(x.v) ? LC[levelOf(k, x.v)] : "#d9ddd0"}"></div></div><span class="v">${fmt(k, x.v)}</span></div>`).join("");
}

function renderDonut() {
  const cnt = [0, 0, 0];
  state.data.plots.forEach((p) => Object.keys(PARAM).forEach((k) => { const v = nilaiTerbaru(p, k); if (has(v)) cnt[levelOf(k, v)]++; }));
  const tot = cnt.reduce((a, b) => a + b, 0), C = 2 * Math.PI * 40; let off = 0;
  if (!tot) { $("donut").innerHTML = `<div class="empty">Belum ada pembacaan.</div>`; return; }
  const arcs = cnt.map((n, i) => { const len = (n / tot) * C, s = `<circle cx="60" cy="60" r="40" fill="none" stroke="${LC[i]}" stroke-width="18" stroke-dasharray="${len} ${C - len}" stroke-dashoffset="${-off}" transform="rotate(-90 60 60)"/>`; off += len; return s; }).join("");
  $("donut").innerHTML = `<svg viewBox="0 0 120 120" width="150" height="150" role="img" aria-label="Komposisi status pembacaan terbaru">${arcs}
    <text x="60" y="64" text-anchor="middle" font-size="20" font-weight="700" fill="#1f2a22">${tot}</text><text x="60" y="78" text-anchor="middle" font-size="9" fill="#5f6b60">pembacaan</text></svg>
    <ul>${cnt.map((n, i) => `<li><i style="background:${LC[i]}"></i>${LEVEL[i]}: <b>${n}</b></li>`).join("")}</ul>`;
}

function renderAlert() {
  const min = Number($("selLevel").value), nm = Object.fromEntries(state.data.plots.map((p) => [p.plot.plot_id, nama(p)]));
  const rows = state.data.alerts.filter((a) => ALERT_LEVEL[a.level] >= min && (state.petak === "all" || a.plot_id === state.petak));
  $("alertBody").innerHTML = rows.length ? rows.map((a, n) => {
    const key = kunciAlert(a), done = state.done.has(key), l = ALERT_LEVEL[a.level], pv = a.provenance;
    return `<tr class="${done ? "done" : ""}"><td><span class="num">${n + 1}</span></td><td>${esc(waktu(a.ts))}</td><td>${esc(nm[a.plot_id] || "petak tidak dikenal")}</td>
      <td title="${esc(pv.source + " v" + pv.version + " · " + pv.method)}">${esc(a.message)}</td><td>${tag("l" + l, ALERT_NAME[l])}</td>
      <td><button type="button" class="act" data-a="${esc(key)}">${done ? "Buka lagi" : "Tandai selesai"}</button></td></tr>`;
  }).join("") : `<tr><td colspan="6" class="empty">Tidak ada peringatan pada pilihan ini.</td></tr>`;
}

const renderFilter = () => { renderKpi(); renderGrafik(); renderHeat(); renderAlert(); };
function renderAll() { renderPilihan(); renderPetak(); renderFilter(); renderBanding(); renderDonut(); }

/* ===== MUAT & EVENT ===== */
async function muat(diam) {
  if (!diam) $("kpi").textContent = "Memuat...";
  try {
    const data = await ambilData(Number($("selRentang").value));
    state.data = data;
    const j = data.hours; $("periodTxt").textContent = j >= 48 ? Math.round(j / 24) + " hari terakhir" : j + " jam terakhir";
    $("updTxt").textContent = new Date(data.generated_at).toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit" });
    if (!data.plots.length) {
      tampilkanBanner("Belum ada petak terdaftar. Daftarkan lewat POST /api/v1/plots atau jalankan demo_server.", false);
      $("kpi").innerHTML = `<div class="empty">Belum ada data untuk ditampilkan.</div>`;
      ["petakList", "heat", "bars", "donut", "legend"].forEach((id) => ($(id).innerHTML = ""));
      $("alertBody").innerHTML = `<tr><td colspan="6" class="empty">Belum ada peringatan.</td></tr>`;
      const cv = $("chart"); cv.getContext("2d").clearRect(0, 0, cv.width, cv.height);
      return;
    }
    tampilkanBanner("", false);
    renderAll();
  } catch (e) {
    tampilkanBanner(e.message + ". Periksa koneksi ke server lalu klik Muat ulang.", true);
    if (!state.data) $("kpi").innerHTML = `<div class="empty">Data belum bisa ditampilkan.</div>`;
  }
}

$("btnMuat").onclick = () => muat(false);
$("selRentang").onchange = () => muat(false);
$("btnReset").onclick = () => { state.petak = "all"; state.param = "moisture"; $("selRentang").value = "24"; $("selLevel").value = "0"; $("selParam").value = "moisture"; muat(false); };
$("selPetak").onchange = (e) => { state.petak = e.target.value; renderFilter(); };
$("selLevel").onchange = () => state.data && renderAlert();
$("selParam").onchange = () => state.data && renderBanding();
document.addEventListener("click", (e) => {
  const c = e.target.closest(".chips button"); if (c && state.data) { state.param = c.dataset.p; renderChips(); renderGrafik(); renderHeat(); }
  const a = e.target.closest("button[data-a]"); if (a && state.data) { const key = a.dataset.a; state.done.has(key) ? state.done.delete(key) : state.done.add(key); renderAlert(); }
});
window.addEventListener("resize", () => state.data && state.data.plots.length && renderGrafik());

const links = [...document.querySelectorAll("#nav a")];
const io = new IntersectionObserver((es) => es.forEach((en) => {
  if (en.isIntersecting) links.forEach((l) => l.classList.toggle("on", l.getAttribute("href") === "#" + en.target.id));
}), { rootMargin: "-30% 0px -60% 0px" });
document.querySelectorAll("main, section").forEach((s) => io.observe(s));

muat(false);
setInterval(() => muat(true), 60000);