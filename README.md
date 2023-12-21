<div align="center">

# bulletformat

</div>

Binary Data Formats, Data Loader and Utilities for [bullet](https://github.com/jw1912/bullet).

### Supported Games
- Ataxx
- Chess

### Text Formats
Exactly one data point per line.

#### Ataxx
- each line is of the form `<FEN> | <score> | <result>`
- `FEN` has 'x'/'r', 'o'/'b' and '-' for red, blue and gaps/blockers, respectively, in the same format as FEN for chess
- `score` is red relative and an integer
- `result` is red relative and of the form `1.0` for win, `0.5` for draw, `0.0` for loss

#### Chess
- each line is of the form `<FEN> | <score> | <result>`
- `score` is white relative and in centipawns
- `result` is white relative and of the form `1.0` for win, `0.5` for draw, `0.0` for loss
