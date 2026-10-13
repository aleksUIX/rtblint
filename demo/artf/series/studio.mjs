import { spawnSync } from "node:child_process";
import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { build, output } from "./build.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const lab = resolve(root, "../../../artf-simulator");
const command = process.argv[2] || "serve";
if (["up", "down"].includes(command)) {
  const stacks = [
    { cwd: root, files: ["compose.yaml", "recording.compose.yaml"], project: "artf-sim" },
    { cwd: lab, files: ["deploy/compose.yaml", "deploy/recording.compose.yaml"], project: "artf-recording" },
  ];
  for (const stack of stacks) {
    const args = ["compose", "-p", stack.project, ...stack.files.flatMap(f => ["-f", f]), ...(command === "up" ? ["up", "-d", "--build", "--wait"] : ["down"])];
    const result = spawnSync("docker", args, { cwd: stack.cwd, stdio: "inherit" });
    if (result.status !== 0) throw new Error(`${stack.project} ${command} failed`);
  }
  console.log(command === "up" ? "Episode 2: http://localhost:8080/?ep=2\nEpisode 3: http://localhost:8081/lab/?ep=3" : "Recording stacks stopped");
} else if (command === "serve") {
  build();
  const types = { ".html": "text/html; charset=utf-8", ".md": "text/plain; charset=utf-8", ".txt": "text/plain; charset=utf-8", ".json": "application/json", ".srt": "text/plain; charset=utf-8", ".mp4": "video/mp4", ".png": "image/png", ".woff2": "font/woff2" };
  createServer((req, res) => {
    const pathname = new URL(req.url || "/", "http://localhost").pathname;
    let file;
    if (pathname.startsWith("/linkedin/fonts/")) file = resolve(root, "." + pathname);
    else file = resolve(output, "." + (pathname === "/" ? "/index.html" : pathname));
    const allowed = file.startsWith(output + sep) || file.startsWith(resolve(root, "linkedin/fonts") + sep);
    if (!allowed) { res.writeHead(403); res.end("Forbidden"); return; }
    try {
      const data = readFileSync(file);
      const range = req.headers.range?.match(/^bytes=(\d+)-(\d*)$/);
      const headers = { "content-type": types[extname(file)] || "application/octet-stream", "accept-ranges": "bytes", "cache-control": "no-store" };
      if (range) {
        const start = Number(range[1]), end = Math.min(range[2] ? Number(range[2]) : data.length - 1, data.length - 1);
        if (start > end || start >= data.length) { res.writeHead(416, {"content-range": `bytes */${data.length}`}); res.end(); return; }
        res.writeHead(206, { ...headers, "content-range": `bytes ${start}-${end}/${data.length}`, "content-length": end-start+1 }); res.end(data.subarray(start,end+1));
      } else { res.writeHead(200, { ...headers, "content-length": data.length }); res.end(data); }
    } catch { res.writeHead(404); res.end("File unavailable. Generate the materials or record the picture reel first."); }
  }).listen(8070, "127.0.0.1", () => console.log("Recording materials: http://localhost:8070"));
} else throw new Error("Use studio.mjs up, down, or serve");
