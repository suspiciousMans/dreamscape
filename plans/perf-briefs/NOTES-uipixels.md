# NOTES: UIPIXELS
Measured by `cargo test -p dreamscape ui_cost_table -- --nocapture` (1280x720, 240-card booklet, DEBUG build: tess_us is only comparable between rows/runs of the same build).

## Baseline
```
screen      shapes  vertices  triangles    tess_us
Warning         87      3040       2022        463
Title          586      5944       6208       1300
Summary        184      1236       1466        332
Paused         411      2552       2254        574
Playing        329      1964       1954        398
Settings       213      2732       2226        426
Codex         9208     66820      83034      21906
Booklet        181      3976       2854        664
Loadout       1734     16212      18070       5864
Merge        10213     43668      52322      12622
Store         4540     37156      45216       9540
Lottery       4594     17928      21062       4809
Tickets        206      2608       2202        412
```
