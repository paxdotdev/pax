#include <windows.h>
#include <unknwn.h>
#include <d3d11.h>
#include <microsoft.ui.xaml.window.h>
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
#include "native_scene.h"
using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace pax::windows;
namespace {
std::ofstream output;
int process_result=1;
void check(bool ok, char const* stage) {
    json::JsonObject row; row.Insert(L"stage",text(to_hstring(stage)));row.Insert(L"passed",json::JsonValue::CreateBooleanValue(ok));
    output<<to_string(row.Stringify())<<'\n';output.flush();
    if(!ok)throw std::runtime_error(stage);
}
struct App:ApplicationT<App,Markup::IXamlMetadataProvider> {
    XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};Window window{nullptr};
    com_ptr<ID3D11Device> device;std::unique_ptr<NativeScene> scene;ScrollViewer outer{nullptr},inner{nullptr},retired{nullptr};
    winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer timer{nullptr};
    unsigned phase=0,outer_acks=0,inner_acks=0,previous_acks=0;double last_x=0,last_y=0,last_inner_y=0;
    bool separate_turn_write_sent=false,latest_write_observing=false,stale_write_ack=false;
    unsigned previous_inner_acks=0,previous_outer_acks=0;
    unsigned separate_turn_acks_before=0;
    double wheel_before_x=0,wheel_before_y=0,wheel_outer_y=0,separate_turn_target=275;
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider(){if(!metadata)metadata=XamlTypeInfo::XamlControlsXamlMetaDataProvider();return metadata;}
    Markup::IXamlType GetXamlType(hstring const& n){return provider().GetXamlType(n);}
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& t){return provider().GetXamlType(t);}
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions(){return provider().GetXmlnsDefinitions();}
    bool interrupt(json::JsonObject const& message) {
        if(message.HasKey(L"ScrollerPosition")) {
            auto value=message.GetNamedObject(L"ScrollerPosition");auto id=uint32_t(number(value,L"id"));
            if(id==1){++outer_acks;last_y=number(value,L"scroll_y");}else if(id==2){++inner_acks;last_x=number(value,L"scroll_x");last_inner_y=number(value,L"scroll_y");
                if(latest_write_observing&&close_enough(last_x,800))stale_write_ack=true;}
            output<<to_string(message.Stringify())<<'\n';output.flush();
        }
        return false;
    }
    void patch(wchar_t const* json_text){scene->apply(json::JsonArray::Parse(json_text));scene->root().UpdateLayout();}
    ScrollViewer find(Canvas const& canvas,uint32_t id) {
        for(auto child:canvas.Children())if(auto host=child.try_as<Grid>())for(auto element:host.Children())
            if(auto scroll=element.try_as<ScrollViewer>())if(Automation::AutomationProperties::GetAutomationId(scroll)==L"pax-node-"+to_hstring(id))return scroll;
        throw std::runtime_error("native ScrollViewer missing");
    }
    void wheel(bool horizontal) {
        HWND hwnd=nullptr;check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));
        check(GetForegroundWindow()==hwnd,"foreground-owned-for-native-wheel");
        auto p=inner.TransformToVisual(nullptr).TransformPoint({100,100});POINT origin{};check(ClientToScreen(hwnd,&origin),"client-origin");
        double dpr=GetDpiForWindow(hwnd)/96.0;POINT point{origin.x+LONG(std::round(p.X*dpr)),origin.y+LONG(std::round(p.Y*dpr))};
        check(GetAncestor(WindowFromPoint(point),GA_ROOT)==hwnd,"wheel-hit-owned-window");
        INPUT input[2]{};input[0].type=input[1].type=INPUT_MOUSE;
        input[0].mi.dwFlags=MOUSEEVENTF_MOVE|MOUSEEVENTF_ABSOLUTE|MOUSEEVENTF_VIRTUALDESK;
        input[0].mi.dx=LONG(std::round((point.x-GetSystemMetrics(SM_XVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CXVIRTUALSCREEN)-1)));
        input[0].mi.dy=LONG(std::round((point.y-GetSystemMetrics(SM_YVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CYVIRTUALSCREEN)-1)));
        input[1].mi.dwFlags=horizontal?MOUSEEVENTF_HWHEEL:MOUSEEVENTF_WHEEL;input[1].mi.mouseData=horizontal?120:DWORD(-120);
        check(SendInput(2,input,sizeof(INPUT))==2,"native-wheel-delivered");
    }
    void separate_turn_writes(double latest=275) {
        separate_turn_target=latest;separate_turn_write_sent=false;stale_write_ack=false;
        latest_write_observing=false;separate_turn_acks_before=inner_acks;
        // Use two real apply calls with a native dispatcher boundary between
        // them, not two patches coalesced by one scene commit. Do not force layout.
        scene->apply(json::JsonArray::Parse(LR"([{"ScrollerUpdate":{"id":2,"scroll_x":800}}])"));
        auto weak=get_weak();
        check(winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().TryEnqueue(
            winrt::Microsoft::UI::Dispatching::DispatcherQueuePriority::High,[weak,latest] {
                auto self=weak.get();if(!self||!self->scene)return;
                try {
                    json::JsonObject row;row.Insert(L"stage",text(L"separate-native-turn-second-write"));
                    row.Insert(L"acksSinceFirstWrite",numeric(self->inner_acks-self->separate_turn_acks_before));
                    row.Insert(L"offsetBeforeSecondWrite",numeric(self->inner.HorizontalOffset()));
                    row.Insert(L"reportedBeforeSecondWrite",numeric(self->last_x));
                    row.Insert(L"latestAuthoredTarget",numeric(latest));
                    row.Insert(L"latestEqualsCurrentOffset",json::JsonValue::CreateBooleanValue(self->close_enough(latest,self->inner.HorizontalOffset())));
                    // Record whether this run actually overlaps an unacknowledged
                    // first request; a synchronous first completion is also legal.
                    row.Insert(L"firstWriteAcknowledged",json::JsonValue::CreateBooleanValue(
                        self->inner_acks>self->separate_turn_acks_before&&self->close_enough(self->last_x,800)));
                    output<<to_string(row.Stringify())<<'\n';output.flush();
                    self->latest_write_observing=true;
                    json::JsonObject value;value.Insert(L"id",numeric(2));value.Insert(L"scroll_x",numeric(latest));
                    json::JsonObject message;message.Insert(L"ScrollerUpdate",value);json::JsonArray messages;messages.Append(message);
                    self->scene->apply(messages);
                    self->separate_turn_write_sent=true;
                }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';self->finish(false);}
                 catch(std::exception const& e){output<<e.what()<<'\n';self->finish(false);}
            }),"separate-native-turn-write-enqueued");
    }
    void vertical_wheel_probe() {
        wheel_before_x=inner.HorizontalOffset();wheel_before_y=inner.VerticalOffset();wheel_outer_y=outer.VerticalOffset();
        wheel(false);
    }
    void expect_vertical_owner(bool inner_owns,char const* stage) {
        const bool x_unchanged=std::abs(inner.HorizontalOffset()-wheel_before_x)<1e-5;
        const bool inner_unchanged=std::abs(inner.VerticalOffset()-wheel_before_y)<1e-5;
        const bool outer_unchanged=std::abs(outer.VerticalOffset()-wheel_outer_y)<1e-5;
        check(x_unchanged&&(inner_owns ?
            inner.VerticalOffset()>wheel_before_y+1e-5&&outer_unchanged :
            outer.VerticalOffset()>wheel_outer_y+1e-5&&inner_unchanged),stage);
    }
    void snapshot(char const* stage) {
        json::JsonObject row;row.Insert(L"stage",text(to_hstring(stage)));
        auto describe=[](ScrollViewer const& scroll){json::JsonObject v;
            v.Insert(L"x",numeric(scroll.HorizontalOffset()));v.Insert(L"y",numeric(scroll.VerticalOffset()));
            v.Insert(L"width",numeric(scroll.ActualWidth()));v.Insert(L"height",numeric(scroll.ActualHeight()));
            v.Insert(L"viewportWidth",numeric(scroll.ViewportWidth()));v.Insert(L"viewportHeight",numeric(scroll.ViewportHeight()));
            v.Insert(L"extentWidth",numeric(scroll.ExtentWidth()));v.Insert(L"extentHeight",numeric(scroll.ExtentHeight()));
            v.Insert(L"scrollableWidth",numeric(scroll.ScrollableWidth()));v.Insert(L"scrollableHeight",numeric(scroll.ScrollableHeight()));
            v.Insert(L"horizontalMode",numeric(int(scroll.HorizontalScrollMode())));v.Insert(L"verticalMode",numeric(int(scroll.VerticalScrollMode())));v.Insert(L"loaded",json::JsonValue::CreateBooleanValue(scroll.IsLoaded()));return v;};
        row.Insert(L"outer",describe(outer));row.Insert(L"inner",describe(inner));output<<to_string(row.Stringify())<<'\n';output.flush();
    }
    bool close_enough(double a,double b){return std::abs(a-b)<0.2;}
    void finish(bool ok){if(timer)timer.Stop();if(window){window.Content(nullptr);window.Close();}if(scene)scene->close();scene.reset();process_result=ok?0:1;Exit();}
    void tick(){try{switch(phase++){
        case 0:
            outer=find(scene->root(),1);inner=find(outer.Content().as<Canvas>(),2);
            // Native axis policy may remain Enabled to avoid wheel axis translation;
            // the authored-disabled axes must nevertheless have no scrollable extent.
            check(std::abs(inner.ScrollableHeight())<1e-5&&std::abs(outer.ScrollableWidth())<1e-5,"authored-disabled-axes-have-zero-native-extent");
            snapshot("before-vertical-wheel");wheel(false);break;
        case 1:
            snapshot("after-vertical-wheel");            check(outer.VerticalOffset()>0&&close_enough(inner.HorizontalOffset(),0),"vertical-wheel-bubbles-across-horizontal-native-child");
            last_y=outer.VerticalOffset();wheel(true);break;
        case 2:
            check(inner.HorizontalOffset()>0&&close_enough(outer.VerticalOffset(),last_y),"horizontal-wheel-stays-in-horizontal-native-child");
            patch(LR"([{"ScrollerUpdate":{"id":1,"scroll_y":173}},{"ScrollerUpdate":{"id":2,"scroll_x":173}}])");break;
        case 3:
            check(close_enough(inner.HorizontalOffset(),173)&&close_enough(outer.VerticalOffset(),173),"authored-scroll-without-snapping");
            patch(LR"([{"ScrollerUpdate":{"id":1,"snap_points_y":[0,100,300,1520],"scroll_y":173}},{"ScrollerUpdate":{"id":2,"snap_points_x":[0,100,300,1160],"scroll_x":173}}])");break;
        case 4:
            check(close_enough(inner.HorizontalOffset(),100)&&close_enough(outer.VerticalOffset(),100),"native-mandatory-snap-targets");
            previous_acks=inner_acks+outer_acks;previous_inner_acks=inner_acks;previous_outer_acks=outer_acks;
            patch(LR"([{"ScrollerUpdate":{"id":1,"scroll_y":173}},{"ScrollerUpdate":{"id":2,"scroll_x":173}}])");break;
        case 5:
            check(close_enough(inner.HorizontalOffset(),100)&&close_enough(outer.VerticalOffset(),100)&&inner_acks+outer_acks>=previous_acks+2&&inner_acks>previous_inner_acks&&outer_acks>previous_outer_acks,"repeated-off-anchor-write-acknowledged-without-movement");
            patch(LR"([{"ScrollerUpdate":{"id":1,"snap_points_y":[],"scroll_y":173}},{"ScrollerUpdate":{"id":2,"snap_points_x":[],"scroll_x":173}}])");break;
        case 6:
            check(close_enough(inner.HorizontalOffset(),173)&&close_enough(outer.VerticalOffset(),173)&&inner.HorizontalSnapPointsType()==SnapPointsType::None&&outer.VerticalSnapPointsType()==SnapPointsType::None&&
                inner.Content().as<Primitives::IScrollSnapPointsInfo>().GetIrregularSnapPoints(Orientation::Horizontal,Primitives::SnapPointsAlignment::Near).Size()==0&&
                outer.Content().as<Primitives::IScrollSnapPointsInfo>().GetIrregularSnapPoints(Orientation::Vertical,Primitives::SnapPointsAlignment::Near).Size()==0,"clear-snapping-restores-exact-authored-offset");
            patch(LR"([{"ScrollerUpdate":{"id":2,"scroll_x":800}},{"ScrollerUpdate":{"id":2,"scroll_x":250}}])");break;
        case 7:
            check(close_enough(inner.HorizontalOffset(),250)&&close_enough(last_x,250),"superseded-scroll-completes-at-latest-authored-value");
            patch(LR"([{"ScrollerUpdate":{"id":2,"size_inner_pane_x":-28,"scroll_x":0}}])");break;
        case 8:
            check(inner.Content().as<Canvas>().Width()==0&&close_enough(inner.HorizontalOffset(),0),"collapsed-inner-extent-is-empty-native-content");
            patch(LR"([{"ScrollerUpdate":{"id":2,"size_inner_pane_x":1800,"snap_points_x":[0,100,300,1160],"scroll_x":173}}])");break;
        case 9:
            check(close_enough(inner.HorizontalOffset(),100)&&inner.HorizontalSnapPointsType()==SnapPointsType::Mandatory&&
                inner.Content().as<Primitives::IScrollSnapPointsInfo>().GetIrregularSnapPoints(Orientation::Horizontal,Primitives::SnapPointsAlignment::Near).Size()==4,"regrown-extent-restores-native-snapping");
            retired=inner;patch(LR"([{"ScrollerUpdate":{"id":2,"scroll_x":1000}},{"ScrollerDelete":2}])");previous_acks=inner_acks;
            retired.ChangeView(300.0,0.0,nullptr,true);break;
        case 10:
            check(inner_acks==previous_acks,"retired-control-callbacks-do-not-touch-scene");
            patch(LR"([{"ScrollerCreate":{"id":2,"parent_frame":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":2,"size_x":640,"size_y":300,"size_inner_pane_x":1800,"size_inner_pane_y":300,"scroll_enabled_x":true,"scroll_enabled_y":false,"transform":[1,0,0,1,0,50]}}])");inner=find(outer.Content().as<Canvas>(),2);break;
        case 11:
            check(inner!=retired&&close_enough(inner.HorizontalOffset(),0),"recreated-node-gets-fresh-native-scroll-state");
            check(scene->focus_canvas(),"canvas-focus-retained-after-native-scrolling");
            patch(LR"([{"ScrollerUpdate":{"id":1,"snap_points_y":[],"scroll_y":0}}])");
            separate_turn_writes();break;
        case 12:
            check(separate_turn_write_sent&&!stale_write_ack&&inner_acks>separate_turn_acks_before&&
                close_enough(inner.HorizontalOffset(),275)&&close_enough(last_x,275),
                "separate-native-turn-latest-write-and-acknowledgement");
            snapshot("separate-native-turn-latest-write");break;
        case 13:
            check(!stale_write_ack&&close_enough(inner.HorizontalOffset(),275)&&close_enough(last_x,275),
                "later-native-turn-does-not-restore-superseded-write");
            latest_write_observing=false;
            // Shared Scroller enables an axis only when pane > viewport+0.5.
            // Positive content and an exact half-pixel overflow are still disabled.
            patch(LR"([{"ScrollerUpdate":{"id":1,"scroll_y":0}},{"ScrollerUpdate":{"id":2,"size_inner_pane_y":300.5,"scroll_enabled_y":false,"scroll_x":0}}])");break;
        case 14:
            snapshot("disabled-half-pixel-axis-before-wheel");vertical_wheel_probe();break;
        case 15:
            expect_vertical_owner(false,"half-pixel-disabled-axis-bubbles-without-inner-motion");
            // Omitted dimensions/policy must survive an unrelated position patch.
            patch(LR"([{"ScrollerUpdate":{"id":1,"scroll_y":0}},{"ScrollerUpdate":{"id":2,"scroll_x":120}}])");break;
        case 16:
            check(close_enough(inner.HorizontalOffset(),120)&&close_enough(last_x,120),"position-only-patch-retains-other-axis-policy");
            vertical_wheel_probe();break;
        case 17:
            expect_vertical_owner(false,"position-only-patch-keeps-disabled-axis-routing");
            // Only the viewport and resulting enable flag change. The original
            // authored pane height is omitted and must remain available.
            patch(LR"([{"ScrollerUpdate":{"id":1,"scroll_y":0}},{"ScrollerUpdate":{"id":2,"size_y":200,"scroll_enabled_y":true,"scroll_y":50}}])");break;
        case 18:
            check(close_enough(inner.VerticalOffset(),50)&&close_enough(last_inner_y,50)&&close_enough(inner.HorizontalOffset(),120),"viewport-only-shrink-restores-retained-content-scrolling");
            snapshot("viewport-only-enabled-axis");vertical_wheel_probe();break;
        case 19:
            expect_vertical_owner(true,"enabled-axis-owns-native-wheel-without-ancestor-motion");
            // The same retained 300.5px content now exceeds the viewport by only
            // 0.25px; no content-size field is supplied in this partial patch.
            patch(LR"([{"ScrollerUpdate":{"id":1,"scroll_y":0}},{"ScrollerUpdate":{"id":2,"size_y":300.25,"scroll_enabled_y":false,"scroll_y":0}}])");break;
        case 20:
            check(std::abs(inner.VerticalOffset())<1e-5&&std::abs(last_inner_y)<1e-5,"viewport-only-disable-acknowledges-zero-offset");
            snapshot("disabled-quarter-pixel-axis-before-wheel");vertical_wheel_probe();break;
        case 21:
            expect_vertical_owner(false,"quarter-pixel-disabled-axis-bubbles-without-inner-motion");
            patch(LR"([{"ScrollerUpdate":{"id":1,"scroll_y":0}},{"ScrollerUpdate":{"id":2,"size_inner_pane_y":900,"scroll_enabled_y":true,"scroll_y":350}}])");break;
        case 22:
            check(close_enough(inner.VerticalOffset(),350)&&close_enough(last_inner_y,350)&&close_enough(inner.HorizontalOffset(),120),"content-only-regrowth-retains-viewport-and-other-axis");
            patch(LR"([{"ScrollerUpdate":{"id":2,"size_inner_pane_y":-1,"scroll_enabled_y":false,"scroll_y":0}}])");break;
        case 23:
            check(std::abs(inner.VerticalOffset())<1e-5&&std::abs(last_inner_y)<1e-5&&close_enough(inner.HorizontalOffset(),120),"content-only-collapse-clears-offset-without-resetting-other-axis");
            vertical_wheel_probe();break;
        case 24:
            expect_vertical_owner(false,"empty-axis-bubbles-after-content-only-collapse");
            patch(LR"([{"ScrollerUpdate":{"id":2,"size_inner_pane_y":900,"scroll_enabled_y":true,"scroll_y":375}}])");break;
        case 25:
            check(close_enough(inner.VerticalOffset(),375)&&close_enough(last_inner_y,375)&&close_enough(inner.HorizontalOffset(),120),"content-only-regrowth-recovers-after-collapse");
            // A new authored value may equal the current native offset while the
            // older accepted request still targets 800. It must still win.
            separate_turn_writes(inner.HorizontalOffset());break;
        case 26:
            check(separate_turn_write_sent&&!stale_write_ack&&inner_acks>separate_turn_acks_before&&
                close_enough(inner.HorizontalOffset(),separate_turn_target)&&close_enough(last_x,separate_turn_target),
                "current-offset-write-supersedes-older-accepted-request");
            snapshot("current-offset-latest-authored-write");break;
        case 27:
            check(!stale_write_ack&&close_enough(inner.HorizontalOffset(),separate_turn_target)&&close_enough(last_x,separate_turn_target),
                "current-offset-latest-write-survives-later-native-turn");
            latest_write_observing=false;check(true,"complete");finish(true);break;
        }}catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}catch(std::exception const& e){output<<e.what()<<'\n';finish(false);}}
    void OnLaunched(LaunchActivatedEventArgs const&){try{
        Resources().MergedDictionaries().Append(XamlControlsResources());
        scene=std::make_unique<NativeScene>([this](auto const& m){return interrupt(m);},[](auto const&){return hstring(L"Segoe UI");},[](auto const&){});
        window=Window();window.Title(L"Pax native scene scroll contract");window.Content(scene->root());window.Activate();
        D3D_FEATURE_LEVEL level;check_hresult(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_HARDWARE,nullptr,D3D11_CREATE_DEVICE_BGRA_SUPPORT,nullptr,0,D3D11_SDK_VERSION,device.put(),&level,nullptr));
        scene->initialize(device.get());scene->resize(700,600,1);
        patch(LR"([{"ScrollerCreate":{"id":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":1,"size_x":640,"size_y":480,"size_inner_pane_x":640,"size_inner_pane_y":2000,"scroll_enabled_x":false,"scroll_enabled_y":true,"transform":[1,0,0,1,0,0]}},{"ScrollerCreate":{"id":2,"parent_frame":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":2,"size_x":640,"size_y":300,"size_inner_pane_x":1800,"size_inner_pane_y":300,"scroll_enabled_x":true,"scroll_enabled_y":false,"transform":[1,0,0,1,0,50]}}])");
        HWND hwnd=nullptr;check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));MoveWindow(hwnd,50,50,800,700,true);SetForegroundWindow(hwnd);
        timer=winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();timer.Interval(std::chrono::milliseconds(900));timer.Tick([this](auto const&,auto const&){tick();});timer.Start();
    }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}catch(std::exception const& e){output<<e.what()<<'\n';finish(false);}}
};
}
int WINAPI wWinMain(HINSTANCE,HINSTANCE,PWSTR,int){wchar_t path[32768];auto n=GetEnvironmentVariableW(L"PAX_SCENE_SCROLL_LOG",path,DWORD(std::size(path)));output.open(n&&n<std::size(path)?path:L"scene-scroll-contract.jsonl");if(!output)return 2;try{SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);PACKAGE_VERSION v{};v.Version=WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,WINDOWSAPPSDK_RELEASE_VERSION_TAG_W,v,MddBootstrapInitializeOptions_None));init_apartment(apartment_type::single_threaded);Application::Start([](auto const&){make<App>();});return process_result;}catch(hresult_error const& e){output<<to_string(e.message())<<'\n';}return 1;}
