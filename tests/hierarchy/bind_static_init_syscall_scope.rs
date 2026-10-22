//! IEEE 1800-2017 §23.11 + §6.21 — a sim-time system call in a MODULE-SCOPE
//! variable initializer of a `bind`-inserted instance.
//!
//! `$sformatf("%m")` (and `$urandom*`, `$test$plusargs`, `$time`, ...) in a
//! module-scope initializer must run at time 0 under the DECLARING instance's
//! scope. Elaboration const-folds such calls to 0/"" and
//! `defer_static_syscall_inits` re-issues them as time-0 `static_init_blocks`
//! assignments — but that pass walked AST module instantiations only, and a
//! `bind`-inserted instance is not an AST instantiation in its host (it is
//! injected from `ElaboratedModule::resolved_binds`). A bound harness's
//! `string inst_path = $sformatf("%m")` therefore kept the folded "" and every
//! config_db path built from it lost its instance hierarchy. Verified against
//! the reference simulator.

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

const HARNESS: &str = r#"
module pctm_harness;
  string inst_path = $sformatf("%m");
  string pctm_direct = $sformatf("%m");
  int    urand_init  = $urandom_range(1, 9999);
  pctm_child u_nested(); // a module instantiated UNDER the bound instance
  function automatic string get_inst_hier(string suffix);
    return {inst_path, ".", suffix};
  endfunction
  initial begin
    $display("BOUND_HIER=%s", get_inst_hier(""));
    $display("BOUND_DIRECT=%s", pctm_direct);
    $display("BOUND_RAND=%0d", urand_init);
    $display("BOUND_NESTED=%s", u_nested.child_path);
  end
endmodule

module pctm_child;
  string child_path = $sformatf("%m");
endmodule

module pctm_plain;
  string inst_path = $sformatf("%m");
  initial $display("PLAIN=%s", inst_path);
endmodule

module host;
  pctm_plain u_plain();
endmodule

bind host pctm_harness v_harness (.*);
bind host pctm_child   u_child (.*);
"#;

/// The bound harness's module-scope `%m` initializers must report the bound
/// instance's own path, not "" — and a module instantiated *under* the bound
/// instance must resolve too.
#[test]
fn bind_inserted_module_scope_syscall_init_uses_bound_scope() {
    let src = format!(
        "{HARNESS}\nmodule test;\n  host u_host();\n  initial begin #1; \
         $display(\"BOUND_CHILD=%s\", u_host.u_child.child_path); end\nendmodule\n"
    );
    let out = lines(&src, "test").join("\n");
    assert!(out.contains("BOUND_HIER=test.u_host.v_harness."), "{out}");
    assert!(out.contains("BOUND_DIRECT=test.u_host.v_harness"), "{out}");
    assert!(out.contains("BOUND_CHILD=test.u_host.u_child"), "{out}");
    assert!(
        out.contains("BOUND_NESTED=test.u_host.v_harness.u_nested"),
        "{out}"
    );
    assert!(out.contains("PLAIN=test.u_host.u_plain"), "{out}");
    let rand: i64 = out
        .lines()
        .find(|l| l.starts_with("BOUND_RAND="))
        .and_then(|l| l.trim_start_matches("BOUND_RAND=").parse().ok())
        .expect("BOUND_RAND printed");
    assert!(
        (1..=9999).contains(&rand),
        "$urandom_range folded to {rand}"
    );
}

/// Two hosts bound from one `bind` directive must each get their own path —
/// the re-issued initializer is per-instance, not shared.
#[test]
fn bind_inserted_static_init_is_per_instance() {
    let src = format!(
        "{HARNESS}\nmodule test;\n  host u_a();\n  host u_b();\n  initial begin #1; \
         $display(\"BOUND_CHILD_A=%s\", u_a.u_child.child_path); \
         $display(\"BOUND_CHILD_B=%s\", u_b.u_child.child_path); end\nendmodule\n"
    );
    let out = lines(&src, "test").join("\n");
    assert!(out.contains("BOUND_HIER=test.u_a.v_harness."), "{out}");
    assert!(out.contains("BOUND_HIER=test.u_b.v_harness."), "{out}");
    assert!(out.contains("BOUND_DIRECT=test.u_a.v_harness"), "{out}");
    assert!(out.contains("BOUND_DIRECT=test.u_b.v_harness"), "{out}");
    assert!(out.contains("BOUND_CHILD_A=test.u_a.u_child"), "{out}");
    assert!(out.contains("BOUND_CHILD_B=test.u_b.u_child"), "{out}");
    assert!(
        out.contains("BOUND_NESTED=test.u_a.v_harness.u_nested"),
        "{out}"
    );
    assert!(
        out.contains("BOUND_NESTED=test.u_b.v_harness.u_nested"),
        "{out}"
    );
}