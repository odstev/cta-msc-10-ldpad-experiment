# K3s VM pilot: evidence and scope

The manuscript's initial K3s-pod results are transcribed from terminal
screenshots provided during the VM session. The subsequent post-warm-up
count was reported in the session. The later CSV itself has not been supplied
to the manuscript workspace and is not in this archive.

The VM terminal showed the Rust CLI running in K3s test pods. Three unseeded
500-sample runs reported 16, 14 and 18 anomaly-bearing ticks, respectively;
each reported 500 Normal ticks, zero Moderate or Strict ticks, and peak risk
45.00. The later run used `kubectl exec` with `/usr/local/bin/ldpad` in the
`ldpad-persistent` pod. A separate VM test output showed 16 unit tests and
one integration test passing.

The existing seeded_detector example was built in release mode, copied into
the `ldpad-persistent` pod as `/tmp/seeded_detector` and run through
`kubectl exec`. Its output was redirected on the VM to
`~/k3s-seeded-run1/ticks.csv`. The terminal reported 15,001 lines,
including the header. Grouped columns injected_cpu_spike, cpu_alert and
any_alert showed:

| Count | Spike | CPU alert | Any alert |
| ---: | ---: | ---: | ---: |
| 14,502 | 0 | 0 | 0 |
| 194 | 0 | 0 | 1 |
| 4 | 0 | 1 | 1 |
| 300 | 1 | 1 | 1 |

Thus, the displayed aggregate contains 300 same-tick CPU matches out of 300
spikes and 198 alert-bearing ticks out of 14,700 non-spike ticks. The latter
denominator includes warm-up. A subsequent VM filter, excluding the first
30 ticks in each seed, was reported as 13,800 eligible non-spike ticks and
198 alert-bearing ticks: 1.43%. Its two-sided exact binomial 95% interval is
1.24%--1.65% under the assumption of independent ticks; outcomes within a
seed can be dependent. These counts describe ticks, not individual metric
alerts. The new CSV has not yet been independently reviewed.

The test repeats the seeded synthetic detector design from the companion
package. It is not a real attack, live registry workload or real-time
Kubernetes telemetry test. The earlier and later traces have not been
compared row by row, and this rerun must not be counted as a new set of
independent events. The separate laptop-VM loopback latency pilot and its
reported intervals measure a different path.

After the seeded example completed, four whole-node readings from
`kubectl top nodes` were shown in the VM terminal and saved there in
`~/k3s-node-baseline.txt`. The readings below are transcribed from that
screenshot; the saved file itself was not provided for independent review.

| Timestamp (UTC+03:00) | Node CPU | CPU percent | Node memory | Memory percent |
| --- | ---: | ---: | ---: | ---: |
| 2026-11-08 05:06:54 | 269m | 26% | 3973Mi | 67% |
| 2026-11-08 05:07:09 | 60m | 6% | 3970Mi | 67% |
| 2026-11-08 05:07:24 | 48m | 4% | 3970Mi | 67% |
| 2026-11-08 05:07:40 | 34m | 3% | 3987Mi | 67% |

These are measurements for the whole VM node, not LDPAD-specific CPU or
memory usage. The four short-interval snapshots do not establish steady-state
utilisation or an agent overhead. A separate 40-reading capture was reported
on the VM but was not supplied here for analysis. Three tests of a new node
replay example subsequently passed on the VM; no detector result from
replaying those 40 readings is reported in this manuscript.

Before using the new run as independently reproducible evidence, preserve the
actual CSV, executable and source hashes, run time, K3s and VM configuration,
and the pod manifest from the VM. Verify the 30 seeds, 500 samples per seed,
post-warm-up counts and event matches from that CSV. Capture data for any new
latency, resource, workload or comparator claim separately.

ldpad-pod-supplied.yaml is the configuration provided for the test pod; the
contents of the file actually applied on the VM have not been compared with
this copy.
