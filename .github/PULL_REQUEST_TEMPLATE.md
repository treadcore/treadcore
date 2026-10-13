<!-- The four lines the roadmap uses. Keep each short; the diff is the detail. -->

**Change.** What changes, in which crate or document.

**Why.** The problem, with evidence (an issue, a trace, a playtest).

**Must not change.** What this promises to leave alone.

**Proof.** How a reviewer sees it worked (a test, a trace line, a screenshot).

---

- [ ] `make test` passes
- [ ] touches `treadcore-sim`: `make debug` run; what the default trace shows differently is written above
- [ ] touches `treadcore-net` or `treadcore-proto`: `make lobby-test` passes; `VERSION` bumped if the wire changed
- [ ] a new or changed number in `rules.rs` has its sentence, and `DESIGN.md` agrees
- [ ] `CHANGELOG.md` line; documents describing the changed thing updated
- [ ] commits are signed off (`git commit -s`)
