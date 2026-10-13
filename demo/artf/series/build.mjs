import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { episodes, timeline, totalSeconds } from "./episodes.mjs";
import { card } from "./cards.mjs";
import { recordingPrompter } from "./recording-prompter.mjs";

const root = dirname(fileURLToPath(import.meta.url));
export const output = join(root, "materials");
export const clock = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
const esc = (s) => s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll('"', "&quot;");
const stamp = (s) => {
  const ms = Math.round(s * 1000);
  return `${String(Math.floor(ms / 3600000)).padStart(2, "0")}:${String(Math.floor(ms / 60000) % 60).padStart(2, "0")}:${String(Math.floor(ms / 1000) % 60).padStart(2, "0")},${String(ms % 1000).padStart(3, "0")}`;
};

export function subtitles(rows) {
  const blocks = [];
  for (const row of rows.filter((b) => b.text)) {
    const words = row.text.split(/\s+/);
    const chunks = [];
    // Keep short sentences together when the episode has explicit speaking windows.
    const sentences = row.voiceEnd ? row.text.match(/[^.!?]+[.!?]?/g).map(s => s.trim()) : [row.text];
    for (const sentence of sentences) {
      const sentenceWords = sentence.split(/\s+/);
      const sentenceChunks = [];
      for (let at = 0; at < sentenceWords.length;) {
        const lines = [""];
        const start = at;
        while (at < sentenceWords.length) {
          const line = lines.length - 1;
          if ((lines[line] + " " + sentenceWords[at]).trim().length > 42) {
            if (lines.length === 2) break;
            lines.push("");
          }
          lines[lines.length - 1] = (lines[lines.length - 1] + " " + sentenceWords[at++]).trim();
        }
        sentenceChunks.push({ text: lines.join("\n"), words: at - start });
      }
      const tail = sentenceChunks.at(-1);
      const previous = sentenceChunks.at(-2);
      if (row.voiceEnd && previous && tail.words < 4) {
        const before = previous.text.split(/\s+/);
        const after = tail.text.split(/\s+/);
        while (after.length < 4 && before.length > 4) after.unshift(before.pop());
        const wrap = list => list.reduce((lines, word) => {
          const last = lines.length - 1;
          if ((lines[last] + " " + word).trim().length > 42) lines.push(word);
          else lines[last] = (lines[last] + " " + word).trim();
          return lines;
        }, [""]).join("\n");
        previous.text = wrap(before);
        previous.words = before.length;
        tail.text = wrap(after);
        tail.words = after.length;
      }
      chunks.push(...sentenceChunks);
    }
    let start = row.voiceAt ?? row.at + 1;
    const spoken = row.voiceEnd ? row.voiceEnd - start : row.seconds - 2;
    for (const chunk of chunks) {
      const end = start + spoken * chunk.words / words.length;
      blocks.push(`${blocks.length + 1}\n${stamp(start)} --> ${stamp(end)}\n${chunk.text}\n`);
      start = end;
    }
  }
  return blocks.join("\n");
}

function cue(a) {
  return {
    card: `Show the ${a.kind} title card.`, console: "Open the episode 2 console at rest.",
    env: `Highlight envelope member ${a.key}.`, scene: `Run scene ${a.beat} and wait for the verdict.`,
    mut: `Highlight proposal ${a.index + 1}.`, focus: `Highlight ${a.selector}.`,
    lab: "Open the episode 3 lab at rest.", "lab-run": `Run lab scene ${a.beat}.`,
    "lab-view": `Show ${a.view}. Highlight ${(a.nodes || []).join(", ")}.`,
    "lab-play": "Run Play VAST and wait for the media and tracker hops.",
    "lab-traffic": `Start synthetic traffic at ${a.qps} auctions/s. Show live operations.`,
    "lab-stop": "Stop synthetic traffic and return to the graph and apply table.",
  }[a.type];
}

function prompter(e, rows) {
  return recordingPrompter(e, rows, output);
}

