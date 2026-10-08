**Calculator**

I have fully vibe coded calculator in similar theme of mac calculator app. The app is fully written in rust.

Any suggestion or issue, please let me know.

## Features

- **Three modes**: Basic, Scientific, Programmer — each with its own pad layout and default window size.
- **Positional (caret) editing** — click anywhere inside an expression and edit it there, Wolfram-style,
  instead of only appending to the end. Backspace, Delete, Home/End, arrows, and Tab slot-cycling all work.
- **Live expression line** above the result, plus a **history panel** you can toggle (window widens to fit).
- **Scientific pad in 3 columns**: full trig with inverses (`2nd` key swaps `sin↔sin⁻¹`), `csc/sec/cot`,
  `nCr/nPr/gcd/lcm`, logs, roots, powers, `mod`, constants `π e φ`, factorial, `Ans`.
- **Typed strings evaluate too**: type or paste function names (`log2(8)`, `sin(30)`, `2^3^2 = 512`,
  right-associative power) directly on the keyboard.
- **Programmer mode**: HEX/DEC/OCT/BIN bases, `AND OR XOR NOT`, shifts — wrapping integer arithmetic.
- **Resizable window** with sane minimums; every **mode or history change snaps the window to that
  layout's exact default size** (Basic 300×540, Scientific 780×560, Programmer 660×630 inner).
- **Unicode math typography**: `− × ÷` and superscript glyphs, rendered from an embedded
  [STIX Two Math](assets/STIXTwoMath.otf) fallback font so output looks identical on every OS.
- **Guardrails**: division by zero and domain errors go to a typed error state (AC recovers) —
  never `NaN`/`inf` on screen; `factorial` and `nCr/nPr` are range-checked (0…170).

## Screenshots

<img width="330" height="597" alt="Screenshot 2026-10-06 at 6 33 43 PM" src="https://github.com/user-attachments/assets/cbf07d60-8e41-467b-9eae-31af1011ffb8" />
<img width="683" height="597" alt="Screenshot 2026-10-06 at 6 33 56 PM" src="https://github.com/user-attachments/assets/0b637682-971d-4c9b-bc46-5ca2d8fe17c6" />
<img width="943" height="597" alt="Screenshot 2026-10-06 at 6 34 20 PM" src="https://github.com/user-attachments/assets/2f5aedd4-63e5-433e-ac51-72308a6affc7" />
<img width="946" height="682" alt="Screenshot 2026-10-06 at 6 35 16 PM" src="https://github.com/user-attachments/assets/60bcc8b1-f7d7-4a0c-9804-e4fe62154499" />

## Keyboard

| Key | Action |
|---|---|
| digits, `.`, `+ - * / ^ %`, `(` `)` | insert at caret |
| `Enter` / `=` | evaluate |
| `Esc` | AC (clear all) |
| `Backspace` / `Delete` | remove before / at caret |
| `← →`, `Home`, `End` | move caret |
| `Tab` | jump to next operand slot |
| `Cmd/Ctrl+C`, `Cmd/Ctrl+V` | copy / paste (paste accepts plain numbers) |
| letters | type function names (`sin`, `log2`, `ncr(5,2)`, …) |

## Build from source

Prerequisites: [rustup](https://rustup.rs) (stable toolchain).

```bash
cargo run --release        # run
cargo build --release      # binary in target/release/
```

### Quality gates (run before every push)

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test                 # 78 tests, incl. ~212 trig formula validations
```

### Cross-OS release builds (how the prebuilt archives are made)

- **macOS universal**: build `aarch64-apple-darwin` + `x86_64-apple-darwin`, join with
  `lipo -create`, embed in `Calculator.app`, ad-hoc sign (`codesign --force --deep --sign -`).
- **Windows x64**: `--target x86_64-pc-windows-gnu` (mingw-w64) with
  `RUSTFLAGS="-Clink-arg=-mwindows"` so the exe is a GUI app with **no console window**.
- **Linux x86-64**: cross-compile inside an Ubuntu 22.04 container to pin the **glibc 2.35** floor.

## Project layout

```
src/main.rs              # engine + parser + display maps + UI + tests (one file, ~4.1k lines)
assets/STIXTwoMath.otf   # embedded math glyph fallback font
Cargo.toml               # single dependency: eframe 0.36
```

Thank you for visiting.
