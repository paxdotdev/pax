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
#include <winrt/Windows.Graphics.h>
#include <windows.graphics.interop.h>
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

    unsigned clicked_id=0;
    unsigned phase=0,outer_acks=0,inner_acks=0,previous_acks=0;double last_x=0,last_y=0,before_inner=0,before_outer=0,before_inner_cross=0;
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider(){if(!metadata)metadata=XamlTypeInfo::XamlControlsXamlMetaDataProvider();return metadata;}
    Markup::IXamlType GetXamlType(hstring const& n){return provider().GetXamlType(n);}
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& t){return provider().GetXamlType(t);}
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions(){return provider().GetXmlnsDefinitions();}
    bool interrupt(json::JsonObject const& message) {
        if(message.HasKey(L"FormButtonClick")) clicked_id=uint32_t(number(message.GetNamedObject(L"FormButtonClick"),L"id"));
        if(message.HasKey(L"ScrollerPosition")) {
            auto value=message.GetNamedObject(L"ScrollerPosition");auto id=uint32_t(number(value,L"id"));
            if(id==1){++outer_acks;last_y=number(value,L"scroll_y");}else if(id==2){++inner_acks;last_x=number(value,L"scroll_x");}
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
    void wheel(bool horizontal, float x=100, float y=100, bool click=false) {
        HWND hwnd=nullptr;check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));
        check(GetForegroundWindow()==hwnd,"foreground-owned-for-native-wheel");
        auto p=inner.TransformToVisual(nullptr).TransformPoint({x,y});POINT origin{};check(ClientToScreen(hwnd,&origin),"client-origin");
        double dpr=GetDpiForWindow(hwnd)/96.0;POINT point{origin.x+LONG(std::round(p.X*dpr)),origin.y+LONG(std::round(p.Y*dpr))};
        check(GetAncestor(WindowFromPoint(point),GA_ROOT)==hwnd,"wheel-hit-owned-window");
        INPUT input[3]{};input[0].type=input[1].type=input[2].type=INPUT_MOUSE;
        input[0].mi.dwFlags=MOUSEEVENTF_MOVE|MOUSEEVENTF_ABSOLUTE|MOUSEEVENTF_VIRTUALDESK;
        input[0].mi.dx=LONG(std::round((point.x-GetSystemMetrics(SM_XVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CXVIRTUALSCREEN)-1)));
        input[0].mi.dy=LONG(std::round((point.y-GetSystemMetrics(SM_YVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CYVIRTUALSCREEN)-1)));
        input[1].mi.dwFlags=horizontal?MOUSEEVENTF_HWHEEL:MOUSEEVENTF_WHEEL;input[1].mi.mouseData=horizontal?120:DWORD(-120);
        if(click){input[1].mi.dwFlags=MOUSEEVENTF_LEFTDOWN;input[1].mi.mouseData=0;input[2].mi.dwFlags=MOUSEEVENTF_LEFTUP;}
        auto count=click?3u:2u;check(SendInput(count,input,sizeof(INPUT))==count,"native-input-delivered");
    }
    void snapshot(char const* stage) {
        json::JsonObject row;row.Insert(L"stage",text(to_hstring(stage)));
        auto describe=[](ScrollViewer const& scroll){json::JsonObject v;
            v.Insert(L"x",numeric(scroll.HorizontalOffset()));v.Insert(L"y",numeric(scroll.VerticalOffset()));
            v.Insert(L"width",numeric(scroll.ActualWidth()));v.Insert(L"height",numeric(scroll.ActualHeight()));
            v.Insert(L"viewportWidth",numeric(scroll.ViewportWidth()));v.Insert(L"viewportHeight",numeric(scroll.ViewportHeight()));
            v.Insert(L"extentWidth",numeric(scroll.ExtentWidth()));v.Insert(L"extentHeight",numeric(scroll.ExtentHeight()));
            v.Insert(L"loaded",json::JsonValue::CreateBooleanValue(scroll.IsLoaded()));return v;};
        row.Insert(L"outer",describe(outer));row.Insert(L"inner",describe(inner));output<<to_string(row.Stringify())<<'\n';output.flush();
    }
    bool owns_id(DependencyObject const& element,hstring const& id) {
        if(Automation::AutomationProperties::GetAutomationId(element)==id)return true;
        auto count=Media::VisualTreeHelper::GetChildrenCount(element);
        for(int i=0;i<count;++i)if(owns_id(Media::VisualTreeHelper::GetChild(element,i),id))return true;
        return false;
    }
    Grid find_host(Canvas const& canvas,uint32_t id) {
        for(auto child:canvas.Children())if(auto host=child.try_as<Grid>())
            if(owns_id(host,L"pax-node-"+to_hstring(id)))return host;
        throw std::runtime_error("native node missing");
    }
    bool close_enough(double a,double b){return std::abs(a-b)<0.2;}
    void finish(bool ok){if(timer)timer.Stop();if(window){window.Content(nullptr);window.Close();}if(scene)scene->close();scene.reset();process_result=ok?0:1;Exit();}

    struct Sample { char const* name; wchar_t const* mask; bool rounded; wchar_t const* clip; float x,y; bool inner_expected; bool click=false; unsigned button=0; bool clear=false; float alpha=1; bool resize=false; bool transition=false; bool transformed=false; bool collapse=false; bool horizontal=false; bool horizontal_only=false; bool retain_scene=false; };
    static constexpr wchar_t const* hole = LR"(M200 0 L300 0 L300 300 L200 300 Z)";
    static constexpr wchar_t const* two = LR"(M50 0 L100 0 L100 300 L50 300 Z M500 0 L550 0 L550 300 L500 300 Z)";
    Sample samples[39] = {
        {"plain-interior",L"",false,L"",100,100,true},
        {"rounded-interior",L"",true,L"",100,100,true},
        {"rounded-outside",L"",true,L"",2,2,false},
        {"rounded-near-curve-inside",L"",true,L"",16,16,true},
        {"rounded-near-curve-outside",L"",true,L"",14,14,false},
        {"rounded-right-curve-inside",L"",true,L"",624,16,true},
        {"rounded-right-curve-outside",L"",true,L"",626,14,false},
        {"opaque-nonhole",hole,false,L"",100,100,true},
        {"opaque-hole",hole,false,L"",250,100,false},
        {"disconnected-gap",two,false,L"",300,100,true},
        {"disconnected-hole",two,false,L"",75,100,false},
        {"ancestor-fully-visible",L"",false,L"M0 50 L640 50 L640 350 L0 350 Z",100,100,true},
        {"ancestor-partial-interior",L"",false,L"M0 50 L300 50 L300 350 L0 350 Z",100,100,true},
        {"ancestor-partial-outside",L"",false,L"M0 50 L300 50 L300 350 L0 350 Z",400,100,false},
        {"ancestor-empty-region",L"",false,L"M1000 50 L1100 50 L1100 350 L1000 350 Z",100,100,false},
        {"mask-and-rounded",hole,true,L"",100,100,true},
        {"mask-cleared",hole,false,L"",250,100,true,false,0,true},
        {"button-unmasked",L"",false,L"",250,140,false,true,4},
        {"button-opaque-hole",hole,false,L"",250,140,false,true,0},
        {"button-nonhole",hole,false,L"",440,140,false,true,3},
        {"button-rounded-inside",L"",true,L"",20,20,false,true,5},
        {"button-rounded-outside",L"",true,L"",2,2,false,true,0},
        {"zero-alpha-does-not-capture",hole,false,L"",250,100,true,false,0,false,0},
        {"opaque-mask-resize",hole,false,L"",600,260,true,false,0,false,1,true},
        {"capture-to-opaque",hole,false,L"",100,100,true,false,0,false,1,false,true},
        {"capture-to-opaque-to-clear",hole,false,L"",250,100,true,false,0,true,1,false,true},
        {"rotated-scaled-mask-interior",hole,false,L"",100,100,true,false,0,false,1,false,false,true},
        {"negative-mask-gap",L"M-50 0 L100 0 L100 300 L-50 300 Z",false,L"",250,100,true},
        {"rotated-scaled-hole-click",hole,false,L"",250,140,false,true,0,false,1,false,false,true},
        {"opaque-mask-collapse-regrow",hole,false,L"",100,100,true,false,0,false,1,false,false,false,true},
        // Fractional pixels remain native input targets. Native movement must
        // survive the production VisualSurface path, including outside its hole.
        {"fractional-interior-wheel",hole,false,L"",100,100,true,false,0,false,.5f},
        {"fractional-covered-wheel",hole,false,L"",250,100,true,false,0,false,.5f},
        {"fractional-horizontal-wheel",hole,false,L"",100,100,true,false,0,false,.5f,false,false,false,false,true},
        {"fractional-horizontal-child-parent-wheel",hole,false,L"",100,100,false,false,0,false,.5f,false,false,false,false,false,true},
        {"button-fractional-covered",hole,false,L"",250,140,false,true,4,false,.5f},
        {"button-fractional-nonhole",hole,false,L"",440,140,false,true,3,false,.5f},
        // Each state is presented and receives real input before changing the
        // same native scroller. Merely constructing an intermediate capture
        // before the next synchronous patch does not establish transition parity.
        {"rendered-capture",hole,false,L"",250,100,true,false,0,false,.5f},
        {"rendered-capture-to-opaque",hole,false,L"",250,100,false,false,0,false,1,false,false,false,false,false,false,true},
        {"rendered-capture-to-opaque-to-clear",hole,false,L"",250,100,true,false,0,true,1,false,false,false,false,false,false,true}
    };
    void tick(){try{
        unsigned index=phase/3,part=phase%3;++phase;
        if(index>=std::size(samples)){
            patch(LR"([{"TextCreate":{"id":9,"render_layer_id":0}},{"TextUpdate":{"id":9,"size_x":-1,"size_y":-1,"content":"W","measure_generation":1,"style":{"font_size":18}}},{"NativeMaskUpdate":{"id":9,"entries":[{"path":"M0 0 L1 0 L1 1 L0 1 Z","opacity":1,"clips":[]}]}}])");
            auto host=find_host(scene->root(),9);auto source=Hosting::ElementCompositionPreview::GetElementVisual(host.Children().GetAt(0));
            auto before=source.Clip();auto width=host.Width();check(bool(before),"autosized-text-has-geometric-mask");
            patch(LR"([{"TextUpdate":{"id":9,"content":"Autosized text grows","measure_generation":2,"style":{"font_size":36}}}])");
            check(host.Width()>width&&source.Clip()!=before,"autosized-text-rebuilds-geometric-mask");before=source.Clip();
            patch(LR"([{"TextUpdate":{"id":9,"size_x":500,"size_y":100,"measure_generation":3}}])");
            check(source.Clip()!=before,"explicit-size-rebuilds-geometric-mask");
            patch(LR"([{"NativeMaskUpdate":{"id":9,"entries":[]}}])");
            check(!source.Clip(),"cleared-text-mask-retires-geometric-clip");
            check(true,"complete");finish(true);return;
        }
        auto sample=samples[index];
        if(part==0){
            if(!sample.retain_scene){
            scene->close();window.Content(nullptr);
            scene=std::make_unique<NativeScene>([this](auto const& m){return interrupt(m);},[](auto const&){return hstring(L"Segoe UI");},[](auto const&){});
            window.Content(scene->root());scene->initialize(device.get());scene->resize(700,600,1);
        patch(LR"([{"ScrollerCreate":{"id":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":1,"size_x":640,"size_y":480,"size_inner_pane_x":640,"size_inner_pane_y":2000,"scroll_enabled_x":false,"scroll_enabled_y":true,"transform":[1,0,0,1,0,0]}},{"ScrollerCreate":{"id":2,"parent_frame":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":2,"size_x":640,"size_y":300,"size_inner_pane_x":1800,"size_inner_pane_y":1600,"scroll_enabled_x":true,"scroll_enabled_y":true,"transform":[1,0,0,1,0,50]}}])");
            outer=find(scene->root(),1);inner=find(outer.Content().as<Canvas>(),2);
            patch(LR"([{"ScrollerUpdate":{"id":2,"clip_content":false,"corner_radius":0,"clip_path":""}},{"NativeMaskUpdate":{"id":2,"entries":[]}}])");
            patch(LR"([{"ButtonCreate":{"id":3,"parent_frame":2,"render_layer_id":0}},{"ButtonUpdate":{"id":3,"size_x":80,"size_y":80,"transform":[1,0,0,1,400,150],"color":{"Rgb":[1,0,0]},"hover_color":{"Rgb":[1,0,0]}}},{"ButtonCreate":{"id":4,"parent_frame":2,"render_layer_id":0}},{"ButtonUpdate":{"id":4,"size_x":60,"size_y":80,"transform":[1,0,0,1,220,150],"color":{"Rgb":[0,1,0]},"hover_color":{"Rgb":[0,1,0]}}},{"ButtonCreate":{"id":5,"parent_frame":2,"render_layer_id":0}},{"ButtonUpdate":{"id":5,"size_x":64,"size_y":64,"transform":[1,0,0,1,0,50],"color":{"Rgb":[0,0,1]},"hover_color":{"Rgb":[0,0,1]}}}])");
            }
            if(!sample.retain_scene)retired=inner;
            if(sample.horizontal_only)patch(LR"([{"ScrollerUpdate":{"id":2,"size_inner_pane_y":300,"scroll_enabled_y":false}}])");
            if(sample.transformed)patch(LR"([{"ScrollerUpdate":{"id":2,"transform":[0.9,0.2,-0.2,0.9,20,60]}},{"ButtonUpdate":{"id":3,"transform":[0.9,0.2,-0.2,0.9,360,230]}},{"ButtonUpdate":{"id":4,"transform":[0.9,0.2,-0.2,0.9,198,194]}},{"ButtonUpdate":{"id":5,"transform":[0.9,0.2,-0.2,0.9,20,60]}}])");
            if(sample.transition){
                patch(LR"([{"NativeMaskUpdate":{"id":2,"entries":[{"path":"M200 0 L300 0 L300 300 L200 300 Z","opacity":0.5,"clips":[]}]}}])");
                check(bool(Hosting::ElementCompositionPreview::GetElementChildVisual(inner.Parent().as<Grid>())),"fractional-mask-has-capture");
            }
            if(sample.resize)patch(LR"([{"ScrollerUpdate":{"id":2,"size_x":320,"size_y":200}}])");
            if(*sample.mask){auto value=std::wstring(L"[{\"NativeMaskUpdate\":{\"id\":2,\"entries\":[{\"path\":\"")+sample.mask+L"\",\"opacity\":"+std::to_wstring(sample.alpha)+L",\"clips\":[]}]}}]";patch(value.c_str());}
            if(sample.rounded)patch(LR"([{"ScrollerUpdate":{"id":2,"clip_content":true,"corner_radius":50}}])");
            if(*sample.clip){auto value=std::wstring(L"[{\"ScrollerUpdate\":{\"id\":2,\"clip_content\":true,\"clip_path\":\"")+sample.clip+L"\"}}]";patch(value.c_str());}
            if(sample.clear)patch(LR"([{"NativeMaskUpdate":{"id":2,"entries":[]}}])");
            if(sample.alpha==0)patch(LR"([{"NativeMaskUpdate":{"id":2,"entries":[{"path":"M200 0 L300 0 L300 300 L200 300 Z","opacity":0,"clips":[]}]}}])");
            if(sample.resize)patch(LR"([{"ScrollerUpdate":{"id":2,"size_x":640,"size_y":300}}])");
            if(sample.collapse){
                Shapes::Path retained{nullptr};
                for(auto child:inner.Parent().as<Grid>().Children())if(auto canvas=child.try_as<Canvas>())
                    for(auto input:canvas.Children())if(auto path=input.try_as<Shapes::Path>())retained=path;
                check(retained&&retained.Data(),"collapse-retains-old-input-path");
                patch(LR"([{"ScrollerUpdate":{"id":2,"size_x":0,"size_y":0}}])");
                check(!retained.Data(),"collapsed-mask-detaches-retained-input-geometry");
                check(!Hosting::ElementCompositionPreview::GetElementVisual(inner).Clip(),"collapsed-mask-clears-geometric-clip");
                patch(LR"([{"ScrollerUpdate":{"id":2,"size_x":640,"size_y":300}}])");
            }
            clicked_id=0;
            // Reset setup only; the measured movement below comes exclusively
            // from the owned physical wheel event, never a commanded offset.
            outer.ChangeView(0.0,0.0,nullptr,true);inner.ChangeView(0.0,0.0,nullptr,true);
            if(sample.retain_scene)check(find(outer.Content().as<Canvas>(),2)==retired,"mask-transition-preserves-native-scroller");
        }else if(part==1){
            json::JsonObject pixels;pixels.Insert(L"stage",text(to_hstring(std::string(sample.name)+"-pixels")));
            HWND hwnd=nullptr;check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));auto dc=GetDC(nullptr);POINT origin{};ClientToScreen(hwnd,&origin);auto dpr=GetDpiForWindow(hwnd)/96.0;
            std::array<bool,2> pixel_matches{};unsigned pixel_index=0;
            for(auto const& item:std::array<std::pair<wchar_t const*,float>,2>{{{L"unmaskedRed",440},{L"maskedGreen",250}}}){
                auto point=inner.TransformToVisual(nullptr).TransformPoint({item.second,140});auto color=GetPixel(dc,origin.x+LONG(point.X*dpr),origin.y+LONG(point.Y*dpr));
                json::JsonArray rgb;rgb.Append(numeric(GetRValue(color)));rgb.Append(numeric(GetGValue(color)));rgb.Append(numeric(GetBValue(color)));pixels.Insert(item.first,rgb);
                const bool red_visible = !*sample.clip || std::wstring_view(sample.clip).find(L"L640") != std::wstring_view::npos;
                const bool green_clipped = std::wstring_view(sample.clip).find(L"M1000") != std::wstring_view::npos;
                const double green_hole_alpha = std::wstring_view(sample.mask) == hole && !sample.clear ? sample.alpha : 0;
                const auto white_channel=BYTE(std::lround(255 * green_hole_alpha));
                const auto expected = item.second == 440 ? (red_visible ? RGB(255,0,0) : RGB(255,255,255)) :
                    (green_clipped ? RGB(255,255,255) : RGB(white_channel,255,white_channel));
                const bool fractional_pixels=item.second==250 && green_hole_alpha>0 && green_hole_alpha<1;
                const bool fractional_match=color!=CLR_INVALID && std::abs(int(GetRValue(color))-GetRValue(expected))<=1 &&
                    std::abs(int(GetGValue(color))-GetGValue(expected))<=1 && std::abs(int(GetBValue(color))-GetBValue(expected))<=1;
                json::JsonArray expected_rgb;expected_rgb.Append(numeric(GetRValue(expected)));expected_rgb.Append(numeric(GetGValue(expected)));expected_rgb.Append(numeric(GetBValue(expected)));
                pixels.Insert(hstring(item.first)+L"Expected",expected_rgb);
                pixel_matches[pixel_index++]=fractional_pixels?fractional_match:color==expected;
            }ReleaseDC(nullptr,dc);output<<to_string(pixels.Stringify())<<'\n';output.flush();
            for(bool matches:pixel_matches)check(matches,"native-mask-pixels-preserved");
            before_outer=outer.VerticalOffset();before_inner=sample.horizontal?inner.HorizontalOffset():inner.VerticalOffset();before_inner_cross=sample.horizontal?inner.VerticalOffset():inner.HorizontalOffset();wheel(sample.horizontal,sample.x,sample.y,sample.click);
        }else{
            json::JsonObject row;row.Insert(L"stage",text(to_hstring(sample.name)));
            row.Insert(L"outerDelta",numeric(outer.VerticalOffset()-before_outer));row.Insert(L"innerDelta",numeric((sample.horizontal?inner.HorizontalOffset():inner.VerticalOffset())-before_inner));
            row.Insert(L"horizontalInput",json::JsonValue::CreateBooleanValue(sample.horizontal));
            row.Insert(L"innerOtherAxisDelta",numeric((sample.horizontal?inner.VerticalOffset():inner.HorizontalOffset())-before_inner_cross));
            auto visual=Hosting::ElementCompositionPreview::GetElementChildVisual(inner.Parent().as<Grid>());
            row.Insert(L"capture",json::JsonValue::CreateBooleanValue(bool(visual)));
            row.Insert(L"spriteHit",json::JsonValue::CreateBooleanValue(visual&&visual.IsHitTestVisible()));
            row.Insert(L"metadataOpacity",numeric(inner.Opacity()));
            row.Insert(L"sourceHit",json::JsonValue::CreateBooleanValue(Hosting::ElementCompositionPreview::GetElementVisual(inner).IsHitTestVisible()));
            row.Insert(L"sourceOpacity",numeric(Hosting::ElementCompositionPreview::GetElementVisual(inner).Opacity()));
            output<<to_string(row.Stringify())<<'\n';output.flush();
            const bool fractional_capture=*sample.mask && !sample.clear && sample.alpha>0 && sample.alpha<1;
            check(bool(visual)==fractional_capture,"capture-exists-only-for-fractional-mask");
            check(!visual||!visual.IsHitTestVisible(),"capture-sprite-never-steals-native-input");
            if(!fractional_capture)check(Hosting::ElementCompositionPreview::GetElementVisual(inner).Opacity()==1,"opaque-or-cleared-restores-native-source");
            if(sample.click){check(clicked_id==sample.button,sample.name);}
            else {const double after_inner=sample.horizontal?inner.HorizontalOffset():inner.VerticalOffset();
                check(close_enough(sample.horizontal?inner.VerticalOffset():inner.HorizontalOffset(),before_inner_cross),"wheel-does-not-move-other-inner-axis");
                check(sample.inner_expected ? (after_inner-before_inner>40 && close_enough(outer.VerticalOffset(),before_outer)) : (outer.VerticalOffset()-before_outer>40 && close_enough(after_inner,before_inner)),sample.name);}
        }
    }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}catch(std::exception const& e){output<<e.what()<<'\n';finish(false);}}
    void OnLaunched(LaunchActivatedEventArgs const&){try{
        Resources().MergedDictionaries().Append(XamlControlsResources());
        scene=std::make_unique<NativeScene>([this](auto const& m){return interrupt(m);},[](auto const&){return hstring(L"Segoe UI");},[](auto const&){});
        window=Window();window.Title(L"Pax native mask input contract");window.Content(scene->root());window.Activate();
        D3D_FEATURE_LEVEL level;check_hresult(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_HARDWARE,nullptr,D3D11_CREATE_DEVICE_BGRA_SUPPORT,nullptr,0,D3D11_SDK_VERSION,device.put(),&level,nullptr));
        scene->initialize(device.get());scene->resize(700,600,1);
        patch(LR"([{"ScrollerCreate":{"id":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":1,"size_x":640,"size_y":480,"size_inner_pane_x":640,"size_inner_pane_y":2000,"scroll_enabled_x":false,"scroll_enabled_y":true,"transform":[1,0,0,1,0,0]}},{"ScrollerCreate":{"id":2,"parent_frame":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":2,"size_x":640,"size_y":300,"size_inner_pane_x":1800,"size_inner_pane_y":1600,"scroll_enabled_x":true,"scroll_enabled_y":true,"transform":[1,0,0,1,0,50]}}])");
        HWND hwnd=nullptr;check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));MoveWindow(hwnd,50,50,800,700,true);SetForegroundWindow(hwnd);
        timer=winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();timer.Interval(std::chrono::milliseconds(900));timer.Tick([this](auto const&,auto const&){tick();});timer.Start();
    }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}catch(std::exception const& e){output<<e.what()<<'\n';finish(false);}}
};
}
int WINAPI wWinMain(HINSTANCE,HINSTANCE,PWSTR,int){wchar_t path[32768];auto n=GetEnvironmentVariableW(L"PAX_NATIVE_MASK_INPUT_LOG",path,DWORD(std::size(path)));output.open(n&&n<std::size(path)?path:L"wheel-input-contract.jsonl");if(!output)return 2;try{SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);PACKAGE_VERSION v{};v.Version=WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,WINDOWSAPPSDK_RELEASE_VERSION_TAG_W,v,MddBootstrapInitializeOptions_None));init_apartment(apartment_type::single_threaded);Application::Start([](auto const&){make<App>();});return process_result;}catch(hresult_error const& e){output<<to_string(e.message())<<'\n';}return 1;}
