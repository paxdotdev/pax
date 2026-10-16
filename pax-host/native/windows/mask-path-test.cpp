#include "mask_path.h"
#include <cassert>

int main() {
    using pax::windows::parse_mask_path;
    auto path = parse_mask_path("M1 2 3 4 l5 -2 Q10 20 30 40 c1 2 3 4 5 6 z");
    assert(path.size() == 6);
    assert(path[1].command == 'L' && path[1].points[0] == 3);
    assert(path[2].points[0] == 8 && path[2].points[1] == 2);
    assert(path[4].points[0] == 31 && path[4].points[5] == 46);
    assert(path.back().command == 'Z');
    auto exponents = parse_mask_path("m1e1,-2E1 l.5,+.25 z m1 1 l2 2");
    assert(exponents[0].points[0] == 10 && exponents[0].points[1] == -20);
    assert(exponents[3].points[0] == 11 && exponents[3].points[1] == -19);
    for (auto malformed : {"L1 2", "Z", "M1", "Mnan 0", "M1 2 A3 4", "M1 2Z1 2", "M3e38 0 l3e38 0"}) {
        bool rejected = false;
        try { parse_mask_path(malformed); } catch (std::invalid_argument const&) { rejected = true; }
        assert(rejected);
    }
}
