#include <windows.h>
#include <unknwn.h>
#include <d3d11.h>
#include <MddBootstrap.h>
#include <WindowsAppSDK-VersionInfo.h>
#undef GetCurrentTime
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.UI.Xaml.Interop.h>
#include <winrt/Microsoft.UI.Dispatching.h>
#include <winrt/Microsoft.UI.Xaml.Markup.h>
#include <winrt/Microsoft.UI.Xaml.XamlTypeInfo.h>
#include <chrono>
#include <fstream>
#include <memory>
#include "native_scene.h"

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace pax::windows;

namespace {
std::ofstream output;
int result = 1;
void record(char const* stage, bool passed, std::string const& detail) {
    json::JsonObject row;
    row.Insert(L"stage", text(to_hstring(stage)));
    row.Insert(L"passed", json::JsonValue::CreateBooleanValue(passed));
    row.Insert(L"detail", text(to_hstring(detail)));
    output << to_string(row.Stringify()) << '\n'; output.flush();
}
void require_check(bool condition, char const* message) {
    if (!condition) throw std::runtime_error(message);
}
struct App : ApplicationT<App, Markup::IXamlMetadataProvider> {
    XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};
    Window window{nullptr};
    winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer timer{nullptr};
    std::unique_ptr<NativeScene> scene;
    com_ptr<ID3D11Device> device;
    unsigned phase = 0;
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider() {
        if (!metadata) metadata = XamlTypeInfo::XamlControlsXamlMetaDataProvider();
        return metadata;
    }
    Markup::IXamlType GetXamlType(hstring const& name) { return provider().GetXamlType(name); }
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& type) { return provider().GetXamlType(type); }
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions() { return provider().GetXmlnsDefinitions(); }
    void finish(bool passed) {
        if (timer) timer.Stop();
        if (scene) { scene->close(); scene.reset(); }
        if (window) window.Close();
        result = passed ? 0 : 1;
        record("complete", passed, "scene closed; Application exit requested");
        Exit();
    }
    void update(wchar_t const* patch) { scene->apply(json::JsonArray::Parse(patch)); }
    void check_blocker_background() {
        Grid blocker{nullptr};
        for (auto const& child : scene->root().Children()) {
            auto host = child.try_as<Grid>();
            if (!host || !host.Children().Size()) continue;
            auto candidate = host.Children().GetAt(0).try_as<Grid>();
            if (candidate && Automation::AutomationProperties::GetAutomationId(candidate) == L"pax-node-2")
                blocker = candidate;
        }
        require_check(bool(blocker), "actual native EventBlocker Grid not found");
        auto check_color = [&](char const* stage, uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
            auto brush = blocker.Background().try_as<Media::SolidColorBrush>();
            require_check(bool(brush), "EventBlocker has no native solid background");
            auto value = brush.Color();
            require_check(value.R == r && value.G == g && value.B == b && value.A == a,
                          "EventBlocker native RGBA does not match authored background");
            require_check(blocker.IsHitTestVisible(), "background update disabled blocker hit testing");
            record(stage, true, "actual Grid background RGBA and hit-test visibility");
        };
        auto initial_brush = blocker.Background().try_as<Media::SolidColorBrush>();
        require_check(bool(initial_brush) && initial_brush.Color().A == 0 && blocker.IsHitTestVisible(),
                      "default blocker must be transparent and hit-testable");
        record("blocker-default-transparent", true, "transparent native Grid remains hit-testable");
        update(LR"([{"EventBlockerUpdate":{"id":2,"background":{"Rgb":[0.2,0.4,0.6]}}}])");
        check_color("blocker-opaque", 51, 102, 153, 255);
        update(LR"([{"EventBlockerUpdate":{"id":2,"background":{"Rgba":[1,0,0,0.5]}}}])");
        check_color("blocker-translucent", 255, 0, 0, 128);
        update(LR"([{"EventBlockerUpdate":{"id":2,"size_x":25}}])");
        scene->apply(json::JsonArray()); scene->root().UpdateLayout();
        check_color("blocker-omitted-background-retained", 255, 0, 0, 128);
        update(LR"([{"EventBlockerUpdate":{"id":2,"background":{"Rgba":[0,0,0,0]}}}])");
        check_color("blocker-transparent-restored", 0, 0, 0, 0);
    }
    void check(bool clipped) {
        // An empty accepted batch repeats ordinary host placement. This catches
        // a clip attached to the Grid handout being overwritten by XAML layout.
        scene->root().UpdateLayout();
        scene->apply(json::JsonArray());
        scene->root().UpdateLayout();
        Grid host{nullptr}; ScrollViewer scroll{nullptr};
        for (auto const& child : scene->root().Children()) {
            auto grid = child.try_as<Grid>();
            if (grid && grid.Children().Size()) {
                auto candidate = grid.Children().GetAt(0).try_as<ScrollViewer>();
                if (candidate) { host = grid; scroll = candidate; break; }
            }
        }
        require_check(bool(scroll), "actual native ScrollViewer not found");
        auto visual = Hosting::ElementCompositionPreview::GetElementVisual(scroll);
        auto clip = visual.Clip().try_as<composition::CompositionGeometricClip>();
        require_check(bool(clip) == clipped, "control visual clip does not match contract");
        if (clipped) {
            auto geometry = clip.Geometry().as<composition::CompositionRoundedRectangleGeometry>();
            auto size = geometry.Size(); auto radius = geometry.CornerRadius();
            require_check(size.x == 250 && size.y == 250 && radius.x == 48 && radius.y == 48,
                          "rounded geometry has wrong local bounds or radius");
        }
        require_check(!Hosting::ElementCompositionPreview::GetElementChildVisual(host),
                      "rounded-only Scroller allocated a capture sprite");
        require_check(visual.Opacity() == 1, "original native visual is hidden");
        auto diagnostic = scene->composition_diagnostics();
        require_check(diagnostic.find(" alpha_masks=0") != std::string::npos &&
                      diagnostic.find(" mask_pixel_bytes=0") != std::string::npos &&
                      diagnostic.find(" capture_surfaces=0") != std::string::npos,
                      "rounded-only Scroller allocated mask/capture resources");
        record(("phase-" + std::to_string(phase)).c_str(), true, diagnostic);
    }
    void tick() {
        try {
            check(phase != 1 && phase != 3);
            switch (phase++) {
            case 0: update(L"[{\"ScrollerUpdate\":{\"id\":1,\"clip_content\":false}}]"); break;
            case 1: update(L"[{\"ScrollerUpdate\":{\"id\":1,\"clip_content\":true}}]"); break;
            case 2: update(L"[{\"ScrollerUpdate\":{\"id\":1,\"size_x\":0,\"size_y\":0}}]"); break;
            case 3: update(L"[{\"ScrollerUpdate\":{\"id\":1,\"size_x\":250,\"size_y\":250}}]"); break;
            case 4: break; // One additional dispatcher turn with unchanged geometry.
            default: finish(true); break;
            }
        } catch (hresult_error const& error) {
            record("failure", false, to_string(error.message())); finish(false);
        } catch (std::exception const& error) {
            record("failure", false, error.what()); finish(false);
        }
    }
    void OnLaunched(LaunchActivatedEventArgs const&) {
        try {
            Resources().MergedDictionaries().Append(XamlControlsResources());
            D3D_FEATURE_LEVEL level;
            check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0, D3D11_SDK_VERSION,
                device.put(), &level, nullptr));
            scene = std::make_unique<NativeScene>([](auto const&) { return false; },
                [](auto const&) -> hstring { throw std::runtime_error("No fonts in clip contract"); },
                [](auto const&) { throw std::runtime_error("No images in clip contract"); });
            window = Window(); window.Title(L"Pax Scroller clip contract");
            window.Content(scene->root()); window.Activate();
            scene->initialize(device.get()); scene->resize(400, 400, 1);
            update(L"[{\"ScrollerCreate\":{\"id\":1,\"render_layer_id\":0}},"
                   L"{\"ScrollerUpdate\":{\"id\":1,\"size_x\":250,\"size_y\":250,"
                   L"\"size_inner_pane_x\":250,\"size_inner_pane_y\":500,"
                   L"\"corner_radius\":48,\"clip_content\":true}}]");
            update(LR"([{"EventBlockerCreate":{"id":2,"render_layer_id":0}},
                        {"EventBlockerUpdate":{"id":2,"size_x":20,"size_y":20,"transform":[1,0,0,1,300,300]}}])");
            check_blocker_background();
            timer = winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();
            timer.Interval(std::chrono::milliseconds(100));
            timer.Tick([this](auto const&, auto const&) { tick(); }); timer.Start();
        } catch (hresult_error const& error) {
            record("startup", false, to_string(error.message())); finish(false);
        } catch (std::exception const& error) {
            record("startup", false, error.what()); finish(false);
        }
    }
};
}

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int) {
    wchar_t log_path[32768];
    auto length = GetEnvironmentVariableW(L"PAX_CLIP_CONTRACT_LOG", log_path, DWORD(std::size(log_path)));
    output.open(length && length < std::size(log_path) ? log_path : L"scene-clip-contract.jsonl");
    if (!output) return 2;
    try {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        PACKAGE_VERSION minimum{}; minimum.Version = WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;
        check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,
            WINDOWSAPPSDK_RELEASE_VERSION_TAG_W, minimum, MddBootstrapInitializeOptions_None));
        init_apartment(apartment_type::single_threaded);
        Application::Start([](auto const&) { make<App>(); });
        record("application-returned", result == 0, "require process exit code zero as well");
        return result;
    } catch (hresult_error const& error) { record("fatal", false, to_string(error.message())); }
      catch (std::exception const& error) { record("fatal", false, error.what()); }
    return 1;
}
