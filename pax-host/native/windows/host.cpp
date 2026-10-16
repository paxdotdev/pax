#include <windows.h>
#include <unknwn.h>
#include <MddBootstrap.h>
#include <WindowsAppSDK-VersionInfo.h>
#include <microsoft.ui.xaml.window.h>
#undef GetCurrentTime
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.UI.Xaml.Interop.h>
#include <winrt/Microsoft.UI.Dispatching.h>
#include <winrt/Microsoft.UI.Windowing.h>
#include <winrt/Microsoft.UI.Xaml.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Markup.h>
#include <winrt/Microsoft.UI.Xaml.XamlTypeInfo.h>
#include <chrono>
#include <cstring>
#include <fstream>
#include <map>
#include <memory>
#include <mutex>
#include <set>
#include "cartridge.h"
#include "composition_presenter.h"
#include "native_scene.h"
#include "assets.h"

using namespace pax::windows;
using namespace winrt;
using namespace winrt::Microsoft::UI::Dispatching;
using namespace winrt::Microsoft::UI::Windowing;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;

namespace {
std::string last_error;
json::JsonObject configuration{nullptr};
DWORD owner_thread = 0;
uint64_t next_session = 1;
bool application_started = false;
bool runtime_initialized = false;
std::mutex wake_mutex;
DispatcherQueue wake_queue{nullptr};
winrt::handle preparation_wake;
struct Session;
std::shared_ptr<Session> active;
std::map<void*, std::shared_ptr<Session>> sessions;

void log(std::string const& message) {
    OutputDebugStringA(("PaxHost: " + message + "\n").c_str());
    // Diagnostic capture is opt-in and never a prerequisite for running an app.
    wchar_t path[32768];
    auto size = GetEnvironmentVariableW(L"PAX_WINDOWS_HOST_LOG", path, DWORD(std::size(path)));
    if (size && size < std::size(path)) { std::ofstream stream(path, std::ios::app); stream << GetTickCount64() << " " << message << '\n'; }
}
std::string exception_message() {
    try { throw; }
    catch (hresult_error const& error) { return to_string(error.message()) + " HRESULT=" + std::to_string(int32_t(error.code())); }
    catch (std::exception const& error) { return error.what(); }
    catch (...) { return "Unknown native host failure"; }
}
int32_t fail(std::string const& error) { last_error = error; log("error " + error); return -1; }
void initialize_runtime() {
    if (runtime_initialized) return;
    log("bootstrap starting minimum=" WINDOWSAPPSDK_RUNTIME_VERSION_DOTQUADSTRING);
    PACKAGE_VERSION minimum{};
    minimum.Version = WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;
    check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,
        WINDOWSAPPSDK_RELEASE_VERSION_TAG_W, minimum, MddBootstrapInitializeOptions_None));
    // The facade retains this DLL for the process lifetime, as WinUI callbacks
    // and apartment teardown can outlive a session. Keep its runtime dependency
    // for that same lifetime; never run COM/bootstrap shutdown from DllMain.
    runtime_initialized = true;
    log("bootstrap initialized process_lifetime=true");
}
void thread_check() {
    if (!owner_thread) {
        // A user-owned Rust executable may not carry a WinUI template manifest.
        // Establish DPI before creating UI; never override an incompatible mode
        // that the application has already fixed through a manifest or API.
        if (!SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) &&
            !AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(), DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2))
            throw std::runtime_error("Pax Windows requires per-monitor-v2 DPI awareness before UI creation; the application already selected an incompatible mode");
        if (!AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(), DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2))
            throw std::runtime_error("Pax Windows requires per-monitor-v2 awareness on its UI thread");
        preparation_wake.attach(CreateEventW(nullptr, FALSE, FALSE, nullptr));
        if (!preparation_wake) winrt::throw_last_error();
        init_apartment(apartment_type::single_threaded);
        owner_thread = GetCurrentThreadId();
    }
    if (owner_thread != GetCurrentThreadId()) throw std::runtime_error("PaxHost operations require their original UI thread");
}
std::string bytes(uint8_t const* data, size_t length, size_t maximum) {
    if (length > maximum || (length && !data)) throw std::invalid_argument("Invalid or oversized host input");
    return length ? std::string(reinterpret_cast<char const*>(data), length) : std::string();
}
template<typename T> T* handle(json::JsonObject const& object, wchar_t const* key) {
    auto value = to_string(object.GetNamedString(key));
    size_t end = 0;
    auto address = std::stoull(value, &end, 10);
    if (!address || end != value.size() || address > UINTPTR_MAX) throw std::invalid_argument("Invalid native GPU handle");
    return reinterpret_cast<T*>(uintptr_t(address));
}
struct Observer {
    void* context;
    void (*notify)(void*, uint32_t);
    void (*release)(void*);
    ~Observer() { if (release) release(context); }
};
struct Tile {
    std::wstring signature;
    uint32_t width = 0, height = 0;
    Grid view{nullptr};
    Canvas parent{nullptr};
    CompositionPresenter::Surface surface{nullptr};
};

