#include <windows.h>
#include <fstream>
#include <iterator>
#include <string>
#include "navigation.h"

using namespace pax::windows;
namespace {
bool equal(json::IJsonValue const& a, json::IJsonValue const& b) {
    if (a.ValueType() != b.ValueType()) return false;
    if (a.ValueType() == json::JsonValueType::Object) {
        auto left = a.GetObject(), right = b.GetObject();
        if (left.Size() != right.Size()) return false;
        for (auto const& pair : left)
            if (!right.HasKey(pair.Key()) || !equal(pair.Value(), right.GetNamedValue(pair.Key()))) return false;
        return true;
    }
    if (a.ValueType() == json::JsonValueType::Array) {
        auto left = a.GetArray(), right = b.GetArray();
        if (left.Size() != right.Size()) return false;
        for (uint32_t i = 0; i < left.Size(); ++i) if (!equal(left.GetAt(i), right.GetAt(i))) return false;
        return true;
    }
    return a.Stringify() == b.Stringify();
}
std::wstring variable(wchar_t const* name, wchar_t const* fallback) {
    wchar_t value[32768];
    auto count = GetEnvironmentVariableW(name, value, DWORD(std::size(value)));
    return count && count < std::size(value) ? std::wstring(value, count) : fallback;
}
}
int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int) {
    std::ofstream output(variable(L"PAX_NAVIGATION_CONTRACT_LOG", L"navigation-contract.jsonl"));
    if (!output) return 2;
    try {
        winrt::init_apartment(winrt::apartment_type::single_threaded);
        std::ifstream source(variable(L"PAX_NAVIGATION_CASES", L"navigation-cases.json"));
        if (!source) throw std::runtime_error("Navigation vectors missing");
        std::string bytes((std::istreambuf_iterator<char>(source)), std::istreambuf_iterator<char>());
        auto cases = json::JsonObject::Parse(winrt::to_hstring(bytes));
        unsigned checks = 0;
        auto check = [&](VirtualRoute& route, json::JsonObject const& value) {
            auto actual = route.navigate(value.GetNamedString(L"url"));
            auto expected = value.GetNamedObject(L"expected");
            json::JsonObject row;
            bool passed = actual && equal(actual, expected);
            row.Insert(L"url", value.GetNamedValue(L"url")); row.Insert(L"passed", json::JsonValue::CreateBooleanValue(passed));
            row.Insert(L"actual", actual ? actual.as<json::IJsonValue>() : json::JsonValue::CreateNullValue());
            row.Insert(L"expected", expected);
            output << winrt::to_string(row.Stringify()) << '\n'; output.flush();
            if (!passed) throw std::runtime_error("Route vector differs");
            ++checks;
        };
        VirtualRoute route;
        for (auto const& item : cases.GetNamedArray(L"fixture_steps")) check(route, item.GetObject());
        for (auto const& item : cases.GetNamedArray(L"native_edge_cases")) {
            auto value = item.GetObject(); VirtualRoute edge;
            if (!edge.navigate(value.GetNamedString(L"initial"))) throw std::runtime_error("Edge initial location rejected");
            check(edge, value);
        }
        for (auto const& value : cases.GetNamedArray(L"non_virtual_destinations")) {
            if (route.navigate(value.GetString())) throw std::runtime_error("External destination became virtual");
            ++checks;
        }
        for (auto value : {L"pax://app/private", L"PAX://app/private", L"relative", L"//example.com/path"}) {
            if (external_navigation_uri(value)) throw std::runtime_error("Private or relative URI allowed for OS handoff");
            ++checks;
        }
        for (auto value : {L"https://example.com/path", L"mailto:test@example.com"}) {
            if (!external_navigation_uri(value)) throw std::runtime_error("Public absolute URI rejected for OS handoff");
            ++checks;
        }
        VirtualRoute isolated;
        auto initial = isolated.navigate(L"/");
        if (!equal(initial, cases.GetNamedObject(L"initial"))) throw std::runtime_error("Session isolation failed");
        ++checks;
        output << "{\"complete\":true,\"checks\":" << checks << "}\n";
        return 0;
    } catch (winrt::hresult_error const& error) {
        output << "{\"failed\":true,\"message\":" << winrt::to_string(text(error.message()).Stringify()) << "}\n";
    } catch (std::exception const& error) {
        output << "{\"failed\":true,\"message\":" << winrt::to_string(text(winrt::to_hstring(error.what())).Stringify()) << "}\n";
    }
    return 1;
}
