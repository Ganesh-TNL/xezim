//! Which clocked blocks may skip an idle edge.
//!
//! A block may skip a fire when none of its data inputs changed. That is only
//! sound while the block alone determines its outputs, so a block whose output
//! is also driven by another block must never skip. The interesting case is
//! the one in between: two generate arms writing `r[3:2]` and `r[1:0]` write
//! the same signal OBJECT but never the same BIT, and each still owns its own
//! bits. Those may skip; `q[2:1]` beside `q[1:0]`, which share bit 1, may not.
use std::path::PathBuf;
use std::process::Command;

fn run(name: &str, src: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("edge_skip_shared_output");
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join(format!("{name}.sv"));
    std::fs::write(&sv, src).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xezim"))
        .args(["--simulate", "-s", "tb", "--no-cache", sv.to_str().unwrap()])
        .output()
        .unwrap();
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "run failed:\n{text}");
    text
}

/// `N of M gateable` from the engine's census line.
fn gateable(text: &str) -> (u32, u32) {
    let line = text
        .lines()
        .find(|l| l.contains("measure (timestamp)"))
        .unwrap_or_else(|| panic!("no measure line:\n{text}"));
    let after = line.split("timestamp): ").nth(1).unwrap();
    let blocks: u32 = after.split(' ').next().unwrap().parse().unwrap();
    let gate: u32 = after
        .split("blocks, ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    (gate, blocks)
}

/// Two blocks writing the WHOLE signal: neither may skip, or the later write
/// is lost. Without the multi-writer check this printed `q=55`.
#[test]
fn two_whole_signal_writers_cannot_skip() {
    let text = run(
        "overlap",
        r#"
module tb;
  reg clk; reg en; reg [7:0] a, b; reg [7:0] q;
  initial begin
    clk = 0; en = 0; a = 8'hAA; b = 8'h55;
    #100 en = 1;
    #10  en = 0;
    #100 $display("q=%h", q);
    $finish;
  end
  always #5 clk = ~clk;
  always @(posedge clk) q <= a;
  always @(posedge clk) if (en) q <= b;
endmodule
"#,
    );
    assert!(text.contains("q=aa"), "a skipped write was lost:\n{text}");
    let (gate, _) = gateable(&text);
    assert_eq!(gate, 0, "a doubly driven output must not be skippable:\n{text}");
}

/// Disjoint BIT slices of one register, one generate arm each: all skippable.
#[test]
fn disjoint_bit_slices_stay_skippable() {
    let text = run(
        "bits",
        r#"
module tb;
  reg clk; reg [7:0] d;
  initial begin clk = 0; d = 0; #2000 $display("d=%0d r=%0d", d, r); $finish; end
  always #5 clk = ~clk;
  always @(posedge clk) d <= d + 8'd1;
  reg [3:0] r;
  genvar i;
  generate for (i = 0; i < 4; i = i + 1) begin: H
      always @(posedge clk) r[i] <= d[i];
  end endgenerate
endmodule
"#,
    );
    let (gate, blocks) = gateable(&text);
    assert_eq!(gate, blocks, "disjoint slice writers lost their skip:\n{text}");
}

/// Overlapping slices of one register: bit 1 has two drivers, so neither
/// block may skip. Relaxing this to "same signal" alone dropped 39 of 40
/// posedge writes.
#[test]
fn overlapping_bit_slices_cannot_skip() {
    let text = run(
        "overlap_bits",
        r#"
module tb;
  reg clk = 0;
  reg [3:0] q = 0;
  int errors = 0;
  always #2 clk = ~clk;
  always @(posedge clk) q[2:1] <= 2'b11;
  always @(negedge clk) q[1:0] <= 2'b00;
  initial begin
    repeat (40) begin
      @(posedge clk); #1; if (q[2:1] !== 2'b11) errors++;
      @(negedge clk); #1; if (q[1:0] !== 2'b00) errors++;
    end
    $display("errors=%0d", errors);
    $finish;
  end
endmodule
"#,
    );
    assert!(text.contains("errors=0"), "a shared bit lost its driver:\n{text}");
    let (gate, _) = gateable(&text);
    assert_eq!(gate, 0, "overlapping slice writers must not skip:\n{text}");
}
