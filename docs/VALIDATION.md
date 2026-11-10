# Packaging validation — 8 November 2026

Executed in this packaging session:

- `make reproduce`: successful.
- Historical SHA-256 input inventory: 42 inputs verified.
- Seeded trace reanalysis: 300/300 matches and 198/13,800 eligible non-injection alerts.
- VM raw observations plus SQLite log reanalysis: rounded latency tables match the retained originals.
- Fusion traces, transition records and 1,771-vector grid reanalysis: passed.
- Statistical and CSV validation tests: 8 passed.
- Literature rebuild tests: 4 passed (this tests coding mechanics, not literature completeness).
- New command-line helpers: parsed and help interfaces checked.
- Manuscript-value regression assertions: passed.
- The VM raw CSV exactly matches the original ZIP retained within the recovered package.
  The separately retrieved earlier latency upload is different and has an inactive sync probe;
  it is preserved separately, not used to regenerate the morning manuscript values.

Not executed in this packaging session:

- Fresh Rust compilation / `make verify-rust`: Rust/Cargo is unavailable in this runtime.
- New latency timing, K3s deployment or live node-resource capture: no access to the user's VM.
- GitHub Actions: workflow supplied, not yet run on GitHub.

The inherited `evidence_outputs/rust_recheck.json` reports an earlier verification;
it must not be described as a fresh result from this packaging session. Run
`make verify-rust` on the Ubuntu VM to perform the hash comparison again.
