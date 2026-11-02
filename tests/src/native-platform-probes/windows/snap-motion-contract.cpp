#include <windows.h>
#include <unknwn.h>
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
#include <limits>
#include <optional>
#include "json_helpers.h"
#include "snap_canvas.h"

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace pax::windows;

namespace {
using Alignment = Primitives::SnapPointsAlignment;
std::ofstream output;
int process_result = 1;
void record(char const* stage, bool passed, std::string const& detail) {
    json::JsonObject row; row.Insert(L"stage", text(to_hstring(stage)));
    row.Insert(L"passed", json::JsonValue::CreateBooleanValue(passed)); row.Insert(L"detail", text(to_hstring(detail)));
    output << to_string(row.Stringify()) << '\n'; output.flush();
}
struct App : ApplicationT<App, Markup::IXamlMetadataProvider> {
    XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};
    Window window{nullptr}; Grid root{nullptr}; ScrollViewer horizontal{nullptr}, vertical{nullptr};
    com_ptr<SnapCanvas> x, y;
    winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer timer{nullptr};
    unsigned phase = 0; bool passed = true; unsigned horizontal_changes=0,vertical_changes=0; bool horizontal_intermediate=false,vertical_intermediate=false;
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider() { if (!metadata) metadata = XamlTypeInfo::XamlControlsXamlMetaDataProvider(); return metadata; }
    Markup::IXamlType GetXamlType(hstring const& name) { return provider().GetXamlType(name); }
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& type) { return provider().GetXamlType(type); }
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions() { return provider().GetXmlnsDefinitions(); }
    void snapshot(char const* stage, std::optional<double> expected_x = {}, std::optional<double> expected_y = {}) {
        json::JsonObject row; row.Insert(L"stage", text(to_hstring(stage)));
        bool ok = (!expected_x || std::abs(horizontal.HorizontalOffset()-*expected_x)<0.1) &&
            (!expected_y || std::abs(vertical.VerticalOffset()-*expected_y)<0.1); passed &= ok;
        row.Insert(L"passed", json::JsonValue::CreateBooleanValue(ok));
        auto describe=[](ScrollViewer const& s, com_ptr<SnapCanvas> const& impl) {
            json::JsonObject v; v.Insert(L"x", numeric(s.HorizontalOffset())); v.Insert(L"y", numeric(s.VerticalOffset()));
            v.Insert(L"viewportWidth",numeric(s.ViewportWidth()));v.Insert(L"viewportHeight",numeric(s.ViewportHeight()));
            v.Insert(L"extentWidth",numeric(s.ExtentWidth()));v.Insert(L"extentHeight",numeric(s.ExtentHeight()));
            v.Insert(L"horizontalMode",numeric(int(s.HorizontalScrollMode())));v.Insert(L"verticalMode",numeric(int(s.VerticalScrollMode())));
            v.Insert(L"horizontalSnapPolicy",numeric(int(s.HorizontalSnapPointsType())));v.Insert(L"verticalSnapPolicy",numeric(int(s.VerticalSnapPointsType())));
            json::JsonArray px,py;for(float p:impl->GetIrregularSnapPoints(Orientation::Horizontal,Alignment::Near))px.Append(numeric(p));
            for(float p:impl->GetIrregularSnapPoints(Orientation::Vertical,Alignment::Near))py.Append(numeric(p));v.Insert(L"pointsX",px);v.Insert(L"pointsY",py);return v;
        };
        row.Insert(L"horizontal",describe(horizontal,x));row.Insert(L"vertical",describe(vertical,y));
        row.Insert(L"horizontalViewChanges",numeric(horizontal_changes));row.Insert(L"verticalViewChanges",numeric(vertical_changes));
        row.Insert(L"horizontalIntermediate",json::JsonValue::CreateBooleanValue(horizontal_intermediate));row.Insert(L"verticalIntermediate",json::JsonValue::CreateBooleanValue(vertical_intermediate));
        output<<to_string(row.Stringify())<<'\n';output.flush();
    }
    void move(double px,double py) {
        auto before_x=horizontal_changes,before_y=vertical_changes;
        bool hx=horizontal.ChangeView(px,0.0,nullptr,true),vy=vertical.ChangeView(0.0,py,nullptr,true);
        record("ChangeView",true,"horizontalAccepted="+std::to_string(hx)+"; verticalAccepted="+std::to_string(vy)+"; requested="+std::to_string(px)+","+std::to_string(py)+
            "; synchronousHorizontalEvents="+std::to_string(horizontal_changes-before_x)+"; synchronousVerticalEvents="+std::to_string(vertical_changes-before_y));
    }
    void points(std::vector<double> px,std::vector<double> py) {
        x->set_horizontal_points(std::move(px));y->set_vertical_points(std::move(py));
        horizontal.HorizontalSnapPointsType(x->has_horizontal_points()?SnapPointsType::Mandatory:SnapPointsType::None);
        horizontal.VerticalSnapPointsType(SnapPointsType::None);
        vertical.VerticalSnapPointsType(y->has_vertical_points()?SnapPointsType::Mandatory:SnapPointsType::None);
        vertical.HorizontalSnapPointsType(SnapPointsType::None);
    }
    void finish(bool success) {if(timer)timer.Stop();if(horizontal)horizontal.Content(nullptr);if(vertical)vertical.Content(nullptr);if(window)window.Close();process_result=success?0:1;record("complete",success,"native snap motion phases completed");Exit();}
    void tick() {
        try {
            switch(phase++) {
            case 0: snapshot("initial",0,0);move(450,240);break;
            case 1: snapshot("page-jump",450,240);points({0,125,325,625},{0,125,325,625});horizontal.UpdateLayout();vertical.UpdateLayout();move(173,173);break;
            case 2: snapshot("pixel-jump",125,125);move(173,173);break;
            case 3: snapshot("pixel-repeated-jump",125,125);points({},{});move(173,173);break;
            case 4: snapshot("cleared-exact",173,173);points({0,450,900},{0,240,480});horizontal.UpdateLayout();vertical.UpdateLayout();move(173,173);break;
            case 5: snapshot("restored-page-jump",0,240);
                horizontal.Width(360);vertical.Height(180);x->Width(1080);y->Height(540);
                x->update_extents(360,240,1080,240);y->update_extents(340,180,340,540);points({0,360,720},{0,180,360});
                horizontal.UpdateLayout();vertical.UpdateLayout();move(173,173);break;
            case 6: snapshot("resized-page-jump",0,180);move(173,173);break;
            case 7: snapshot("resized-repeated-jump",0,180);points({},{});move(173,173);break;
            case 8: snapshot("resized-clear",173,173);points({0,360,720},{0,180,360});move(173,173);break;
            case 9: snapshot("restore-without-forced-layout",0,180);finish(passed);break;
            }
        }catch(hresult_error const&e){record("failure",false,to_string(e.message()));finish(false);}catch(std::exception const&e){record("failure",false,e.what());finish(false);}
    }
    void OnLaunched(LaunchActivatedEventArgs const&) {
        try {
            Resources().MergedDictionaries().Append(XamlControlsResources());root=Grid();root.Width(900);root.Height(500);
            horizontal=ScrollViewer();vertical=ScrollViewer();x=make_self<SnapCanvas>();y=make_self<SnapCanvas>();
            horizontal.Width(450);horizontal.Height(240);vertical.Width(340);vertical.Height(240);
            horizontal.HorizontalAlignment(HorizontalAlignment::Left);vertical.HorizontalAlignment(HorizontalAlignment::Right);
            horizontal.HorizontalScrollMode(ScrollMode::Enabled);horizontal.VerticalScrollMode(ScrollMode::Disabled);
            vertical.HorizontalScrollMode(ScrollMode::Disabled);vertical.VerticalScrollMode(ScrollMode::Enabled);
            for(auto s:{horizontal,vertical}){s.HorizontalScrollBarVisibility(ScrollBarVisibility::Hidden);s.VerticalScrollBarVisibility(ScrollBarVisibility::Hidden);
                s.ZoomMode(ZoomMode::Disabled);s.HorizontalSnapPointsAlignment(Alignment::Near);s.VerticalSnapPointsAlignment(Alignment::Near);
                s.ViewChanged([this](auto const& sender,ScrollViewerViewChangedEventArgs const& args){auto v=sender.template as<ScrollViewer>();
                    if(v==horizontal){++horizontal_changes;horizontal_intermediate=args.IsIntermediate();}else{++vertical_changes;vertical_intermediate=args.IsIntermediate();}
                    record("ViewChanged",true,"x="+std::to_string(v.HorizontalOffset())+"; y="+std::to_string(v.VerticalOffset())+"; intermediate="+std::to_string(args.IsIntermediate()));});}
            x->Width(1350);x->Height(240);y->Width(340);y->Height(720);x->update_extents(450,240,1350,240);y->update_extents(340,240,340,720);
            horizontal.Content(x.as<Canvas>());vertical.Content(y.as<Canvas>());points({0,450,900},{0,240,480});root.Children().Append(horizontal);root.Children().Append(vertical);
            window=Window();window.Title(L"Pax native ScrollViewer snap motion contract");window.Content(root);window.Activate();
            timer=winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();timer.Interval(std::chrono::milliseconds(700));timer.Tick([this](auto const&,auto const&){tick();});timer.Start();
        }catch(hresult_error const&e){record("startup",false,to_string(e.message()));finish(false);}catch(std::exception const&e){record("startup",false,e.what());finish(false);}
    }
};
}
int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int) {
    wchar_t log_path[32768];
    auto length = GetEnvironmentVariableW(L"PAX_SNAP_MOTION_LOG", log_path, DWORD(std::size(log_path)));
    output.open(length && length < std::size(log_path) ? log_path : L"snap-motion-contract.jsonl");
    if (!output) return 2;
    try {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        PACKAGE_VERSION minimum{}; minimum.Version = WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;
        check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,
            WINDOWSAPPSDK_RELEASE_VERSION_TAG_W, minimum, MddBootstrapInitializeOptions_None));
        init_apartment(apartment_type::single_threaded);
        Application::Start([](auto const&) { make<App>(); });
        record("application-returned", process_result == 0, "require process exit code zero as well");
        return process_result;
    } catch (hresult_error const& error) { record("fatal", false, to_string(error.message())); }
      catch (std::exception const& error) { record("fatal", false, error.what()); }
    return 1;
}
