(() => {
  const plan = JSON.parse(document.getElementById("plan").textContent);
  const el = id => document.getElementById(id);
  const video = el("picture");
  const flat = value => value.trim().replace(/\s+/g, " ");
  const fmt = seconds => Math.floor(seconds / 60) + ":" + String(Math.floor(seconds % 60)).padStart(2, "0");
  // A decoded seek can report a timestamp a few microseconds below the frame.
  const snap = time => Math.min(plan.total, Math.max(0, Math.floor((time + 0.00001) * plan.fps) / plan.fps));
  const recording = () => el("mode").value === "recording";
  let ready = false, wantsPlay = false, starting = false, hasPlayed = false, generation = 0;
  let waitTimer;
  let at = 0, presentedAt = 0, presentedWall = -Infinity, renderedRow = -1, renderedCue = -2, objectURL;
  let spans = [];
  const status = text => { el("status").textContent = text; };

  function rowIndex() {
    const index = plan.rows.findIndex(row => at < row.end);
    return index < 0 ? plan.rows.length - 1 : index;
  }

  function fitText() {
    const container = el("narration");
    const available = el("read").clientHeight - 50;
    let size = el("show-picture").checked ? 46 : 60;
    el("text").style.fontSize = size + "px";
    while (container.scrollHeight > available && size > 24) el("text").style.fontSize = --size + "px";
  }

  function renderRow(index) {
    const row = plan.rows[index];
    renderedRow = index;
    renderedCue = -2;
    el("chapter").textContent = row.chapter;
    el("text").replaceChildren();
    spans = [];
    if (row.text) {
      for (const cue of plan.cues.filter(cue => cue.start >= row.at && cue.end <= row.end + 0.001)) {
        const span = document.createElement("span");
        span.textContent = flat(cue.text);
        span.dataset.cue = cue.index;
        if (spans.length) el("text").append(" ");
        el("text").append(span);
        spans.push({ span, cue });
      }
    } else el("text").textContent = "Pause while the picture continues.";
    const next = plan.rows.slice(index + 1).find(row => row.text);
    el("next").textContent = next ? "Next: " + next.text.match(/^[^.!?]+[.!?]?/)[0] : "The episode ends here.";
    fitText();
  }

  function paint() {
    const index = rowIndex(), row = plan.rows[index], frame = Math.round(at * plan.fps);
    if (index !== renderedRow) renderRow(index);
    const current = plan.cues.find(cue => cue.startFrame <= frame && frame < cue.endFrame);
    const cueIndex = current?.index ?? -1;
    if (cueIndex !== renderedCue) {
      for (const { span, cue } of spans) {
        span.classList.toggle("current", cue.index === cueIndex);
        span.classList.toggle("spoken", frame >= cue.endFrame);
      }
      renderedCue = cueIndex;
    }
    const next = plan.cues.find(cue => cue.startFrame > frame);
    if (at >= plan.total) el("cue").textContent = "Finished. Your take ends at " + fmt(plan.total) + ".";
    else if (current) el("cue").textContent = "Read the highlighted phrase. " + Math.ceil(row.voiceEnd - at) + " s remain for this section.";
    else el("cue").textContent = next ? "Pause. Start speaking in " + Math.max(0, next.startFrame / plan.fps - at).toFixed(1) + " s." : "Pause for the closing card.";
    el("clock").textContent = fmt(at) + "." + String(Math.floor((at % 1) * 10)).padStart(1, "0") + " / " + fmt(plan.total);
    el("progress").value = at;
    el("seek").value = index;
    el("toggle").textContent = !ready ? "Preparing" : starting ? "Pause" : wantsPlay ? "Pause" : at >= plan.total ? "Start again" : at > 0 ? "Resume" : recording() ? "Start take" : "Start rehearsal";
    el("toggle").disabled = !ready;
    for (const id of ["reset", "prev", "next-btn", "seek"]) el(id).disabled = !ready || (id !== "reset" && recording() && wantsPlay);
    el("mode").disabled = wantsPlay;
    el("pace").disabled = recording() || wantsPlay;
  }

  function readClock() {
    if (video.ended) at = plan.total;
    else if (!video.seeking) {
      at = !video.paused && performance.now() - presentedWall < 250 ? presentedAt : snap(video.currentTime);
    }
  }

  function pause(message = "Paused. Pause your audio recorder too, or reset for a new take.") {
    clearTimeout(waitTimer);
    generation++;
    wantsPlay = false;
    starting = false;
    video.pause();
    at = snap(video.currentTime);
    status(message);
    paint();
  }

  async function play() {
    if (!ready) return;
    if (at >= plan.total) { video.currentTime = 0; at = 0; }
    const request = ++generation;
    wantsPlay = true;
    starting = true;
    hasPlayed = false;
    presentedWall = -Infinity;
    video.playbackRate = recording() ? 1 : Number(el("pace").value);
    status("Starting the picture and narration together.");
    paint();
    try {
      await video.play();
      if (request !== generation) return;
      starting = false;
      status(recording() ? "Recording at 100%. The full video is loaded." : "Rehearsal. Picture and narration share the same clock.");
      paint();
    } catch (error) {
      if (request !== generation) return;
      pause("Playback could not start. Press Start again.");
    }
  }

  function seek(time) {
    if (!ready || (recording() && wantsPlay)) return;
    pause("Selected section. Reset to zero for a complete take.");
    at = Math.round(Math.min(plan.total, Math.max(0, time)) * plan.fps) / plan.fps;
    presentedWall = -Infinity;
    video.currentTime = at;
    paint();
  }

  plan.rows.forEach((row, i) => el("seek").add(new Option(fmt(row.at) + " · " + row.chapter, i)));
  el("toggle").onclick = () => wantsPlay ? pause() : play();
  el("reset").onclick = () => {
    pause("Ready for a new take. Start your audio recorder, then Start take.");
    video.currentTime = 0;
    at = 0;
    presentedWall = -Infinity;
    paint();
  };
  el("prev").onclick = () => seek(plan.rows[Math.max(0, rowIndex() - 1)].at);
  el("next-btn").onclick = () => seek(plan.rows[Math.min(plan.rows.length - 1, rowIndex() + 1)].at);
  el("seek").onchange = () => seek(plan.rows[Number(el("seek").value)].at);
  el("mode").onchange = () => {
    pause("Mode changed. Reset for a complete recording.");
    if (recording()) el("pace").value = "1";
    video.playbackRate = recording() ? 1 : Number(el("pace").value);
    paint();
  };
  el("pace").onchange = () => { video.playbackRate = recording() ? 1 : Number(el("pace").value); };
  el("show-picture").onchange = () => {
    document.body.classList.toggle("with-picture", el("show-picture").checked);
    fitText();
  };
  el("mirror").onchange = () => el("narration").classList.toggle("mirror", el("mirror").checked);
  el("full").onclick = async () => {
    try { await document.documentElement.requestFullscreen(); }
    catch { status("Open this HTML in your browser to use fullscreen."); }
  };
  document.addEventListener("fullscreenchange", () => { document.body.classList.toggle("full", !!document.fullscreenElement); fitText(); });
  window.addEventListener("resize", fitText);
  document.addEventListener("keydown", event => {
    if (["INPUT", "SELECT"].includes(event.target.tagName)) return;
    if (event.code === "Space") { event.preventDefault(); el("toggle").click(); }
    if (event.key === "Home") { event.preventDefault(); el("reset").click(); }
    if (event.key === "ArrowLeft") { event.preventDefault(); el("prev").click(); }
    if (event.key === "ArrowRight") { event.preventDefault(); el("next-btn").click(); }
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden && wantsPlay && recording()) pause("Take paused because the prompter tab is hidden. Reset for a fresh take.");
  });
  video.addEventListener("seeked", () => { presentedWall = -Infinity; at = snap(video.currentTime); paint(); });
  video.addEventListener("playing", () => { hasPlayed = true; clearTimeout(waitTimer); });
  video.addEventListener("waiting", () => {
    if (ready && wantsPlay && hasPlayed && !video.seeking && recording()) {
      const request = generation;
      clearTimeout(waitTimer);
      waitTimer = setTimeout(() => {
        // Let the final silent fade reach the native ended event.
        if (request === generation && wantsPlay && !video.seeking && video.readyState < 3 && plan.total - video.currentTime > .5) pause("Playback was interrupted. Reset for a fresh take.");
      }, 150);
    }
  });
  video.addEventListener("pause", () => { if (wantsPlay && video.paused && !video.ended && !starting) pause(); });
  video.addEventListener("ended", () => { wantsPlay = false; starting = false; at = plan.total; status("Finished. Save your voice take."); paint(); });
  video.addEventListener("error", () => { if (ready) pause("Video playback failed. Reload the prompter before another take."); });
  video.addEventListener("ratechange", () => { if (recording() && video.playbackRate !== 1) video.playbackRate = 1; });

  function frameCallback(now, metadata) {
    presentedAt = snap(metadata.mediaTime);
    presentedWall = performance.now();
    if (ready && !video.seeking && !video.paused) { at = presentedAt; paint(); }
    video.requestVideoFrameCallback(frameCallback);
  }
  if (video.requestVideoFrameCallback) video.requestVideoFrameCallback(frameCallback);
  function tick() {
    if (ready && !video.seeking) { readClock(); paint(); }
    requestAnimationFrame(tick);
  }

  async function sha256(bytes) {
    if (!globalThis.crypto?.subtle) throw new Error("Open this HTML in Chrome or Safari to record.");
    return Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)), byte => byte.toString(16).padStart(2, "0")).join("");
  }
  async function prepare() {
    try {
      if (!plan.embedded) throw new Error("The picture reel is missing. Finish the episode video, then rebuild the prompter.");
      const report = plan.presentation, captions = plan.captionReport;
      if (report.cueSha256 !== plan.cueSha256 || report.subtitleSha256 !== plan.subtitleSha256 || report.seconds !== plan.total) throw new Error("The video and script need a matching rebuild before recording.");
      if (captions.cues.length !== plan.cues.length || captions.cues.some((cue, i) => cue.start !== plan.cues[i].start || cue.end !== plan.cues[i].end || flat(cue.text) !== flat(plan.cues[i].text))) throw new Error("The subtitle timing does not match this script. Rebuild the episode video.");
      if (flat(plan.rows.filter(row => row.text).map(row => row.text).join(" ")) !== flat(plan.cues.map(cue => cue.text).join(" "))) throw new Error("The narration and captions do not match.");
      const binary = atob(document.getElementById("video-data").textContent.trim());
      const decoded = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) decoded[i] = binary.charCodeAt(i);
      const bytes = decoded.buffer;
      if (await sha256(bytes) !== report.sha256) throw new Error("The video changed. Reload the recording materials before starting.");
      status("The video and script match. Preparing playback.");
      objectURL = URL.createObjectURL(new Blob([bytes], { type: "video/mp4" }));
      await new Promise((resolve, reject) => {
        const timer = setTimeout(() => { cleanup(); reject(new Error("The video did not become ready. Reload the prompter.")); }, 45000);
        function cleanup() { clearTimeout(timer); video.removeEventListener("canplay", loaded); video.removeEventListener("error", failed); }
        function loaded() { cleanup(); resolve(); }
        function failed() { cleanup(); reject(new Error("This browser could not play the video. Try Chrome or Safari.")); }
        video.addEventListener("canplay", loaded);
        video.addEventListener("error", failed);
        video.src = objectURL;
        video.load();
      });
      if (Math.abs(video.duration - plan.total) > 1 / plan.fps || video.videoWidth !== 1920 || video.videoHeight !== 1080) throw new Error("The video length or format does not match the script.");
      ready = true;
      status("Ready. The complete video is loaded and matches the script.");
      paint();
    } catch (error) { ready = false; status(error.message); paint(); }
  }
  document.body.classList.add("interactive", "with-picture");
  if (new URLSearchParams(location.search).get("picture") === "0") {
    el("show-picture").checked = false;
    document.body.classList.remove("with-picture");
  }
  window.artfPrompter = { get state() { return { ready, at, running: wantsPlay, section: rowIndex(), cue: renderedCue, mode: el("mode").value }; } };
  paint();
  requestAnimationFrame(tick);
  prepare();
})();