void wake(uint64_t token);
struct Session : std::enable_shared_from_this<Session> {
    uint64_t token = next_session++;
    Cartridge cartridge;
    Assets assets;
    PaxDesktopEngine* engine = nullptr;
    Window window{nullptr};
    DispatcherQueueTimer timer{nullptr};
    std::unique_ptr<NativeScene> scene;
    std::unique_ptr<CompositionPresenter> presenter;
    composition::Compositor compositor{nullptr};
    std::map<std::pair<uint32_t,std::wstring>, Tile> tiles;
    std::map<uint64_t,std::vector<uint64_t>> publications;
    std::shared_ptr<Observer> observer;
    hstring title = L"Pax";
    double width = 900, height = 650, min_width = 0, min_height = 0;
    bool resizable = true, user_released = false, running_tick = false, failed = false;
    uint32_t status = 0;
    uint64_t frames = 0, blocked_frames = 0;
    json::JsonArray pending_messages;
    size_t pending_message_bytes = 0;
    std::chrono::steady_clock::time_point closing_started;

    void set_status(uint32_t value) {
        status = value;
        if (auto current = observer) {
            auto queue = DispatcherQueue::GetForCurrentThread();
            if (queue) queue.TryEnqueue([current, value] { if (current->notify) current->notify(current->context, value); });
            else if (current->notify) current->notify(current->context, value);
        }
    }
    void prepare(std::string const& startup) {
        cartridge.load(configuration.GetNamedString(L"cartridge"));
        assets.configure(configuration);
        title = string(configuration, L"title", L"Pax");
        auto entry = to_string(string(configuration, L"entry"));
        engine = cartridge.pax_init_entry(0, 0, reinterpret_cast<uint8_t const*>(entry.data()), entry.size(),
            reinterpret_cast<uint8_t const*>(startup.data()), startup.size());
        if (!engine) throw std::runtime_error("Pax cartridge initialization failed");
        cartridge.pax_desktop_set_property_waker(engine, token, wake);
        cartridge.pax_desktop_set_shutdown_waker(token, wake);
    }
    bool interrupt(json::JsonObject const& value) {
        if (status >= 3 || failed) return false;
        try { return cartridge.json_call(cartridge.pax_desktop_interrupt, engine, value).GetNamedBoolean(L"preventDefault"); }
        catch (...) {
            failed = true; fail(exception_message());
            if (timer) timer.Stop();
            if (window) window.Title(L"Pax — stopped: " + to_hstring(last_error));
            return false;
        }
    }
    void release_publication(uint64_t batch, ID3D12Fence* fence, uint64_t value) {
        auto found = publications.find(batch);
        if (found == publications.end()) throw std::runtime_error("Missing native publication ownership");
        json::JsonObject completion;
        completion.Insert(L"transport", text(L"dx12-composition-v1"));
        completion.Insert(L"fence", text(to_hstring(reinterpret_cast<uintptr_t>(fence))));
        completion.Insert(L"value", numeric(double(value)));
        auto encoded = to_string(completion.Stringify());
        for (auto image : found->second) cartridge.result(cartridge.pax_desktop_release(engine, image,
            reinterpret_cast<uint8_t const*>(encoded.data()), encoded.size()));
        publications.erase(found);
    }
    struct SurfacePlan {
        json::JsonArray registry;
        std::map<uint32_t, std::optional<uint32_t>> owners;
    };
    void enqueue_messages(json::JsonArray const& messages) {
        // Preserve creation/update/deletion ordering during GPU backpressure.
        // A permanently stalled consumer cannot grow an unbounded UI queue.
        constexpr size_t max_bytes = 8 * 1024 * 1024;
        for (auto const& message : messages) {
            size_t bytes = message.Stringify().size() * sizeof(wchar_t);
            if (pending_messages.Size() == 65536 || bytes > max_bytes - pending_message_bytes)
                throw std::runtime_error("Native patch backlog exceeded its bounded capacity while waiting for GPU publication");
            pending_messages.Append(message); pending_message_bytes += bytes;
        }
    }
    SurfacePlan plan(uint32_t count, float scale) {
        SurfacePlan result;
        uint64_t active_bytes = 0, active_images = 0;
        // Registry planning must not move or detach visible native/GPU peers.
        // A blocked runtime keeps the previously committed scene intact.
        for (uint32_t layer = 0; layer < count; ++layer) {
            auto response = cartridge.result(cartridge.pax_desktop_plan(engine, layer, scale));
            if (!present(response, L"plan")) continue;
            auto plan = response.GetNamedObject(L"plan");
            if (!pax::windows::boolean(plan, L"active")) continue;
            std::optional<uint32_t> owner;
            if (present(response, L"owner")) owner = uint32_t(number(response, L"owner"));
            result.owners.emplace(layer, owner);
            for (auto const& value : plan.GetNamedArray(L"surfaces")) {
                auto description = value.GetObject();
                double logical_width = number(description, L"width"), logical_height = number(description, L"height");
                if (logical_width <= 0 || logical_height <= 0) continue;
                double pixels_x = std::ceil(logical_width * scale), pixels_y = std::ceil(logical_height * scale);
                if (!std::isfinite(pixels_x * pixels_y) || pixels_x > 16384 || pixels_y > 16384)
                    throw std::invalid_argument("Windows composition tile exceeds its dimension limit");
                active_bytes += uint64_t(pixels_x) * uint64_t(pixels_y) * 4;
                if (++active_images > CompositionPresenter::ImageCapacity || active_bytes > CompositionPresenter::MaxFrameBytes)
                    throw std::invalid_argument("Windows active composition tiles exceed their image/byte budget");
                auto width_px = uint32_t(pixels_x), height_px = uint32_t(pixels_y);
                auto signature = description.GetNamedString(L"hostSignature") + L":" + to_hstring(width_px) + L"x" + to_hstring(height_px) + L"@" + to_hstring(scale);
                json::JsonObject item;
                item.Insert(L"layer", numeric(layer)); item.Insert(L"key", text(description.GetNamedString(L"key")));
                item.Insert(L"hostSignature", text(signature));
                item.Insert(L"originX", numeric(number(description, L"left"))); item.Insert(L"originY", numeric(number(description, L"top")));
                item.Insert(L"logicalWidth", numeric(logical_width)); item.Insert(L"logicalHeight", numeric(logical_height));
                item.Insert(L"width", numeric(width_px)); item.Insert(L"height", numeric(height_px));
                json::JsonArray dpr; dpr.Append(numeric(scale)); dpr.Append(numeric(scale)); item.Insert(L"dpr", dpr);
                item.Insert(L"replayPriority", numeric(number(description, L"replayPriority"))); result.registry.Append(item);
            }
        }
        cartridge.json_call(cartridge.pax_desktop_set_surfaces, engine, result.registry);
        return result;
    }
    void commit_surfaces(SurfacePlan const& plan) {
        std::set<std::pair<uint32_t,std::wstring>> desired;
        for (auto const& value : plan.registry) {
            auto info = value.GetObject();
            auto layer = uint32_t(number(info, L"layer"));
            auto parent = scene->layer(layer, plan.owners.at(layer));
            auto key = std::make_pair(layer, std::wstring(info.GetNamedString(L"key")));
            desired.insert(key);
            auto signature = info.GetNamedString(L"hostSignature");
            auto width_px = uint32_t(number(info, L"width")), height_px = uint32_t(number(info, L"height"));
            auto logical_width = number(info, L"logicalWidth"), logical_height = number(info, L"logicalHeight");
            auto& tile = tiles[key];
            if (!tile.view || tile.signature != signature.c_str()) {
                if (tile.view) { uint32_t index = 0; if (tile.parent.Children().IndexOf(tile.view, index)) tile.parent.Children().RemoveAt(index); }
                tile = {}; tile.signature = signature.c_str(); tile.width = width_px; tile.height = height_px;
                tile.view = Grid(); tile.view.IsHitTestVisible(false);
                tile.surface = presenter->create_surface(width_px, height_px);
                auto visual = compositor.CreateSpriteVisual(); visual.Size({float(logical_width), float(logical_height)});
                visual.Brush(compositor.CreateSurfaceBrush(tile.surface));
                xaml::Hosting::ElementCompositionPreview::SetElementChildVisual(tile.view, visual);
            }
            if (tile.parent != parent) {
                if (tile.parent) { uint32_t index = 0; if (tile.parent.Children().IndexOf(tile.view, index)) tile.parent.Children().RemoveAt(index); }
                parent.Children().Append(tile.view); tile.parent = parent;
            }
            tile.view.Width(logical_width); tile.view.Height(logical_height);
            Canvas::SetLeft(tile.view, number(info, L"originX")); Canvas::SetTop(tile.view, number(info, L"originY"));
        }
        for (auto it = tiles.begin(); it != tiles.end();) {
            if (desired.contains(it->first)) { ++it; continue; }
            uint32_t index = 0;
            if (it->second.parent.Children().IndexOf(it->second.view, index)) it->second.parent.Children().RemoveAt(index);
            it = tiles.erase(it);
        }
    }
    void publish(json::JsonObject const& frame) {
        if (frame.GetNamedString(L"transport") != L"dx12-composition-v1") throw std::runtime_error("Unexpected Windows export transport");
        auto images = frame.GetNamedArray(L"images");
        if (!images.Size()) return;
        uint64_t batch = uint64_t(number(images.GetObjectAt(0), L"token"));
        std::vector<uint64_t> tokens;
        std::vector<CompositionPresenter::Copy> copies;
        for (auto const& item : images) {
            auto image = item.GetObject(), info = image.GetNamedObject(L"surface");
            tokens.push_back(uint64_t(number(image, L"token")));
            auto key = std::make_pair(uint32_t(number(info, L"layer")), std::wstring(info.GetNamedString(L"key")));
            auto found = tiles.find(key);
            if (found == tiles.end() || found->second.signature != info.GetNamedString(L"hostSignature").c_str())
                throw std::runtime_error("Runtime published an unregistered Windows tile generation");
            auto& tile = found->second;
            copies.push_back({handle<ID3D12Resource>(image, L"resource"), tile.surface, tile.width, tile.height});
        }
        publications.emplace(batch, std::move(tokens));
        presenter->submit(batch, copies);
        ++frames;
        if (status == 1) { set_status(2); log("first_frame_published source=pax-runtime"); }
        if (frames % 120 == 0) {
            auto const& stats = presenter->statistics();
            log("frame=" + std::to_string(frames) + " pending=" + std::to_string(stats.in_flight) +
                " submitted=" + std::to_string(stats.submitted) + " retired=" + std::to_string(stats.retired) +
                " peak=" + std::to_string(stats.peak_in_flight) + " backpressure=" + std::to_string(blocked_frames) +
                " pending_native_messages=" + std::to_string(pending_messages.Size()) + " pending_native_bytes=" + std::to_string(pending_message_bytes));
            log("gpu=" + to_string(cartridge.result(cartridge.pax_desktop_poll(engine)).GetNamedObject(L"gpu").Stringify()));
        }
    }
    void tick() {
        if (running_tick || status == 4 || failed) return;
        running_tick = true;
        try {
            if (presenter) presenter->poll();
            assets.poll([this](DecodedImage image) {
                if (status < 3) cartridge.result(cartridge.pax_desktop_load_image(engine,
                    reinterpret_cast<uint8_t const*>(image.path.data()), image.path.size(),
                    image.rgba.data(), image.rgba.size(), image.width, image.height));
            });
            if (status == 3) {
                // A terminal render error does not revoke release/close. The
                // shared runtime still reports whether its resources drained.
                auto state = cartridge.response(cartridge.pax_desktop_poll(engine));
                if (assets.idle() && (!presenter || presenter->idle()) && state.GetNamedBoolean(L"drained") &&
                    cartridge.pax_desktop_pump_shutdowns() == 0 && cartridge.pax_desktop_finish_close(engine)) {
                    // finish_close consumes the engine even when this session
                    // was only prepared and never created a GPU or window.
                    engine = nullptr;
                    log("closed_gpu=" + to_string(state.GetNamedValue(L"gpu").Stringify()));
                    presenter.reset();
                    set_status(4); log("closed exports_drained=true");
                    if (timer) timer.Stop();
                    if (window) window.Close();
                    active.reset();
                    if (user_released) sessions.erase(this);
                } else if (std::chrono::steady_clock::now() - closing_started > std::chrono::seconds(15)) {
                    throw std::runtime_error("Windows close could not retire GPU/application work within 15 seconds; resources remain owned");
                }
            } else {
                auto layout = window.Content().as<Grid>();
                float viewport_width = float(layout.ActualWidth()), viewport_height = float(layout.ActualHeight());
                auto xaml_root = layout.XamlRoot();
                // Activate returns before attachment/layout has necessarily
                // established XamlRoot. Start rendering with its real scale.
                if (viewport_width > 0 && viewport_height > 0 && xaml_root) {
                    float scale = float(xaml_root.RasterizationScale());
                    auto update = cartridge.result(cartridge.pax_desktop_tick(engine, viewport_width, viewport_height, scale));
                    enqueue_messages(update.GetNamedArray(L"messages"));
                    // Native input and runtime ticks remain responsive, but
                    // control/mask patches advance with accepted GPU output.
                    if (presenter->can_submit()) {
                        auto surfaces = plan(uint32_t(number(update, L"layerCount", scene->layer_count())), scale);
                        auto rendered = cartridge.result(cartridge.pax_desktop_render(engine));
                        if (pax::windows::boolean(rendered, L"blocked")) ++blocked_frames;
                        else {
                            scene->resize(viewport_width, viewport_height, scale);
                            scene->apply(pending_messages);
                            commit_surfaces(surfaces);
                            if (present(rendered, L"frame")) publish(rendered.GetNamedObject(L"frame"));
                            pending_messages.Clear(); pending_message_bytes = 0;
                        }
                    } else ++blocked_frames;
                }
            }
        } catch (...) {
            failed = true; fail(exception_message());
            if (timer) timer.Stop();
            // Retain the session on terminal GPU/ABI failure. Reclaiming it here
            // could free exports still read by a driver; process teardown owns
            // that exceptional path, and diagnostics never call it a clean exit.
            if (window) window.Title(L"Pax — stopped: " + to_hstring(last_error));
        }
        running_tick = false;
    }
    void launch() {
        window = Window(); window.Title(title);
        auto weak = weak_from_this();
        scene = std::make_unique<NativeScene>(
            [weak](auto const& value) { auto self = weak.lock(); return self ? self->interrupt(value) : false; },
            [weak](auto const& metadata) { auto self = weak.lock(); if (!self) throw std::runtime_error("Font requested after session release"); return self->assets.font(metadata); },
            [weak](auto const& path) { if (auto self = weak.lock()) self->assets.request(path); });
        Grid layout; layout.RequestedTheme(ElementTheme::Light); layout.Children().Append(scene->root());
        window.Content(layout); window.Activate();
        HWND hwnd = nullptr;
        check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));
        auto dpi = GetDpiForWindow(hwnd);
        if (!dpi) winrt::throw_last_error();
        float initial_scale = float(dpi) / 96;
        window.AppWindow().ResizeClient({int32_t(std::ceil(width * initial_scale)), int32_t(std::ceil(height * initial_scale))});
        if (auto overlapped = window.AppWindow().Presenter().try_as<OverlappedPresenter>()) {
            overlapped.IsResizable(resizable);
            if (min_width > 0) overlapped.PreferredMinimumWidth(int32_t(std::ceil(min_width * initial_scale)));
            if (min_height > 0) overlapped.PreferredMinimumHeight(int32_t(std::ceil(min_height * initial_scale)));
        }
        compositor = xaml::Hosting::ElementCompositionPreview::GetElementVisual(scene->root()).Compositor();
        auto gpu = cartridge.result(cartridge.pax_desktop_initialize_gpu(engine)).GetNamedObject(L"gpu");
        log("initialized_gpu=" + to_string(gpu.Stringify()));
        presenter = std::make_unique<CompositionPresenter>();
        presenter->initialize(compositor, handle<ID3D12Device>(gpu, L"device"), handle<ID3D12CommandQueue>(gpu, L"queue"),
            [weak](uint64_t token, ID3D12Fence* fence, uint64_t value) { if (auto self = weak.lock()) self->release_publication(token, fence, value); });
        scene->initialize(presenter->drawing_device());
        window.AppWindow().Closing([weak](auto const&, AppWindowClosingEventArgs const& args) {
            if (auto self = weak.lock()) {
                if (self->failed) {
                    // Returning to user main is permitted; reclaiming unfinished
                    // exports is not. The registry retains this failed session.
                    try { if (self->scene) self->scene->close(); } catch (...) { fail(exception_message()); }
                    log("failed_window_closed resources_retained_until_process_exit=true");
                    Application::Current().Exit();
                } else if (self->status != 4) { args.Cancel(true); self->close(); }
            }
        });
        timer = DispatcherQueue::GetForCurrentThread().CreateTimer(); timer.Interval(std::chrono::milliseconds(16));
        timer.Tick([weak](auto const&, auto const&) { if (auto self = weak.lock()) self->tick(); });
        { std::lock_guard lock(wake_mutex); wake_queue = DispatcherQueue::GetForCurrentThread(); }
        set_status(1); cartridge.pax_desktop_activate(engine); timer.Start();
        log("running native runtime + WinUI; asynchronous DX12 composition exports");
    }
    void close() {
        if (failed) { if (window) window.Close(); return; }
        if (status >= 3) return;
        set_status(3); closing_started = std::chrono::steady_clock::now();
        cartridge.pax_desktop_begin_close(engine);
        if (presenter) presenter->stop();
        if (scene) scene->close();
        tiles.clear(); pending_messages.Clear(); pending_message_bytes = 0;
        if (!timer) {
            // PreparedApp can be dropped before Application.Start creates a
            // dispatcher. Its thread-affine cleanup still needs an STA pump;
            // do not pretend that merely retaining the engine advances it.
            log("prepared_close pump_started no_window=true");
            tick();
            while (status == 3 && !failed) {
                HANDLE signal = preparation_wake.get();
                auto result = MsgWaitForMultipleObjectsEx(1, &signal, 100, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
                if (result == WAIT_FAILED) {
                    failed = true; fail("Prepared cleanup message wait failed; resources remain owned"); break;
                }
                // Limit each drain so continuous input cannot starve the
                // existing cleanup deadline or worker-acknowledgement checks.
                MSG message{};
                for (unsigned count = 0; count < 64 && PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE); ++count) {
                    if (message.message == WM_QUIT) {
                        PostQuitMessage(int(message.wParam));
                        failed = true; fail("Application quit during prepared cleanup; resources remain owned"); break;
                    }
                    TranslateMessage(&message); DispatchMessageW(&message);
                }
                tick();
            }
            log(std::string("prepared_close pump_finished closed=") + (status == 4 ? "true" : "false"));
        }
    }
};

