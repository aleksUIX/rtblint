# Record the final two ARTF episodes.

Episode 1 stays as recorded. The remaining material becomes episode 2, 5:39,
and episode 3, 6:26. Both include opening and closing cards and deliberate
pauses for the simulator. Episode 2 uses 435 words and episode 3 uses
513 words. Both run at about 110 words per minute.

Episode 2 combines the previous validation and envelope scripts. Episode 3
combines fan-in, isolated shaders, the delivery boundary, deployment, and
operations. The earlier scripts and reels remain as source material.

## The followups build on the recorded examples.

Episode 1 introduced curation, removing a deal, shading, and the timing of
one host extension. Episode 2 follows those same examples from the agent's
proposal through acceptance, apply, and checks of the resulting OpenRTB.
Episode 3 splits the familiar work among specialists and follows the combined
request through buyers, shading, and downstream delivery.

Read the bridge sentences as part of the narration. They establish which
lesson carries forward. The scripts explain seller and buyer platforms on
first use, distinguish a skipped proposal from an invalid result, and give
the viewer a concrete next step: trace one auction before increasing traffic.
Episode 2 uses short narration windows and brief pauses to inspect each
result before the next shot. The prompter marks when to speak and when to
pause. Episode 3 has a newly recorded picture at the same unhurried reading pace,
with at least three seconds between spoken sections.

Episode 2 introduces OpenRTB, ARTF, the host, and the specialist service for
viewers who are new to the framework. Each example connects the proposed
change to its effect: campaign consideration, bid eligibility, or keeping
an invalid request away from buyers. The narration describes possible
effects, without promising higher revenue or better campaign performance.

The videos teach the integration through these examples. The separate
`linkedin-posts.txt` supplies the symposium context for publishing each video.
The post explains why the four parts announced with episode 1 became three.
Use the symposium wording while the event is upcoming.

## Open the recording materials.

Run these commands from `rtblint/demo/artf`:

```sh
npm ci
npx playwright install chromium --only-shell
npm run series:build
npm run studio:up
npm run studio
```

The studio opens at http://localhost:8070. It links both scripts, screenplays,
teleprompters, subtitles, and picture reels. Keep this server running while
using the browser studio. Both prompters also open directly from
`materials/episode-2.prompter.html` or `materials/episode-3.prompter.html`.
Their video, script, styles, and playback code are embedded. It verifies and preloads the complete video before
enabling Start. If preview scripting is disabled, the full script remains readable.

The stacks can run together:

- Episode 2 console: http://localhost:8080/?ep=2
- Episode 2 Grafana: http://localhost:3001
- Episode 3 lab: http://localhost:8081/lab/?ep=3
- Episode 3 Grafana: http://localhost:3002