export function build() {
  mkdirSync(output, { recursive: true });
  copyFileSync(join(root, "RECORDING.md"), join(output, "recording-guide.md"));
  copyFileSync(join(root, "linkedin-posts.txt"), join(output, "linkedin-posts.txt"));
  const index = [];
  for (const e of Object.values(episodes)) {
    const rows = timeline(e);
    const duration = totalSeconds(e);
    if (duration < 300 || duration > 480) throw new Error(`Episode ${e.number} is ${duration}s, outside 5 to 8 minutes`);
    for (const row of rows.filter(b => b.text)) {
      const words = row.text.trim().split(/\s+/).length;
      if (words > Math.floor((row.seconds - 2) * (e.narrationWpm ?? 132) / 60)) throw new Error(`Episode ${e.number} beat ${row.index} narration exceeds its picture hold`);
    }
    const stem = `episode-${e.number}`;
    writeFileSync(join(output, `${stem}.srt`), subtitles(rows));
    writeFileSync(join(output, `${stem}.cues.json`), JSON.stringify({ episode: e.number, title: e.title, seconds: duration, rows }, null, 2) + "\n");
    writeFileSync(join(output, `${stem}.voice.txt`), rows.filter(b => b.text).map(b => b.text).join("\n\n") + "\n");
    const delivery = e.narrationWpm ? `Read at about ${e.narrationWpm} words per minute. Start with each caption, then pause while the shot holds. The prompter marks the speaking and pause windows.` : "Read the paragraphs only. Directions are in the screenplay.";
    writeFileSync(join(output, `${stem}.script.md`), `# ${e.title}\n\nEpisode ${e.number} of 3. Planned duration: ${clock(duration)}, including pauses and cards.\n\n${delivery}\n\n` + rows.filter(b => b.text).map(b => `## ${clock(b.at)} · ${b.chapter}\n\n${b.text}\n`).join("\n"));
    writeFileSync(join(output, `${stem}.screenplay.md`), `# ${e.title}\n\nEpisode ${e.number}. Duration: ${clock(duration)}. Simulator: ${e.url}\n\n` + rows.map(b => `## ${clock(b.at)} to ${clock(b.end)} · ${b.chapter}\n\nPicture: ${cue(b.action)}\n\n${b.voiceEnd ? `Voice: ${clock(b.voiceAt)} to ${clock(b.voiceEnd)}. Then pause until the next shot.\n\n` : ""}${b.text || b.cue}\n`).join("\n"));
    writeFileSync(join(output, `${stem}.prompter.html`), prompter(e, rows));
    for (const kind of ["intro", "outro"]) writeFileSync(join(output, `${stem}.${kind}.html`), card(e, kind));
    index.push(`<article><h2>${esc(e.title)}</h2><p>Episode ${e.number} · ${clock(duration)}</p><a href="${stem}.prompter.html?picture=1">Rehearse with picture</a> · <a href="${stem}.prompter.html">Teleprompter</a> · <a href="${stem}.script.md">Narration</a> · <a href="${stem}.screenplay.md">Picture cues</a> · <a href="${stem}.mp4">Picture reel</a> · <a href="${stem}.srt">Subtitles</a> · <a href="${e.url}">Live simulator</a><p><a href="${stem}.voice.txt">Plain voice script</a> · <a href="${stem}.intro.png">Opening card</a> · <a href="${stem}.outro.png">Closing card</a> · <a href="${stem}.cues.json">Cue timeline</a></p></article>`);
    console.log(`Episode ${e.number}: ${clock(duration)}, ${rows.filter(b => b.text).reduce((n,b) => n + b.text.split(/\s+/).length, 0)} words`);
  }
  writeFileSync(join(output, "index.html"), `<!doctype html><html lang="en"><meta charset="utf-8"><title>ARTF recording materials</title><style>body{background:#f3f0e7;color:#161813;font:20px/1.6 system-ui;max-width:1050px;margin:70px auto;padding:0 30px}h1,h2{font-weight:500;line-height:1.3}h1{font-size:42px}h2{font-size:28px}article{border-top:1px solid #c8c7bf;padding:24px 0}a{color:#525c2c}p{color:#666c60}</style><h1>The ARTF series finishes with two episodes.</h1><p>Episode 1 showed curation, deal removal, and shading. These followups explain the host's checks, then coordination around one auction. <a href="recording-guide.md">Read the recording guide.</a> <a href="linkedin-posts.txt">Open the LinkedIn drafts.</a></p>${index.join("")}<p>Both episodes include the full narration as burned-in captions. Picture reels contain no voice. Captions and SRT files follow the planned script timing.</p></html>`);
}
if (process.argv[1] === fileURLToPath(import.meta.url)) build();
