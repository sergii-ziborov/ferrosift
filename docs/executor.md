# The executor

What is settled before a recipe runs, what is settled while it runs, and where
the line between them is.

## Two phases, on purpose

Preparation splits in two, and the split is what lets a prepared recipe be
reused. `resolve` does the work that depends only on the recipe, the registry
and the granted capabilities. `check_runtime` does what genuinely depends on
the call: the input size and representation, the budget, and cancellation.

Running the same recipe over a thousand inputs pays for the per-step registry
lookups and the argument resolution once.

## What preflight settles

Before the first operation is invoked:

- **Structure.** The recipe satisfies the model's invariants — unique step ids,
  a valid operation id per step, well-formed metadata.
- **Resolution.** Every enabled step names an operation the registry holds.
- **Capabilities.** Every operation's declared host capabilities were granted.
  A recipe that would need one it does not have is refused, not attempted.
- **Arguments.** Every argument the recipe supplies is declared, is of the
  declared kind, and every required argument without a default is present.
- **Budget.** The step count and the input size are inside their ceilings.
- **Type flow along the recipe as written.** Each step's declared output must
  reach the next step's declared input, directly or through a conversion the
  value model defines.

A failure in any of these names the responsible step and leaves no effect
behind, because nothing has run.

## What preflight does not settle

**Where a jump sends the counter.**

The type-flow check walks the recipe in source order. That was the whole of the
control flow until `Jump`, `Conditional Jump`, `Return` and `Subsection`
arrived; now a step can send the program counter backwards, and the value
arriving at a label on the second pass is whatever the loop body produced
rather than whatever the previous line declared.

So the straight-line reading is a real check and an incomplete one. A recipe
whose loop changes the value's kind can pass preflight and be refused at the
step that received the kind it cannot take — with the step named, the trace up
to that point intact, and nothing partially applied beyond what the earlier
steps legitimately did.

This is stated rather than papered over because the README used to say
"complete preflight", and after the flow-control work that sentence had
quietly stopped being true.

**The fix is known and not yet built.** The prepared recipe is a control-flow
graph: an ordinary step has one edge to `pc + 1`, a `Jump` one edge to its
label, a `Conditional Jump` two, a `Return` an edge out of the region, and Fork
and Subsection the region edges they already have. Running the same
`accepts`/`converts_to` transfer function over that graph to a fixed point
would settle the dynamic edges too, and would additionally report unreachable
steps and impossible branches. The per-step runtime check stays either way, as
defence in depth.

## What the runtime settles

Per step, every step:

- The value in hand is a kind the step accepts, or converts to one.
- The step's output is a kind it declared.
- The output is inside the absolute ceiling and the expansion ratio that its
  declared `OutputBehavior` makes meaningful.
- Cancellation has not been requested.
- The invocation count, the flow depth, the branch count and the total bytes
  processed are all inside their ceilings.

Operations that can spend disproportionately before producing an answer — a key
derivation, a decompressor — additionally ask before they allocate
(`ensure_transient`) or before they work (`ensure_work`), because a limit
applied after the harm is a description rather than a limit.

## Flow control

`Fork` maps a body over branches, `Subsection` over the spans a pattern
selects, and both are closed by the same `Merge`. `Jump`, `Conditional Jump`
and `Return` move the counter. All of it goes through one recursive region
interpreter, so nested regions compose.

The reference's own semantics are followed exactly, including the parts that
are easier to get wrong than to read — a shared jump allowance, a destination
one step past the label, a label lookup scoped to the enclosing region. They
are listed in
[the compatibility page](compatibility/cyberchef-v11.3.0.md#flow-control-is-a-program-counter-not-a-cursor)
and pinned against the reference's own interpreter in
`crates/ferrosift-operations/tests/flow.rs`.
