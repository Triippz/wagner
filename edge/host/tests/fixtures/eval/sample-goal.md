# Construct eval goal — slugify utility

A small, self-contained, realistic project task used to exercise The Construct
end-to-end. Small enough to finish in a couple of iterations, but real enough to
require both factions: a Forger (Codex) to write the implementation and an
Architect (Claude) to design the test and judge completion.

## Goal (paste into the goal-entry screen)

> Add a `slugify(text: string)` utility that lowercases the input, trims it,
> replaces runs of non-alphanumeric characters with a single hyphen, and strips
> leading/trailing hyphens. Add a unit test covering the empty string, a simple
> phrase, and a string with punctuation and repeated spaces. Then run the test
> suite and confirm it is green.

## Doc pointers (optional, for the docs field)

- `README.md` — project conventions
- the existing test directory — match its style and runner

## Done condition (what the judge should confirm)

1. A `slugify` function exists and is exported.
2. A unit test for it exists and passes.
3. The full project test suite is green.

## Expected shape of a run

- **Iteration 1:** the Oracle decomposes into ~2 subtasks — Forger implements
  `slugify`, Architect writes the test — both dispatched, both succeed.
- **Iteration 2:** the Oracle hypothesizes goal-met → suite runs green → the
  Architect judge confirms → run ends `met`.

## How this is used

- **Live:** paste the goal above into the goal-entry screen against any small
  target repo to validate the floor, cost accounting, and the met gate by hand.
- **Deterministic regression:** `tests/eval.rs` models this exact run with
  scripted engines (no subscription burn) and asserts the loop produces the
  right artifacts — both factions active, a schema-valid persisted run-state,
  and a `met` verdict.
