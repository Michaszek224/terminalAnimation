# shapes

Animowane bryły 3D w terminalu (Rust + crossterm). Bez flag kształt, tryb i paleta są losowane przy starcie.

```sh
cargo run --release                                   # wszystko losowe
cargo run --release -- -s torus -m fill -c fire       # konkretny wybór
cargo run --release -- --cycle 8 --speed 1.5          # nowy losowy kształt co 8 s
cargo run --release -- --once -s cube -c none         # jedna klatka na stdout
cargo install --path .                                # instaluje binarkę `shapes`
```

| Flaga | Wartości |
|---|---|
| `-s, --shape` | `cube`, `pyramid`, `octahedron`, `dodecahedron`, `icosahedron`, `sphere`, `torus`, `tesseract` |
| `-m, --mode` | `fill` (cieniowane znaki ASCII), `wire` (krawędzie z kropek Braille'a) |
| `-c, --color` | `rainbow`, `fire`, `ocean`, `matrix`, `neon`, `mono`, `none` |
| `--speed`, `--size`, `--fps` | mnożnik prędkości, wielkość bryły, limit klatek (domyślnie 30) |
| `--cycle <SEKUNDY>` | automatyczna zmiana kształtu |
| `--no-hud` | bez paska statusu |

Klawisze: `←/→` (lub `p/n`) kształt, `m` tryb, `c` paleta, `r` losuj wszystko, `spacja` pauza, `+/-` prędkość, `h` pasek statusu, `q`/`Esc` wyjście.

Tesserakt (hipersześcian 4D) obraca się w płaszczyznach XW i ZW i zawsze jest rysowany jako wireframe.
