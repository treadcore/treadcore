# Contributing to treadcore

Thank you. The short version:

1. Build it: `make configure`, `make test`, `make debug`. See
   `documentation/BUILD.md` if the machine is small.
2. Read `documentation/CONCEPT.md` and `documentation/DESIGN.md` so a
   change argues with the design rather than around it.
3. Pick an issue (`good first issue` to start) or open one. A change to
   the rules, the architecture or the wire is a proposal first:
   `documentation/GOVERNANCE.md` section 5.
4. Branch `topic/short-name`, commit small and signed off
   (`git commit -s`), message `area: what changed`. Keep `make test`
   green at every commit.
5. Open a pull request; the template asks for the change, why, what
   it must not change, and how it is proven. For a simulation change,
   say what the default `make debug` trace shows differently.
6. A maintainer reviews within seven days (`GOVERNANCE.md` section 7).

The cycle, versions and releases: `documentation/RELEASE.md`. Roles,
joining, leaving, recognition: `documentation/GOVERNANCE.md`. Every
contributor with a merged change is listed in `CONTRIBUTORS.md`.

Licence: MIT. Your sign-off line says you have the right to contribute
the work under it.
