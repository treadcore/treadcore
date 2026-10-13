treadcore/journal -- the creation journal
=========================================

This directory is the dated development log of treadcore. It is part
of the change-log notes: CHANGELOG.md at the root says what changed in
each version; the journal says what happened, in what order, why, and
what was learned, one entry per working session.

Each entry is a plain-text file. Plain text because it is read in a
terminal, in a pager, over gopher, in a mail reader, in a diff, and in
fifty years; because it is grepped; and because the form is older than
the project and will outlive its tools.

Files
-----

    README.txt                 this
    TEMPLATE.txt               the form every entry follows
    NNNN-YYYY-MM-DD-slug.txt   one entry per session, numbered from 0001

Numbering is sequential and never reused. The date is the session's
date in the author's local time. The slug is three or four words.

The form
--------

Two traditions, deliberately combined.

The HEADER is a Usenet/RFC 822 article header: From, Date, Subject,
Message-ID, Newsgroups, Organization, Keywords. It is what a newsgroup
reader or a mail archive would index; it makes an entry a message, with
an author, a time and an address, and lets one entry reference another
by Message-ID the way a follow-up references its parent.

The BODY is the IEEE paper structure as it is commonly used in MIT
course and lab writing: Abstract; Index Terms; numbered sections
(Introduction, Background, Method, Chronology, Results, Discussion,
Open Problems, Next Steps); Acknowledgments; References in IEEE
numeric style [n]; Appendices. The sections are kept even when short,
so entries can be compared across months.

Rules
-----

  1. Write it the same day, or mark the entry as reconstructed.
  2. The chronology is what happened in the order it happened,
     including the false starts. A journal that records only successes
     is a press release.
  3. Numbers in tables, with units. "It was slow" is not a result.
  4. Every claim about the outside world gets a reference. Every
     reference gets a URL and the date it was read.
  5. Commits are listed in the appendix with their short hash so the
     entry and the history can be cross-checked.
  6. A decision records the alternatives it rejected and why.
  7. Open problems are carried forward: the next entry's Background
     section starts from the previous entry's Open Problems.
  8. 72 columns. No tabs. ASCII where possible.
  9. Message-ID is <NNNN.YYYYMMDD@treadcore.org>. A reply to a point
     in an earlier entry cites it as In-Reply-To.
 10. The conversation that produced a session (with a person, with an
     AI, on a mailing list) is not the record. The journal is. Rewrite
     it; do not paste it.
