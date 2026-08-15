# Nobody Is Checking: measurement artifacts

Capture harness, analysis pipeline, and PII-free datasets for the preprint
*Nobody Is Checking: Measuring OpenRTB Conformance in Live Programmatic
Traffic* (Sekowski, 2026). Companion to the machine-checkability study in
`../openrtb-checkability/` (doi:10.13140/RG.2.2.27937.57448).

## What is here

| | |
|---|---|
| `dataset_sampleA/` | random sample, wave 0: issues, payloads, endpoints |
| `dataset_sampleB/` | purposive sample, wave 0 |
| `waves/*-dataset/` | the four repeat waves, each analysed independently |
| `frame_tranco.json`, `sites-tranco.txt` | sampling frame with its seed |
| `sites.txt` | the purposive publisher list |
| `figures/` | the three figures as published |
| `*.py`, `*.mjs` | capture harness and every analysis in the paper |

Each dataset ships its own README with provenance and the pinned validator
version. Rows carry rule identifiers and structural JSON paths only, never
payload values, with a filter that redacts anything resembling an
identifier.

## What is not here, and why

The raw capture corpus is withheld. Bid requests carry user identifiers,
extended IDs, address-derived geolocation, and commercial terms including
floor prices and deal identifiers. Publishing it would expose the
operator's own browsing and third-party commercial terms. It is available
to researchers on request.

## Reproducing

Requires **RTBlint 0.6.0 or newer, release build**. Older builds ship
degraded 2.0-2.5 catalogs; because best-fit attribution selects the
version with fewest errors, a weak catalog silently wins and shifts every
result. The paper documents this failure because it happened to us.

```bash
python3 clustered.py dataset_sampleA     # headline rate + site-clustered CI
python3 sensitivity.py                   # rate with disputable classes removed
python3 stability.py                     # all six waves, analysed separately
python3 nobid.py                         # the no-bid/invalid identity
python3 origin.py                        # adapter vs ecosystem vs publisher
python3 bidoutcome.py                    # does non-conformance cost a bid?
python3 hypothesis.py                    # the companion study's predictions
```

Re-capturing needs a residential connection. Datacenter and VPN exits are
scored as invalid traffic and observe zero auctions, which the paper
reports as a measurement precondition. `vantage.py` fails closed if the
network does not look residential.

## Note on script defaults

Several scripts default to Sample B paths for historical reasons: it was
the pilot corpus. Pass the dataset explicitly when reproducing a Sample A
number. Quoting a Sample B figure as though it were general was the most
common error found in review.
