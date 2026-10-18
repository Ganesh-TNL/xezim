// Regression tests for specialization-argument parameter resolution.
//
// Each case is a reproducer from the specialization-identity report set,
// embedded VERBATIM so it stays diffable against
// final-issues-folder/<file>.sv.  Every case self-reports: its `tb` module
// prints one `| PASS`/`| FAIL` line per check, then `| SUMMARY fails=N` and a
// final `| TEST_PASS`/`| TEST_FAIL`.  The reference simulator prints
// TEST_PASS for all of them, which is what these tests pin.

use std::process::Command;

fn run_case(tag: &str, src: &str) -> String {
    let dir = std::env::temp_dir().join(format!("xezim_specid_spec_arg_param_folding_{0}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join(format!("spec_arg_param_folding_{tag}.sv"));
    std::fs::write(&sv, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_xezim"))
        .arg(sv.to_str().unwrap())
        .output()
        .expect("failed to run xezim");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push('\n');
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

fn assert_case_passes(tag: &str, src: &str) {
    let out = run_case(tag, src);
    let failed: Vec<&str> = out.lines().filter(|l| l.contains("| FAIL")).collect();
    assert!(
        out.contains("| TEST_PASS") && failed.is_empty(),
        "{tag} did not match the reference simulator:{0}",
        failed
            .iter()
            .map(|l| format!("\n  {l}"))
            .collect::<String>()
    );
}

/// A class parameter name declared by some UNRELATED class must not stop a design parameter argument folding to its value (8.25.1, 6.20.2).
#[test]
fn unrelated_class_param_name_does_not_veto_folding() {
    const SRC: &str = r#"// issue-03-unrelated-class-param-name-vetoes-folding.sv
//
// Issue 3 of 9 (root-cause cluster B2)  *** RECOMMENDED FIRST FIX ***
// "Folding a value argument to its constant is vetoed by a same-named
//  parameter declared by ANY class anywhere in the design"
//
// Pure SystemVerilog - NO UVM.
//
// This is not one of the three umbrella issues: it is an independent defect in
// the same code path, and it is why the umbrella reproducers under-report.
//
// Groups b1..b4 show the veto coming from an UNRELATED class.  Group C, folded
// in from the value-parameter umbrella, shows the same veto coming from the
// TARGET class's own parameter - the common real-world form, and the one no
// other reproducer exercised.
//
// `vw#(QLP)` folds to `vw#(7)` correctly -- see the QZ/QY controls below, which
// pass today.  Declaring an entirely UNRELATED class that merely happens to have
// a parameter *named* QLP makes the two spellings resolve to two different
// specializations, even though that class is never instantiated, referenced or
// mentioned again in this file.  A `type` parameter name
// (`unrelated2#(type QW = int)`) vetoes folding in exactly the same way.
//
// IEEE 1800-2017 6.20.2 / 8.25.1: the argument denotes the constant 7 (resp. 5),
// so both spellings must select the SAME specialization and share ONE static
// cell.
//
// Reference-simulator behaviour: every check below prints PASS.
// Current behaviour of this simulator: ctl1 and ctl2 PASS, b1..b4 FAIL.
//
// The veto is therefore DESIGN-WIDE, not scope based: the only thing the two
// unrelated classes contribute to the compilation is the SPELLING of their
// parameter names, and that alone suppresses constant folding at a call site
// that has nothing to do with them.
//
// Probe ids: v72 (controls) v73 v74 v40 v41 v42 v43 v44 v45, plus the true
// control v76 versus the mislabelled v40 (probes_value.py)

package pv;
  localparam int QLP = 7;          // name collides with unrelated#(int QLP)
  localparam int QZ  = 11;         // names used by NO class parameter
  localparam int QY  = 13;
  parameter int  QW  = 5;          // name collides with unrelated2#(type QW)
  parameter int  W   = 5;          // name collides with vdb's OWN parameter
endpackage

class vw #(int N = 1);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

// Group C target: the TARGET class's own parameter is named W, the same name as
// the package parameter.  Nothing else in the design uses the name W.
class vdb #(int W = 1);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

// Nothing below is ever instantiated or referenced.  The only thing these
// classes contribute to the design is the SPELLING of their parameter names.
class unrelated #(int QLP = 1);
endclass

class unrelated2 #(type QW = int);
endclass

import pv::*;

module tb;
  // control: a bare MODULE localparam, a name no class parameter uses
  localparam int LP = 5;

  int fails = 0;

  function void check(string name, string got, string want);
    if (got == want)
      $display("| PASS %s (got %s)", name, got);
    else begin
      fails++;
      $display("| FAIL %s: got '%s' want '%s'", name, got, want);
    end
  endfunction

  initial begin
    // ---- control: QZ is not a class parameter name anywhere -> folds ----
    vw#(QZ)::set("ctl1");
    check("ctl1_unshadowed_pkglocalparam_folds", vw#(11)::get(), "ctl1");

    // ---- control: a second unshadowed name folds the same way ----
    vw#(QY)::set("ctl2");
    check("ctl2_second_unshadowed_name_folds", vw#(13)::get(), "ctl2");

    // ---- b1: QLP is a parameter name of an unrelated class -> no fold ----
    vw#(QLP)::set("b1");
    check("b1_pkglocalparam_blocked_by_unrelated_class", vw#(7)::get(), "b1");

    // ---- b2: the two blocked spellings must still agree with each other --
    vw#(QLP)::set("b2");
    check("b2_bare_and_qual_blocked_spellings_agree", vw#(pv::QLP)::get(), "b2");

    // ---- b3: writing through the qualified spelling, reading bare ----
    vw#(pv::QLP)::set("b3");
    check("b3_qual_blocked_set_bare_get", vw#(QLP)::get(), "b3");

    // ---- b4: a TYPE-parameter name of an unrelated class also vetoes ----
    vw#(QW)::set("b4");
    check("b4_pkgparam_blocked_by_unrelated_typeparam", vw#(5)::get(), "b4");

    // ============ group C: SELF-shadowing, target class owns the name ========
    // Folded in from the value-parameter umbrella.  b1..b4 are vetoed by an
    // UNRELATED class's parameter name; here the veto comes from the TARGET
    // class's own parameter.  Same code path, opposite end of it: the argument
    // is written at module scope and must be resolved THERE, where `W` is
    // pv::W == 5 -- not in the scope of the class being specialized, where `W`
    // is vdb's own parameter.  This is the common real-world form, e.g.
    // `class cfg #(int T)` alongside a package parameter `T`.
    //
    // If a fix for b1..b4 leaves these three failing, the veto has TWO sources
    // and this group must be split into its own issue.
    //
    // (Renamed from the umbrella: the package here is `pv`, so `my_pkg::W`
    //  became `pv::W`; nothing else changed.)
    vdb#(W)::set("c6a");                       // bare imported package parameter
    check("c6_qual_value_get", vdb#(pv::W)::get(), "c6a");
    check("c6_literal_value_get", vdb#(5)::get(), "c6a");
    vdb#(pv::W)::set("c6b");                   // package-qualified parameter
    check("c6_bare_value_get", vdb#(W)::get(), "c6b");

    // ---- control: the class's own parameter does NOT block an unshadowed
    //      name -- proves group C is about the name, not about the class ----
    vdb#(LP)::set("ctl3");
    check("ctl3_localparam_vs_literal", vdb#(5)::get(), "ctl3");

    $display("| SUMMARY fails=%0d", fails);
    if (fails == 0) $display("| TEST_PASS");
    else             $display("| TEST_FAIL");
  end
endmodule
"#;
    assert_case_passes("a", SRC);
}

/// A package-qualified design parameter argument folds to the same value as the bare imported spelling, so both spellings select one specialization (6.20.2, 8.25.1).
#[test]
fn qualified_package_param_folds_to_its_value() {
    const SRC: &str = r#"// issue-04-qualified-package-param-not-folded.sv
//
// Issue 4 of 8 (root-cause cluster B1)
// "A QUALIFIED package parameter is never folded to its value, even though the
//  bare (imported) spelling folds correctly"
//
// Pure SystemVerilog - NO UVM.
//
// The umbrella value-parameter reproducer uses `class vdb #(int W)` together
// with `my_pkg::W`: the class parameter and the package parameter share a name,
// which also trips the separate design-wide name-veto defect (issue 3).  Here
// the class parameter is `N` and NO class anywhere in this file declares a
// parameter called QW / QLP / QZ, so the only thing left failing is the
// package-qualified form of the argument.
//
// IEEE 1800-2017 6.20.2 / 8.25.1: a value argument selects a specialization by
// VALUE, so `vw#(pv::QW)` and `vw#(5)` name the SAME specialization.
//
// Reference-simulator behaviour: every check below prints PASS.
// Current behaviour of this simulator: the control PASSes, b1..b6 all FAIL.
//
// The control is the whole point: the BARE imported spelling `vw#(QW)` folds to
// `vw#(5)` today, so the folding machinery works -- it is only the qualified
// name lookup that is missing.
//
// Probe ids: v60 (control) v61 v62 v64 v65 v66 v69 (probes_value.py)

package pv;
  parameter int  QW  = 5;
  localparam int QLP = 7;
  localparam int QZ  = 2;
endpackage

class vw #(int N = 1);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

import pv::*;

module tb;
  int fails = 0;

  function void check(string name, string got, string want);
    if (got == want)
      $display("| PASS %s (got %s)", name, got);
    else begin
      fails++;
      $display("| FAIL %s: got '%s' want '%s'", name, got, want);
    end
  endfunction

  initial begin
    // ---- control: the BARE imported package parameter folds today ----
    vw#(QW)::set("ctl");
    check("ctl_bare_pkgparam_vs_literal", vw#(5)::get(), "ctl");

    // ---- b1: qualified package parameter vs literal ----
    vw#(pv::QW)::set("b1");
    check("b1_qual_pkgparam_set_literal_get", vw#(5)::get(), "b1");

    // ---- b2: qualified package parameter vs the bare spelling ----
    vw#(pv::QW)::set("b2");
    check("b2_qual_pkgparam_set_bare_get", vw#(QW)::get(), "b2");

    // ---- b3: qualified package LOCAL parameter vs literal ----
    vw#(pv::QLP)::set("b3");
    check("b3_qual_pkglocalparam_set_literal_get", vw#(7)::get(), "b3");

    // ---- b4: constant expression over qualified package parameters ----
    vw#(pv::QW + 1)::set("b4");
    check("b4_qual_expr_set_literal_get", vw#(6)::get(), "b4");

    // ---- b5: sum of two qualified package parameters ----
    vw#(pv::QW + pv::QZ)::set("b5");
    check("b5_two_qual_pkgparams_set_literal_get", vw#(7)::get(), "b5");

    // ---- b6: named argument with a qualified package parameter ----
    vw#(.N(pv::QW))::set("b6");
    check("b6_named_qual_pkgparam_set_literal_get", vw#(5)::get(), "b6");

    $display("| SUMMARY fails=%0d", fails);
    if (fails == 0) $display("| TEST_PASS");
    else             $display("| TEST_FAIL");
  end
endmodule
"#;
    assert_case_passes("b", SRC);
}

/// Value arguments that are nested specializations, enum members or constant expressions fold to their value before the specialization key is built (8.25.1).
#[test]
fn nested_defaulted_value_arg_folds() {
    const SRC: &str = r#"// issue-05-nested-enum-expression-value-arg.sv
//
// Issue 5 of 8 (root-cause cluster B3)
// "Value arguments are not folded when they sit inside a nested type argument,
//  when they are defaulted, or when they come from an enum-typed or
//  expression-valued package parameter"
//
// Pure SystemVerilog - NO UVM.
//
// The umbrella value-parameter reproducer only exercises a single top-level
// value argument.  The folding pass has to recurse into nested specializations
// and has to run for padded default arguments as well; neither happens today.
// No class in this file declares a parameter named QW / QLP / SZ, so the
// design-wide name-veto defect (issue 3) is deliberately out of the picture --
// what is left is the folding pass not reaching these positions.
//
// IEEE 1800-2017 6.20.2 / 8.25.1: `db#(iw#(pv::QW))` and `db#(iw#(5))` name the
// same specialization; a missing argument is replaced by its default expression,
// which is itself a constant.
//
// Reference-simulator behaviour: every check below prints PASS.
// Current behaviour of this simulator: the control PASSes, b1..b6 all FAIL.
//
// Probe ids: t24 t25 v70 v16 v15 v19 (probes_value.py, probes_type.py)

package pv;
  parameter int  QW  = 5;
  localparam int QLP = 7;
  parameter int  QD  = QW*2;
  typedef enum { SMALL = 2, BIG = 9 } sz_t;
  parameter sz_t SZ = BIG;
endpackage

class iw #(int N = 1);
endclass

class db #(type T = int);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

class vi #(int N = 1);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

// second parameter defaults to an expression over the first one
class vdb2 #(int W = 1, int D = W*2);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

// default value of the class parameter is itself a package parameter
class vp #(int N = pv::QW);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

module tb;
  int fails = 0;

  function void check(string name, string got, string want);
    if (got == want)
      $display("| PASS %s (got %s)", name, got);
    else begin
      fails++;
      $display("| FAIL %s: got '%s' want '%s'", name, got, want);
    end
  endfunction

  initial begin
    // ---- control: nested literal argument ----
    db#(iw#(3))::set("ctl");
    check("ctl_nested_literal_same_spelling", db#(iw#(3))::get(), "ctl");

    // ---- b1: value argument inside a nested type argument ----
    db#(iw#(pv::QW))::set("b1");
    check("b1_nested_qual_pkgparam_set_literal_get", db#(iw#(5))::get(), "b1");

    // ---- b2: reverse direction (literal set, nested package param get) --
    db#(iw#(5))::set("b2");
    check("b2_nested_literal_set_qual_pkgparam_get", db#(iw#(pv::QW))::get(), "b2");

    // ---- b3: defaulted second argument, first is a qualified pkg param ----
    vdb2#(pv::QW)::set("b3");
    check("b3_default_pad_qual_pkgparam_set_full_get", vdb2#(5,10)::get(), "b3");

    // ---- b4: enum-typed package parameter as a value argument ----
    vi#(pv::SZ)::set("b4");
    check("b4_enum_pkgparam_set_literal_get", vi#(9)::get(), "b4");

    // ---- b5: package parameter whose value is an expression ----
    vi#(pv::QD)::set("b5");
    check("b5_pkgparam_expr_set_literal_get", vi#(10)::get(), "b5");

    // ---- b6: class parameter whose DEFAULT is a package parameter ----
    vp#()::set("b6");
    check("b6_default_from_pkgparam_set_literal_get", vp#(5)::get(), "b6");

    $display("| SUMMARY fails=%0d", fails);
    if (fails == 0) $display("| TEST_PASS");
    else             $display("| TEST_FAIL");
  end
endmodule
"#;
    assert_case_passes("c", SRC);
}

/// Umbrella: value-parameter argument folding across the forms that must all select one specialization.
#[test]
fn value_param_folding_umbrella() {
    const SRC: &str = r#"// umbrella-10-value-param-folding.sv
//
// UMBRELLA issue 10 of 10 -- TRACKING ONLY, DO NOT FIX DIRECTLY
// "Value arguments of a parameterized class do not fold to their value when the
//  parameter lives in a package"
//
// Pure SystemVerilog - NO UVM.
//
// This reproducer is the SYMPTOM report.  It is NOT one root cause: the class
// parameter `W` and the package parameter `my_pkg::W` deliberately share a name,
// so this single file trips two independent defects at once, and its three
// failing checks under-report what each defect alone covers.  They are filed
// separately:
//
//   issue-03-unrelated-class-param-name-vetoes-folding.sv  (cluster B2)
//   issue-04-qualified-package-param-not-folded.sv         (cluster B1)
//   issue-05-nested-enum-expression-value-arg.sv           (cluster B3)
//
// This issue should be closed by the pull requests for those three issues and
// should not attract a pull request of its own.
//
// IEEE 1800-2017 6.20.2 / 8.25.1: a value parameter argument selects the
// specialization by VALUE, so with `my_pkg::W == 5` the spellings `vdb#(W)`,
// `vdb#(my_pkg::W)` and `vdb#(5)` all name the SAME specialization and must
// share ONE static cell.
//
// Reference-simulator behaviour: every check below prints PASS.

package my_pkg;
  parameter int W = 5;          // c6: package value parameter
endpackage

class vdb #(int W = 1);
  static string stored;
  static function void set(string s);
    stored = s;
  endfunction
  static function string get();
    return stored;
  endfunction
endclass

module tb;
  import my_pkg::*;

  // control: bare DESIGN parameter (module localparam) — folds on this simulator today
  localparam int LP = 5;

  int fails = 0;

  function void check(string name, string got, string want);
    if (got == want)
      $display("| PASS %s (got %s)", name, got);
    else begin
      fails++;
      $display("| FAIL %s: got '%s' want '%s'", name, got, want);
    end
  endfunction

  initial begin
    // ---- c6 (core2 cousin 6): package value-parameter spellings ----
    vdb#(W)::set("c6a");                       // bare imported package parameter
    check("c6_qual_value_get", vdb#(my_pkg::W)::get(), "c6a");
    check("c6_literal_value_get", vdb#(5)::get(), "c6a");
    vdb#(my_pkg::W)::set("c6b");               // package-qualified parameter
    check("c6_bare_value_get", vdb#(W)::get(), "c6b");

    // ---- control: module localparam folds (passes on this simulator today) ----
    vdb#(LP)::set("ctl3");
    check("ctl3_localparam_vs_literal", vdb#(5)::get(), "ctl3");

    $display("| SUMMARY fails=%0d", fails);
    if (fails == 0) $display("| TEST_PASS");
    else             $display("| TEST_FAIL");
  end
endmodule
"#;
    assert_case_passes("d", SRC);
}