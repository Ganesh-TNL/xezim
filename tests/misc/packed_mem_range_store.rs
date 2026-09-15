//! `XEZIM_PACKED_MEM=1`: a blocking range store into a packed-arena cell
//! (`mem[i][7:0] = v` on a 16-bit element array) advanced nothing — the
//! executor arm ended in `continue` without stepping the program counter,
//! so the store re-ran forever and the C906 testbench's memory-image loop
//! never finished under the arena. The loop must terminate and the upper
//! byte must survive the partial write.
use xezim::simulate;

const SRC: &str = r#"
module top;
  logic [15:0] mem [0:255];
  int i, errs = 0;
  initial begin
    for (i = 0; i < 256; i = i + 1) mem[i] = 16'hFFFF;
    for (i = 0; i < 256; i = i + 1) mem[i][7:0] = i[7:0];
    for (i = 0; i < 256; i = i + 1) if (mem[i] !== {8'hFF, i[7:0]}) errs++;
  end
endmodule
"#;

fn u(sim: &xezim::compiler::Simulator, n: &str) -> u64 {
    sim.get_signal(n)
        .or_else(|| sim.get_signal(&format!("top.{}", n)))
        .unwrap_or_else(|| panic!("signal not found: {}", n))
        .to_u64()
        .unwrap_or_else(|| panic!("{} not u64-able", n))
}

#[test]
fn packed_mem_range_store_terminates_and_keeps_upper_bits() {
    // SAFETY: single-threaded test setup; no other thread reads the environment here.
    unsafe { std::env::set_var("XEZIM_PACKED_MEM", "1") };
    let sim = simulate(SRC, 100).expect("simulate failed");
    unsafe { std::env::remove_var("XEZIM_PACKED_MEM") };
    assert_eq!(u(&sim, "errs"), 0, "partial range stores into packed cells");
}
