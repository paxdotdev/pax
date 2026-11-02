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
    void update(wchar_t const* patch) { scene->apply(json::JsonArray::Parse(patch)); scene->root().UpdateLayout(); }
    bool owns_id(DependencyObject const& element,hstring const& id) {
        if(Automation::AutomationProperties::GetAutomationId(element)==id)return true;
        auto count=Media::VisualTreeHelper::GetChildrenCount(element);
        for(int i=0;i<count;++i)if(owns_id(Media::VisualTreeHelper::GetChild(element,i),id))return true;
        return false;
    }
    Grid find(Canvas const& canvas,uint32_t id) {
        for(auto child:canvas.Children())if(auto host=child.try_as<Grid>())
            if(owns_id(host,L"pax-node-"+to_hstring(id)))return host;
        throw std::runtime_error("placement contract native node missing");
    }
    void check(bool ok,char const* stage){require_check(ok,stage);record(stage,true,"actual XAML object/geometry");}
    void tick() {
        try {
            auto parent=find(scene->root(),1);auto child=find(parent.Children().GetAt(0).as<Canvas>(),2);
            auto transform=child.RenderTransform().as<Media::MatrixTransform>();auto clip=child.Clip().as<Media::RectangleGeometry>();
            auto world=[&] {return child.TransformToVisual(scene->root()).TransformPoint({0,0});};
            for(unsigned i=0;i<20;++i)scene->apply(json::JsonArray());
            check(child.RenderTransform()==transform,"empty-accepted-batches-retain-transform-identity");
            check(child.Clip()==clip,"empty-accepted-batches-retain-rectangular-clip-identity");
            update(LR"([{"FrameUpdate":{"id":2,"transform":[1,0,0,1,55,60]}}])");
            auto point=world();check(std::abs(point.X-55)<.01&&std::abs(point.Y-60)<.01,"child-only-transform-updates-world-geometry");
            check(child.RenderTransform()==transform,"child-update-retains-transform-object");
            update(LR"([{"FrameUpdate":{"id":1,"transform":[1,0,0,1,60,40]}}])");
            point=world();check(std::abs(point.X-55)<.01&&std::abs(point.Y-60)<.01,"ancestor-only-transform-recomputes-child-relative-geometry");
            update(LR"([{"FrameUpdate":{"id":1,"transform":[0,1,-1,0,60,40]}}])");
            point=world();check(std::abs(point.X-55)<.01&&std::abs(point.Y-60)<.01,"rotated-ancestor-preserves-child-world-geometry");
            update(LR"([{"FrameUpdate":{"id":2,"parent_frame":3}}])");
            point=world();check(std::abs(point.X-55)<.01&&std::abs(point.Y-60)<.01,"reparenting-recomputes-relative-transform");
            check(child.RenderTransform()==transform,"reparenting-retains-transform-object");
            update(LR"([{"FrameUpdate":{"id":2,"size_x":70,"size_y":50}}])");
            auto rect=clip.Rect();check(child.Width()==70&&child.Height()==50&&rect.Width==70&&rect.Height==50,"resizing-refreshes-retained-clip-bounds");
            update(LR"([{"FrameUpdate":{"id":2,"clip_content":false}}])");
            check(!child.Clip(),"disabled-clip-detaches-geometry");
            update(LR"([{"FrameUpdate":{"id":2,"clip_content":true}}])");
            check(child.Clip()==clip,"reenabled-clip-reuses-retained-geometry");
            update(LR"([{"TextCreate":{"id":4,"render_layer_id":0}},{"TextUpdate":{"id":4,"size_x":-1,"size_y":-1,"content":"Autosize then grow","measure_generation":1,"style":{"font_size":18}}}])");
            auto label=find(scene->root(),4);auto label_transform=label.RenderTransform();auto before=label.Width();
            check(before>0&&label.Height()>0,"autosize-second-placement-updates-native-bounds");
            update(LR"([{"TextUpdate":{"id":4,"measure_generation":2,"style":{"font_size":36}}}])");
            check(label.Width()>before&&label.RenderTransform()==label_transform,"autosize-remeasurement-retains-placement-objects");
            auto retired=child;update(LR"([{"FrameDelete":2},{"FrameCreate":{"id":2,"render_layer_id":0}},{"FrameUpdate":{"id":2,"size_x":30,"size_y":30,"clip_content":true}}])");
            child=find(scene->root(),2);check(child!=retired&&child.RenderTransform()!=transform&&child.Clip()!=clip,"recreated-node-has-independent-placement-objects");
            record("diagnostics",true,scene->composition_diagnostics());finish(true);
        } catch(hresult_error const& error){record("failure",false,to_string(error.message()));finish(false);}
          catch(std::exception const& error){record("failure",false,error.what());finish(false);}
    }
    void OnLaunched(LaunchActivatedEventArgs const&) {
        try {
            Resources().MergedDictionaries().Append(XamlControlsResources());
            D3D_FEATURE_LEVEL level;
            check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0, D3D11_SDK_VERSION,
                device.put(), &level, nullptr));
            scene = std::make_unique<NativeScene>([](auto const&) { return false; },
                [](auto const&) -> hstring { return hstring(L"Segoe UI"); },
                [](auto const&) { throw std::runtime_error("No images in clip contract"); });
            window = Window(); window.Title(L"Pax placement retention contract");
            window.Content(scene->root()); window.Activate();
            scene->initialize(device.get()); scene->resize(400, 400, 1);
            update(LR"([{"FrameCreate":{"id":1,"render_layer_id":0}},{"FrameUpdate":{"id":1,"size_x":200,"size_y":200,"transform":[1,0,0,1,40,30]}},{"FrameCreate":{"id":2,"parent_frame":1,"render_layer_id":0}},{"FrameUpdate":{"id":2,"size_x":60,"size_y":40,"clip_content":true,"transform":[1,0,0,1,50,50]}},{"FrameCreate":{"id":3,"render_layer_id":0}},{"FrameUpdate":{"id":3,"size_x":200,"size_y":200,"transform":[1,0,0,1,100,80]}}])");
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
    auto length = GetEnvironmentVariableW(L"PAX_PLACEMENT_CONTRACT_LOG", log_path, DWORD(std::size(log_path)));
    output.open(length && length < std::size(log_path) ? log_path : L"placement-contract.jsonl");
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
