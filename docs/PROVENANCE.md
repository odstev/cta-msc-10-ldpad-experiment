# Evidence and source provenance

Retrieved 8 November 2026:

- `LDPAD_Correction_7_Reproducibility_Package.zip`, retained Library version 9:
  original Rust source, both Cargo lockfiles, labelled detector trace, fusion
  scenarios and traces, existing protocol bundles, analysis and test code.
- `laptop_vm_20261102_original.zip` inside the recovered package: its original
  `vm_pilot_01/raw.csv` is byte-identical to the archived CSV used in the morning
  manuscript (SHA-256 4d035715df3ae41c41bf111d9657b6b1f6186c7a9f662d2426e88637e6498f2b).
- The separately retrieved `latency_results.zip` is an earlier, different capture
  with the sync probe inactive. It is retained as `evidence/earlier_latency_upload.zip`
  for provenance and is not mixed with the manuscript's later instrumented run.
- `CTA_Manuscript_Overleaf_Source.zip`, morning version 15: manuscript master.
- `k3s_node_replay.rs`: previously supplied example; only its three unit tests
  were shown running in the session, not an actual trace replay.
- K3s aggregate counts and node snapshots: user-supplied screenshots, explicitly
  transcribed into `evidence/k3s/`. Original VM CSV remains unavailable here.

The original `protocol/input_sha256.txt` remains intact and checks 42 historical
inputs. The release inventory is retrospective and is not a preregistration.
`evidence_outputs/rust_recheck.json` is a historical report inherited from the
source package; it is not evidence of a fresh Rust build during this packaging.
A successful `make verify-rust` replaces that report with the new check's time.

The new wrappers run original experiment code in isolated copies. They never
alter a recorded result to make a new observation match the manuscript. Expected
values in `analysis/check_reported_results.py` are regression assertions over
computed results, not observations fed into the detector or latency harness.

Command references:
- https://doc.rust-lang.org/cargo/commands/cargo.html
- https://kubernetes.io/docs/reference/kubectl/generated/kubectl_cp/
- https://kubernetes.io/docs/reference/kubectl/quick-reference/
