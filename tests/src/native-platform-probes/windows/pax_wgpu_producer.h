#pragma once
#include <cstdint>
#include <string>
#include <windows.h>

// Loads only the adjacent, explicitly built probe library. The context owns
// WGPU/Pax resources; the caller drains its queue and releases native wrappers
// before destroying this owner. Rust TLS/deferred callback code requires the
// module itself to remain loaded until process teardown, independently of GPU
// resource ownership. Windows' loader owns that explicit pin.
class PaxWgpuProducer
{
  public:
    struct ExportInfo
    {
        void *device;
        void *queue;
        void *scene;
        void *mask;
    };

  private:
    using Create = void *(*)(bool, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t, ExportInfo *);
    using Render = bool (*)(void *, float);
    using Drop = void (*)(void *);
    using Error = char const *(*)();
    HMODULE module = nullptr;
    void *context = nullptr;
    Render renderFrame = nullptr;
    Drop drop = nullptr;
    Error lastError = nullptr;

    template <typename T> T function(char const *name)
    {
        auto result = GetProcAddress(module, name);
        if (!result)
            winrt::throw_last_error();
        return reinterpret_cast<T>(result);
    }
    [[noreturn]] void fail()
    {
        throw winrt::hresult_error(E_FAIL, winrt::to_hstring(lastError()));
    }

  public:
    ~PaxWgpuProducer()
    {
        if (context)
            drop(context);
        if (module)
            FreeLibrary(module);
    }
    ExportInfo initialize(bool allowSoftware, uint32_t sceneWidth, uint32_t sceneHeight, uint32_t editorWidth,
                          uint32_t editorHeight, uint32_t clearWidth)
    {
        wchar_t path[32768];
        auto length = GetModuleFileNameW(nullptr, path, static_cast<DWORD>(std::size(path)));
        if (!length || length == std::size(path))
            winrt::throw_last_error();
        std::wstring library(path, length);
        library.resize(library.find_last_of(L"\\/") + 1);
        library += L"pax_windows_wgpu_export_probe.dll";
        module = LoadLibraryExW(library.c_str(), nullptr,
                                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
        if (!module)
            winrt::throw_last_error();
        // Unloading a Rust DLL while its calling thread lives can leave TLS
        // destructor callbacks pointing into unmapped code. Balance our normal
        // LoadLibrary reference in the destructor, but let the loader keep code
        // resident through thread/process shutdown. This does not retain context.
        HMODULE processLifetimeModule = nullptr;
        if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_PIN, library.c_str(), &processLifetimeModule))
            winrt::throw_last_error();
        FreeLibrary(processLifetimeModule); // Balance GetModuleHandleEx; its process pin remains.
        auto create = function<Create>("pax_probe_gpu_new");
        renderFrame = function<Render>("pax_probe_gpu_render");
        drop = function<Drop>("pax_probe_gpu_drop");
        lastError = function<Error>("pax_probe_last_error");
        ExportInfo result{};
        context =
            create(allowSoftware, sceneWidth, sceneHeight, editorWidth, editorHeight, clearWidth, &result);
        if (!context)
            fail();
        return result;
    }
    void render(float phase)
    {
        if (!renderFrame(context, phase))
            fail();
    }
};
