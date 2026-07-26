# Artifacts: How Machine-Checkable Is OpenRTB?

Dataset and scripts for the preprint *How Machine-Checkable Is OpenRTB?
Classifying the Normative Content of the Protocol That Clears Real-Time
Advertising* (Sekowski, July 2026).

## Contents

| File | What it is |
| --- | --- |
| `data/statements_final.csv` | 417 normative-keyword sentences from OpenRTB 2.6-202606 and 3.0 with location, obligation strength, heuristic pre-label with rationale, and final hand-coded class (A/B/C/D/X) |
| `data/field_constraints.csv` | every object-table field row parsed from both spec texts |
| `data/catalog_versions.csv` | objects/fields/required/recommended/deprecated per release, 2.0 through 2.6-202606 plus 3.0 |
| `data/final_stats.json` | all aggregate numbers quoted in the paper, including deduplicated distributions |
| `data/spec_examples.json` | validation results for the 9 example payloads embedded in the 2.6-202606 text (2 invalid) and defect persistence across releases |
| `data/reliability_sample.csv` | the 60-statement reliability sample (seed 42) |
| `data/coder1_labels.csv`, `data/coder2_labels.csv` | two blind recodings (independent runs of a large language model given only the codebook) |
| `data/reliability_results.json` | percent agreement, Cohen's kappa, and the disagreement list |
| `data/rtblint_rules.json` | the 96 rule ids and 43 AdCOM list names of the validator instrument |
| `figures/` | the three paper figures |

## Reproduction

Run from this directory against a checkout of this repository (the scripts
read the archived spec texts in `.openrtb-specs/` and the catalogs in
`crates/rtblint-core/specs/`):

```bash
python3 extract.py        # statements.csv, field_constraints.csv, summary.json
python3 finalize.py       # applies the 417 hand-coded labels, final_stats.json
python3 catalog_stats.py  # catalog_versions.csv, rtblint_rules.json
python3 spec_examples.py  # validates the spec's own examples (needs target/release/rtblint)
python3 reliability.py    # kappa computation
python3 figures.py        # figures (needs matplotlib)
```

The classification codebook is Section 3.3 of the paper. Class labels:
A = JSON Schema on a single document, B = stateless lint on a single
message, C = cross-message/cross-artifact/runtime, D = no machine-decidable
criterion, X = not a conformance statement.
