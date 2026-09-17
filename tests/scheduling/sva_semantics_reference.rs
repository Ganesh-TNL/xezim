//! Concurrent-assertion semantics reported as issues #176–#183, one
//! reproduction each, plus a mixed testbench exercising property `or`/`and`,
//! `##[m:n]` consequents, `not` of a matching sequence, `$past(x, 2)` with
//! too little history and `disable iff` on a multi-cycle antecedent. Every
//! expected FAIL line (and every absence of one) is the reference
//! simulator's; the timestamps are the clock ticks the reference reports.
use std::path::PathBuf;
use std::process::Command;

fn run(name: &str, src: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sva_semantics_reference");
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join(format!("{name}.sv"));
    std::fs::write(&sv, src).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xezim"))
        .args(["--simulate", "-s", "t", "--no-cache", sv.to_str().unwrap()])
        .output()
        .unwrap();
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "run failed:\n{text}");
    text
}

/// The FAIL lines the run printed, sorted: the order in which two
/// different assertions report at the same clock tick is not specified.
fn fails(text: &str) -> Vec<String> {
    let mut v: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with("FAIL"))
        .map(|l| l.trim().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn past_in_nonoverlap_consequent_reads_previous_cycle() {
    let text = run("past_in_nonoverlap_consequent_reads_previous_cycle", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0; logic [1:0] c = 0;
  p: assert property (@(posedge clk) a |=> c == $past(c)) else $display("FAIL %0d", $time);
  initial begin #1 a = 1; c = 1; #10 c = 2; #30 $finish; end
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    assert_eq!(fails(&text), vec!["FAIL 15"] as Vec<&str>, "FAIL lines:\n{text}");
}

#[test]
fn multi_cycle_antecedent_triggers_on_its_last_cycle() {
    let text = run("multi_cycle_antecedent_triggers_on_its_last_cycle", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0, b = 0, c = 0;
  p: assert property (@(posedge clk) a ##1 b |-> c) else $display("FAIL %0d", $time);
  initial begin #1 a = 1; #10 a = 0; b = 1; #30 $finish; end
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    assert_eq!(fails(&text), vec!["FAIL 15"] as Vec<&str>, "FAIL lines:\n{text}");
}

#[test]
fn delay_range_consequent_waits_for_the_whole_window() {
    let text = run("delay_range_consequent_waits_for_the_whole_window", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0, b = 0;
  p: assert property (@(posedge clk) a |-> ##[1:2] b) else $display("FAIL %0d", $time);
  initial begin #1 a = 1; #10 a = 0; #10 b = 1; #30 $finish; end
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    assert_eq!(fails(&text), vec![] as Vec<&str>, "FAIL lines:\n{text}");
}

#[test]
fn not_of_a_sequence_fails_only_on_a_match() {
    let text = run("not_of_a_sequence_fails_only_on_a_match", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0, b = 0;
  p: assert property (@(posedge clk) not (a ##1 b)) else $display("FAIL %0d", $time);
  initial begin #1 a = 1; #40 $finish; end
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    assert_eq!(fails(&text), vec![] as Vec<&str>, "FAIL lines:\n{text}");
}

#[test]
fn disable_iff_cancels_an_attempt_in_flight() {
    let text = run("disable_iff_cancels_an_attempt_in_flight", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0, b = 0, r = 0;
  p: assert property (@(posedge clk) disable iff (r) a |=> b) else $display("FAIL %0d", $time);
  initial begin
    @(posedge clk); #1 a = 1;
    @(posedge clk); #1 a = 0; r = 1;
    @(posedge clk); #1 r = 0;
    repeat (3) @(posedge clk); $finish;
  end
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    assert_eq!(fails(&text), vec![] as Vec<&str>, "FAIL lines:\n{text}");
}

#[test]
fn property_and_or_parse_and_run() {
    let text = run("property_and_or_parse_and_run", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0, b = 0, c = 0;
  p1: assert property (@(posedge clk) a |-> b or c);
  p2: assert property (@(posedge clk) (a |-> b) and (c |-> a));
  initial #20 $finish;
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    assert_eq!(fails(&text), vec![] as Vec<&str>, "FAIL lines:\n{text}");
}

#[test]
fn parenthesised_consequent_matches_the_bare_form() {
    let text = run("parenthesised_consequent_matches_the_bare_form", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0, b = 0;
  p1: assert property (@(posedge clk) a |-> ##2 b) else $display("FAIL p1 %0d", $time);
  p2: assert property (@(posedge clk) (a) |-> (##2 (b))) else $display("FAIL p2 %0d", $time);
  initial begin #1 a = 1; #10 a = 0; #10 b = 1; #30 $finish; end
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    assert_eq!(fails(&text), vec![] as Vec<&str>, "FAIL lines:\n{text}");
}

#[test]
fn mixed_property_shapes_match_the_reference() {
    let text = run("mixed", r#"
module t; logic clk = 0; always #5 clk = ~clk;
  logic a = 0, b = 0, c = 0, d = 0, e = 0, r = 0; logic [1:0] v = 0;
  // property or: a=1 at 5 with b=0,c=1 holds; at 15 with b=0,c=0 fails
  p_or:  assert property (@(posedge clk) a |-> b or c) else $display("FAIL or %0d", $time);
  // property and of two implications: d=0 while a,c=1 fails at 5
  p_and: assert property (@(posedge clk) (a |-> b or c) and (c |-> d)) else $display("FAIL and %0d", $time);
  // ranged consequent: e rises 2 cycles after a's first pulse -> ok; never after a's second -> fail when window closes
  p_rng: assert property (@(posedge clk) a |=> ##[1:2] e) else $display("FAIL rng %0d", $time);
  // not of a sequence that DOES match: a at 25 then b at 35
  p_not: assert property (@(posedge clk) not (a ##1 b)) else $display("FAIL not %0d", $time);
  // $past depth 2 in a consequent
  p_p2:  assert property (@(posedge clk) a |=> v == $past(v, 2)) else $display("FAIL p2 %0d v=%0d", $time, v);
  // disable iff kills a multi-cycle antecedent in flight
  p_dis: assert property (@(posedge clk) disable iff (r) a ##1 b |-> d) else $display("FAIL dis %0d", $time);
  initial begin
    #1 a = 1; c = 1; v = 1;          // sampled at 5
    #10 c = 0; v = 2;                // 15: a=1 b=0 c=0
    #10 a = 0; e = 1; v = 3;         // 25: e at 25 -> covers |=> ##[1:2] from attempt at 5 (window 15..25)
    #10 a = 1; e = 0; b = 0;         // 35: a=1
    #10 a = 0; b = 1; r = 1;         // 45: b=1 (matches a ##1 b) but r=1 disables p_dis
    #10 b = 0; r = 0;
    #40 $finish;
  end
endmodule
"#);
    assert!(text.contains("$finish called"), "did not finish:\n{text}");
    let mut expected = vec![
        "FAIL and 5",
        "FAIL or 15",
        "FAIL p2 15 v=2",
        "FAIL and 15",
        "FAIL p2 25 v=3",
        "FAIL or 35",
        "FAIL and 35",
        "FAIL rng 45",
        "FAIL not 45",
        "FAIL rng 65",
    ];
    expected.sort();
    assert_eq!(fails(&text), expected, "FAIL lines:\n{text}");
}
