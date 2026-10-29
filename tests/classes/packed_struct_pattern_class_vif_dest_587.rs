//! Packed-struct assignment patterns into class properties and vif
//! array elements.
//!
//! Regression test for the layout fix on this branch: the generic
//! ExprKind::AssignmentPattern evaluation concatenated the pattern
//! items at their own literal widths and zero-extended to the target
//! (157 bits into a 242-bit struct) while every read extracted members
//! at their declared offsets -- so a '{...} literal written into a
//! packed-struct CLASS property (obj.prop, this.prop, or a bare
//! property inside a method) or a whole element of a vif FIXED array
//! scrambled the upper fields, and the corrupted struct then made the
//! downstream randomize pre-solve UNSAT in production.
//!
//! Cases: the issue #587 (Struct-Literal Assignment Mishandled) matrix
//! -- P1 positional literal into a class property, P3 literal into a
//! vif fixed-array element, P4 the production whole-struct copy out of
//! a literal-filled element, controls K1-K7 (plain-variable literal,
//! vif packed/unpacked copy, per-field workaround, per-field fill
//! sanity, local var / local unpacked array) -- extended with P2/P2b
//! literal into a bare/this property inside a method, P5 re-assign of
//! the same literal (a pattern equal to the current contents must
//! still count as handled), and P6/P7 named-member + default-key
//! patterns (unlisted members get the default, sized to the member
//! width).
use xezim::simulate;

#[test]
fn packed_struct_pattern_class_vif_dest() {
    let sim = simulate(include_str!("packed_struct_pattern_class_vif_dest_587.sv"), 100)
        .expect("simulate failed");
    let failures = sim
        .get_signal("failures")
        .or_else(|| sim.get_signal("tb_top.failures"))
        .unwrap_or_else(|| panic!("signal not found: failures"))
        .to_u64()
        .unwrap_or_else(|| panic!("failures not u64-able"));
    assert_eq!(
        failures, 0,
        "packed-struct pattern sweep: every case must hold (see FAIL lines above)"
    );
    assert!(
        sim.output.iter().any(|o| o.message.trim() == "TEST_PASS"),
        "SVTEST_PASSFAIL must print TEST_PASS"
    );
}