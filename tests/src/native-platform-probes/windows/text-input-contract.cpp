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
    std::vector<hstring> edits;
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
        throw std::runtime_error("text input contract native node missing");
    }
    void check(bool ok,char const* stage){require_check(ok,stage);record(stage,true,"actual XAML object/geometry");}
    void tick() {
        try {
            auto host=find(scene->root(),1);
            auto editor=host.Children().GetAt(0).as<TextBox>();
            if(phase++ == 0) {
                check(edits.empty(),"initial-application-patch-emits-no-input");
                // This contract checks callback ordering. The packaged Rust
                // probe separately supplies real keyboard input to this control.
                editor.Text(L"first native edit");
                check(edits.size()==1&&edits.back()==L"first native edit",
                    "native-edit-reaches-runtime-before-next-render-tick");
                editor.Text(L"second native edit");
                check(edits.size()==2&&edits.back()==L"second native edit",
                    "consecutive-native-edits-publish-in-order");
                editor.Select(2,5);
                update(LR"([{"TextboxUpdate":{"id":1,"text":"second native edit"}}])");
                check(edits.size()==2&&editor.SelectionStart()==2&&editor.SelectionLength()==5,
                    "identical-engine-echo-retains-selection-without-input");
                update(LR"([{"TextboxUpdate":{"id":1,"text":"formatted by application"}}])");
                check(editor.Text()==L"formatted by application"&&edits.size()==2,
                    "application-replacement-updates-control-without-input");
                return;
            }
            check(edits.size()==2,"application-replacement-has-no-delayed-input-echo");
            auto retained=editor;
            update(LR"([{"TextboxDelete":1}])");
            retained.Text(L"retired edit");
            check(edits.size()==2,"retired-control-cannot-publish-input");
            finish(true);
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
            scene = std::make_unique<NativeScene>([this](auto const& value) {
                if(value.HasKey(L"FormTextboxInput"))
                    edits.push_back(value.GetNamedObject(L"FormTextboxInput").GetNamedString(L"text"));
                return false;
            },
                [](auto const&) -> hstring { return hstring(L"Segoe UI"); },
                [](auto const&) { throw std::runtime_error("No images in text input contract"); });
            window = Window(); window.Title(L"Pax synchronous text input contract");
            window.Content(scene->root()); window.Activate();
            scene->initialize(device.get()); scene->resize(400, 400, 1);
            update(LR"([{"TextboxCreate":{"id":1,"render_layer_id":0}},{"TextboxUpdate":{"id":1,"size_x":300,"size_y":40,"text":"initial application value","transform":[1,0,0,1,30,30]}}])");
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
    auto length = GetEnvironmentVariableW(L"PAX_TEXT_INPUT_CONTRACT_LOG", log_path, DWORD(std::size(log_path)));
    output.open(length && length < std::size(log_path) ? log_path : L"text-input-contract.jsonl");
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
