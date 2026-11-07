"use strict";
const scene = JSON.parse(document.getElementById("scene").textContent),
  ns = "http://www.w3.org/2000/svg";
const $ = (id) => document.getElementById(id),
  colors = [
    "var(--v-1)",
    "var(--v-2)",
    "var(--v-3)",
    "var(--v-4)",
    "var(--v-feed)",
    "var(--v-main)",
    "var(--v-dim)",
  ];
let selected = null,
  category = -1;
function node(tag, attrs = {}, text) {
  const e = document.createElementNS(ns, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  if (text !== undefined) e.textContent = text;
  return e;
}
function textElement(tag, text, parent) {
  const e = document.createElement(tag);
  e.textContent = text;
  parent.append(e);
  return e;
}
$("description").textContent = scene.description;
for (const r of scene.regions) {
  const b = document.createElement("div");
  b.className = "bank";
  textElement("strong", r.name, b);
  textElement("p", r.description, b);
  $("banks").append(b);
}
for (let z = 0; z < scene.layers; z++) {
  const o = document.createElement("option");
  o.value = z;
  o.textContent = `第 ${z + 1} 层`;
  $("layer").append(o);
}
const all = document.createElement("option");
all.value = "all";
all.textContent = "三层同时展开";
$("layer").append(all);
function legend() {
  $("legend").replaceChildren();
  for (let c = -1; c < scene.categories.length; c++) {
    const b = document.createElement("button");
    b.textContent =
      c < 0
        ? "全部建筑"
        : scene.categories[c].glyph + " " + scene.categories[c].name;
    b.setAttribute("aria-pressed", category === c);
    if (c >= 0) b.style.color = colors[c % colors.length];
    b.onclick = () => {
      category = category === c ? -1 : c;
      legend();
      render();
    };
    $("legend").append(b);
  }
}
function details(tile) {
  const box = $("detail");
  box.replaceChildren();
  if (!tile) {
    textElement(
      "strong",
      `${new Set(scene.tiles.map((t) => t.owner)).size} 个建筑 · ${scene.layers} 层`,
      box,
    );
    textElement(
      "p",
      "选择格子查看类型、占格、方向与每秒流量；选中建筑的跨层占格会一起高亮。",
      box,
    );
    return;
  }
  textElement("strong", tile.label, box);
  textElement("p", tile.description, box);
  const [x, y, z] = tile.position;
  const region = scene.regions.find(
    (r) =>
      x >= r.bounds[0] &&
      x < r.bounds[2] &&
      y >= r.bounds[1] &&
      y < r.bounds[3],
  );
  textElement(
    "p",
    `坐标 (${x}, ${y}) · 第 ${z + 1} 层${region ? " · " + region.name : ""}${tile.auxiliary ? " · 建筑附属占格" : ""}`,
    box,
  );
  const owned = scene.tiles
      .filter((t) => t.owner === tile.owner)
      .map((t) => t.position),
    has = (p) => owned.some((q) => p.every((v, i) => q[i] === v));
  const links = scene.connections.filter((e) => has(e.from) || has(e.to));
  const ul = document.createElement("ul");
  for (const e of links) {
    const p = has(e.from) ? e.to : e.from;
    textElement(
      "li",
      `${has(e.from) ? "输出 →" : "输入 ←"} (${p[0]},${p[1]},第${p[2] + 1}层) · ${e.material} · ${e.rate} 个/秒${e.jump ? " · 跳跃" : ""}`,
      ul,
    );
  }
  box.append(ul);
}
function draw(z) {
  const svg = node("svg", {
    class: "drawing",
    viewBox: `-2 -2 ${scene.width + 4} ${scene.height + 4}`,
    role: "img",
    "aria-label": `第${z + 1}层实际建筑布局`,
  });
  svg.style.width = `${Number($("zoom").value) * 100}%`;
  const defs = node("defs");
  const marker = node("marker", {
    id: `arrow-${z}`,
    viewBox: "0 0 6 6",
    refX: 5,
    refY: 3,
    markerWidth: 4,
    markerHeight: 4,
    orient: "auto-start-reverse",
  });
  marker.append(
    node("path", { d: "M0 0 L6 3 L0 6 Z", fill: "context-stroke" }),
  );
  defs.append(marker);
  svg.append(defs);
  for (let x = 0; x <= scene.width; x++)
    svg.append(node("path", { d: `M${x} 0V${scene.height}`, class: "grid" }));
  for (let y = 0; y <= scene.height; y++)
    svg.append(node("path", { d: `M0 ${y}H${scene.width}`, class: "grid" }));
  if ($("regions").checked)
    scene.regions.forEach((r, i) => {
      const [x, y, x2, y2] = r.bounds;
      svg.append(
        node("rect", {
          x,
          y,
          width: x2 - x,
          height: y2 - y,
          fill: colors[i % 4],
          stroke: colors[i % 4],
          class: "region",
        }),
      );
    });
  for (const t of scene.tiles.filter((t) => t.position[2] === z)) {
    const [x, y] = t.position;
    const g = node("g", {
      class: `tile${selected === t.owner ? " selected" : ""}${category >= 0 && category !== t.category ? " dim" : ""}`,
      tabindex: 0,
      role: "button",
      "aria-label": `${t.label} ${x},${y}`,
      transform: `translate(${x} ${y})`,
    });
    g.append(
      node("rect", {
        x: 0.08,
        y: 0.08,
        width: 0.84,
        height: 0.84,
        rx: 0.12,
        class: "body",
      }),
    );
    let glyph = scene.categories[t.category].glyph;
    if (t.auxiliary) glyph = "·";
    g.append(
      node(
        "text",
        {
          x: 0.5,
          y: 0.48,
          class: "glyph",
          fill: colors[t.category % colors.length],
        },
        glyph,
      ),
    );
    const d = [
      [1, 0],
      [0, 1],
      [-1, 0],
      [0, -1],
    ][t.direction % 4];
    g.append(
      node("path", {
        d: `M${0.5 + d[0] * 0.23} ${0.5 + d[1] * 0.23}l${d[0] * 0.13} ${d[1] * 0.13}`,
        class: "rotation",
        stroke: colors[t.category % colors.length],
        "marker-end": `url(#arrow-${z})`,
      }),
    );
    g.append(node("title", {}, t.label + " · " + t.description));
    const select = () => {
      selected = t.owner;
      details(t);
      render();
    };
    g.onclick = select;
    g.onkeydown = (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        select();
      }
    };
    svg.append(g);
  }
  for (const e of scene.connections) {
    if (!($("links").checked || ($("feed").checked && e.highlighted))) continue;
    const a = e.from,
      b = e.to;
    if (a[2] !== z && b[2] !== z) continue;
    const stroke = e.highlighted ? "var(--v-feed)" : "var(--v-2)";
    if (a[2] === b[2])
      svg.append(
        node("path", {
          d: `M${a[0] + 0.5} ${a[1] + 0.5}L${b[0] + 0.5} ${b[1] + 0.5}`,
          class: `connection${e.highlighted ? " feed" : ""}${e.jump ? " jump" : ""}`,
          stroke,
          "marker-end": `url(#arrow-${z})`,
        }),
      );
    else {
      const p = a[2] === z ? a : b;
      svg.append(
        node(
          "text",
          { x: p[0] + 0.78, y: p[1] + 0.23, class: "glyph", fill: stroke },
          b[2] > a[2] ? "↥" : "↧",
        ),
      );
    }
  }
  for (const p of scene.ports) {
    const c = p.input ? "var(--v-feed)" : "var(--v-2)",
      [x, y] = p.from,
      [x2, y2] = p.to;
    svg.append(
      node("path", {
        d: `M${x + 0.5} ${y + 0.5}L${x2 + 0.5} ${y2 + 0.5}`,
        stroke: c,
        class: "port",
      }),
    );
    if (x === x2 && x < 0) {
      svg.append(
        node(
          "text",
          {
            x: -1.15,
            y: (y + y2) / 2 + 0.5,
            class: "port-label",
            transform: `rotate(-90 -1.15 ${(y + y2) / 2 + 0.5})`,
          },
          p.name,
        ),
      );
    } else
      svg.append(
        node(
          "text",
          {
            x: (x + x2) / 2 + 0.5,
            y: y < 0 ? -1.1 : scene.height + 1.2,
            class: "port-label",
          },
          p.name,
        ),
      );
  }
  for (let x = 0; x < scene.width; x += 4)
    svg.append(
      node("text", { x: x + 0.5, y: scene.height + 0.48, class: "tiny" }, x),
    );
  for (let y = 0; y < scene.height; y += 2)
    svg.append(
      node("text", { x: scene.width + 0.45, y: y + 0.65, class: "tiny" }, y),
    );
  return svg;
}
function render() {
  const target = $("views");
  target.replaceChildren();
  const layers =
    $("layer").value === "all"
      ? Array.from({ length: scene.layers }, (_, i) => i)
      : [Number($("layer").value)];
  for (const z of layers) {
    textElement("p", `第 ${z + 1} 层`, target).className = "layer-title";
    target.append(draw(z));
  }
}
for (const id of ["layer", "regions", "feed", "links", "zoom"])
  $(id).onchange = render;
legend();
details(null);
render();