void wake(uint64_t token) {
    DispatcherQueue queue{nullptr};
    {
        std::lock_guard lock(wake_mutex);
        if (preparation_wake) SetEvent(preparation_wake.get());
        queue = wake_queue;
    }
    if (queue) queue.TryEnqueue([token] { if (active && active->token == token) active->tick(); });
}
std::shared_ptr<Session> session(void* pointer) {
    thread_check();
    auto found = sessions.find(pointer);
    if (found == sessions.end()) throw std::invalid_argument("Invalid PaxHost session handle");
    return found->second;
}
struct App : ApplicationT<App, Markup::IXamlMetadataProvider> {
    XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider() { if (!metadata) metadata = XamlTypeInfo::XamlControlsXamlMetaDataProvider(); return metadata; }
    Markup::IXamlType GetXamlType(hstring const& name) { return provider().GetXamlType(name); }
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& type) { return provider().GetXamlType(type); }
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions() { return provider().GetXmlnsDefinitions(); }
    void OnLaunched(LaunchActivatedEventArgs const&) {
        try { Resources().MergedDictionaries().Append(XamlControlsResources()); active->launch(); }
        catch (...) { fail(exception_message()); if (active) active->failed = true; Exit(); }
    }
};
} // namespace

#define PAX_HOST extern "C" __declspec(dllexport)
PAX_HOST uint32_t pax_host_version() { return 2; }
PAX_HOST int32_t pax_host_configure(uint8_t const* data, size_t length) {
    try { thread_check(); initialize_runtime(); if (active) throw std::runtime_error("Close the active Pax session before configuring another");
        configuration = json::JsonObject::Parse(to_hstring(bytes(data, length, 1024 * 1024))); return 0;
    } catch (...) { return fail(exception_message()); }
}
PAX_HOST void* pax_host_prepare_with_data(uint8_t const* data, size_t length) {
    try { thread_check(); if (active || !configuration) throw std::runtime_error("Pax requires configuration and one live native session");
        auto value = std::make_shared<Session>(); value->prepare(bytes(data, length, 65536));
        sessions.emplace(value.get(), value); active = value; return value.get();
    } catch (...) { fail(exception_message()); return nullptr; }
}
PAX_HOST void* pax_host_prepare() { return pax_host_prepare_with_data(nullptr, 0); }
PAX_HOST int32_t pax_host_window_options(void* pointer, uint8_t const* title, size_t length,
    double width, double height, double min_width, double min_height, bool resizable) {
    try { auto value = session(pointer); if (value->status != 0) throw std::runtime_error("Configure a fresh native session");
        if (!(width > 0 && height > 0 && min_width >= 0 && min_height >= 0) || !std::isfinite(width + height + min_width + min_height)) throw std::invalid_argument("Invalid native window dimensions");
        // The v2 facade uses an empty span for WindowOptions::title == None.
        if (length) value->title = to_hstring(bytes(title, length, 65536));
        value->width = width; value->height = height;
        value->min_width = min_width; value->min_height = min_height; value->resizable = resizable; return 0;
    } catch (...) { return fail(exception_message()); }
}
PAX_HOST int32_t pax_host_run(void* pointer) {
    try { auto value = session(pointer); if (value->status != 0 || value != active) throw std::runtime_error("Run consumes one prepared native session");
        if (application_started) throw std::runtime_error("WinUI standalone Application.Start is supported once per process");
        application_started = true;
        Application::Start([](auto const&) { make<App>(); });
        return value->failed || value->status != 4 ? fail(last_error.empty() ? "Native host stopped without clean shutdown" : last_error) : 0;
    } catch (...) { return fail(exception_message()); }
}
PAX_HOST int32_t pax_host_mount(void*, void*, void*) { return fail("Embedding a Windows native view is not implemented; use run/run_with"); }
PAX_HOST void pax_host_observe(void* pointer, void* context, void (*notify)(void*, uint32_t), void (*release)(void*)) {
    try { auto value = session(pointer); value->observer = std::make_shared<Observer>(Observer{context, notify, nullptr});
        value->observer->release = release; if (notify) notify(context, value->status);
    } catch (...) { fail(exception_message()); if (release) release(context); }
}
PAX_HOST void pax_host_close(void* pointer) { try { session(pointer)->close(); } catch (...) { fail(exception_message()); } }
PAX_HOST void pax_host_release(void* pointer) {
    try { auto value = session(pointer); value->user_released = true; value->close(); if (value->status == 4) sessions.erase(pointer); }
    catch (...) { fail(exception_message()); }
}
PAX_HOST size_t pax_host_error(uint8_t* buffer, size_t capacity) {
    size_t size = last_error.size(); if (buffer && capacity) std::memcpy(buffer, last_error.data(), std::min(size, capacity)); return size;
}
PAX_HOST void pax_host_request(void* pointer, uint8_t const* data, size_t length, void* context, PaxDesktopReply reply) {
    try { auto value = session(pointer); auto input = bytes(data, length, 65536);
        auto queue = DispatcherQueue::GetForCurrentThread();
        if (!queue || !queue.TryEnqueue([value, input = std::move(input), context, reply] {
            if (value->status >= 3) { if (reply) reply(context, 1, nullptr, 0); return; }
            value->cartridge.pax_desktop_request(value->engine, reinterpret_cast<uint8_t const*>(input.data()), input.size(), context, reply);
        })) throw std::runtime_error("Native request requires a running dispatcher");
    } catch (...) { fail(exception_message()); if (reply) reply(context, 1, nullptr, 0); }
}
