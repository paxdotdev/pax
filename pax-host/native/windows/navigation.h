#pragma once
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <string_view>
#include "json_helpers.h"

namespace pax::windows {

inline winrt::Windows::Foundation::Uri external_navigation_uri(winrt::hstring const& destination) {
    try {
        winrt::Windows::Foundation::Uri uri(destination);
        // The application-local scheme is never handed to an OS handler.
        if (uri.SchemeName().empty() || uri.SchemeName() == L"pax") return nullptr;
        return uri;
    } catch (winrt::hresult_error const&) { return nullptr; }
}

// Native routing has an application-local location, not a browser history.
// Keep the same decoded path/query contract as the Swift chassis.
class VirtualRoute {
    winrt::Windows::Foundation::Uri base_{L"pax://app/"};
    static bool relative(std::wstring_view value) {
        if (value.starts_with(L"//")) return false;
        auto colon = value.find(L':');
        auto delimiter = value.find_first_of(L"/?#");
        if (colon == std::wstring_view::npos || (delimiter != std::wstring_view::npos && colon > delimiter)) return true;
        if (colon == 0 || !((value[0] >= L'a' && value[0] <= L'z') || (value[0] >= L'A' && value[0] <= L'Z'))) return true;
        for (auto c : value.substr(1, colon - 1))
            if (!((c >= L'a' && c <= L'z') || (c >= L'A' && c <= L'Z') ||
                  (c >= L'0' && c <= L'9') || c == L'+' || c == L'-' || c == L'.')) return true;
        return false;
    }
public:
    json::JsonObject navigate(winrt::hstring const& destination) {
        using winrt::Windows::Foundation::Uri;
        // Swift ignores an empty reference. It changes neither route state
        // nor OS navigation, and WinRT also rejects it in CombineUri.
        if (destination.empty() || !relative(destination)) return nullptr;
        auto resolved = base_.CombineUri(destination);
        if (resolved.SchemeName() != L"pax" || resolved.Host() != L"app") return nullptr;
        json::JsonArray segments;
        // Swift URLComponents decodes the whole path before splitting, so an
        // escaped slash separates native route segments as well.
        auto path = Uri::UnescapeComponent(resolved.Path());
        std::wstring_view remaining(path);
        while (!remaining.empty()) {
            auto end = remaining.find(L'/');
            auto part = remaining.substr(0, end);
            if (!part.empty()) segments.Append(text(winrt::hstring(part)));
            if (end == std::wstring_view::npos) break;
            remaining.remove_prefix(end + 1);
        }
        json::JsonObject query;
        auto encoded_query = resolved.Query();
        remaining = std::wstring_view(encoded_query);
        if (remaining.starts_with(L"?")) remaining.remove_prefix(1);
        bool query_item = !remaining.empty();
        while (query_item) {
            auto end = remaining.find(L'&');
            auto part = remaining.substr(0, end);
            auto equal = part.find(L'=');
            auto key = Uri::UnescapeComponent(winrt::hstring(part.substr(0, equal)));
            auto value = equal == std::wstring_view::npos ? winrt::hstring{} :
                Uri::UnescapeComponent(winrt::hstring(part.substr(equal + 1)));
            auto values = query.HasKey(key) ? query.GetNamedArray(key) : json::JsonArray();
            values.Append(text(value)); query.Insert(key, values);
            if (end == std::wstring_view::npos) break;
            remaining.remove_prefix(end + 1);
        }
        auto encoded_fragment = resolved.Fragment();
        bool has_fragment = !encoded_fragment.empty();
        auto fragment = has_fragment ? Uri::UnescapeComponent(winrt::hstring(std::wstring_view(encoded_fragment).substr(1))) : winrt::hstring{};
        json::JsonObject location;
        location.Insert(L"path_segments", segments); location.Insert(L"query", query);
        location.Insert(L"fragment", has_fragment ? text(fragment) : json::JsonValue::CreateNullValue());

        // Reconstruct from decoded route state, including Swift's removal of a
        // trailing path slash. Query '+' stays literal; no form-url decoding.
        std::wstring next = L"pax://app/";
        for (uint32_t i = 0; i < segments.Size(); ++i) {
            if (i) next += L'/';
            next += Uri::EscapeComponent(segments.GetStringAt(i));
        }
        bool first = true;
        for (auto const& entry : query) for (auto const& value : entry.Value().GetArray()) {
            next += first ? L'?' : L'&'; first = false;
            next += Uri::EscapeComponent(entry.Key()); next += L'=';
            next += Uri::EscapeComponent(value.GetString());
        }
        if (has_fragment) { next += L'#'; next += Uri::EscapeComponent(fragment); }
        base_ = Uri(next);
        return location;
    }
};

} // namespace pax::windows
