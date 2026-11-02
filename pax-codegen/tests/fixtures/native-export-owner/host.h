#include <stddef.h>
#include <stdint.h>
uint32_t pax_cartridge_abi_version(void);
size_t pax_cartridge_program_id(uint8_t *, size_t);
void *pax_init_entry(const uint8_t *, size_t);
uint64_t pax_tick(void *, uint64_t);
void pax_dealloc_engine(void *);
size_t probe_count(size_t);
size_t probe_live(void);