[Docker Compose 2.24.4 or later](https://docs.docker.com/reference/compose-file/merge/#replace-value) supports the recording port overrides. The
ports bind locally. The lab has separate internal request and shader
networks. Recording mode changes the presentation; it runs the actual
simulator APIs and does not fake verdicts or telemetry.

## Rehearse against the prepared picture.

Choose **Rehearse with picture** in the studio for the episode you are
recording. The page shows the silent picture beside the narration. One
Start button controls both. Pause, reset, and chapter seeks stay synchronized.
The teleprompter can also hide the picture for a dedicated camera display.
Both prompters continue following the video clock when the picture is hidden.
Each highlighted phrase uses the exact subtitle boundaries burned into the MP4.
The app verifies the video and script before enabling Start take, then
plays the downloaded video without depending on later network requests.

Use Recording mode for either episode. It locks playback to 100%. Start from zero.
The opening card is eight seconds. The first spoken words begin at 0:09.
Read only the large narration text, following the highlighted phrase.
Silent beats allow the simulator action to finish. Leave those pauses in
your audio. Captions appear one second after each narration shot starts.
Read at about 110 words per minute. Every paragraph has enough room at that
pace, with at least three seconds between spoken sections. Pause when the
highlight clears. Rehearsal mode allows a slower practice run.
Space pauses the prompter. Left and right change beat. The beat
selector jumps to a chapter. Mirror and fullscreen support a camera setup.

The script, picture cues, subtitle timing, and prompter all come from
`episodes.mjs`. Edit that file and rebuild to keep them aligned. The 90%
rehearsal pace runs longer than the supplied picture. Use 100% for a final
take. A take that runs long needs editing rather than automatic speed-up.
The video is the clock in both modes for either episode. Hiding the picture
changes only the layout. Recording pauses if the tab is hidden or playback
is interrupted. For a continuous voice take, keep the prompter visible and
reset after an interruption. Pausing this app does not pause an external
audio recorder, so a resumed voice take needs its extra silence trimmed.

## Capture your narration.

Use the same microphone setup as episode 1. Record a continuous voice track
while following the reel, including the opening silence and action pauses.
Record the camera separately if you want a talking-head layer. The picture
reels are 1920 by 1080 at 25 fps.

Export the voice as M4A or WAV. Trim any extra recorder lead-in so the first
spoken word lands at 0:09 on the picture timeline. Combine a take that
starts at timeline zero:

```sh
node series/mux.mjs 2 /absolute/path/episode-2-voice.m4a
node series/mux.mjs 3 /absolute/path/episode-3-voice.m4a
```

If the audio file starts with the first spoken word and retains every later
pause, add `--offset 9`. The helper adds leading silence, pads the ending,
and copies the video. It refuses to truncate a voice track that extends past
the picture. It does not repair pacing within a take.

The outputs are `materials/episode-2.narrated.mp4` and
`materials/episode-3.narrated.mp4`. Both have the full narration burned
into the picture as 64-pixel white text on an 82% opaque black background,
using episode 1's subtitle font. The mux helper preserves those captions
when it adds your voice. They follow the
planned script timing, including the silent action beats and inspection pauses.

Throughout both episodes, a 56-pixel circular portrait and
`Now presenting: Alex Sekowski` remain at the lower right, including
pauses and title cards. The portrait, Geist font, and blue outline match
the opening card. A 312-pixel footer reserves separate areas for subtitles
and the presenter. Subtitles sit
107 pixels above the bottom, about one single-line subtitle box height,
and clear during pauses and title cards.
The export enlarges and focuses the actual simulator pane relevant to each beat. The
saved picture master retains the full simulator framing for future edits.
The descriptor shot shows declared intents first, then the actual image and resource fields.
At a 360-pixel display width the captions have a nominal 12-pixel font size.
This is our editorial readability target, not an IAB or LinkedIn requirement.

The matching SRT remains available for editing and accessibility. Use the
final voice take to check caption timing before publishing. The clean
`episode-2.picture.mp4` and `episode-3.picture.mp4` preserve the simulator
captures without captions.
The opening and closing cards reuse episode 1's fonts, blue edge, panels,
and author portrait. Both episodes start with the opening card fully visible
from the first frame. To rebuild a presentation from its saved capture, run
`npm run finish:two` or `npm run finish:three`. If picture actions or shot
lengths change, use the recording command to capture the new timeline.

Check the caption boundaries, reading layouts, hidden picture playback,
and script/video version matching with `npm run prompter:check -- 3`.
Run `npm run prompter:check -- 3 --full-take` for a complete 100% playback.
Episode 3's standalone prompter embeds the exact captioned video, so it
can run without a server or later network requests.

## Re-record or present the simulator live.

```sh
npm run series:check
npm run record:two
npm run record:three
```

`series:check` runs every picture action without waiting for narration. It
checks the UI, expected outcomes, and chapter views, and saves screenshots.
The recording commands follow the full timeline. They overwrite only their
new episode picture reels, keeping episode 1 and the earlier cuts intact.

For a live presentation, follow `episode-N.screenplay.md`. Episode 2 buttons
run envelope, host policy, wrong response, applied object, and bid response.
Click an episode 2 mutation card to inspect its operation, target, and actual
payload. The recording highlights the call id and budget, shows the wrong
response id, and traces the string metric to the resulting OpenRTB finding.
The recorder checks DSP receipts and keeps the real scene responses in
`episode-2.recording.json`. The completed MP4 replaces the previous reel only
after capture and encoding finish.

Episode 3's view tabs show apply, tickets, bids, RPC hops, the descriptor,
delivery, and operations. Start 20/s on Operations and stop after the shot.
Traffic stops when the page closes. This driver requests twenty per second;
Prometheus reports the achieved rate separately.

## Keep the spoken claims tied to the implementation.

The episode 2 agent replays HTTP JSON fixtures. RTBlint runs real gRPC.
The string metric is a JSON integration regression case. ARTF's typed
Metric.value cannot carry a string. The policy scene skips an unoffered
proposal. The wrong-call scene rejects the response before applying changes. It leaves
the original request and buyer receipt count unchanged.

Episode 3 agents use real typed GetMutations. This lab's patcher, fallback
choices, budgets, first price auction, and network arrangement are examples.
Both the SSP and DSP check response identity and offered intents before apply.
At the HTTP boundary, known OpenRTB flags use integer 0/1 values. The adapter
converts those values back to protobuf booleans for typed agent calls and
preserves extension booleans. The captured outbound request is checked with
RTBlint separately. The local validator includes a catalog correction for
Video.skip: its legal flag values are 0 and 1, not creative attribute codes. It does not include RTBlint in its live auction path.
The clean agents mostly
write separate fields. Configured order is not a complete production
conflict resolution policy. The bid view includes filtered bids and their
rejection reasons. Summary and tracker events use the SSP auction id.

VAST fetching and pixel firing are scripted. Media, identity, consent, and
measurement are stubs. A tracker event does not prove actual playback.
The descriptor is a sample declaration, not an installed public image.
Observed telemetry is synthetic local traffic, not a production benchmark.
The operations panel counts all failed buyer calls, not just timeouts.
Its feed closeup labels the agent table as a sample of the first three rows.
The full monitoring data remains available in the live lab. The delivery
closeup similarly labels the latest four events; the tracker retains the full list. Its
proposed and applied mutation counters both cover SSP request mutations.

Primary reference: [IAB Tech Lab ARTF v1.0 and its protobuf sources](https://github.com/IABTechLab/agentic-real-time-framework).
The API and independent acceptance come from that contract. The demos and
operating choices are this workspace's implementations.
The floor and bid amounts follow [OpenRTB 2.6's CPM price fields](https://github.com/InteractiveAdvertisingBureau/openrtb2.x/blob/main/2.6.md#objectdeal),
and the fixtures use USD. Episode 2 defines the per-thousand-impressions unit
when revisiting the deal floor.

The introduction follows [IAB Tech Lab's ARTF overview](https://iabtechlab.com/standards/artf/).
The discussion of audience labels and minimum prices follows
[OpenRTB 2.6's Data and Deal objects](https://github.com/InteractiveAdvertisingBureau/openrtb2.x/blob/main/2.6.md).

To stop the recording stacks after your session:

```sh
npm run studio:down
```
