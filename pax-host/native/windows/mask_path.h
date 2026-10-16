#pragma once
#include <array>
#include <cctype>
#include <cmath>
#include <cstdlib>
#include <stdexcept>
#include <string>
#include <vector>

namespace pax::windows {
// Pax's occlusion serializer emits these SVG commands. Parse once for both
// GPU mask drawing and native input geometry, so their coverage cannot drift.
struct MaskSegment {
    char command;
    std::array<float, 6> points{};
};

inline std::vector<MaskSegment> parse_mask_path(std::string const& text) {
    if (text.size() > 1024 * 1024) throw std::invalid_argument("Native mask path exceeds 1 MiB");
    std::vector<MaskSegment> result;
    char const* cursor = text.c_str();
    char command = 0;
    float x = 0, y = 0, start_x = 0, start_y = 0;
    bool open = false;
    auto whitespace = [&] {
        while (*cursor && (std::isspace(static_cast<unsigned char>(*cursor)) || *cursor == ',')) ++cursor;
    };
    auto number = [&] {
        whitespace();
        char* end = nullptr;
        float value = std::strtof(cursor, &end);
        if (end == cursor || !std::isfinite(value)) throw std::invalid_argument("Invalid native mask coordinate");
        cursor = end;
        return value;
    };
    while (true) {
        whitespace();
        if (!*cursor) break;
        if (std::isalpha(static_cast<unsigned char>(*cursor))) command = *cursor++;
        bool relative = command >= 'a' && command <= 'z';
        char kind = static_cast<char>(std::toupper(static_cast<unsigned char>(command)));
        size_t count;
        switch (kind) {
        case 'M': case 'L': count = 2; break;
        case 'Q': count = 4; break;
        case 'C': count = 6; break;
        case 'Z':
            if (!open) throw std::invalid_argument("Native mask closes an absent figure");
            result.push_back({'Z'});
            x = start_x; y = start_y; open = false; command = 0;
            continue;
        default: throw std::invalid_argument("Unsupported native mask SVG command");
        }
        if (kind != 'M' && !open) throw std::invalid_argument("Native mask must begin with moveto");
        MaskSegment segment{kind};
        for (size_t i = 0; i < count; i += 2) {
            segment.points[i] = number() + (relative ? x : 0);
            segment.points[i + 1] = number() + (relative ? y : 0);
            if (!std::isfinite(segment.points[i]) || !std::isfinite(segment.points[i + 1]))
                throw std::invalid_argument("Native mask coordinate overflow");
        }
        x = segment.points[count - 2]; y = segment.points[count - 1];
        if (kind == 'M') {
            start_x = x; start_y = y; open = true;
            command = relative ? 'l' : 'L';
        }
        result.push_back(segment);
        if (result.size() > 65536) throw std::invalid_argument("Native mask path exceeds segment budget");
    }
    return result;
}
} // namespace pax::windows
