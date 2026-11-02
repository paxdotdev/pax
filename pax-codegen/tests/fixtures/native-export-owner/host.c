#include "host.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
int main(int argc, char **argv) {
    assert(argc == 2);
    uint64_t expected = strtoull(argv[1], NULL, 10);
    assert(pax_cartridge_abi_version() == 1);
    assert(probe_live() == 0 && probe_count(0) == 0);
    assert(pax_init_entry((const uint8_t *)"missing", 7) == NULL);
    assert(probe_count(0) == 0);
    void *engine = pax_init_entry(NULL, 0);
    assert(engine != NULL && pax_tick(engine, 1) == expected + 1);
    assert(probe_count(0) == 1 && probe_live() == 1);
    for (size_t index = 1; index < 4; ++index) assert(probe_count(index) == 0);
    pax_dealloc_engine(engine);
    assert(probe_live() == 0);
    puts("PASS static C host: selected root once; dependency roots never; owner freed engine");
    return 0;
}
