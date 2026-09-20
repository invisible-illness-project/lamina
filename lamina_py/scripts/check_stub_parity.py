#!/usr/bin/env python3
"""Stub-drift guard for lamina_py (issue #29).

Fails non-zero when `python/lamina/_lamina.pyi` drifts from the PyO3 surface
declared in `src/lib.rs` and `src/*.rs`:

1. Every `m.add_class` / `m.add_function` / registered exception in lib.rs has
   a stub entry, and vice versa.
2. Every pyclass attribute (`#[pyo3(get...)]` field, enum variant, `#[getter]`)
   appears in the stub class body, and vice versa.
3. Every `#[pymethods]` function (`#[new]`, `#[staticmethod]`, methods) appears
   as a `def` in the stub class body (dunders other than `__init__` are
   advisory).
4. Type parity: Rust `Option<...>` fields/getters must be `Optional[...]` in
   the stub and non-Option must not be; scalar types must match the Rust->
   Python mapping.

Run from the lamina_py directory:  python3 scripts/check_stub_parity.py
Run from the repo root:            python3 lamina_py/scripts/check_stub_parity.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC = HERE.parent / "src"
STUB = HERE.parent / "python" / "lamina" / "_lamina.pyi"

RUST_SCALAR_MAP = {
    "f64": "float",
    "f32": "float",
    "i8": "int",
    "i16": "int",
    "i32": "int",
    "i64": "int",
    "i128": "int",
    "isize": "int",
    "u8": "int",
    "u16": "int",
    "u32": "int",
    "u64": "int",
    "u128": "int",
    "usize": "int",
    "bool": "bool",
    "String": "str",
    "&str": "str",
}


def map_rust_type(ty: str) -> str | None:
    """Best-effort Rust type -> Python stub type. None means 'unknown, skip'."""
    ty = " ".join(ty.split())
    optional = False
    while True:
        m = re.fullmatch(r"PyResult<\s*(.*)\s*>", ty)
        if m:
            ty = m.group(1)
            continue
        m = re.fullmatch(r"Option<\s*(.*)\s*>", ty)
        if m:
            optional = True
            ty = m.group(1)
            continue
        break
    inner: str | None
    if ty in RUST_SCALAR_MAP:
        inner = RUST_SCALAR_MAP[ty]
    elif re.fullmatch(r"Vec<\s*String\s*>", ty):
        inner = "List[str]"
    elif re.fullmatch(r"Vec<\s*f(32|64)\s*>", ty):
        inner = "List[float]"
    elif re.fullmatch(r"Vec<\s*(usize|u64|i64|u32|i32)\s*>", ty):
        inner = "List[int]"
    elif re.fullmatch(r"Vec<\s*bool\s*>", ty):
        inner = "List[bool]"
    elif "PyArray" in ty:
        m = re.search(r"PyArray1<\s*f(32|64)\s*>", ty)
        inner = f"NDArray[np.float{'64' if (m and m.group(1) == '64') else '64' if not m else '32'}]"
        if m and m.group(1) == "32":
            inner = "NDArray[np.float32]"
    elif ty == "()":
        inner = "None"
    else:
        inner = None
    if inner is None:
        return None
    return f"Optional[{inner}]" if optional else inner


def rust_names() -> tuple[set[str], set[str], dict[str, str]]:
    """(classes, functions, exceptions). Exceptions map name -> Python base."""
    lib = (SRC / "lib.rs").read_text()
    classes = set(re.findall(r"m\.add_class::<\w+::(\w+)>", lib))
    funcs = set(re.findall(r"wrap_pyfunction!\(\s*\w+::(\w+)\s*,", lib))
    err = (SRC / "error.rs").read_text()
    base_map = {"PyException": "Exception", "PyValueError": "ValueError", "PyRuntimeError": "RuntimeError"}
    exc: dict[str, str] = {}
    for m in re.finditer(r"create_exception!\(\s*_lamina,\s*(\w+),\s*(\w+)\s*\)", err, re.S):
        exc[m.group(1)] = base_map.get(m.group(2), m.group(2))
    return classes, funcs, exc


def parse_rust_classes() -> dict[str, dict]:
    """pyclass name -> {attrs: {name: rust_type}, methods: {name: rust_type}, enum: bool}"""
    text = "\n".join(p.read_text() for p in sorted(SRC.glob("*.rs")))
    out: dict[str, dict] = {}

    # --- pyclass structs and enums (brace-matched) ---
    for m in re.finditer(r"#\[pyclass\]", text):
        head = re.compile(r"(?:\s*#\[[^\]]*\])*\s*pub\s+(struct|enum)\s+(\w+)\s*[{\(]").match(
            text, m.end()
        )
        if not head:
            continue
        kind, name = head.group(1), head.group(2)
        open_idx = head.end() - 1
        depth, i = 1, open_idx + 1
        while depth and i < len(text):
            depth += {"{": 1, "}": -1}.get(text[i], 0)
            i += 1
        body = text[open_idx + 1 : i - 1]
        entry = out.setdefault(name, {"attrs": {}, "methods": {}, "enum": kind == "enum"})
        if kind == "enum":
            for line in body.splitlines():
                vm = re.match(r"\s*(\w+)\s*(?:[,{(]|$)", line)
                if vm and not line.strip().startswith("#"):
                    entry["attrs"].setdefault(vm.group(1), "enum-variant")
        else:
            # fields: optional #[pyo3(get...)] attr immediately before `pub f: T,`
            for fm in re.finditer(
                r"#\[pyo3\(\s*get[^)]*\)\s*\]\s*pub\s+(\w+)\s*:\s*([^,]+),", body
            ):
                entry["attrs"][fm.group(1)] = fm.group(2).strip()

    # --- pymethods impls (brace-matched, attribute-aware) ---
    for m in re.finditer(r"#\[pymethods\]", text):
        head = re.compile(r"\s*impl\s+(\w+)\s*{").match(text, m.end())
        if not head:
            continue
        name = head.group(1)
        open_idx = head.end() - 1
        depth, i = 1, open_idx + 1
        while depth and i < len(text):
            depth += {"{": 1, "}": -1}.get(text[i], 0)
            i += 1
        body = text[open_idx + 1 : i - 1]
        entry = out.setdefault(name, {"attrs": {}, "methods": {}, "enum": False})
        attrs_pending: list[str] = []
        for line in body.splitlines():
            stripped = line.strip()
            if stripped.startswith("#["):
                attrs_pending.append(stripped)
                continue
            fm = re.match(r"(?:pub\s+)?fn\s+(\w+)\s*\(", stripped)
            if fm:
                fname = fm.group(1)
                # collect return type across to the opening brace
                sig_start = body.find("fn " + fname, 0)
                ret = ""
                sig = re.search(
                    r"fn\s+" + re.escape(fname) + r"\s*\([^)]*\)\s*(?:->\s*(.*?))?\s*\{",
                    body,
                    re.S,
                )
                if sig and sig.group(1):
                    ret = sig.group(1).strip()
                if any("getter" in a for a in attrs_pending):
                    entry["attrs"][fname] = ret or "unknown"
                else:
                    pyname = "__init__" if any(a.startswith("#[new]") for a in attrs_pending) else fname
                    ov = [a for a in attrs_pending if 'name = "' in a]
                    if ov:
                        pyname = re.search(r'name = "([^"]+)"', ov[0]).group(1)
                    entry["methods"][pyname] = ret or "unknown"
                attrs_pending = []
                continue
            if stripped and not stripped.startswith("//"):
                attrs_pending = []
    return out


def parse_stub() -> tuple[dict[str, dict], set[str]]:
    """Returns ({class: {attrs: {name: type}, defs: set, bases: [str]}}, module_level_funcs)."""
    text = STUB.read_text()
    classes: dict[str, dict] = {}
    module_funcs = set(re.findall(r"^def (\w+)\(", text, re.M))
    lines = text.splitlines(keepends=True)
    i = 0
    while i < len(lines):
        m = re.match(r"^class (\w+)(?:\(([^)]*)\))?:[ ]*(.*)$", lines[i])
        if not m:
            i += 1
            continue
        name, bases, tail = m.group(1), m.group(2) or "", m.group(3)
        attrs: dict[str, str] = {}
        defs: set[str] = set()
        if tail.strip() == "...":
            i += 1  # one-liner declaration
        else:
            i += 1
            body_lines: list[str] = []
            while i < len(lines):
                line = lines[i]
                if line.strip() == "" :
                    # blank line: continue only if the block resumes after it
                    j = i
                    while j < len(lines) and lines[j].strip() == "":
                        j += 1
                    if j < len(lines) and lines[j].startswith((" ", "\t")):
                        i = j
                        continue
                    break
                if line.startswith((" ", "\t")):
                    body_lines.append(line)
                    i += 1
                else:
                    break
            body = "".join(body_lines)
            attrs = dict(re.findall(r"^    (\w+): ([^\n]+)$", body, re.M))
            defs = set(re.findall(r"^    def (\w+)\(", body, re.M))
        classes[name] = {
            "attrs": attrs,
            "defs": defs,
            "bases": [b.strip() for b in bases.split(",") if b.strip()],
        }
    return classes, module_funcs


def main() -> int:
    classes_reg, funcs_reg, exc_reg = rust_names()
    rust = parse_rust_classes()
    stub, stub_funcs = parse_stub()

    errors: list[str] = []
    advisory: list[str] = []

    # 1. module-level name coverage
    for c in sorted(classes_reg | set(exc_reg)):
        if c not in stub:
            errors.append(f"class {c}: registered in lib.rs but missing from stub")
    for c in sorted(stub):
        if c not in classes_reg and c not in exc_reg:
            errors.append(f"class {c}: in stub but not registered in lib.rs")
    for ename, ebase in sorted(exc_reg.items()):
        if ename in stub and stub[ename]["bases"] != [ebase]:
            errors.append(
                f"class {ename}: stub base {stub[ename]['bases']} != Rust base {ebase}"
            )
    for f in sorted(funcs_reg):
        if f not in stub_funcs:
            errors.append(f"function {f}: registered in lib.rs but missing from stub")
    for f in sorted(stub_funcs):
        if f not in funcs_reg:
            errors.append(f"function {f}: in stub but not registered in lib.rs")

    # 2-4. per-class member + type parity
    for cname in sorted(classes_reg):
        if cname not in rust:
            continue
        r = rust[cname]
        s = stub.get(cname, {"attrs": {}, "defs": set(), "bases": []})
        for aname, aty in sorted(r["attrs"].items()):
            if aname not in s["attrs"]:
                errors.append(f"{cname}.{aname}: attribute exists in Rust but missing from stub")
                continue
            mapped = None if aty == "enum-variant" else map_rust_type(aty)
            if mapped and mapped != s["attrs"][aname].rstrip(":"):
                errors.append(
                    f"{cname}.{aname}: stub type '{s['attrs'][aname]}' != Rust '{aty}' -> '{mapped}'"
                )
        for aname, sty in sorted(s["attrs"].items()):
            if aname not in r["attrs"]:
                errors.append(f"{cname}.{aname}: stub attribute not exposed by Rust bindings")
        for mname in sorted(r["methods"]):
            if mname not in s["defs"]:
                msg = f"{cname}.{mname}: method exists in Rust but missing from stub"
                (advisory if re.fullmatch(r"__\w+__", mname) else errors).append(msg)
        for dname in sorted(s["defs"]):
            if dname not in r["methods"]:
                msg = f"{cname}.{dname}: stub method not exposed by Rust bindings"
                (advisory if re.fullmatch(r"__\w+__", dname) else errors).append(msg)

    for msg in errors:
        print(f"ERROR   {msg}")
    for msg in advisory:
        print(f"note    {msg}")
    if errors:
        print(f"\n{len(errors)} stub-parity error(s)")
        return 1
    print("stub parity OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
