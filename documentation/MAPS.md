# Map files

A treadcore map is a text file. The editor is any text editor: that
was the prior art's second editor too, after the graphical one, and it is this
game's first. `make map` writes the generated map out as a starting
point; `MAP=maps/x.map make debug` (or `make run`, `make host`) plays
on one; `--save-map FILE` on the server writes the map as it was
played, craters and walls and captured pillboxes included, so the next
match can start where this one ended.

## The format

    treadcore map 1
    size 128 128
    seed 0x7eadc0de
    # rows: one character per tile, west to east, north first
    ....TTT....~~~....=====....
    (128 rows of 128 characters)
    # places
    start 64 10
    depot 64 19.72
    pillbox 64 29.44 neutral
    pillbox 73 64 3

| line | meaning |
| --- | --- |
| `treadcore map 1` | the first line, exactly; anything else is not a map |
| `size W H` | the grid, and the size the world is built at: the generator makes 128 x 128, the loader takes 1 to 1024, and a game on a 256 x 256 prior-art map is 256 x 256 |
| `seed N` | informational: which seed the map came from, if any (`0x...` or decimal) |
| a row | `W` glyphs, one per tile, west to east; `H` of them, north row first |
| `start x y` | a tank's start, in seat order: the first line is seat 0 (north), and a scenario of N players uses the first N; one to sixteen of them |
| `depot x y` | a refuelling depot; the first N depots belong to the first N seats at the start, the rest are neutral |
| `pillbox x y [seat\|neutral]` | a pillbox, neutral unless a seat number (0 to 15) is given; a seat the scenario does not seat makes it neutral |
| `# ...` | a comment; blank lines are ignored |

Positions are tile coordinates, `x` east and `y` south, as floats; the
centre of tile (10, 20) is `10.5 20.5`. Rows must all be present
before any `start`, `depot` or `pillbox` line.

## Glyphs

| glyph | terrain | the tank | fire | notes |
| --- | --- | --- | --- | --- |
| `.` | grass | medium | | the default ground |
| `T` | forest | slow | blocks a flat shot; halves vision | harvested for material |
| `=` | road | fast | | the squad lays them |
| `~` | water | impassable | | a shell's crater does not flood |
| `+` | bridge | fast | | a road laid on water |
| `%` | rubble | slow | | what a shell or a mine leaves; what a wall becomes |
| `D` | depot | medium | | the depot's own tiles; the `depot` line is what counts |
| `W` | wall | impassable | blocks a flat shot and a pillbox's line | the squad builds them; a splash knocks them down |

Mines are not in a map: they are laid in play and hidden, and a saved
map does not carry them.

## What the loader checks

The loader refuses, naming the line: a first line that is not the
format line; a row of the wrong length or with a glyph it does not
know; a place off the map; a start or a depot on water or a wall; a
pillbox on water; no starts; more than sixteen. It repairs, as the prior art's
loader did, a pillbox owner that is not a seat: it becomes neutral.
`treadcore-server --map FILE` checks the file before anything starts
and exits with the reason if it cannot be used.

## Carrying a map

A scenario names its map as a seed or as the text of a file. A map
from a file travels inside the scenario, so a replay of a match on it
is self-contained and the four peers of a LAN game need no copy of the
file: the host's `Welcome` carries the whole map, as it is at that
tick.

## Making one

1. `make map` (or `SEED=0x1234 make map`) writes `maps/generated-SEED.map`.
2. Edit it. A river is a line of `~`; a bridge where a road must cross
   is `+`; forest for a defender to harvest is `T` within twenty tiles
   of their depot; a wall ring is `W`. Move or delete `start`, `depot`
   and `pillbox` lines; give a pillbox a seat to start it owned.
3. `MAP=maps/mine.map make debug` and read the trace; `make run` on it
   when the window opens.

The maps in `maps/`:

| file | what |
| --- | --- |
| `maps/island.map` | the generated default (seed 0x7EADC0DE), written by `make map`, so a reader can see the format at full size |
| `maps/prior-art/*.map` | five prior-art maps converted by `scripts/priorart2map.py` (below) |

## Converting prior-art maps

`scripts/priorart2map.py` reads a prior-art map (the binary
format, version 1, as the prior art and prior-art editors wrote it) and writes
a treadcore map at the prior art's size, 256 x 256:

    make convert FROM=path/to/Some.map        # -> maps/prior-art/some.map
    python3 -I scripts/priorart2map.py Some.map out.map

The prior art's terrains that treadcore has not got are mapped: buildings become
walls; shot buildings, craters, rubble and swamp become rubble; river,
deep sea and boats become water; mined variants become their plain
terrain (mines are laid in play). Every pillbox and base keeps its
owner (the prior art's bases are usually neutral, and the file says so). Starts,
which the prior art puts at sea on a boat, move to the nearest passable tile,
because treadcore has no boats. The start direction is dropped. The
input is read as data only; run the script with `python3 -I`.

The five in `maps/prior-art/` came from the author's own collection of the prior art
community maps (`vendor/THIRD-PARTY.md` records their provenance and
the open question of their licence):

| file | prior-art name | starts | depots | pillboxes | walls | forest |
| --- | --- | --- | --- | --- | --- | --- |
| `constanine.map` | Constanine | 10 | 16 | 16 | 1 059 | 6 301 |
| `fidos-airstrip.map` | Fido's Airstrip | 16 | 16 | 5 | 826 | 3 837 |
| `fortressisles2.map` | FortressIsles2 | 16 | 10 | 15 | 6 803 | 5 328 |
| `fortressislesx.map` | FortressIslesX | 16 | 12 | 16 | 7 361 | 5 320 |
| `big-lizard.map` | (a film monster's name; dropped) | 16 | 16 | 16 | 0 | 2 229 |

`MAP=maps/prior-art/big-lizard.map make debug` plays sixteen bots on one. The
bots' doctrine (firing position 25 tiles short of an objective, a squad
sent 20 tiles) was written for a 128-tile island; on a prior-art map the
distances are double and the islands are often several, so expect
refused moves across water and little capturing until step 6d.

