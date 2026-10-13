import { spawnSync } from "node:child_process";
import { join } from "node:path";

// Recognize the blue series edge, older gold edge, and the card's black fades.
// Total capture duration also includes browser shutdown holds, so it is not a start marker.
export function firstSimulatorAt(source, episode) {
  if (episode === 3) {
    // The lab is dark. Its bright header text distinguishes it from a black fade.
    const width = 200, height = 16, bytesPerFrame = width * height * 3;
    const probe = spawnSync("ffmpeg", ["-v", "error", "-i", source, "-t", "30", "-vf", `fps=25,crop=1000:80:0:0,scale=${width}:${height},format=rgb24`, "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], { maxBuffer: 16 * 1024 * 1024 });
    if (probe.status) throw new Error(probe.stderr.toString());
    let transition = -1;
    for (let frame = 0; frame < probe.stdout.length / bytesPerFrame; frame++) {
      const pixels = probe.stdout.subarray(frame * bytesPerFrame, (frame + 1) * bytesPerFrame);
      const edge = pixels.subarray((height - 1) * width * 3, (height - 1) * width * 3 + 3);
      let bright = 0;
      for (let i = 6; i < pixels.length; i += 3) if (Math.min(...pixels.subarray(i, i + 3)) > 110) bright++;
      if (Math.max(...edge) < 70 && bright >= 5) {
        if (transition < 0) transition = frame;
        if (frame - transition >= 12) return transition / 25;
      } else transition = -1;
    }
    throw new Error("Cannot locate the lab's bright header on its dark background.");
  }
  const probe = spawnSync("ffmpeg", ["-v", "error", "-i", source, "-t", "30", "-vf", "fps=25,crop=10:10:0:100,scale=1:1,format=rgb24", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], { maxBuffer: 1024 * 1024 });
  if (probe.status) throw new Error(probe.stderr.toString());
  let seenCard = false, transition = -1;
  for (let frame = 0; frame < probe.stdout.length / 3; frame++) {
    const [r, g, b] = probe.stdout.subarray(frame * 3, frame * 3 + 3);
    const card = (r > 130 && b < 150 && r - g > 12 && g - b > 20)
      || (b > 130 && r < 120 && b - r > 80)
      || (r < 70 && g < 70 && b < 70);
    if (card) { seenCard = true; transition = -1; }
    else if (seenCard) {
      if (transition < 0) transition = frame;
      if (frame - transition >= 12) return transition / 25;
    }
  }
  throw new Error("Cannot locate the opening card's transition to the simulator.");
}

export function encodePicture({ source, destination, materials, episode, rows, seconds, captions, frame, crf = 19 }) {
  const inputFirstSceneSeconds = firstSimulatorAt(source, episode);
  const correctedFirstSceneSeconds = rows[1].at;
  const shiftSeconds = correctedFirstSceneSeconds - inputFirstSceneSeconds;
  const timing = shiftSeconds >= 0
    ? `fps=25,tpad=start_mode=clone:start_duration=${shiftSeconds}:stop_mode=clone:stop_duration=10`
    : `trim=start=${-shiftSeconds},setpts=PTS-STARTPTS,fps=25,tpad=stop_mode=clone:stop_duration=10`;
  // Explicit stills preserve complete card holds when the first browser paint arrives late.
  const introEnd = rows[0].end;
  const outroStart = rows.at(-1).at;
  let framing = "";
  if (frame) {
    const regions = (frame.foci || []).flatMap(f => f.cuts || [f]);
    const expr = (key, fallback) => regions.filter(f => !f.overview).reduceRight((next, f) => `if(gte(in_time,${f.at})*lt(in_time,${f.end}),${Number(f[key]).toFixed(6)},${next})`, String(fallback));
    framing = `,pad=2700:1080:390:0:color=0xf4f3ee,zoompan=z='${expr("zoom", 1)}':x='${expr("x", 0)}':y='${expr("y", 0)}':d=1:s=${frame.width}x${frame.height}:fps=25,pad=1920:1080:0:0:color=0xf4f3ee,setsar=1`;
  }
  const layers = [
    `[0:v]${timing}${framing},drawbox=x=0:y=0:w=iw:h=ih:color=black:t=fill:enable='lt(t,${introEnd})+gte(t,${outroStart})'[picture]`,
    `[1:v]format=rgba,fade=t=out:st=${introEnd - 0.6}:d=0.6:alpha=1[intro-card]`,
    `[2:v]format=rgba,fade=t=in:st=${outroStart}:d=0.5:alpha=1,fade=t=out:st=${seconds - 0.6}:d=0.6:alpha=1[outro-card]`,
    `[picture][intro-card]overlay=enable='lt(t,${introEnd})'[intro]`,
    `[intro][outro-card]overlay=enable='gte(t,${outroStart})'[cards]`,
    ...(captions ? ["[3:v]fps=25[caption-track]", "[cards][caption-track]overlay=eof_action=pass[reel]"] : ["[cards]null[reel]"]),
  ];
  const result = spawnSync("ffmpeg", ["-y", "-v", "error", "-i", source,
    "-loop", "1", "-framerate", "25", "-i", join(materials, `episode-${episode}.intro.png`),
    "-loop", "1", "-framerate", "25", "-i", join(materials, `episode-${episode}.outro.png`),
    ...(captions ? ["-f", "concat", "-safe", "0", "-i", captions] : []),
    "-filter_complex_threads", "2", "-filter_complex", layers.join(";"), "-map", "[reel]", "-t", String(seconds), "-c:v", "libx264", "-threads", "4", "-preset", "fast", "-crf", String(crf), "-pix_fmt", "yuv420p", "-an", "-movflags", "+faststart", destination], { encoding: "utf8" });
  if (result.status) throw new Error(result.stderr);
  return { inputFirstSceneSeconds, correctedFirstSceneSeconds, shiftSeconds };
}
