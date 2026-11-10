# Put this repository on GitHub

1. Unzip the package and open the `CTA_Experiment_Repository` folder.
2. Create an empty GitHub repository named `cta-ldpad-experiments`.
3. In the Ubuntu terminal, from this folder, run:

```bash
git init
git add .
git commit -m "Add LDPAD experiment source and archived evidence"
git branch -M main
git remote add origin https://github.com/YOUR_USERNAME/cta-ldpad-experiments.git
git push -u origin main
```

Replace YOUR_USERNAME with your GitHub username. Use GitHub's normal authentication;
never place a token in any source file. The `.github/workflows/reproduce.yml`
workflow rebuilds archived results on push and pull requests. Fresh VM timings
and K3s deployment are deliberately run on your VM, not silently on a hosted runner.

If Git asks for author identity, set your own name and email before committing.
Choose public/private visibility and a license according to your project ownership.
The repository includes author contact details from the original Cargo manifest
and manuscript; review these before publishing publicly.
