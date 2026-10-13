// Focus actual simulator panes. Overview beats retain the complete interface.
export async function focusRegion(page, row, episode) {
  const a = row.action;
  let selector;
  if (episode.number === 2) {
    if (a.type === "env") selector = "#flyout";
    if (a.type === "mut") selector = a.index === 2 ? "#t-deals" : a.index === 0 ? "#t-segs" : "#flyout";
    if (a.type === "mut" && row.at >= 164) selector = "#flyout";
    if (a.type === "focus") selector = "#flyout";
    await page.evaluate(closeup => {
      if (!document.getElementById("feed-detail-style")) {
        const style = document.createElement("style");
        style.id = "feed-detail-style";
        style.textContent = `body.feed-detail #flyout{position:fixed;left:460px;top:200px;width:1000px;height:auto;padding:24px;z-index:20;background:#fff;border:1px solid #bbb}
body.feed-detail #fly-title{font-size:32px;line-height:1.2;margin:0 0 18px}
body.feed-detail #fly-body .inspect-row{grid-template-columns:190px minmax(0,1fr);font-size:40px;line-height:1.25;padding:6px 0}
body.feed-detail #fly-body .inspect-row b{font-size:26px;align-self:center}
body.feed-detail #fly-keys{font-size:16px;line-height:1.4;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;max-height:50px}`;
        document.head.append(style);
      }
      document.body.classList.toggle("feed-detail", closeup);
    }, selector === "#flyout");
  } else if (a.type === "lab-view") {
    selector = `#${a.view}-section`;
    if (a.view === "hops") selector = ".graph-pane";
  } else if (a.type === "lab-traffic") {
    selector = "#ops-section";
  }
  if (episode.number === 3) {
    await page.evaluate(selector => {
      if (!document.getElementById("feed-detail-style")) {
        const style = document.createElement("style");
        style.id = "feed-detail-style";
        style.textContent = `.feed-pane{position:fixed;left:410px;top:160px;width:1100px;z-index:20;background:#101010;border:1px solid #555;padding:24px!important}
body.recording .feed-pane h2{font-size:32px;margin-bottom:20px}
body.recording .feed-pane table{font-size:26px;line-height:1.25;table-layout:fixed;width:100%}
body.recording .feed-pane th,body.recording .feed-pane td{padding:8px;overflow-wrap:anywhere}
body.recording #apply-section.feed-pane th:first-child,body.recording #apply-section.feed-pane td:first-child,
body.recording #apply-section.feed-pane th:last-child,body.recording #apply-section.feed-pane td:last-child{display:none}
body.recording #apply-section.feed-pane tr[data-response-agent]{display:none}
body.recording #apply-section.feed-pane th:nth-child(2){width:19%}
body.recording #apply-section.feed-pane th:nth-child(3){width:34%}
body.recording #apply-section.feed-pane th:nth-child(4){width:33%}
body.recording #apply-section.feed-pane th:nth-child(5){width:14%}
body.recording #ticket-section.feed-pane{font-size:28px;line-height:1.35}
body.recording #ticket-section.feed-pane h3{font-size:26px}
body.recording #ticket-section.feed-pane #ticket-before>div:nth-child(n+4),body.recording #ticket-section.feed-pane #ticket>div:nth-child(n+4){display:none}
body.recording #ticket-section.feed-pane .card{padding:16px}
body.recording #events-section.feed-pane tbody tr:nth-child(n+5){display:none}
body.recording #events-section.feed-pane table caption{font-size:18px;text-align:left;margin-bottom:12px}
body.recording #manifest-section.feed-pane pre{font-size:24px;line-height:1.3}
body.recording #ops-section.feed-pane #ops-summary{display:none}
body.recording #ops-section.feed-pane #feed-ops-summary{display:grid;grid-template-columns:repeat(3,1fr);gap:20px;font-size:22px;line-height:1.35}
body.recording #feed-ops-summary{display:none}
body.recording #feed-ops-summary strong{display:block;font-size:60px;font-weight:500;margin-top:8px}
body.recording #ops-section.feed-pane #ops-counts{font-size:28px;line-height:1.4}
body.recording #ops-section.feed-pane .beats,body.recording #ops-section.feed-pane>p:not([id]):not(.muted){display:none}
body.recording #ops-section.feed-pane #ops-agents tr:nth-child(n+4){display:none}
body.recording #ops-section.feed-pane table caption{font-size:18px;text-align:left;margin-bottom:12px}
body.recording #ops-section.feed-pane .muted{font-size:18px}`;
        document.head.append(style);
        const source = document.getElementById("ops-summary");
        const readings = document.createElement("p");
        readings.id = "feed-ops-summary";
        source.after(readings);
        const update = () => {
          const match = source.textContent.match(/Observed auctions\/s ([\d.]+) · SSP hop p95 ([\d.]+|warming up) ms · DSP call failures \(cumulative\) (\d+)/);
          readings.replaceChildren();
          if (!match) { readings.textContent = source.textContent; return; }
          for (const [label, value] of [["Achieved auctions/s", match[1]], ["SSP hop p95 (ms)", match[2]], ["DSP call failures (cumulative)", match[3]]]) {
            const cell = document.createElement("span");
            const number = document.createElement("strong");
            cell.textContent = label;
            number.textContent = value;
            cell.append(number);
            readings.append(cell);
          }
        };
        new MutationObserver(update).observe(source, { childList: true, subtree: true, characterData: true });
        update();
        const caption = document.createElement("caption");
        caption.textContent = "Agent sample: first three rows. Rates and p95 use a one minute window.";
        document.querySelector("#ops-section table").prepend(caption);
        const deliveryCaption = document.createElement("caption");
        deliveryCaption.textContent = "Latest four delivery events. The tracker retains the full event list.";
        document.querySelector("#events-section table").prepend(deliveryCaption);
      }
      document.querySelectorAll(".feed-pane").forEach(el => el.classList.remove("feed-pane"));
      if (selector && selector !== ".graph-pane") document.querySelector(selector)?.classList.add("feed-pane");
      document.querySelectorAll("#outcomes tr").forEach(row => row.toggleAttribute("data-response-agent", /shader/.test(row.children[1]?.textContent || "")));
    }, selector);
  }
  if (!selector) return { at: row.at, end: row.end, overview: true };
  let bounds;
  if (episode.number === 3 && selector === "#ops-section") {
    selector = "#feed-ops-summary";
    bounds = await page.locator(selector).boundingBox();
    if (a.type === "lab-view") {
      const second = await page.locator("#feed-ops-summary>span").nth(1).boundingBox();
      const third = await page.locator("#feed-ops-summary>span").nth(2).boundingBox();
      if (second && third) bounds = { x: second.x, y: Math.min(second.y, third.y), width: third.x + third.width - second.x, height: Math.max(second.y + second.height, third.y + third.height) - Math.min(second.y, third.y) };
    }
  } else bounds = await page.locator(selector).boundingBox();
  if (!bounds) throw new Error(`Recording pane ${selector} is not visible`);
  if (episode.number === 3 && a.view === "manifest") {
    const details = await page.locator("#manifest").evaluate(pre => {
      const node = pre.firstChild;
      const text = node.textContent;
      return [["intents", "description"], ["image", "health"]].map(([start, end]) => {
        const from = text.indexOf(`"${start}"`), to = text.indexOf(`"${end}"`);
        if (from < 0 || to <= from) throw new Error("Descriptor closeup fields are missing");
        const range = document.createRange();
        range.setStart(node, from); range.setEnd(node, to);
        const r = range.getBoundingClientRect();
        return { x: r.x, y: r.y, width: r.width, height: r.height };
      });
    });
    const split = row.at + 10;
    return { ...cropRegion(bounds, row.at, row.end, selector), cuts: [
      cropRegion(details[0], row.at, split, '#manifest intents'),
      cropRegion(details[1], split, row.end, '#manifest image and resources'),
    ] };
  }
  return cropRegion(bounds, row.at, row.end, selector);
}

function cropRegion(bounds, at, end, selector) {
  if (bounds.y < 0 || bounds.y + bounds.height > 1080) throw new Error(`Recording pane ${selector} does not fit the captured frame`);
  const width = Math.min(2700, Math.max(550, bounds.width + 80, (bounds.height + 40) * 2.5));
  const height = width / 2.5;
  const x = Math.max(0, Math.min(2700 - width, bounds.x + bounds.width / 2 + 390 - width / 2));
  const y = Math.max(0, Math.min(1080 - height, bounds.y + bounds.height / 2 - height / 2));
  return { at, end, selector, x, y, width, height, zoom: 2700 / width };
}
