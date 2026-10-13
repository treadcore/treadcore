# How treadcore is run

treadcore is maintained by the treadcore.org organization. Today that is
one person, Paul Richeson, who founded it on 12 October 2026. This
document is written before it is needed: it says how people join, how
decisions are made, how work is reviewed, how people are recognised,
and how they leave, so that the second contributor finds the door
already open and the rules already written.

Everything here is enforced through GitHub, where the project lives:
the organization `treadcore`, the repository `treadcore/treadcore`,
its teams, branch protection, issues, discussions, milestones and
releases. Section 2 maps each rule to the GitHub mechanism that
carries it.

## 1. Roles

| role | who | may |
| --- | --- | --- |
| visitor | anyone | read, play, open issues and discussions, send pull requests |
| contributor | anyone with a merged change | be named in release notes and `CONTRIBUTORS.md` |
| member | accepted by the application process (section 3) | triage issues, review, be assigned work, vote on proposals, be nominated for awards |
| maintainer | a member appointed by the maintainers | merge to `main`, cut releases, administer teams |
| steward | the founder, until the members elect otherwise | tie-break, final say on direction, custody of the domain and the organization |
| emeritus | a former member who left in good standing | be listed, be welcomed back by the entry process without re-applying |

A role is a set of permissions and a set of duties. Nobody holds a
role they do not use: the annual process (section 14) confirms every
one.

## 2. The organization on GitHub

| rule | mechanism |
| --- | --- |
| membership is visible | GitHub organization `treadcore`; members have public membership |
| members, maintainers | teams `@treadcore/members` (triage) and `@treadcore/maintainers` (maintain) |
| `main` always builds | branch protection on `main`: pull request required, one approving review, `make test` green, linear history, signed commits |
| rules changes are reviewed by the people who own them | `.github/CODEOWNERS`: `source/treadcore-sim/src/rules.rs` and `documentation/DESIGN.md` need a maintainer |
| proposals are discussed in the open | GitHub Discussions, category *Proposals*; the issue template `proposal.md` for small ones |
| work is tracked | issues with labels `bug`, `rules`, `net`, `client`, `build`, `docs`, `good first issue`; milestones named after roadmap steps |
| releases are marked | git tags `vX.Y.Z` and `vX.Y.Z-rc.N`, GitHub Releases with notes and checksums (see `RELEASE.md`) |
| the record is kept | `journal/`, `CHANGELOG.md`, `reports/`, release notes; nothing that matters lives only in a chat |

Administrators of the organization: the steward and the maintainers.
Two-factor authentication is required for every member.

## 3. Application process

Anyone may apply to become a member after two merged changes, or one
substantial one (a document, a map, a playtest campaign counts as much
as code).

1. Open an issue from the `membership.md` template: who you are, what
   you have done here, what you want to work on, and a member who will
   sponsor you (ask in Discussions if you do not know one).
2. The sponsor comments to confirm.
3. Maintainers decide within 14 days by lazy consensus (section 7).
   Silence is consent; one maintainer's objection, with reasons, is
   not, and is resolved by discussion or the steward.
4. Accepted: the entry process starts. Declined: the reasons are given
   in the issue, and the applicant may apply again after three months.

## 4. Entry process

For a new member, within a week of acceptance:

1. A maintainer invites them to the organization and to
   `@treadcore/members`, and adds them to `CONTRIBUTORS.md`.
2. They read `CONCEPT.md`, `DESIGN.md`, `BUILD.md`, `RELEASE.md` and
   this document, and run `make test`, `make debug` and `make
   lobby-test` on their machine, recording the machine and the numbers
   in their first journal entry (the template is `journal/TEMPLATE.txt`).
3. They agree to the member agreement (section 6) by saying so in
   their application issue.
4. Their sponsor picks a first issue with them.
5. The next journal entry's Acknowledgments names them, and the next
   release notes welcome them. That is the entry ceremony (section 11).

## 5. Proposal process

A proposal is for anything that changes the rules of the game, the
architecture, the wire protocol, the build, or this document. A bug
fix, a document correction, or a bot improvement is a pull request,
not a proposal.

A proposal is written in the form the roadmap uses, because that form
has to be filled in before the work can be judged:

    Change.          what changes, in which crate or document
    Why.             the problem, with evidence (a trace, a playtest)
    Must not change. what the proposal promises to leave alone
    Proof.           how we will know it worked

1. Post it in Discussions under *Proposals*, or as a `proposal.md`
   issue if it is small.
2. Seven days of discussion, during which the author revises it.
3. Decision by lazy consensus of the members; maintainers confirm.
   A proposal that changes `rules.rs` numbers needs a trace or a
   playtest showing the effect before it is accepted.
4. Accepted: it becomes a roadmap step or backlog item in `ROADMAP.md`
   with a milestone, and a design note in `DESIGN.md` if it changes
   play. Rejected: the discussion records why, so it is not re-argued
   from nothing.

## 6. Agreement process

There is no contributor licence agreement to sign. Instead:

- Every commit carries a `Signed-off-by:` line (the Developer
  Certificate of Origin, `git commit -s`): the author has the right to
  submit the work under the project's licence, MIT.
- Every contribution is MIT, like the rest; third-party material
  follows `vendor/README.md` (licence beside it, a line in
  `THIRD-PARTY.md`, a reason it is vendored).
- Members agree to the code of conduct (section 15) and to the
  review, removal and resignation processes here.

A member states the agreement once, in their application issue; the
sign-off line restates it on every commit.

## 7. Review process

