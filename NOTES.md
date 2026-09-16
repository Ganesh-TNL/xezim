# xezim technical notes

Release notes, verified workloads and compliance results. The user guide
and the development workflow are in [README.md](README.md).

# What's new in 0.10

### Unreleased

**Correctness**

* A constant part-select write with a negative label (`x[4:-1] = v`,
  `x[0:-1] <= v`) in a clocked block or compiled process keeps its in-range
  bits (IEEE 1800 §11.5.1); the compiled path folded the bounds unsigned,
  dropped the write, and the non-blocking form stored a bit past the MSB.
* A non-blocking read of an array element through an index that holds x or
  z (`q <= mem[idx]`) queues an all-x element instead of element 0, in the
  two-state and four-state engines alike.
* A constant part-select `[l:r]` whose bound is a variable is now an
  elaboration error, as IEEE 1800 §11.5.1 requires; only `[base +: width]`
  and `[base -: width]` take a variable base. Every engine previously read
  the bound from the variable's current value.
* Bit- and part-selects follow IEEE 1800 §11.5.1 in every engine: an
  index or bound with an x or z bit reads x and discards the write instead of
  using bit 0; a constant select below a vector's declared low bound reads x
  instead of the low bits; an ascending vector (`logic [3:10] a`) places label
  `p` at bit `10 - p` for reads and writes alike, so `a[3] = 1` sets the MSB;
  an out-of-range or unknown element index into a packed 2-D value reads a
  whole-x element. A part-select that is partially out of range keeps the
  §11.5.1 per-bit form (only the out-of-range bits read x);
  `XEZIM_OOB_SELECT=whole` makes such a select read x as a whole and
  discards a write through it, as some simulators do.
