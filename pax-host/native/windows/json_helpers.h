#pragma once
#include <cmath>
#include <stdexcept>
#include <winrt/Windows.Data.Json.h>
namespace pax::windows {
namespace json = winrt::Windows::Data::Json;
inline bool present(json::JsonObject const& object, wchar_t const* key) {
    return object.HasKey(key) && object.GetNamedValue(key).ValueType() != json::JsonValueType::Null;
}
inline double number(json::JsonObject const& object, wchar_t const* key, double fallback = 0) {
    if (!present(object, key)) return fallback;
    double value = object.GetNamedNumber(key);
    if (!std::isfinite(value)) throw std::invalid_argument("Nonfinite Pax native property");
    return value;
}
inline bool boolean(json::JsonObject const& object, wchar_t const* key, bool fallback = false) {
    return present(object, key) ? object.GetNamedBoolean(key) : fallback;
}
inline winrt::hstring string(json::JsonObject const& object, wchar_t const* key, winrt::hstring const& fallback = {}) {
    return present(object, key) ? object.GetNamedString(key) : fallback;
}
inline json::JsonValue numeric(double value) { return json::JsonValue::CreateNumberValue(value); }
inline json::JsonValue text(winrt::hstring const& value) { return json::JsonValue::CreateStringValue(value); }
inline json::JsonObject event(wchar_t const* kind, json::JsonObject const& args) {
    json::JsonObject result; result.Insert(kind, args); return result;
}
} // namespace pax::windows
