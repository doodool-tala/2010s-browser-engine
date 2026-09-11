# ADR-0007: IDL codegen v1
Status: accepted

## Decision
- The DOM interface surface is defined in WebIDL (a minimal subset) at
  crates/nbe-dom/idl/dom.webidl and generated into
  crates/nbe-dom/src/interfaces.rs by nbe-bindings (parser + Rust
  generator). Generated code is committed, reviewable, and never
  hand-edited; regeneration uses the golden update flag, so IDL edits
  land as reviewed diffs.
- v1 subset: interfaces with single inheritance; attributes and
  methods; types unsigned long, long, boolean, double, DOMString.
- Mapping: lowerCamelCase to snake_case (per-uppercase split);
  DOMString to String for values/returns and &str for parameters;
  attributes get a getter and (unless readonly) a &mut self setter;
  methods take &self (interior mutability arrives with the real DOM).
- The generated module is exempt from rustfmt; the generator emits
  canonical formatting regardless.

## Consequences
- The DOM surface grows by editing IDL, never by hand-writing Rust —
  the same discipline Chrome and Servo use.
- A freshness test fails CI whenever the committed file does not match
  fresh generator output.
- Interface-typed attributes, overload/callback types, and acronym
  naming are deferred to the packages that need them.
