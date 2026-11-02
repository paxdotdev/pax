#pragma once
#include "../common/pax-desktop.h"
#include "json_helpers.h"
#include <windows.h>
#include <string>

namespace pax::windows {
class Cartridge {
    HMODULE module_ = nullptr;
    template<typename T> T symbol(char const* name) {
        auto address = GetProcAddress(module_, name);
        if (!address) throw std::runtime_error(std::string("Missing desktop cartridge symbol: ") + name);
        return reinterpret_cast<T>(address);
    }
public:
#define PAX_API(name) decltype(&::name) name = nullptr
    PAX_API(pax_cartridge_abi_version);
    PAX_API(pax_init_entry);
    PAX_API(pax_desktop_free_buffer);
    PAX_API(pax_desktop_initialize_gpu);
    PAX_API(pax_desktop_tick);
    PAX_API(pax_desktop_plan);
    PAX_API(pax_desktop_set_surfaces);
    PAX_API(pax_desktop_render);
    PAX_API(pax_desktop_interrupt);
    PAX_API(pax_desktop_load_image);
    PAX_API(pax_desktop_release);
    PAX_API(pax_desktop_poll);
    PAX_API(pax_desktop_activate);
    PAX_API(pax_desktop_set_property_waker);
    PAX_API(pax_desktop_has_pending_properties);
    PAX_API(pax_desktop_set_shutdown_waker);
    PAX_API(pax_desktop_pump_shutdowns);
    PAX_API(pax_desktop_request);
    PAX_API(pax_desktop_begin_close);
    PAX_API(pax_desktop_finish_close);
#undef PAX_API
    ~Cartridge() { if (module_) FreeLibrary(module_); }
    void load(winrt::hstring const& path, json::JsonObject const& exports) {
        auto abi_name = program_symbol(exports, L"abiVersion");
        auto init_name = program_symbol(exports, L"initEntry");
        module_ = LoadLibraryExW(path.c_str(), nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
        if (!module_) winrt::throw_last_error();
        // Resource sessions are drained independently. Rust thread-local cleanup
        // and callbacks can outlive a session, so code stays pinned to process exit.
        HMODULE pinned = nullptr;
        if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_PIN, path.c_str(), &pinned)) winrt::throw_last_error();
        FreeLibrary(pinned);
#define PAX_LOAD(name) name = symbol<decltype(name)>(#name)
        pax_cartridge_abi_version = symbol<decltype(pax_cartridge_abi_version)>(abi_name.c_str());
        pax_init_entry = symbol<decltype(pax_init_entry)>(init_name.c_str());
        PAX_LOAD(pax_desktop_free_buffer); PAX_LOAD(pax_desktop_initialize_gpu);
        PAX_LOAD(pax_desktop_tick); PAX_LOAD(pax_desktop_plan); PAX_LOAD(pax_desktop_set_surfaces);
        PAX_LOAD(pax_desktop_render); PAX_LOAD(pax_desktop_interrupt); PAX_LOAD(pax_desktop_load_image);
        PAX_LOAD(pax_desktop_release); PAX_LOAD(pax_desktop_poll); PAX_LOAD(pax_desktop_activate);
        PAX_LOAD(pax_desktop_set_property_waker); PAX_LOAD(pax_desktop_has_pending_properties);
        PAX_LOAD(pax_desktop_set_shutdown_waker); PAX_LOAD(pax_desktop_pump_shutdowns);
        PAX_LOAD(pax_desktop_request); PAX_LOAD(pax_desktop_begin_close); PAX_LOAD(pax_desktop_finish_close);
#undef PAX_LOAD
        if (pax_cartridge_abi_version() != 4) throw std::runtime_error("Incompatible desktop cartridge ABI (requires 4)");
    }
    static std::string program_symbol(json::JsonObject const& exports, wchar_t const* field) {
        auto name = winrt::to_string(exports.GetNamedString(field));
        if (name.empty() || name.size() > 128 || name.find_first_not_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_") != std::string::npos)
            throw std::runtime_error("Invalid native program export; rebuild the launcher, cartridge and host together");
        return name;
    }
    json::JsonObject response(PaxDesktopBuffer buffer) const {
        try {
            if (!buffer.data || !buffer.len || buffer.len > 16 * 1024 * 1024)
                throw std::runtime_error("Invalid desktop cartridge response");
            auto object = json::JsonObject::Parse(winrt::to_hstring(std::string_view(reinterpret_cast<char const*>(buffer.data), buffer.len)));
            pax_desktop_free_buffer(buffer); buffer = {};
            return object;
        } catch (...) { if (buffer.data) pax_desktop_free_buffer(buffer); throw; }
    }
    json::JsonObject result(PaxDesktopBuffer buffer) const {
        auto object = response(buffer);
        if (!object.GetNamedBoolean(L"ok")) throw std::runtime_error(winrt::to_string(object.GetNamedString(L"error")));
        return object;
    }
    template<typename F> json::JsonObject json_call(F function, PaxDesktopEngine* engine, json::IJsonValue const& value) const {
        auto bytes = winrt::to_string(value.Stringify());
        return result(function(engine, reinterpret_cast<uint8_t const*>(bytes.data()), bytes.size()));
    }
};
} // namespace pax::windows
