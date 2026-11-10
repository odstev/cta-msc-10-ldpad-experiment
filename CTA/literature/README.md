# Literature correction: author-review working files

This package carries forward the correction 5 review copy. The original `data/matrix.csv`, database exports and coder decisions were not present in the supplied project. The new files are a scaffold and are not a reconstruction of unavailable per-paper coding. The historical 19-implementation denominator and the identities of the six exclusions cannot yet be verified.

## Rebuild

From the manuscript directory, install Python 3 and Matplotlib, then run:

```bash
python3 literature/rebuild.py
```

The script regenerates `generated_matrix.tex`, `generated_counts.tex`, `generated_evidence.tex` and the flow illustration. It does **not** search a database, verify quotations, judge study eligibility or perform screening. Compile `cta.tex` with pdfLaTeX twice afterwards. The already-generated outputs are included for Overleaf and do not require Python merely to compile the manuscript.

Structural checks can be run with:

```bash
python3 -m unittest discover -s literature/tests -v
```

## What the files mean

- `papers.csv` transcribes the 25 bibliography entries, listed venues and years. The initial `b24` publication date and availability by the reported search date are supported by the IEEE publisher record. Other dates, the six implementation exclusions, and implementation decisions remain unassessed.
- `coding.csv` has one row for each of the seven criteria for each bibliography entry. `U` means unknown, `Y` means the positive criterion is present, and `N` means it is not. A `Y` or `N` requires a source URL, location, and short supporting extract in that row. Use full-text evidence and context when making an absence decision; an abstract that does not mention a feature is not proof of absence.
- `source_observations.csv` contains a limited, identifiable observation from the IEEE abstract for reference 3. This names an ESP8266 but does not establish that a physical evaluation was completed.
- `protocol.json` stores the date range and the **reported** historical cutoff from the prior draft. The range is used by the script to check entered date-screening decisions; this does not establish that those decisions occurred historically. A new search needs its actual run date, database-specific strings, results and screening decisions.
- `legacy_paper_summaries.tex` preserves the earlier narrative for comparison. Its paper numbering and substantive descriptions have not been source-verified and it is not included in this review PDF.
- `tests/test_rebuild.py` uses synthetic values in temporary directories to check calculations and error handling. It never writes those values into the supplied data.

## Required evidence to complete the correction

1. The original `data/matrix.csv`, including eligibility and the six excluded reference IDs; the full papers or stable links for each decision.
2. Original database query strings, search dates, result exports, duplicate records and stage-by-stage screening decisions, including reasons for full-text exclusions.
3. The actual second-coder sampling frame, number and identity of records, both coders' original decisions and adjudications. The prior draft reported 20% and kappa 0.81 but did not supply these records.
4. An independently checked source extract and locator for each coded cell. Complete date screening before defining the implementation denominator. Rebuild the output and review whether every numerator and denominator is supported by the underlying sources.

The script checks whether evidence fields are populated; it cannot determine whether a quotation really appears in a source or supports a classification. The source zip, code and manuscript changes are AI-assisted working materials; the authors must follow their applicable course and publication rules.
