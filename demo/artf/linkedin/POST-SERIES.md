I'm building a production ad stack with ARTF in the auction, before a winner is chosen. @IABTechLab published the spec so the agent runs inside the SSP, DSP, or exchange that already holds the bid. On the way out I call a few agents in parallel against the same bid request. On the way back each DSP runs its own shade. Only the host writes OpenRTB. After the win, delivery is VAST. ARTF does not go past the auction.

I'm opening the stack. If you are putting this next to an exchange or a DSP, you should be able to clone it and see the hop run, including the step the spec left out: apply what the agent sent, then check that the auction is still valid OpenRTB.

I shot four clips of the simulator and I'll publish them in order.

The first one is where the agent sits. It is a container inside the SSP, DSP, or exchange you already run. It proposes edits. Buyers bid on the ticket you actually sent.

The second is what happens after those edits. The call can succeed and the ticket can still be illegal OpenRTB. I run that failure, then the same hop when the ticket is clean.

The third is the call. The host says which stage this is, how long the agent has, and which writes it will even consider. The agent sends patches. The host still accepts or rejects each one.

The fourth is the shape I actually run. Several agents answer on the request. Each DSP shades its own bid. Nobody but the host writes the ticket.

The TV is a dummy. The SSP, the agents, and the DSPs are stubs. The traffic is synthetic. The apply path is real gRPC.

I'll put the first clip up next.

#ARTF #OpenRTB #IABTechLab #AdTech #Programmatic #CTV #OpenSource