* An intra-assignment delay inside an edge-triggered `always` block is
  honoured: `q <= #5 v;` schedules the update five time units out and
  `q = #5 v;` suspends the block, as in an `initial` block. Both forms
  previously assigned at once with no warning (#160).
* A class property that is a fixed array of collections (`int q[2][2][$]`,
  `int d[3][]`, `int a[2][int]`) has storage for every element: `q[i][j]`
  accepts `push_back`, `size`, `new[n]`, `exists` and element reads and
  writes, from inside the class and through a handle. Previously each
  element collection was silently empty while `$size(q)` and `foreach`
  answered off the outer shape.
* An unpacked array parameter whose elements are assignment patterns
  (`localparam cfg_t A [3] = '{'{4,2}, …}`) evaluates each element instead
  of reading 0, for packed-struct, unpacked-struct and packed-vector
  element types declared at compilation-unit scope. A constant function
  containing `signed'(e)` or `unsigned'(e)` is now evaluated at
  elaboration, so a `localparam` or typedef width derived from it in a
  sub-instance is correct (it read 0, giving one-bit typedefs).
* A continuous assignment accepts the rise/fall/turn-off delay form,
  `assign #(rise, fall[, turnoff]) net = expr;`, and applies the delay by
  transition as §10.3.3 specifies; a transition to x takes the smallest.
* Collections declared in a module keep one copy per instance. Sibling
  instances of the same module no longer share a queue, dynamic array or
  associative array, and a packed-struct element of such a collection
  (`q[i].field`) reads and writes correctly inside any sub-instance. A
  UVM-style BFM with ten per-client request queues now grants requests.
* Constrained random: without `solve … before`, the antecedent of an
  implication is drawn in proportion to the solution space it selects, as
  §18.5.10 requires, and the consequent's variables are drawn inside the
  implied ranges. `solve … before` keeps the antecedent uniform.
* VPI: `vpi_iterate(vpiPort, module)` yields port objects with name,
  full name, direction and size for the top module and sub-instances, and
  values read through the connected signal; nets and variables keep their
  `vpiNet`/`vpiReg` types. `vpiPort`, `vpiPortBit`, `vpiDirection` and the
  direction values are in `include/vpi_user.h`.
* Write-path fixes: a bit-select write whose index is x or z modifies
  nothing; a continuous assign with a constant right-hand side drives its
  net in a UVM testbench; a non-blocking assignment to a packed-array
  element through a virtual interface keeps its width; a write to a nested
  packed-struct member (`f.hdr.d = v`) lands; elements of a class
  unpacked-struct array read back what was written.
* `always @(sig)` keeps firing after a write made by a process that a
  clock edge resumed.
* A named event or signal written by a process that a clocking block
  resumed (`##2; -> ev;`) wakes its waiters in the same time step instead
  of one clock toggle later.
* A narrow actual bound to an `int`, `logic signed` or `bit signed`
  class-method or constructor formal is extended correctly (`new(2'b10)`
  read −2).
* `#delay` inside a package class, a compilation-unit class, a `$unit`
  task or function, or a `program` scales by the timescale in effect; a
  `timeunit` declared inside a package is honoured.
* Handle chains of any length (`w.r.c`) read correctly from a task inside a
  sub-instance, including a task-local handle named like a sibling
  instance.
* Locals declared inside an instance's tasks, functions and blocks shadow
  the module's own names.
* `bind` with a parameter value assignment is applied; previously the
  bound harness never existed.
* A `ref` formal named like its actual no longer overflows the stack.
* Class methods reach sibling module instances by hierarchical reference
  (issue #155): `core.seq`, `core.get_seq()` and `u_w.p.peek()` work from a
  method of a class declared inside a module.

**Performance** (instruction counts, output identical)

* A clocked block whose only uncompilable statement is a `$display`-style
  call on a cold branch (a check macro's failing arm) now runs on the
  two-state fast path; the interpreter is called for that one statement
  only when it is reached. Blocks that read a signal they later overwrite
  (`failures++`) are admitted too, with those signals restored on a bail
  so the four-state re-run stays exact, and `'0`/`'1` fill literals lower.
  A 100k-cycle self-checking testbench: 2.50G to 2.28G instructions;
  c906 neutral.
* Small clocked testbenches run about 15% fewer instructions: the per-tick
  scratch vectors of the waiter drain, the clock generators and the
  in-process combinational snapshot are reused instead of reallocated,
  parked waiters that do not fire stay in place, and a compiled stimulus
  process skips snapshotting the outputs it can never write. A 100k-cycle
  self-checking testbench: 2.94G to 2.50G instructions, output identical;
  c906 neutral.
* Experimental, opt-in: `XEZIM_TS_WIDE512=1` lets the two-state path carry
  registers up to 512 bits (the c906 vector-unit buses) instead of 128. It
  is off by default because on c906 it costs 5% more instructions: the
  admitted blocks read x-holding buses and fall back every evaluation.
* A testbench's stimulus loop in an `initial` block — clocked waits,
  blocking or non-blocking assignments, `if`/`case`, counted loops and
  system tasks — now runs as a compiled process by default instead of on
  the AST interpreter. A 100k-cycle self-checking testbench went from
  2.8 s to 0.37 s. `XEZIM_PROC_FSM=0` restores the interpreter for every
  initial block; `XEZIM_PROC_FSM=1` still compiles every body that can be.
* The `--profile` report prints the time that fell outside combinational
  entries and clocked blocks (interpreted processes, waiters) as its own
  line, so a run dominated by an interpreted testbench loop no longer
  reports 90% attributed to the little that was sampled.
* `--profile` no longer slows the run it measures: construct times come
  from a 10 kHz sampler thread instead of two clock reads around every
  evaluation. On a 16k-cell gate-level DRAM the profiled simulation phase
  went from 39% slower than an unprofiled run to about 5%; the report's
  percentages are unchanged and its header now states the sample count.
* Two- and three-part concatenations on the two-state fast path carry their
  operands in the instruction, and the settle loop prefetches the header of
  the next injected entry. C906 memcpy at 300 iterations: 2 % fewer cycles
  across both changes, output identical.
* A testbench loop that writes a memory through an absolute hierarchical
  path (`tb.x_soc.<...>.ram0.mem[i][7:0] = ...`) now compiles to bytecode
  like a local-array store. The XuanTie C910 and C906 memory-image loops
  (2.1 million stores at time 0) run in 0.9 s instead of 2.9 s; C906
  memcpy at 300 iterations: 7.7 % fewer instructions, output identical.
* A successful run exits as soon as its output is complete instead of
  first freeing the whole design; on the C910 that was 1.6 s after the
  last line of output.
* Statements run by the interpreter no longer scan the design's whole
  instance list to tell an interface instance from a signal on every
  execution; C910 memcpy at 200 iterations: 135 s -> 127 s.
* The edge detector no longer re-baselines every edge signal after each
  pass: under the dirty-edge scan only the signals that changed are
  re-baselined (plus the operands of sampled-value functions), which is
  what the scan already did for them. C906 CoreMark: 4.0 % fewer
  instructions, output identical; UVM benchmarks unchanged.
* `casez`/`casex` decoders compiled to a jump table now lay their wildcard
  chains out with forward jumps only, and a `casez`/`casex` compare against
  a constant pattern runs on the two-state fast path. The C906 instruction
  decoders (up to 7,500 instructions each) leave the 4-state interpreter;
  CoreMark 0.3 % fewer instructions, output identical.
* The bytecode compiler folds constant register chains: an unrolled
  `i = 0; bus[i] = v; i = i + 1; …` sequence (the C906 decode blocks carried
  230 chained constant adds each) becomes static bit writes, and constants
  nothing reads are dropped. C906 CoreMark: 5.2 % fewer instructions,
  output identical; UVM benchmarks unchanged. `XEZIM_FOLD_CONST_REGS=0`
  disables it.
* The idle-edge prefilter that decides whether a clocked block runs at a
  clock edge now reads one packed state byte per block instead of four
  flag arrays. C906 CoreMark: 1.3 % fewer instructions, output identical.
* Combinational blocks that write one bit or a constant-bound slice into a
  bus wider than 64 bits (`bus[k] = v;`, `dst[63:0] = src[127:64];`, the
  C906 decode-bus shapes) now run on the two-state fast path instead of the
  4-state interpreter, and clocked blocks that read wide buses skip idle
  edges in the prefilter. C906 CoreMark: 2.6 % fewer instructions on top of
  the array-arming change, output identical; UVM benchmarks unchanged.
* Edge-triggered blocks that read or write an unpacked array through a
  dynamic index (register files, memories: `q <= mem[raddr]`,
  `mem[waddr] <= d`) now take part in the idle-edge skip. Every element of
  the array arms the block on write, so an edge with no input change is
  skipped instead of re-executed. On the C906 CoreMark run 858 of 906
  previously always-executed flop blocks now skip: 8.1 % fewer
  instructions, 11 % fewer cycles. UVM benchmarks unchanged.
* `XEZIM_CYCLE_MODE=cycle` selects the cycle-based engine (default
  `event` is the engine as before). Its first stage evaluates the clock
  tree eagerly at each clock-generator edge instead of through the
  combinational worklist; on the C906 CoreMark run it converts the clocks
  of 64 % of the edge blocks, cuts settle passes by 12 %, and produces
  identical output for 0.4 % fewer instructions. Later stages will add
  cycle stepping after reset with event-driven fallback.
* The statement interpreter's three largest routines keep smaller stack
  frames (the statement dispatcher went from 7.8 KB to 3.8 KB per nested
  call), so a deeply nested testbench statement chain stays in cache: the
  UVM benchmark runs about 1.7 % fewer cycles, output identical.
* Process wake-ups are cheaper: the scheduler no longer hashes with
  SipHash, allocates an empty continuation, or clones the process scope
  string on every wake-up, and the timing wheel covers 4096 ticks before
  spilling to the ordered overflow; the next event time is memoized and
  an empty waiter list is skipped. A timed real-number model runs 26.9 %
  fewer instructions; the UVM and CPU benchmarks are unchanged.
* A delay-driven `always` block with a compound body, the timed
  integration step of a real-number model, runs from compiled bytecode
  instead of the AST interpreter: 3.8x faster per step on a fitted-lag
  model (4.1 µs to 1.07 µs), results unchanged (#159).
* Combinational settle passes track entries triggered mid-pass in a bitset
  instead of a heap, array element accesses in compiled blocks resolve
  inline, and an assignment whose value already has the target width copies
  it directly: 5.6 % fewer instructions on the C906 CoreMark run.
* Reads and writes from class methods no longer build a scoped name string
  for every lookup, and virtual-interface bindings are probed without
  allocating, an assignment no longer probes for a pending interface
  return on every write, and the width of a plain variable target is
  remembered per statement: 3.6 % fewer instructions on the axi4 AVIP.
* Clocked monitor blocks that contain a rare `#delay` run compiled instead
  of interpreted, with the same process semantics: 0.3 % fewer instructions
  on C906 CoreMark.
* Combinational settle passes evaluate each entry once per pass, and
  clocked blocks with a blocking statement no longer copy their body on
  every activation: 5.3 % fewer instructions on the C906 CoreMark run.
* Two-state blocks check for x/z as they load: 3.6 % fewer on C906
  CoreMark.
* Wide values (over 64 bits) are copied, tested and resized a word at a
  time; 128-bit concatenations and single-bit replications are built in
  place: 20 % fewer on C906 CoreMark.
* Faster process re-parks and two-state block execution:
  2 % fewer on C906 CoreMark.
* Arithmetic operands are evaluated once when their width is needed: 9 %
  fewer on the axi4 AVIP.
* Parked `wait(cond)` processes that read only class state stay parked
  until that state changes: 11 % fewer on a UVM bench.
* Fewer per-identifier lookups inside class methods: 4.9 % fewer on the
  axi4 AVIP.
* Clocked blocks that only need arming skip the value compare, two-state
  blocks run from one contiguous code arena with a packed per-entry
  header, and the settle loop takes the two-state path before touching
  the entry table: C906 memcpy (2000 iterations) 375 s to 313 s, output
  identical.
* Two-state blocks may now carry signed narrow registers (`integer` loop
  counters and their compares), dynamic bit selects, non-blocking dynamic
  bit writes (`q[i] <= v`) and range writes into buses wider than 64 bits;
  a block is admitted after a read-before-write analysis of its control
  flow instead of a fixed statement-order rule. Every clocked block is
  offered to the two-state lowering regardless of size, and the eight most
  common adjacent instruction pairs run fused. The four `for`-loop flop
  blocks that dominated the C906 interpreter time (arbiters, fill buffer,
  GPIO) now run two-state: memcpy 313 s to 295 s, CoreMark 18 % fewer
  cycles, output identical; UVM benchmarks unchanged.
* Single-bit gates whose output only clocked blocks read (half of all
  combinational evaluations on the C906) are evaluated in one batch at the
  end of each settle pass instead of through the worklist; a default
  assignment of a wide bus (`bus = {265{1'b0}}`) and a 65..128-bit window
  written into a wider bus now lower to the two-state path; the signal
  mirror that only native (JIT) code reads is no longer maintained on
  every write in ordinary runs. C906 memcpy (2000 iterations) 295 s to
  275 s, output identical; UVM benchmarks unchanged.
* A write to a signal that no clocked block or edge sensitivity observes
  (59 % of all writes on the C906) skips the write observer after one
  lookup, and single-bit gates feeding only clocked blocks commit through
  a direct inline path. C906 memcpy 275 s to 263 s, output identical; UVM
  benchmarks unchanged.

### 0.10.5 — class covergroups, DPI exports and unit scope, faster UVM (September 2026)
* **Typedef'd packed arrays keep their dimensions inside instances**: a
  `u7_t [4:0][1:0] a` declared in an instantiated module (including every
  top of a multi-top design, which runs under the synthetic wrapper) had no
  packed geometry recorded, so `foreach (a[i, j])` walked its 70 bits
  instead of its 10 elements while the same module run as the selected top
  was right. The declared dimensions are now chained with the typedef's for
  instance variables, ports and nets alike.
* **`--profile`** prints the end-of-run profile report (by design unit,
  instance and construct, plus the opcode and entry histograms); the same
  as `XEZIM_PROFILE_REPORT=1`.

* **`foreach` and `std::randomize` over multi-dimensional targets**: a
  `foreach (a[i, j])` over a purely packed array (`u7_t [4:0][1:0]`,
  `bit [6:0][4:0][1:0]`) now iterates every named dimension, declared
  dimensions first and then the typedef's (it used to iterate one and leave
  `j` x). `std::randomize(...) with { foreach (a[i, j]) ... }` now draws a
  packed target wider than 64 bits and every element of a 2-D or N-D unpacked
  array (both were left at 0), checks the constraint body with all loop
  variables bound (it passed vacuously before), and repairs per element:
  relational bounds, `elem == e` pins, and `$countones(mask[i][j]) ==
  count[i][j]` couplings, which draw the mask with exactly that many ones.
  A `rand` class property wider than 64 bits is drawn in full as well.
* **`export "DPI-C"` aliases and package-scope exports reach C**: an export
  with a C linkage name (`export "DPI-C" c_reg_write = task reg_write;`)
  now emits the `c_reg_write` symbol the loaded library calls (it emitted the
  SV name, and the library died with `undefined symbol: c_reg_write` on its
  first call). Exports declared inside a package are registered whether the
  package is wildcard-imported, imported by name, or never imported (they
  name a global symbol either way); an unimported package's subroutine is
  reached under its qualified name.
* **Loop variables shadow a same-named variable of an inlined instance**: a
  `for (integer i = 0; ...)` or `foreach (a[i])` inside a child module that
  also declares `integer i` at module scope now binds `i` to the loop. The
  inliner used to prefix every use of `i` to the child's module variable
  while the loop's own declaration stayed bare, so the loop compared an
  x-valued `u.i` and never ran (a gray-code pointer decoder stayed at x and
  an asynchronous FIFO popped the same word forever). The interpreted form
  had the matching runtime defect: the loop variable was written by name
  through the process scope, which for a `foreach` re-triggered the block
  on its own write.
* **Associative-array probes no longer scan the whole signal table**:
  `exists()` on an absent key, the nested-element probe behind every
  associative-array check, and `first()` / `next()` key enumeration now read
  the per-array element index (one set per array) instead of comparing
  every signal name in the design against a prefix. The associative-array
  check itself exits after one byte scan when the name can only be a plain
  collection, the static-collection key is borrowed instead of allocated on
  every builtin-method call, and a dozen per-call debug and tuning flags
  (`XEZIM_ACTIVE_REGION`, `XEZIM_TRACE_SPIN`, `XEZIM_PSETTLE_STATS`, the
  `*_DBG` switches) are read once. The axi4 AVIP retires 6.7 % fewer
  instructions, output identical. `XEZIM_BM_CENSUS=1` prints every builtin
  method call as `[bm] <receiver> <method>` for aggregation.
* **`obj.randomize()` over multi-dimensional properties**: a packed
  multi-dimensional class property (`rand u7_t [4:0][1:0] d`) now has element
  geometry, so `d[i][j]` reads and writes address the element (they were
  single-bit selects) and `foreach (d[i, j])` iterates every element from a
  method or from the module. Elements of a 2-D array property wider than
  64 bits are drawn (they stayed 0). Every fixed array property is drawn on
  every call and the constraint repair then runs over the fresh draws: arrays
  under a `foreach` used to be skipped by the draw and repaired from their
  previous values, so `e[i] < 100` kept zeros and repeated calls returned the
  same values, and the draw used to clobber element pins (`a[0] == 5`
  returned 0) and `foreach` bodies that read another drawn array
  (`$countones(m[i][j]) == e[i][j]`).
* **A `forever` / `always` process no longer re-clones its loop body on every
  wake-up**: the continuation it parks with is built once per loop and
  shared afterwards (C906 memcpy retires 4.2 % fewer instructions, output
  identical). A subroutine-local `virtual` interface variable now binds in
  the frame that owns it, so two class tasks interleaved on delays keep their
  own bindings instead of reading each other's. `cover property` sites are
  tallied as covers in every clocked path, including `s_eventually` /
  `s_always` watchers and vacuous implications. From Thomas Burg's PR #150:
  the two condition-waiter drains are one parameterised routine, the
  `--max-time` hang report lists processes parked for the NBA region, and the
  `this`-property probe no longer clones the class name per lookup.
* **Packed-struct member selects no longer collide with same-named arrays**:
  inside an instance, `inp.sram_renA[2]` on a struct port compiled as a
  two-bit element select whenever any other module declared a packed
  multi-dimensional array called `sram_renA`, because the compiler's
  element-width and dimension lookups fell back to the bare leaf name. They
  now try the exact name, then the instance-scoped name, and use the bare
  leaf only for single-segment names. Elaboration now also removes the bare
  declarator keys that inlining a submodule registers for its own body
  (element widths, packed dimensions, struct layouts, string signals) once
  that instance is fully inlined, so they can no longer be matched from
  anywhere else in the design.
* **System-function results keep their LRM width in compiled blocks** (§20,
  §21): `$countones`, `$clog2`, `$bits`, `$size`, `$countbits`, and the other
  `int`-valued functions contribute 32 bits to an expression's context, the
  `bit`-valued ones 1, `$time` 64, and `$signed`/`$unsigned`/`$past` their
  argument's width. Inside an `always_ff`, `narrow <= $countones(be) >> 3`
  used to size the shift at the 4-bit target and truncate the count before
  shifting; the procedural path was already right. From the audit that
  followed: `int`-valued results are now SIGNED everywhere (`$countones(x) - 8
  < 0` compares signed, `$fgetc` end-of-file tests below zero), the
  interpreter no longer sizes a system call by evaluating it (`$fgetc(fd) &
  mask` consumed two bytes and `$urandom % n` advanced the generator twice),
  `$test$plusargs`/`$value$plusargs` return `int`, a procedural `$past(v)` is
  no longer one edge late and reports "no history" at the operand's width,
  `$sampled(e)` evaluates outside properties, and `$onehot`/`$onehot0`/
  `$isunknown` fold to one bit in constant expressions.
* **Performance round (measured with interleaved `perf stat`, output
  byte-identical in every case):** whole-net identity buffers (`assign y = x`)
  now collapse onto their source by default (`XEZIM_BUF_COLLAPSE=0` opts
  out) — the pass leaves alone any net that is a `force`/`release`/procedural
  `assign` target, any source a process writes (the copy's delta step stays
  observable), gate-driven nets, 2-state/4-state pairs, SDF designs, and
  designs with DPI/VPI libraries; C906 memcpy runs 10.7 % fewer instructions
  and 14 % less wall time, bit-exact against the reference transcript. On UVM
  workloads the runtime scalar-index helper no longer hands calls and member
  accesses to the elaboration-time constant folder (which cloned the whole
  function table per attempt), process contexts are moved rather than cloned
  across wakeups, and clocking blocks poll their clock by signal id; the axi4
  AVIP base test retires 3.7 % fewer instructions.
* **Default timescale for untimed units is `1ns/1ns`** for any module,
  interface, or package without a `` `timescale `` directive (IEEE 1800
  §3.14.2.2 leaves the default tool-defined; this matches the reference
  simulator). Previously an untimed unit reported `1s/1s` while its delays
  counted the design's global tick; now `#1`, `$time`, and `$realtime` all agree
  on nanoseconds and `--dump-timescales` flags every defaulted unit. Pass
  `--module-timescale` to pick a different default.
* **Covergroups declared inside classes work** (§19.3): the class-body
  covergroup is registered, the implicit variable it declares exists, `cg = new`
  in the constructor instantiates it, `cg.sample()` reads the object's
  properties (also when sampled from outside through `obj.cg`), a derived class
  that redeclares `cg` gets its own coverpoints, constructor formals
  (`covergroup cg (int lo, int hi)`) reach the bins, `with function
  sample(...)` formals are bound per call, `option.auto_bin_max` (coverpoint or
  covergroup level) and `cg::type_option.<field>` are honoured, and
  `$get_coverage()` reports the mean over covergroup types. Covergroup and class handles no longer share one
  integer namespace, which had dispatched class object 1 as covergroup 1.
* **DPI at compilation-unit scope** (§35.5.4): `import "DPI-C"` and
  `export "DPI-C"` written at the top of a file are visible in every module,
  like a `$unit` function; they used to be reported as undeclared. Small
  integral returns (`byte unsigned`, `shortint`) read at their declared width
  and sign in expressions, and a 1-bit `logic` argument carries x/z as svLogic.
* **A real assigned to an integral subroutine local rounds** (§6.12.2), as it
  always did for module variables; the local used to keep the real value, so
  `int div = freq / rate;` compared as 32.55 forever and a baud-clock divider
  written that way never toggled.
* **Concurrent assertions inside instantiated modules and interfaces** are
  registered and fire; inlining used to drop them silently. Sequence
  consequents (`a |-> a ##1 b ##1 c`, `a |=> s`) walk their steps cycle by
  cycle, named sequences with unclocked bodies expand, and `cover property`
  is tallied as cover with misses not counted as failures.

### 0.10.4 — power intent, packed-struct codegen, opt-in waveforms (September 2026)

* **Packed-struct member assignments compile** instead of falling back to the
  AST interpreter. `s.m`, nested `s.p.m` (and every `union`-in-struct form),
  `arr[i].m`, an assignment pattern into an array element (`arr[i] <= '{...}`),
  and a function whose body is `return '{...}` were all interpreted at roughly
  3.8 µs per statement. On a struct-payload pipeline benchmark — 8 lanes × 3
  stages of an 88-bit struct, 20k cycles — this took the run from **16.05s to
  0.83s**, reference-exact throughout. Neutral where the shape is absent
  (Ibex is instruction-identical).
* **Streaming concatenations and 2-D array stores compile.** `{>>{…}}` and
  `{<<N{…}}` lower to constant range selects plus one concat instead of the
  AST interpreter (a byte swap written `{<<8{x}}` was ~32% slower than the
  same swap written by hand; it is now within 7%). A store to a 2-D unpacked
  element (`a[i][j] <= v`) reuses the row-major flat index the read path
  already had — a 4×4 array written element-wise every cycle went from 1.92s
  to 0.16s (**12×**), and a loop containing one no longer drops to the AST
  path wholesale. The 1-D memory case always compiled.
* **Mailbox and semaphore ARRAY elements allocate on `new()`.** `mb[i] = new()`
  stored a live-looking handle with nothing behind it, so every `put` silently
  vanished, `num()` stayed 0 and `try_get` always failed, while the same
  mailbox declared as a scalar worked. Fixed for every lvalue shape: module
  scope, inside a class method, through `this.`, through a class handle, and
  in associative / dynamic / queue / multi-dimensional collections.
* **Waveform dumping is opt-in** via `--wave` (see Features). An active dump
  forces loops that would otherwise compile onto the AST path and builds a
  per-signal trace table, so a run that never dumps no longer pays for it, and
  a design that calls `$dumpvars` no longer starts writing a file
  unannounced. `--fst`/`--xtrace` imply it, so existing command lines are
  unchanged.

* **IEEE 1801 power intent** via `--upf` / `--upf-top`: supply nets with
  state and voltage, power switches, corruption of powered-down elements,
  isolation clamps and retention, driven from the testbench through the
  standard `UPF` package (`supply_on` and friends). Multi-file intent chains
  with `load_upf -scope`, `-update` merges into a named strategy, and
  `-elements {.}` names the scope instance. `examples/upf/` is a runnable
  example; see "Power intent (UPF)".
* **`release` inside a level-sensitive block** (`always @(en)`, `@*`, or a
  process resumed by `@(en)`) now returns the net to its continuous drivers
  immediately. It used to keep the forced value until the driver happened to
  change again, because the re-drive was lost inside the settle pass.
### 0.10.1 – 0.10.3 — native compilation and process conformance (August 2026)

* **AOT native backend** (`XEZIM_JIT=1 XEZIM_AOT=1`, needs a `--features jit` build) — the
  compiler emits Rust for eligible combinational entries, edge blocks, and
  process FSMs, builds it with `rustc`, and loads it through a single exported
  API symbol. On the C910 CoreMark run 18,916 / 21,305 edge blocks and
  108,248 / 215,494 combinational entries compile natively.
* **Persistent native cache** — the generated library is keyed on the source,
  optimization level, and xezim build, then stored under
  `$XEZIM_CACHE_DIR` / `$XDG_CACHE_HOME/xezim/native` / `~/.cache/xezim/native`.
  The first run pays the `rustc` cost; later runs load the cached `.so`
  directly. `XEZIM_NO_NATIVE_CACHE=1` opts out.
* **Compiled process FSMs** (`XEZIM_PROC_FSM=1`) — a blocking `always` body
  compiles into a bytecode state machine with explicit wait instructions, so a
  resume re-enters at the saved program counter with per-process registers
  instead of re-walking the AST continuation chain. Blocking tasks and
  `initial` blocks inline into the same FSM, and the FSMs themselves are
  eligible for the native backend.
* **Blocking task calls inside clocked blocks follow process semantics**
  (§9.2.2) — an `always @(posedge clk)` body that calls a task which consumes
  time is no longer executed on the fast edge path. Previously the callee's
  delay advanced simulation time while that slot's non-blocking updates were
  still queued, so a `q <= d;` scheduled before the call committed a few
  picoseconds late; edges arriving mid-call were also mishandled. Such blocks
  now run as processes: NBAs commit in their own slot, and edges that arrive
  while the body is busy are missed, matching the reference simulator.
* **Delays quantize at the declaring scope's precision** (§3.14.3) — a constant
  fractional delay such as `#0.002` is folded and snapped to the precision grid
  of the scope that declares it, at elaboration time, including delays inside
  interface and class methods.
* **`bind` by instance path** (§23.11) — `bind top.u_dut.u_sub tb u_tb();` and
  the colon form specialize only the named instances; module-name binds are
  applied before path binds, and upward references from the bound module
  resolve against the instance it was bound into.
* **Opt-in combinational region fusion** (`XEZIM_REGIONS=1`) — dependency-
  connected compiled entries fuse into topologically ordered region blocks.
  Measured net-negative on the current benchmark set (recompute cost outweighs
  the dispatch saving), so it ships off by default and stays available for
  experiments.

### 0.10.0 — waveform integrity, cocotb, and class-storage fixes (August 2026)

* **FST dumps are correct at scale** — a break-even compression case wrote the
  time table raw while flagging it compressed, corrupting large dumps in
  GTKWave; the writer now records what it actually wrote, dumps are finalized on
  `Ctrl-C`, the final time slot is flushed, and values render on the writer
  thread instead of the simulation thread. Cross-format agreement is checked by
  decoding VCD, FST, and XTrace, not by comparing file sizes.
* **cocotb backend** — Python testbenches run against xezim via
  `contrib/cocotb/xezim_runner.py`, backed by VPI timed and synchronous
  callbacks and a repaired VPI object model.
* **Scheduling-region fixes** — the postponed region is serviced from the nested
  event loop and on livelock recovery, and a delay closes out the slot it
  resumed in, which removed a `$monitor`-vs-waveform timestamp skew.
* **Class and scope storage** — class member arrays resolve through the runtime
  class, `localparam` arrays inside classes elaborate, each instance gets its
  own static task local under non-blocking assignment, packed-struct formals
  read their members from the call frame, and `%m` no longer leaks the scope of
  a suspended task.

---

# What's new in 0.9

### 0.9.8 — reference-parity audit campaign (August 2026)

Dozens of differential test batteries were run against a commercial reference
simulator; every divergence found was measured construct-by-construct, fixed,
and pinned with a regression test citing the LRM section:

* **Per-evaluator continuous-assign propagation is now the default** (#35) —
  combinational updates propagate with LRM evaluation ordering instead of a
  single batched settle, resolving process-observation orderings that were
  previously unattainable with either batching mode. Escape hatches:
  `XEZIM_EAGER_PROC_SETTLE=1` (previous default) and `XEZIM_LAZY_PROC_SETTLE=1`.
* **UVM `run_test()` termination** (#109) — the phase scheduler advances time
  through run-phase objections; live regression pins run the real Accellera
  library (1.2, 1800.2-2017, 1800.2-2020) in every CI gate.
* **Package export semantics** (§26.6) — `export P::*`, `export P::sym` and
  `export *::*` are honored: a wildcard import re-exposes a package's own
  imports only when exported, and a wildcard export covers only names the
  exporting package references — unexported/unreferenced names are rejected
  exactly as the reference rejects them.
* **Implicitly-static initializer legality** (§6.21) — a local variable with an
  initializer in a static-lifetime task/function is now a compile error
  (explicit `static`/`automatic` required), matching reference behavior;
  for-header declarations, block locals and class methods stay accepted.
* **`alias` as true net unification** (§10.11) and `trireg` charge storage
  (§6.6.4) — aliased nets share one signal slot rather than lowering to an
  assign cycle.
* **Cycle delays synchronize** (§14.11) — `##0` (and a runtime `##(n)` that
  evaluates to 0) waits for the default clocking event when off-edge and is a
  no-op at the edge; `##n` without a designated `default clocking` is rejected.
* **Formatting parity** (§21.2.1.7, §21.2.1.3) — associative arrays print with
  the reference's `'{k:v, ... }` spacing; explicit-width `%h`/`%b`/`%o`
  zero-pad to the minimal core without truncation.
* **Array-method iterators** (§7.12) — `q.sort(x) with (x)` binds the declared
  iterator (sorts and `with`-reductions no longer act on zeros); event controls
  on packed-struct fields (§9.4.2) arm the base vector with a field-value
  compare instead of waking spuriously.
* **`ref` formals alias the actual** (§13.5.2) — callee writes are visible to
  parallel observers mid-call, observer writes reach the callee, and the
  element identity of `ref arr[i]` is frozen at call time.

* **UVM 1800.2-2020.3.1 runs green** — the reference testbench passes against the
  2020.3.1 library (`UVM_ERROR : 0` / `UVM_FATAL : 0`, in/out monitors agree).
  Closing this required a general preprocessor fix (inline
  `` `ifdef ``/`` `endif `` mid-line, §22.6), class-body `localparam` constants,
  and sequencer-path fixes (`process::self()`, fork/join_none automatic-variable
  sharing).
* **User-defined nettypes** (LRM §6.6.7) — `nettype` declarations with
  user resolution functions, Z-skip, and built-in resolution.
* **Per-module timescales** — `$time`/`$realtime` scale to the calling module's
  unit; `timeunit`/`timeprecision` declarations scale delays; `$timeformat`/`%t`
  and `$printtimescale` honored; sub-ns precision down to `fs`; new
  [`--module-timescale`](#module-timescale-extension) CLI extension for
  legacy RTL with no source-level timescale.
* **String & aggregate conformance fixes** — `s[i]` read/write on string
  variables (§11.4.13), `ref`/`output` queue arguments copy back on return
  (§13.5.2), `%p` renders function-local queues/associative arrays (§21.2.1.7),
  `foreach` over a string iterates its content length, `q = {}` clears string
  queues, and a never-touched module-scope queue reports `size() == 0`.
* **Free functions no longer see the caller's class context** (§13.4) — a bare
  name in a package/module function that collided with a caller class property
  used to silently alias the property; queue-property access from outside the
  class (`obj.q.push_back(x)`, `%p` of `obj.q`) now resolves correctly.
* **Gate-level & structural robustness** — multi-dimensional packed arrays of
  unpacked elements (`arr[i][j]`, §7.4), `foreach` over negative/descending and
  packed dimensions (§12.7.3), non-ANSI ports completed by a `reg`/`logic`
  declaration (§23.2.2.1), and per-iteration uniquification of declarations
  inside **nested** generate-for loops (`for(a) for(b) localparam Idx = f(a,b)`).
* **Behavioral clocks & PLLs** — a clock generator whose delay reads a runtime
  variable (`always #(half) clk = ~clk`) now re-evaluates its period every
  toggle, so a PLL reprogrammed at runtime actually changes frequency; verified
  against a commercial simulator alongside UDP primitives, tristate/pull
  strengths, `specify`/timing-check, and divider chains.
* **Dead-clock watchdog** — `XEZIM_STUCK_CLOCK` flags a process parked on a
  clock/reset that never changes while the design keeps churning edges (an
  undriven-net / dropped-cell hang), turning a silent multi-minute grind into an
  immediate, actionable diagnostic (`warn` by default; `abort` for CI).

---

# Verified Workloads

End-to-end TEST PASSED with bit-identical results vs the workloads' own
golden expectations:

| Design | Test | sim_time / cycles | baseline wall | +O1 wall |
|---|---|---|---|---|
| XuanTie C910 (dual-core) | hello | sim_time 44695 | 95s | **73s** (1.30×) |
| XuanTie C910 | memcpy ×7000 | sim_time 101965 | 216s | **166s** (1.30×) |
| XuanTie C910 | cmark ×1 (`+iterations=1`, INIT_ZERO=1) | 167124 cycles | 87 min | **73 min** (1.19×) |
| XuanTie C906 (single-core) | memcpy ×50 | — | 99s | **88s** (1.13×) |
| XuanTie C906 | cmark ×1 (INIT_ZERO=1) | 295294 cycles | 714s | **587s** (1.22×) |
| riscv-dv (UVM 1.2) | `+num_of_tests=10` random RV32IMC | — | — | 10/10 assemble clean |

Larger runs measured during the 0.10 campaign:

| Design | Test | Result | wall |
|---|---|---|---|
| lowRISC Ibex (`simple_system`) | CoreMark ×10 | score 2.477304 CoreMark/MHz, 2,765,321 instret, halt at 41,454,505 ns — byte-identical | 447s |
| XuanTie C906 | cmark ×2 | TEST PASSED, 286,469 cycles/iteration | 516s |
| XuanTie C910 (dual-core) | cmark ×2 | TEST PASSED, CoreMark 6.327752, halt at 34,985,250 | 8,028s, including a cold native compile of the whole design |
| mbits-mirafra AVIP suite (UVM) | apb / spi / i3c / axi4 / axi4Lite / uart base tests | 6 of 6 reproduce the reference's `UVM_ERROR` counts and end times, run unmodified with no `--module-timescale` (the untimed BFMs take the `1ns/1ns` default; uart alone needs `--module-timescale 1ps/1ps`); `ahb` runs in xezim but the reference fails to elaborate it | 33s for axi4Lite (28s with FSM + AOT), about 60s for uart, seconds for the rest |

On these CPU workloads a commercial reference simulator is still roughly
4–5× faster; the campaign narrowed the Ibex CoreMark gap from about 30× to
4.3×. **Where the remaining cost sits depends on the design**, and the two
cores profile as opposites:

* **C906 is scheduling-bound.** Running the reference with its optimizer
  disabled (321s) against optimized (77s) and xezim (489s) puts ~4.2× on its
  optimizer and only ~1.5× on the kernel itself, and a symbol profile spends
  ~34% of the run evaluating the design against ~22% deciding what to
  evaluate. Every net stays externally visible, so each combinational result
  is published and its readers notified — the cost the reference's optimizer
  removes by keeping intermediate nets in registers.
* **Ibex is evaluation-bound.** ~62% of the run is in the bytecode
  interpreter (`exec_insns` alone is 38%) against ~22% scheduling. It has
  1,553 combinational entries to C906's 35,267, so the same work is spread
  over ~23× fewer, ~37× hotter blocks.

That split is why native compilation is opt-in rather than default: it is
worth ~23% on Ibex and a net loss on C906 (see **Native compilation**).

The picture is design-shape dependent, and the benchmark set above — all
CPU cores and class-based UVM — under-represents struct-heavy modern RTL. On a
struct-payload pipeline microbenchmark (8 lanes × 3 stages of an 88-bit packed
struct written member-wise, 20k cycles) xezim runs it in 0.83s against the
reference's 59.5s. That is a microbenchmark, not a workload, but it is the
shape the table above contains none of.

UVM run-phase (see [docs/uvm-guide.md](docs/uvm-guide.md)):

| Testbench | Result |
|---|---|
| GettingVerilatorStartedWithUVM vs **1800.2-2017** (`data0`/`data1`/`random`/`many_random`) | 4/4 — exact Verilator parity (monitors agree, `UVM_ERROR`/`UVM_FATAL` = 0) |
| GettingVerilatorStartedWithUVM vs **1800.2-2020.3.1** | green — in/out monitors agree (77/77 packets), `UVM_ERROR`/`UVM_FATAL` = 0 |
| sv-tests UVM 1800.2-2017 example suite | 32/35 pass (3 out of scope: deprecated UVM-1.0 macros, DPI backdoor) |

---

# Compliance

Full [sv-tests](https://github.com/chipsalliance/sv-tests) run with the
suite's own `xezim` runner (`make report RUNNERS=Xezim`), xezim 0.8.1. The
generated HTML report and per-test CSV are checked in under `reports/`
(`svtests_index.html`, `svtests_report.csv`, and `sv-tests-compliance.md`).

| Category | Pass / Total | Rate |
|---|---|---|
| **All tests** | **4354 / 4768** | **91.3 %** |
| &nbsp;&nbsp;UVM (1800.2-2017) | 484 / 487 | 99.4 % |
| &nbsp;&nbsp;non-`ivtest` | 2153 / 2237 | 96.2 % |
| &nbsp;&nbsp;Icarus `ivtest` suite | 2201 / 2531 | 87.0 % |

An earlier run scored only 52 % because a `-I` library directory
(`ivtest/ivltests/`, ~1000 mutually independent single-file tests) was scanned
too eagerly: xezim honors IEEE §23.3.2 library semantics — an `-I` dir supplies
module definitions to satisfy unresolved instantiations — but it was adopting
*every* definition in the directory, so typedefs/enums from unrelated sibling
files leaked into the primary design and failed a spurious §6.18 base-type
check. `resolve_library_modules` now pulls in only the library modules actually
reachable from the compiled design (transitively), which reclaimed ~1870
`ivtest` cases with no change to the native LRM/UVM results.

---
