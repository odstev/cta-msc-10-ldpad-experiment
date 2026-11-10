.PHONY: tables figures test reproduce verify-rust pdf

# These rebuild captured data and the manuscript's working tables. They do not
# rerun the VM or create new timing/clinical measurements.
tables:
	python3 analysis/verify_inputs.py
	python3 analysis/rebuild_tables.py
	python3 analysis/rebuild_captured.py
	python3 CTA/literature/rebuild.py
	python3 analysis/rebuild_k3s.py

figures:
	python3 analysis/rebuild_figures.py

test:
	python3 -m unittest discover -s tests -v
	python3 -m unittest discover -s CTA/literature/tests -v

reproduce: tables figures test
	python3 analysis/check_reported_results.py
	@echo "Captured tables, figures and tests rebuilt; no DDIL or comparator experiment was run."

# Separate from replay: compiles and reruns deterministic Rust components in
# a temporary copy and checks their output against the recorded raw files.
verify-rust:
	python3 analysis/verify_rust.py

pdf:
	cd CTA && pdflatex -interaction=nonstopmode -halt-on-error cta.tex
	cd CTA && pdflatex -interaction=nonstopmode -halt-on-error cta.tex

.PHONY: help
help:
	@echo "make reproduce: reanalyse archived data; make verify-rust: rerun deterministic Rust and compare hashes"
	@echo "Fresh measurements: python3 scripts/run_experiment.py --help; K3s: python3 scripts/k3s_experiment.py --help"
