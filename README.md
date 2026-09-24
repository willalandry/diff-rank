# diff-rank

A large diff (a big refactor, a vendor sync, `git log -p` over a year of
history) buries the two or three files that actually mattered under a pile
of one-line changes. `diff-rank` answers one question: given a unified
diff, which files changed the most?

It reads a diff and prints each file it touches, sorted by total lines
added plus removed, most-changed first.

## Usage

```
$ git diff | diff-rank
   142  +98     -44      src/parser.rs
    31  +31     -0       src/main.rs
     6  +3      -3       README.md
```

Or point it at a saved patch file:

```
$ diff-rank changes.patch
```

Pass `-` (or nothing) to read from stdin, which also makes it a natural
pipeline stage:

```
$ git log -p --since=6.months | diff-rank | head -20
```

## Why streaming matters here

`diff-rank` reads its input one line at a time through a `BufReader` and
only keeps a small running counter per file. It never buffers the diff
text itself. Running it on a 3 GB `git log -p` dump uses about as much
memory as running it on a 3 KB patch. That's the whole point: the tool
that goes looking for "what mattered" is often exactly the tool you run
on the input too large to eyeball.

## Current limitations

This is an early skeleton. Known gaps, in rough order of how much they'll
bite you:

- Binary diffs (`GIT binary patch`) aren't specially detected, so their
  encoded body lines can be miscounted as added/removed text lines.
- Renames aren't recognized as a distinct kind of change.

See the roadmap in the project notes for what's planned next.

## Building

Standard library only, no dependencies:

```
$ cargo build --release
```

## License

MIT. See LICENSE.
