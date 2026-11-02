let expected = UInt64(CommandLine.arguments[1])!
precondition(pax_cartridge_abi_version() == 1)
precondition(probe_live() == 0 && probe_count(0) == 0)
let unknown = Array("missing".utf8)
precondition(unknown.withUnsafeBufferPointer { pax_init_entry($0.baseAddress, $0.count) } == nil)
precondition(probe_count(0) == 0)
let engine = pax_init_entry(nil, 0)!
precondition(pax_tick(engine, 1) == expected + 1)
precondition(probe_count(0) == 1 && probe_live() == 1)
for index in 1..<4 { precondition(probe_count(index) == 0) }
pax_dealloc_engine(engine)
precondition(probe_live() == 0)
print("PASS static Swift host: selected root once; dependency roots never; owner freed engine")
