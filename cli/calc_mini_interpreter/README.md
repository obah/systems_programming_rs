# CALC MINI INTERPRETER

A small arithmetic interpreter with a REPL, built as part of my Rust and systems engineering learning journey, to solidy my rust basics and also an intro to compiler design. It's a complete lexer → parser → evaluator pipeline in miniature.

> ⚠️ **Work in progress.** Arithmetic works end to end. Variables are implemented in the evaluator but not yet reachable from typed input — the lexer doesn't group letters into an identifier token yet. See [Roadmap](#roadmap).

## Features

- Interactive REPL that evaluates one expression per line.
- `+`, `-`, `*`, `/` with the usual precedence (`*`//` bind tighter) and left associativity (`10-2-3`is`5`, not `11`).
- Parentheses, nested to any depth, checked for balance while tokenizing.
- Decimal literals, with a clear error on malformed ones like `1..2`.
- Unary minus — both negative literals (`-9`) and negated groups (`-(2+2)`).
- Implicit multiplication: `2(3+4)` is `14`, `-(2+2)` is `-4`.
- Errors never kill the session — a bad line prints and the prompt comes back.

## Usage

The REPL reads one expression per line and prints its value. Blank lines are ignored; `exit`, `quit`, or Ctrl-D ends the session.

### Running from the repo (no install)

Use `cargo run -p calc_mini_interpreter` from the workspace root:

```sh
cargo run -p calc_mini_interpreter
```

### Installed

Install the binary into `~/.cargo/bin` and use it directly:

```sh
cargo install --path .

calc_mini_interpreter
```

Example session:

```text
> 1- 2+3 * (-9/3)
-10
> 2(3+4)
14
> 8/4/2
1
> 1/0
error: Unknown("division by zero")
> 2+
error: Unknown("expected a value, got None")
> exit
```

Note that the error line doesn't end the session — the next prompt appears as usual.

## Implementation

A library crate holding the three stages, with a thin REPL binary on top:

```text
src/
├── main.rs       # REPL: prompt, read, run the pipeline, print result or error
├── lib.rs        # crate root: declares modules and the shared CalcError type
├── lexer.rs      # stage 1: raw text -> Vec<Token>
├── parser.rs     # stage 2: tokens -> Expr (recursive descent AST)
└── evaluator.rs  # stage 3: Expr + Env -> f64
```

The pipeline for `1- 2+3 * (-9/3)`:

```text
tokenize  [Num(1), Minus, Num(2), Plus, Num(3), Star, LParen, Num(-9), Slash, Num(3), RParen]
parse     Add(Sub(1, 2), Mul(3, Div(-9, 3)))
eval      -10.0
```

Some design details:

- **Precedence lives in the call graph** — the parser is one function per precedence level (`expr` for `+`/`-`, `term` for `*`//`, `factor`for atoms). Each level takes its operands from the next-tighter one, so a`Mul`node can only ever be a *child* of an`Add`, never its parent. Parentheses are the sole escape hatch: `factor`recurses back into`expr`, which is how a loose operator gets buried under a tight one.
- **Left associativity from a loop** — each level folds with `lhs = BinOp { lhs, rhs }` in a `loop`, reassigning the accumulator into its own left slot. Recursing at the same level instead would give right-nesting, which is wrong for `-` and `/`.
- **The lexer does some of the parser's work** — it folds a leading `-` into the number literal (`-9` is one `Num` token, not `Minus` then `Num`) and inserts a synthetic `Star` for implicit multiplication (`2(3+4)` becomes `2 * (3+4)`, `-(2+2)` becomes `-1 * (2+2)`). It also tracks paren depth so unbalanced input fails before parsing.
- **Evaluation is a post-order walk** — precedence was settled by the tree shape at parse time, so the evaluator just collapses children to numbers and applies the operator. The `match` on `Expr` is exhaustive with no `_` arm on purpose: a new variant won't compile until it's handled.
- **Division by zero is an explicit guard** — `f64` would quietly hand back `inf` or `NaN`, so the evaluator checks the divisor rather than relying on a panic that never comes.
- **The pipeline is pure** — `run(&str, &mut Env) -> Result<f64, CalcError>` does no I/O, which is what keeps a future `--file` mode trivial: feed it lines from a file instead of stdin and nothing else changes.
- **Errors** — a single `CalcError` enum (`Unknown`, `Parse`) is propagated with `?` through all three stages. The REPL prints it and loops rather than exiting.

## Roadmap

Known gaps, roughly in priority order:

- [ ] **String/identifier token** — the lexer errors on any letter (`x` → `Unknown("x")`). Group consecutive letters into `Token::Ident` so names can reach the parser at all. Everything below depends on this.
- [ ] **Identifier expressions** — wire `Ident` through the parser into `Expr::Var` and `Expr::Assign` (`x = 2 * 3`). Both variants and their evaluation already exist and are tested; the parser needs a statement rule above `expr` (`stmt := Ident '=' expr | expr`), which means two-token lookahead to tell `x = 1` from `x + 1`.
- [ ] **Power operator `^`** — a new precedence level between `term` and `factor`. Note it's _right_ associative (`2^3^2` is `2^9`, not `8^2`), so unlike the other levels it recurses into itself for the right operand instead of looping.
- [ ] **Multiple expressions per line** — allow `x = 10, x + 10 = ?` on a single line: parse a comma-separated list of statements sharing one `Env`, with a trailing `= ?` marking the one whose value gets printed. Currently one line is exactly one expression.
- [ ] **`Display` for `CalcError`** — errors currently print via `Debug`, so you get `Unknown("division by zero")` instead of a clean message.
- [ ] **`--file` mode** — run a script of expressions instead of reading stdin.

## Development

```sh
cargo test    # run all unit tests
cargo clippy  # lint
cargo fmt     # format
```

Unit tests live in `#[cfg(test)]` modules inside each source file: bracket balancing in `lexer.rs`, the AST shape for `1- 2+3 * (-9/3)` in `parser.rs`, and evaluation plus error cases in `evaluator.rs`.
