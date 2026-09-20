#[test]
fn unsupported_inlined_call_discards_partial_control_flow() {
    let src = r#"
`timescale 1ns/1ns
module tb;
  logic clk = 0;
  logic [31:0] state_word = 0;
  logic [63:0] result_word;
  integer edge_count = 0;

  always #1 clk = ~clk;

  function automatic logic [63:0] make_word(input logic [31:0] source_word);
    logic [63:0] temporary_word;
    temporary_word = 0;
    if (source_word[0]) temporary_word[39:36] = 6;
    return temporary_word;
  endfunction

  always @(posedge clk) begin
    edge_count <= edge_count + 1;
    state_word <= state_word + 1;
    result_word <= make_word(state_word);
  end

  initial begin
    #100;
    if (edge_count != 50 || state_word != 50 || result_word != 64'h6000000000)
      $fatal(1, "unexpected state count=%0d state=%0d result=%h",
             edge_count, state_word, result_word);
    $display("ROLLBACK_PASS");
    $finish;
  end
endmodule
"#;

    let sim = xezim::simulate(src, 200).expect("simulation must terminate");
    assert!(sim.output.iter().any(|line| line.message == "ROLLBACK_PASS"));
}
