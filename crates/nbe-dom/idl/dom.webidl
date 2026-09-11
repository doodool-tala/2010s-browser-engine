// DOM interface definitions (WebIDL subset, PP-05 v1).
// The real DOM surface grows with the DOM core package (PP-12).
// After editing, regenerate: NBE_UPDATE_GOLDENS=1 cargo test -p nbe-bindings

interface Node {
  readonly attribute unsigned long nodeType;
  attribute DOMString nodeName;
};

interface Element : Node {
  attribute DOMString tagName;
};

// Synthetic machinery-validation interface — NOT web API.
interface CodegenProbe {
  void poke(unsigned long count);
  boolean check(DOMString name, long delta, double ratio);
};
