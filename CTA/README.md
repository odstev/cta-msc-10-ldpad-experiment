# Overleaf source: start here

## Create a new project

1. Download the complete source ZIP file. Do not upload only cta.tex.
2. On the Overleaf dashboard, choose New Project, then Upload Project.
3. Choose the ZIP file. Overleaf should extract it into the new project.
4. In Menu / Settings, choose pdfLaTeX as the compiler, and set Main document to cta.tex.
5. Select Recompile. If old warnings remain, use Recompile from scratch from the Recompile menu.

The project should contain cta.tex, ieeeaccess.cls, IEEEtran.cls, and the figures, generated, and literature folders. Preserve these names and their files. In particular, figures/reported_prisma_flow.png and literature/generated_counts.tex are needed for compilation. File names and folder names are case-sensitive.

Use the ieeeaccess.cls supplied in this ZIP. It contains the formatting changes for this manuscript and initializes optional publication fields. The source also supplies a safe empty default for an unspecified DOI. Do not replace the included class with an older copy from another template.

The generated tables and figures are included. Compiling the paper does not require Rust, Cargo, or Python. The bibliography is included in cta.tex; there is no separate .bib file to upload. Citations normally resolve after a successful multi-pass compile.

If an older project contains a file named output.pdf in its root, rename or remove that old file before compiling that project. Keep the required figures/architecture_single_column.pdf; it is part of Figure 2.

Figure 1 is black and white, and Figure 2 fits one column. The literature inventory is available in the accompanying files with unverified cells marked. The original Table 16 (illustrative failure-mode scores) has been removed; subsequent tables are renumbered automatically. The unverified historical EWMA results have also been removed. The companion reproducibility package contains the earlier data, Rust sources, scripts, and protocol.

The paper includes the observed K3s test-pod execution on the Ubuntu VM and four later whole-node resource snapshots. The counts and snapshots are transcribed from terminal screenshots; the post-warm-up count was also reported in the session. The new raw CSV, node capture and full environment record remain on the VM and are not included in this source archive. Whole-node readings are not measurements of LDPAD-specific resource overhead. See k3s/VM_PILOT_NOTES.md before interpreting or reproducing these observations. The companion reproducibility package was assembled before the K3s run; historical statements in its README that the pod run was still pending describe that earlier date, not the present manuscript.

No DOI or publication history is invented in this source. If assigned later, the authors may add the relevant \doi{...} and \history{...} fields.
