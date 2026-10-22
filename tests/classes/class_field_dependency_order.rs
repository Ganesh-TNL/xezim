//! A CLASS-FIELD initializer that READS another field must see that field's
//! computed value, not its default.
//!
//! `construction.rs` evaluates property initializers against a half-built
//! object: the ones containing a call are deferred to construction time and
//! evaluated once each in declaration order, but a plain initializer such as
//! `string h_derived = {h, ".derived"};` is captured at declaration time while
//! `h` is still `""`, so the derived field is born as `".derived"`.  The same
//! hole covers `int r_plus1 = r + 1;` over a `$urandom_range` field, which comes
//! out as 1.  The settlement pass re-evaluates the plain fields that (transitively)
//! read a deferred field, each exactly once, and leaves everything else alone.
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
class k_c;
  string lit       = "literal";                  // no dependency
  string lit2      = {lit, "-tail"};             // reads a plain field
  int    r         = $urandom_range(1, 9999);    // deferred, no dependency
  string h         = $sformatf("%m");             // deferred
  string h_derived = {h, ".derived"};            // one hop
  string h_deep    = {h_derived, ".deep"};       // two hops
  int    r_plus1   = r + 1;                      // integer chain
  function new();
  endfunction
  function string self_derived();
    return {this.h, ".derived"};                 // `this.` read of a deferred field
  endfunction
endclass

module k_leaf;
  k_c obj = new();
  initial begin
    #1;
    $display("LIT=%s",      obj.lit);
    $display("LIT2=%s",     obj.lit2);
    $display("R=%0d",       obj.r);
    $display("H=%s",        obj.h);
    $display("DERIVED=%s",  obj.h_derived);
    $display("DEEP=%s",     obj.h_deep);
    $display("RPLUS1=%0d",  obj.r_plus1);
    $display("THIS_DERIVED=%s", obj.self_derived());
    $finish;
  end
endmodule

module tb;
  k_leaf u_leaf();
endmodule
"#;

/// Fields derived from a deferred field — one hop, two hops, and the integer
/// chain — hold the dependency's value, not the default captured at
/// declaration time.
#[test]
fn class_field_reader_of_deferred_field_is_re_evaluated() {
    let out = lines(SRC, "tb").join("\n");
    let get = |k: &str| -> String {
        out.lines()
            .find(|l| l.starts_with(&format!("{k}=")))
            .and_then(|l| l.split_once('=').map(|(_, v)| v.to_string()))
            .unwrap_or_else(|| panic!("{k} printed: {out}"))
    };
    let h = get("H");
    assert!(!h.is_empty(), "$sformatf(\"%m\") folded to \"\": {out}");
    assert_eq!(get("DERIVED"), format!("{h}.derived"), "{out}");
    assert_eq!(get("DEEP"), format!("{h}.derived.deep"), "{out}");
    let r: i64 = get("R").parse().expect("R numeric");
    let rp: i64 = get("RPLUS1").parse().expect("RPLUS1 numeric");
    assert!((1..=9999).contains(&r), "$urandom_range folded: {out}");
    assert_eq!(rp, r + 1, "r_plus1 kept the folded dependency: {out}");
    // A method reading `this.h` runs after construction, so it is the oracle
    // for what the field initializer must have captured.
    assert!(
        out.contains(&format!("THIS_DERIVED={}", get("DERIVED"))),
        "`this.h` and the field initializer disagree: {out}"
    );
}

/// Fields with no deferred dependency keep their declaration-order value:
/// literals and a field built from another plain field.
#[test]
fn plain_class_field_initializers_are_untouched() {
    let out = lines(SRC, "tb").join("\n");
    assert!(out.contains("LIT=literal"), "{out}");
    assert!(out.contains("LIT2=literal-tail"), "{out}");
}