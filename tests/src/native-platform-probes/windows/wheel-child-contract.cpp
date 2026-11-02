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
    com_ptr<ID3D11Device> device;std::unique_ptr<NativeScene> scene;
    ScrollViewer outer{nullptr},inner{nullptr},text_scroll{nullptr};
    FrameworkElement child{nullptr};TextBox editor{nullptr};Slider slider{nullptr};ComboBox combo{nullptr};
    winrt::Microsoft::UI::Composition::CompositionVisualSurface capture{nullptr};
    winrt::Microsoft::UI::Composition::SpriteVisual paint{nullptr};
    winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer timer{nullptr};
    unsigned index=0,step=0,wheels=0;bool prevent=false;double before=0;HWND hwnd=nullptr;
    bool chrome=false,empty=false,horizontal=false,textbox_chrome=false;
    double target_x=150,target_y=180;
    Primitives::ScrollBar bar{nullptr};FrameworkElement chrome_target{nullptr};
    winrt::Windows::Foundation::IInspectable bar_observer{nullptr};unsigned bar_wheels=0;bool bar_original_source_owned=false;
    void detach_bar_observer(){
        if(bar&&bar_observer)bar.RemoveHandler(UIElement::PointerWheelChangedEvent(),bar_observer);
        bar_observer=nullptr;bar=nullptr;
    }
    void observe_bar(Primitives::ScrollBar const& next){
        if(bar==next&&bar_observer)return;
        detach_bar_observer();bar=next;
        bar_observer=box_value(Input::PointerEventHandler([this](auto const&,auto const& args){
            ++bar_wheels;bool owned=false;auto source=args.OriginalSource().template try_as<DependencyObject>();
            auto current=source;
            while(current){if(current==bar){owned=true;break;}current=Media::VisualTreeHelper::GetParent(current);}
            bar_original_source_owned=bar_original_source_owned||owned;
            json::JsonObject row;row.Insert(L"stage",text(L"native-chrome-original-source"));row.Insert(L"case",numeric(index));
            row.Insert(L"ownedBarInAncestry",json::JsonValue::CreateBooleanValue(owned));row.Insert(L"handledObserved",json::JsonValue::CreateBooleanValue(args.Handled()));
            row.Insert(L"originalClass",text(source?get_class_name(source):hstring{}));output<<to_string(row.Stringify())<<'\n';output.flush();
            // Observation only: never changes Handled or invokes a scroll method.
        }));
        bar.AddHandler(UIElement::PointerWheelChangedEvent(),bar_observer,true);
    }
    Primitives::ScrollBar find_bar(DependencyObject const& node){
        if(auto viewer=node.try_as<ScrollViewer>();viewer&&viewer!=(textbox_chrome?text_scroll:inner))return nullptr;
        if(auto found=node.try_as<Primitives::ScrollBar>();found&&found.Orientation()==(horizontal?Orientation::Horizontal:Orientation::Vertical))return found;
        for(int i=0;i<Media::VisualTreeHelper::GetChildrenCount(node);++i)
            if(auto found=find_bar(Media::VisualTreeHelper::GetChild(node,i)))return found;
        return nullptr;
    }
    FrameworkElement find_thumb(DependencyObject const& node){
        if(auto thumb=node.try_as<Primitives::Thumb>();thumb&&thumb.Visibility()==Visibility::Visible&&thumb.ActualWidth()>2&&thumb.ActualHeight()>2)return thumb;
        for(int i=0;i<Media::VisualTreeHelper::GetChildrenCount(node);++i)
            if(auto found=find_thumb(Media::VisualTreeHelper::GetChild(node,i)))return found;
        return nullptr;
    }
    void update_target(){
        if(!chrome&&!textbox_chrome)return;
        auto found=find_bar(textbox_chrome?text_scroll:inner);check(bool(found)&&found.Visibility()==Visibility::Visible,"actual-owned-native-scrollbar-visible");observe_bar(found);
        chrome_target=find_thumb(bar);if(!chrome_target)chrome_target=bar;
        auto bounds=chrome_target.TransformToVisual(scene->root()).TransformBounds({0,0,float(chrome_target.ActualWidth()),float(chrome_target.ActualHeight())});
        check(bounds.Width>2&&bounds.Height>2,"actual-scrollbar-target-has-area");target_x=bounds.X+bounds.Width/2;target_y=bounds.Y+bounds.Height/2;
        json::JsonObject row;row.Insert(L"stage",text(L"native-chrome-target"));row.Insert(L"case",numeric(index));row.Insert(L"class",text(get_class_name(chrome_target)));row.Insert(L"barName",text(bar.Name()));
        row.Insert(L"left",numeric(bounds.X));row.Insert(L"top",numeric(bounds.Y));row.Insert(L"width",numeric(bounds.Width));row.Insert(L"height",numeric(bounds.Height));row.Insert(L"x",numeric(target_x));row.Insert(L"y",numeric(target_y));output<<to_string(row.Stringify())<<'\n';output.flush();
    }
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider(){if(!metadata)metadata=XamlTypeInfo::XamlControlsXamlMetaDataProvider();return metadata;}
    Markup::IXamlType GetXamlType(hstring const& n){return provider().GetXamlType(n);}
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& t){return provider().GetXamlType(t);}
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions(){return provider().GetXmlnsDefinitions();}
    template<class T>T find(DependencyObject const& node,hstring const& id){
        if(auto element=node.try_as<T>();element&&Automation::AutomationProperties::GetAutomationId(element)==id)return element;
        for(int i=0;i<Media::VisualTreeHelper::GetChildrenCount(node);++i)
            if(auto result=find<T>(Media::VisualTreeHelper::GetChild(node,i),id))return result;
        return nullptr;
    }
    ScrollViewer text_view(DependencyObject const& node){
        if(auto view=node.try_as<ScrollViewer>();view&&view.Name()==L"ContentElement")return view;
        for(int i=0;i<Media::VisualTreeHelper::GetChildrenCount(node);++i)
            if(auto view=text_view(Media::VisualTreeHelper::GetChild(node,i)))return view;
        return nullptr;
    }
    void record_text_layout(hstring const& stage){
        json::JsonObject row;row.Insert(L"stage",text(stage));row.Insert(L"case",numeric(index));row.Insert(L"hasContentElement",json::JsonValue::CreateBooleanValue(bool(text_scroll)));
        row.Insert(L"textLength",numeric(editor.Text().size()));row.Insert(L"acceptsReturn",json::JsonValue::CreateBooleanValue(editor.AcceptsReturn()));row.Insert(L"editorHeight",numeric(editor.ActualHeight()));row.Insert(L"editorWidth",numeric(editor.ActualWidth()));row.Insert(L"fontSize",numeric(editor.FontSize()));
        if(text_scroll){row.Insert(L"extentHeight",numeric(text_scroll.ExtentHeight()));row.Insert(L"viewportHeight",numeric(text_scroll.ViewportHeight()));row.Insert(L"scrollableHeight",numeric(text_scroll.ScrollableHeight()));row.Insert(L"offsetY",numeric(text_scroll.VerticalOffset()));}
        output<<to_string(row.Stringify())<<'\n';output.flush();
    }
    bool interrupt(json::JsonObject const& message){
        if(message.HasKey(L"Wheel")){
            ++wheels;auto v=message.GetNamedObject(L"Wheel");
            check(std::abs(number(v,L"x")-target_x)<((chrome||textbox_chrome)?1.1:1.0)&&std::abs(number(v,L"y")-target_y)<((chrome||textbox_chrome)?1.1:1.0),"window-space-wheel-coordinates");
            check(number(v,L"delta_x")== (horizontal?120:0)&&number(v,L"delta_y")== (horizontal?0:120),"raw-wheel-axis-and-sign");
            output<<to_string(message.Stringify())<<'\n';output.flush();return prevent;
        }return false;
    }
    void patch(wchar_t const* value){scene->apply(json::JsonArray::Parse(value));scene->root().UpdateLayout();}
    double value(){if(chrome)return horizontal?inner.HorizontalOffset():inner.VerticalOffset();if(empty)return outer.VerticalOffset();if(slider)return slider.Value();if(combo)return combo.SelectedIndex();return text_scroll?text_scroll.VerticalOffset():0;}
    void configure(){
        detach_bar_observer();window.Content(nullptr);if(scene)scene->close();scene.reset();capture=nullptr;paint=nullptr;
        child=nullptr;editor=nullptr;slider=nullptr;combo=nullptr;text_scroll=nullptr;bar=nullptr;chrome_target=nullptr;
        chrome=index>=12&&index<20;empty=index>=20&&index<24;textbox_chrome=index>=24;horizontal=chrome&&(index-12)/4==1;target_x=150;target_y=180;
        scene=std::make_unique<NativeScene>([this](auto const& m){return interrupt(m);},[](auto const&){return hstring(L"Segoe UI");},[](auto const&){});
        window.Content(scene->root());scene->initialize(device.get());
        RECT client{};check(GetClientRect(hwnd,&client),"actual-client-size");double dpr=GetDpiForWindow(hwnd)/96.0;
        scene->resize((client.right-client.left)/dpr,(client.bottom-client.top)/dpr,dpr);
        patch(LR"([{"ScrollerCreate":{"id":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":1,"size_x":640,"size_y":480,"size_inner_pane_x":640,"size_inner_pane_y":2000,"scroll_enabled_x":false,"scroll_enabled_y":true,"transform":[1,0,0,1,0,0]}},{"ScrollerCreate":{"id":2,"parent_frame":1,"render_layer_id":0}},{"ScrollerUpdate":{"id":2,"size_x":600,"size_y":280,"size_inner_pane_x":1800,"size_inner_pane_y":280,"scroll_enabled_x":true,"scroll_enabled_y":false,"transform":[1,0,0,1,0,60]}}])");
        auto kind=index/4;prevent=index%2;
        if(chrome){
            patch(LR"([{"ScrollerUpdate":{"id":2,"size_inner_pane_y":900,"scroll_enabled_y":true}}])");
        }else if(empty){
            patch(LR"([{"ScrollerUpdate":{"id":2,"size_inner_pane_y":0,"scroll_enabled_y":false}}])");
        }else if(kind==0){patch(LR"([{"SliderCreate":{"id":3,"parent_frame":2,"render_layer_id":0}},{"SliderUpdate":{"id":3,"size_x":500,"size_y":200,"transform":[1,0,0,1,50,80],"min":0,"max":100,"value":50,"step":1}}])");slider=find<Slider>(scene->root(),L"pax-node-3");child=slider;}
        else if(kind==1){patch(LR"([{"DropdownCreate":{"id":3,"parent_frame":2,"render_layer_id":0}},{"DropdownUpdate":{"id":3,"size_x":500,"size_y":200,"transform":[1,0,0,1,50,80],"options":["Zero","One","Two"],"selected_id":0}}])");combo=find<ComboBox>(scene->root(),L"pax-node-3");child=combo;}
        else {
            patch(LR"([{"TextboxCreate":{"id":3,"parent_frame":2,"render_layer_id":0}},{"TextboxUpdate":{"id":3,"size_x":500,"size_y":200,"transform":[1,0,0,1,50,80],"is_text_area":true}}])");editor=find<TextBox>(scene->root(),L"pax-node-3");child=editor;
            hstring lines;for(int i=0;i<80;++i)lines=lines+L"Editable native line "+to_hstring(i)+L"\r\n";editor.Text(lines);editor.Select(0,0);
            ScrollViewer::SetVerticalScrollBarVisibility(editor,ScrollBarVisibility::Visible);
            // Ordinary editor content/chrome is qualified independently of the
            // native Template-null reset diagnostic, which can destroy extent.
            editor.ApplyTemplate();
        }
        scene->root().UpdateLayout();
        if(!chrome&&!empty){check(bool(child),"genuine-native-child-created");child.Focus(FocusState::Programmatic);scene->root().UpdateLayout();}
        outer=find<ScrollViewer>(scene->root(),L"pax-node-1");inner=find<ScrollViewer>(scene->root(),L"pax-node-2");
        if(chrome||empty){
            check(bool(inner)&&bool(outer),"genuine-nested-native-viewers-created");
            if(chrome){inner.HorizontalScrollBarVisibility(ScrollBarVisibility::Visible);inner.VerticalScrollBarVisibility(ScrollBarVisibility::Visible);}
            inner.ApplyTemplate();auto original=inner.Template();check(bool(original),"viewer-template-present");
            inner.Template(nullptr);scene->root().UpdateLayout();inner.Template(original);scene->root().UpdateLayout();
            // Input occurs on a later dispatcher turn, after the bridge refresh.
            inner.Focus(FocusState::Programmatic);
        }
        if(editor){text_scroll=text_view(editor);record_text_layout(L"immediate-native-layout");}
        if(index%4>=2){
            auto original=Hosting::ElementCompositionPreview::GetElementVisual(inner);auto compositor=original.Compositor();
            capture=compositor.CreateVisualSurface();capture.SourceVisual(original);capture.SourceSize({float(inner.ActualWidth()),float(inner.ActualHeight())});
            paint=compositor.CreateSpriteVisual();paint.Brush(compositor.CreateSurfaceBrush(capture));paint.Size({float(inner.ActualWidth()),float(inner.ActualHeight())});paint.IsHitTestVisible(false);
            Hosting::ElementCompositionPreview::SetElementChildVisual(inner.Parent().as<UIElement>(),paint);original.Opacity(0);
        }
        if(empty){check(inner.Content().as<FrameworkElement>().ActualHeight()==0&&inner.ScrollableHeight()==0,"authored-empty-pane-stays-empty");}
        wheels=0;bar_wheels=0;bar_original_source_owned=false;before=value();
        json::JsonObject row;row.Insert(L"stage",text(L"case-start"));row.Insert(L"case",numeric(index));row.Insert(L"cancel",json::JsonValue::CreateBooleanValue(prevent));row.Insert(L"chrome",json::JsonValue::CreateBooleanValue(chrome));row.Insert(L"textboxChrome",json::JsonValue::CreateBooleanValue(textbox_chrome));row.Insert(L"empty",json::JsonValue::CreateBooleanValue(empty));row.Insert(L"horizontal",json::JsonValue::CreateBooleanValue(horizontal));row.Insert(L"captured",json::JsonValue::CreateBooleanValue(bool(capture)));output<<to_string(row.Stringify())<<'\n';output.flush();
    }
    void activate(){
        if(GetForegroundWindow()==hwnd)return;
        RECT bounds{};check(GetWindowRect(hwnd,&bounds),"activation-window-bounds");POINT point{bounds.left+150,bounds.top+14};
        check(GetAncestor(WindowFromPoint(point),GA_ROOT)==hwnd,"activation-target-owned");
        INPUT events[3]{};events[0].type=events[1].type=events[2].type=INPUT_MOUSE;
        events[0].mi.dwFlags=MOUSEEVENTF_MOVE|MOUSEEVENTF_ABSOLUTE|MOUSEEVENTF_VIRTUALDESK;
        events[0].mi.dx=LONG(std::round((point.x-GetSystemMetrics(SM_XVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CXVIRTUALSCREEN)-1)));
        events[0].mi.dy=LONG(std::round((point.y-GetSystemMetrics(SM_YVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CYVIRTUALSCREEN)-1)));
        events[1].mi.dwFlags=MOUSEEVENTF_LEFTDOWN;events[2].mi.dwFlags=MOUSEEVENTF_LEFTUP;
        check(SendInput(3,events,sizeof(INPUT))==3,"owned-caption-activation");
    }
    void input(bool wheel){
        update_target();
        check(GetForegroundWindow()==hwnd,"foreground-owned");POINT origin{};check(ClientToScreen(hwnd,&origin),"client-origin");double dpr=GetDpiForWindow(hwnd)/96.0;POINT p{origin.x+LONG(std::round(target_x*dpr)),origin.y+LONG(std::round(target_y*dpr))};
        check(GetAncestor(WindowFromPoint(p),GA_ROOT)==hwnd,"input-hit-owned-window");INPUT events[2]{};events[0].type=events[1].type=INPUT_MOUSE;
        events[0].mi.dwFlags=MOUSEEVENTF_MOVE|MOUSEEVENTF_ABSOLUTE|MOUSEEVENTF_VIRTUALDESK;events[0].mi.dx=LONG(std::round((p.x-GetSystemMetrics(SM_XVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CXVIRTUALSCREEN)-1)));events[0].mi.dy=LONG(std::round((p.y-GetSystemMetrics(SM_YVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CYVIRTUALSCREEN)-1)));
        events[1].mi.dwFlags=horizontal?MOUSEEVENTF_HWHEEL:MOUSEEVENTF_WHEEL;events[1].mi.mouseData=DWORD(horizontal?120:-120);UINT count=wheel?2:1;check(SendInput(count,events,sizeof(INPUT))==count,wheel?"real-wheel-delivered":"real-pointer-hover-delivered");
    }
    void observe(){
        json::JsonObject row;row.Insert(L"stage",text(L"case-result"));row.Insert(L"case",numeric(index));row.Insert(L"wheels",numeric(wheels));row.Insert(L"before",numeric(before));row.Insert(L"after",numeric(value()));row.Insert(L"outerY",numeric(outer.VerticalOffset()));row.Insert(L"innerX",numeric(inner.HorizontalOffset()));row.Insert(L"innerY",numeric(inner.VerticalOffset()));output<<to_string(row.Stringify())<<'\n';output.flush();
        check(wheels==1,"one-interrupt-per-original-native-wheel");
        if(chrome||textbox_chrome)check(bar_wheels==1&&bar_original_source_owned,"actual-original-wheel-targets-owned-native-scrollbar");
        if(prevent)check(value()==before&&outer.VerticalOffset()==0&&inner.HorizontalOffset()==0&&inner.VerticalOffset()==0,"prevent-default-precedes-native-child-and-parent");
        else if(chrome)check(value()>before&&outer.VerticalOffset()==0&&(horizontal?inner.VerticalOffset()==0:inner.HorizontalOffset()==0),"native-scrollbar-wheel-default-preserved");
        else if(empty)check(outer.VerticalOffset()>0&&inner.HorizontalOffset()==0&&inner.VerticalOffset()==0&&inner.Content().as<FrameworkElement>().ActualHeight()==0,"empty-viewport-original-wheel-routes-to-parent");
        else if(combo)check(combo.SelectedIndex()==1&&outer.VerticalOffset()==0,"native-combobox-default-preserved");
        else if(editor)check(text_scroll.VerticalOffset()>0&&outer.VerticalOffset()==0,"native-textbox-default-preserved");
        else check(slider.Value()==50&&outer.VerticalOffset()>0,"slider-nonconsumed-wheel-retains-native-parent-default");
    }
    void finish(bool ok){if(timer)timer.Stop();detach_bar_observer();if(scene)scene->close();scene.reset();if(window){window.Content(nullptr);window.Close();}process_result=ok?0:1;Exit();}
    void tick(){try{
        if(step==0)configure();if(step==1)activate();if(step==2){
            if(editor){text_scroll=text_view(editor);record_text_layout(L"later-dispatcher-native-layout");check(bool(text_scroll)&&text_scroll.ScrollableHeight()>0,"native-textbox-content-element-scrolls");before=value();}
            input(false);
        }if(step==3)input(true);if(step==6){observe();step=0;if(++index==28){check(true,"complete");finish(true);}return;}++step;
    }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}catch(std::exception const& e){output<<e.what()<<'\n';finish(false);}}
    void OnLaunched(LaunchActivatedEventArgs const&){try{
        Resources().MergedDictionaries().Append(XamlControlsResources());window=Window();window.Title(L"Pax native child wheel contract");window.Content(Grid());window.Activate();check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));MoveWindow(hwnd,50,50,800,700,true);SetForegroundWindow(hwnd);
        D3D_FEATURE_LEVEL level;check_hresult(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_HARDWARE,nullptr,D3D11_CREATE_DEVICE_BGRA_SUPPORT,nullptr,0,D3D11_SDK_VERSION,device.put(),&level,nullptr));
        timer=winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();timer.Interval(std::chrono::milliseconds(500));timer.Tick([this](auto const&,auto const&){tick();});timer.Start();
    }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}}
};
}
int WINAPI wWinMain(HINSTANCE,HINSTANCE,PWSTR,int){wchar_t path[32768];auto n=GetEnvironmentVariableW(L"PAX_WHEEL_CHILD_LOG",path,DWORD(std::size(path)));output.open(n&&n<std::size(path)?path:L"wheel-child-contract.jsonl");if(!output)return 2;try{SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);PACKAGE_VERSION v{};v.Version=WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,WINDOWSAPPSDK_RELEASE_VERSION_TAG_W,v,MddBootstrapInitializeOptions_None));init_apartment(apartment_type::single_threaded);Application::Start([](auto const&){make<App>();});return process_result;}catch(hresult_error const& e){output<<to_string(e.message())<<'\n';}return 1;}
