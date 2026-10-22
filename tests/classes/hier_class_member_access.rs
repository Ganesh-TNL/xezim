//! Reading or writing a class member through a HIERARCHICAL path
//! (`tb.u_leaf.obj.field`) must walk the object, not the flat signal table.
//!
//! The member-access branch of `eval_hierarchical_ident` tried
//! `get_signal_value_by_name(<full path>)` before resolving the handle, so the
//! path was never validated against an object: a same-named flat signal in the
//! observing module won (`tb.u_leaf.obj.field` returned `tb.field`), and with a
//! member name that collides with nothing the read returned `""`.  Writes were
//! mis-routed the same way, so `tb.u_leaf.obj.n = 7` updated the shadow signal
//! and left the object at 42.  `hier_parent_handle` now resolves the parent
//! (`tb.u_leaf.obj`) to a live heap object — but only when the name statically
//! denotes a class object, which is what keeps a struct, vector, or net whose
//! value happens to be a small integer from being read back as a heap index.
//! Verified against the reference simulator.

fn lines(src: &str, top: &str) -> Vec<String> {
    xezim::simulate_multi(
        &[src.to_string()],
        1000,
        Some(top),
        &[],
        &[],
        None,
        false,
        None,
        None,
        &[],
        &[],
        None,
        &[],
        0,
        u64::MAX,
        None,
        &[],
        None,
        None,
        None,
        None,
        false,
    )
    .expect("sim")
    .output
    .iter()
    .map(|o| o.message.clone())
    .collect()
}

const SRC: &str = r#"
class n4_c;
  string field     = "LIT";
  string only_here = "UNIQ";
  int    n         = 42;
  function new();
  endfunction
  function string f();
    return "FN";
  endfunction
endclass

typedef struct { int field; int n; } n4_p;

module n4_leaf;
  n4_c obj = new();
endmodule

module n4_leaf2;
  n4_p   p;
  int    plain = 5;
  logic [7:0] vec;
  initial begin
    p.field = 11;
    p.n  = 3;
    vec  = 8'h3c;
  end
endmodule

module tb;
  n4_leaf  u_leaf();
  n4_leaf2 u_leaf2();
  n4_c     tmp;
  // Deliberately collides with the class member name: a tool that collapses
  // `tb.u_leaf.obj.field` to the bare `field` returns THIS value.
  string   field = "SHADOW";
  string   only_here = "SHADOW2";
  initial begin
    #1;
    tmp = tb.u_leaf.obj;
    $display("VIA_TMP=%s",   tmp.field);
    $display("METHOD=%s",    tb.u_leaf.obj.f());
    $display("MEMBER=%s",    tb.u_leaf.obj.field);
    $display("UNIQ=%s",      tb.u_leaf.obj.only_here);
    $display("MEMBER_N=%0d", tb.u_leaf.obj.n);
    tb.u_leaf.obj.n = 7;
    $display("WR_PATH=%0d",  tb.u_leaf.obj.n);
    $display("WR_OBJ=%0d",   tmp.n);
    $display("WR_FIELD=%s",  tb.u_leaf.obj.field);
    tb.u_leaf.obj.field = "SET";
    $display("WR_FIELD2=%s", tmp.field);
    // non-class parents must keep the ordinary signal path
    $display("STRUCT_FIELD=%0d", tb.u_leaf2.p.field);
    $display("STRUCT_N=%0d",     tb.u_leaf2.p.n);
    $display("PLAIN=%0d",    tb.u_leaf2.plain);
    $display("VEC=%0d",      tb.u_leaf2.vec);
    $finish;
  end
endmodule
"#;

/// A member read through a hierarchical path comes from the object even when a
/// same-named signal exists in the observing module.
#[test]
fn hier_read_of_class_member_walks_the_object() {
    let out = lines(SRC, "tb").join("\n");
    assert!(out.contains("VIA_TMP=LIT"), "{out}");
    assert!(out.contains("METHOD=FN"), "{out}");
    assert!(out.contains("MEMBER=LIT"), "leaf-name collapse: {out}");
    assert!(out.contains("UNIQ=UNIQ"), "leaf-name collapse: {out}");
    assert!(out.contains("MEMBER_N=42"), "{out}");
}

/// A write through the path lands in the object — the handle that already holds
/// it sees the new value too.
#[test]
fn hier_write_of_class_member_lands_in_the_object() {
    let out = lines(SRC, "tb").join("\n");
    assert!(out.contains("WR_PATH=7"), "{out}");
    assert!(
        out.contains("WR_OBJ=7"),
        "write hit the shadow signal: {out}"
    );
    assert!(out.contains("WR_FIELD=LIT"), "{out}");
    assert!(out.contains("WR_FIELD2=SET"), "{out}");
}

/// The handle guard is a static class-object check, so struct members, plain
/// variables, and vectors under an instance still resolve through the signal
/// table instead of being read back as heap indices — including `p.field`,
/// whose name collides with the class member above.
#[test]
fn non_class_parents_stay_on_the_signal_path() {
    let out = lines(SRC, "tb").join("\n");
    assert!(out.contains("STRUCT_FIELD=11"), "{out}");
    assert!(out.contains("STRUCT_N=3"), "{out}");
    assert!(out.contains("PLAIN=5"), "{out}");
    assert!(out.contains("VEC=60"), "{out}");
}