Every change to `main` goes through a pull request, including the
steward's.

What a reviewer checks, in this order:

1. **The world is still deterministic.** `make test` passes;
   `make debug` with the default seed produces a trace; if the change
   touches `treadcore-sim`, the author says in the pull request what
   the trace shows differently and why that is intended. When the
   replay corpus exists (roadmap step 1), replays must still match or
   the mismatch must be explained.
2. **The layering holds.** No crate imports a crate above it; the sim
   knows nothing of sockets or engines.
3. **The numbers have sentences.** A new or changed constant in
   `rules.rs` has its reason beside it, and `DESIGN.md` agrees.
4. **The wire is versioned.** A change to `Packet` or to any sim type
   it carries bumps `treadcore-proto::VERSION` and `PROTOCOL.md`.
5. **The record is kept.** `CHANGELOG.md` has a line; a session's
   work has a journal entry; documents that describe the changed thing
   are updated in the same pull request.
6. **It reads.** Code and prose in the project's voice: plain, with
   the reason beside the decision.

Decisions: lazy consensus. One approving review from a maintainer (or
from a member, for changes outside `CODEOWNERS` paths) and no
unresolved objection merges. Reviews are answered within seven days;
a pull request with no review in fourteen may be escalated to the
steward. Disagreement is resolved by discussion, then by the
maintainers, then by the steward, and the decision is recorded in the
pull request.

## 8. Removal process

A member may be removed for breaking the code of conduct, or for
twelve months without activity after the annual confirmation (section
14) went unanswered.

1. A maintainer raises it privately with the member and with the other
   maintainers, with specifics.
2. The member has 14 days to respond.
3. The maintainers decide by majority; the steward breaks a tie and is
   the appeal, once, within 30 days.
4. Removal takes effect on GitHub the same day. The record says that a
   member left and when; the reasons stay with the maintainers unless
   the member asks for them to be published.
5. Work already merged stays, under its licence and with its credit.

Inactivity removal is not a sanction: the member becomes emeritus and
is welcome back through the entry process.

## 9. Resignation process

A member may resign at any time, for any reason or none, by issue,
Discussion post or email to the steward.

1. Access is removed within a week.
2. The member chooses whether to be listed as emeritus.
3. Open work is reassigned; the resigning member is asked, not
   required, to hand over.
4. The next journal entry and release notes thank them. That is the
   farewell ceremony (section 11).

## 10. Ceremony process

Ceremonies are how the record marks a moment, so that in fifty years
the journal still shows when things happened and who was there. Each
one is a fixed set of acts, not a party.

| ceremony | when | acts |
| --- | --- | --- |
| entry | a member joins | named in the journal's Acknowledgments and in `CONTRIBUTORS.md`; welcomed in the next release notes |
| release | a release monument is reached (`RELEASE.md`) | the tag; a journal entry; a report page under `reports/`; release notes naming every contributor to it; a note on treadcore.org |
| farewell | a member resigns or becomes emeritus | thanked in the journal and release notes; listed as emeritus if they wish |
| annual | the founding month, October | the annual report (section 14), awards (section 13), confirmations |

## 11. Recognition process

Credit is given continuously, not only at ceremonies.

- Every merged pull request's author is named in the next release
  notes and the `CHANGELOG.md` line for it.
- `CONTRIBUTORS.md` lists everyone with a merged change, in order of
  first merge, with what they did.
- Journal entries name in Acknowledgments everyone who helped that
  session, including playtesters and people who reported bugs.
- Playtesters who submit a playtest report (`RELEASE.md`) are
  contributors.

## 12. Assessment process

Once a year, as part of the annual process, each member's year is
assessed, in the open, by the maintainers, in one paragraph: what
they did, what the project asked of them, what they want next. It is
a record and a conversation, not a grade. It is the basis for awards,
for maintainer appointments, and for confirming roles.

## 13. Award process

Awards are given once a year at the annual ceremony.

1. Nominations are open to all members for three weeks in October, in
   a Discussion thread, one post per nomination with the reason.
2. The categories are fixed and few: *the Rules* (a design or balance
   contribution the trace can show), *the Wire* (network and
   architecture), *the Window* (client and overlay), *the Record*
   (documentation, journal, research), *the Field* (playtesting and
   bug reports). A category with no worthy nominee is not awarded.
3. Members vote, one vote per category; maintainers resolve ties.
4. Awards are recorded in the annual report and on the site. The
   steward may give one additional award, the steward's award, at
   discretion.

## 14. Annual member process

Every October, in the founding month:

1. Every member is asked to confirm membership by replying to one
   issue. No reply in 30 days: emeritus (section 8).
2. Assessments are written (section 12).
3. Maintainer roles are confirmed or changed by the maintainers;
   a member may be appointed maintainer after a year of membership and
   a nomination by an existing maintainer.
4. The steward's role is confirmed by the members. Should the members
   wish to elect a different steward, a two-thirds vote of members
   does it, and the domain and organization pass with the role.
5. Awards (section 13).
6. The roadmap is re-planned for the year and the annual report is
   written as a report page under `reports/` and a journal entry.

## 15. Code of conduct

Be straight and be kind. Disagree with the work, not the person.
Write down the reasons for what you decide. Do not harass, demean or
exclude anyone, in the project's spaces or in connection with it.
Report a problem to any maintainer or to the steward, privately;
they will act under section 8 and will keep your report confidential.

## 16. Amending this document

By the proposal process, with a 14-day discussion instead of seven,
and confirmation by the maintainers and the steward.
