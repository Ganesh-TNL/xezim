//! §6.6, §23.2.2.1 — net types on ports resolve to their IEEE pull values.
//!
//! A port declared with an explicit net type (tri0, tri1, supply0, supply1,
//! wand, wor, triand, trior, trireg) takes its idle value from the net type's
//! pull behaviour, NOT from the port direction. The default_port_value() and
//! the NetDeclaration completion path both had bugs where only direction or
//! only supply0/supply1 were considered, leaving tri0/tri1 ports at x.
//!
//! Golden values per reference simulator:
//!   wire / tri               -> z
//!   tri0  / supply0          -> 0
//!   tri1  / supply1          -> 1
//!   trireg / wand / wor /
//!     triand / trior         -> z
//!   uwire / interconnect     -> z
//!
//! A strong driver overrides the pull: d_hi(tri0)=1, d_lo(tri1)=0.

use xezim::simulate;

/// Output ports with explicit net types (ANSI style) must hold their pull value.
/// When connected, the parent's net type wins; so check the PORT's own value.
const ANSI_OUTPUT_PORTS: &str = r#"
module dut (
    output tri0    o_tri0,
    output tri1    o_tri1,
    output supply0 o_sup0,
    output supply1 o_sup1,
    output wand    o_wand,
    output wor     o_wor,
    output triand  o_triand,
    output trior   o_trior,
    output trireg  o_trireg
);
endmodule
module tb;
    wire w_tri0;
    wire w_tri1;
    wire w_sup0;
    wire w_sup1;
    wire w_wand;
    wire w_wor;
    wire w_triand;
    wire w_trior;
    wire w_trireg;
    dut u (
        .o_tri0   (w_tri0),
        .o_tri1   (w_tri1),
        .o_sup0   (w_sup0),
        .o_sup1   (w_sup1),
        .o_wand   (w_wand),
        .o_wor    (w_wor),
        .o_triand (w_triand),
        .o_trior  (w_trior),
        .o_trireg (w_trireg)
    );
    int failures;
    initial begin
        failures = 0;
        // Parent actual is plain wire -> z; the port's own value is what
        // the port's net type resolves to (also z when no driver overrides).
        // For tri0/tri1/supply0/supply1, the PORT inside dut pulls to 0/1.
        if (u.o_tri0   !== 1'b0) begin failures++; end
        if (u.o_tri1   !== 1'b1) begin failures++; end
        if (u.o_sup0   !== 1'b0) begin failures++; end
        if (u.o_sup1   !== 1'b1) begin failures++; end
        if (u.o_wand   !== 1'bz) begin failures++; end
        if (u.o_wor    !== 1'bz) begin failures++; end
        if (u.o_triand !== 1'bz) begin failures++; end
        if (u.o_trior  !== 1'bz) begin failures++; end
        if (u.o_trireg !== 1'bz) begin failures++; end
    end
endmodule
"#;

/// Input ports with explicit net types must hold their pull value when unconnected.
const ANSI_INPUT_PORTS_UNCONN: &str = r#"
module dut (
    input tri0    i_tri0,
    input tri1    i_tri1,
    input supply0 i_sup0,
    input supply1 i_sup1,
    input wand    i_wand,
    input wor     i_wor
);
endmodule
module tb;
    dut u ();  // unconnected ports
    int failures;
    initial begin
        failures = 0;
        if (u.i_tri0 !== 1'b0) begin failures++; end
        if (u.i_tri1 !== 1'b1) begin failures++; end
        if (u.i_sup0 !== 1'b0) begin failures++; end
        if (u.i_sup1 !== 1'b1) begin failures++; end
        if (u.i_wand !== 1'bz) begin failures++; end
        if (u.i_wor  !== 1'bz) begin failures++; end
    end
endmodule
"#;

/// Non-ANSI port declaration + net declaration pair (completion path RC-C).
const PORT_NET_DECL_PAIR: &str = r#"
module pair_mod (p_tri0, p_tri1, p_sup0, p_sup1, p_wand);
    input  tri0    p_tri0;
    input  tri1    p_tri1;
    output supply0 p_sup0;
    output supply1 p_sup1;
    inout  wand    p_wand;
    // Net declarations override PortDeclaration types.
    tri0    p_tri0;
    tri1    p_tri1;
    supply0 p_sup0;
    supply1 p_sup1;
    wand    p_wand;
endmodule
module tb;
    tri0    n_tri0;
    tri1    n_tri1;
    supply0 n_sup0;
    supply1 n_sup1;
    wand    n_wand;
    pair_mod u (
        .p_tri0 (n_tri0),
        .p_tri1 (n_tri1),
        .p_sup0 (n_sup0),
        .p_sup1 (n_sup1),
        .p_wand (n_wand)
    );
    int failures;
    initial begin
        failures = 0;
        if (n_tri0 !== 1'b0) begin failures++; end
        if (n_tri1 !== 1'b1) begin failures++; end
        if (n_sup0 !== 1'b0) begin failures++; end
        if (n_sup1 !== 1'b1) begin failures++; end
        if (n_wand !== 1'bz) begin failures++; end
    end
endmodule
"#;

/// Strong driver beats the weak pull value.
const STRONG_DRIVER_OVERRIDES_PULL: &str = r#"
module driver (
    output tri0 d_lo,
    output tri1 d_hi
);
    // Strong assigns override the pull.
    assign d_lo = 1'b1;  // strong 1 beats tri0 pull-down
    assign d_hi = 1'b0;  // strong 0 beats tri1 pull-up
endmodule
module tb;
    tri0 w_lo;
    tri1 w_hi;
    driver u (.d_lo(w_lo), .d_hi(w_hi));
    int failures;
    initial begin
        failures = 0;
        if (w_lo !== 1'b1) begin failures++; end
        if (w_hi !== 1'b0) begin failures++; end
    end
endmodule
"#;

fn failures_count(sim: &xezim::compiler::Simulator) -> u64 {
    sim.get_signal("failures")
        .or_else(|| sim.get_signal("tb.failures"))
        .expect("signal 'failures' not found")
        .to_u64()
        .unwrap_or(1)
}

#[test]
fn ansi_output_ports_resolve_net_type_pull_values() {
    let sim = simulate(ANSI_OUTPUT_PORTS, 1000).expect("simulate failed");
    assert_eq!(failures_count(&sim), 0, "ANSI output ports with net types did not resolve to their pull values");
}

#[test]
fn ansi_input_ports_unconnected_resolve_pull_values() {
    let sim = simulate(ANSI_INPUT_PORTS_UNCONN, 1000).expect("simulate failed");
    assert_eq!(failures_count(&sim), 0, "Unconnected ANSI input ports with net types did not resolve to their pull values");
}

#[test]
fn port_net_decl_pair_completion_path_resolves_net_types() {
    let sim = simulate(PORT_NET_DECL_PAIR, 1000).expect("simulate failed");
    assert_eq!(failures_count(&sim), 0, "PortDeclaration + NetDeclaration pair did not complete correctly");
}

#[test]
fn strong_driver_overrides_weak_pull() {
    let sim = simulate(STRONG_DRIVER_OVERRIDES_PULL, 1000).expect("simulate failed");
    assert_eq!(failures_count(&sim), 0, "Strong driver did not override the weak pull value");
}