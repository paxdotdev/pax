#include <windows.h>
#include <unknwn.h>
#include <d3d11.h>
#include <MddBootstrap.h>
#include <microsoft.ui.xaml.window.h>
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
bool layout_only = false;
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
    unsigned dropdown_changes = 0;
    unsigned clicks = 0;
    std::map<uint32_t, json::JsonObject> measurements;
    TextBlock zero_label{nullptr};
    ScrollViewer extent_scroll{nullptr};
    TextBlock button_label{nullptr}, first_radio_label{nullptr}, first_dropdown_label{nullptr};
    static TextBlock content_label(ContentControl const& control, bool underlined, hstring const& expected) {
        require_check(bool(control), "native label control exists");
        auto value = control.Content().try_as<TextBlock>();
        require_check(bool(value), "native control retains a TextBlock content label");
        require_check(value.Text() == expected, "native label text");
        require_check(Automation::AutomationProperties::GetName(control) == expected, "native control accessible name");
        require_check(value.TextDecorations() == (underlined ? winrt::Windows::UI::Text::TextDecorations::Underline : winrt::Windows::UI::Text::TextDecorations::None), "native control underline");
        return value;
    }
    static TextBlock dropdown_label(ComboBox const& dropdown, uint32_t index, bool underlined, hstring const& expected) {
        auto item = dropdown.Items().GetAt(index).as<ComboBoxItem>();
        auto label = item.Content().try_as<TextBlock>();
        if (!label && dropdown.SelectedIndex() == int(index)) label = dropdown.SelectionBoxItem().try_as<TextBlock>();
        require_check(bool(label), "Dropdown label exists in popup item or selected faceplate");
        require_check(label.Text() == expected, "Dropdown native label text");
        require_check(Automation::AutomationProperties::GetName(item) == expected, "Dropdown item accessible name");
        require_check(label.TextDecorations() == (underlined ? winrt::Windows::UI::Text::TextDecorations::Underline : winrt::Windows::UI::Text::TextDecorations::None), "Dropdown native label underline");
        return label;
    }
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
    std::pair<Grid, FrameworkElement> node(uint32_t id) {
        for(auto child:scene->root().Children()) if(auto host=child.try_as<Grid>())
            for(auto control:host.Children()) {
                if(Automation::AutomationProperties::GetAutomationId(control)==L"pax-node-"+to_hstring(id))return {host,control.as<FrameworkElement>()};
                if(auto content=control.try_as<Grid>()) for(auto text:content.Children())
                    if(Automation::AutomationProperties::GetAutomationId(text)==L"pax-node-"+to_hstring(id))return {host,text.as<FrameworkElement>()};
            }
        throw std::runtime_error("missing node");
    }
    void tap_automatic_text() {
        HWND hwnd = nullptr; check_hresult(window.as<IWindowNative>()->get_WindowHandle(&hwnd));
        record("gesture-foreground", GetForegroundWindow()==hwnd, "expected="+std::to_string(reinterpret_cast<uintptr_t>(hwnd))+" actual="+std::to_string(reinterpret_cast<uintptr_t>(GetForegroundWindow())));
        require_check(GetForegroundWindow() == hwnd, "native gesture test owns foreground");
        auto label = node(2).second.as<TextBlock>();
        auto local = label.TransformToVisual(nullptr).TransformPoint({float(label.ActualWidth()/2), float(label.ActualHeight()/2)});
        POINT origin{}; require_check(bool(ClientToScreen(hwnd, &origin)), "native gesture client origin");
        auto scale = GetDpiForWindow(hwnd)/96.0;
        auto x = origin.x + int(std::round(local.X*scale)); auto y = origin.y + int(std::round(local.Y*scale));
        record("tap-input", true, "phase="+std::to_string(phase)+" x="+std::to_string(x)+" y="+std::to_string(y)+
            " width="+std::to_string(label.ActualWidth())+" height="+std::to_string(label.ActualHeight()));
        POINT target{x,y};require_check(GetAncestor(WindowFromPoint(target),GA_ROOT)==hwnd,"native gesture point belongs to test window");
        INPUT input[3]{};
        input[0].type = INPUT_MOUSE; input[0].mi.dwFlags = MOUSEEVENTF_MOVE|MOUSEEVENTF_ABSOLUTE|MOUSEEVENTF_VIRTUALDESK;
        input[0].mi.dx = LONG(std::round((x-GetSystemMetrics(SM_XVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CXVIRTUALSCREEN)-1)));
        input[0].mi.dy = LONG(std::round((y-GetSystemMetrics(SM_YVIRTUALSCREEN))*65535.0/(GetSystemMetrics(SM_CYVIRTUALSCREEN)-1)));
        input[1].type = input[2].type = INPUT_MOUSE; input[1].mi.dwFlags = MOUSEEVENTF_LEFTDOWN; input[2].mi.dwFlags = MOUSEEVENTF_LEFTUP;
        require_check(SendInput(3,input,sizeof(INPUT)) == 3, "native gesture input delivered");
    }
    void tick() {
        try {
            scene->root().UpdateLayout();scene->apply(json::JsonArray());scene->root().UpdateLayout();
            auto [host,element]=node(1);auto label=element.as<TextBlock>();require_check(bool(label),"Text node is a native TextBlock");auto position=label.TransformToVisual(host).TransformPoint({0,0});
            if(phase==0){
                timer.Interval(std::chrono::milliseconds(100));
                extent_scroll=node(8).second.as<ScrollViewer>();
                auto pane=extent_scroll.Content().as<Canvas>();
                require_check(pane.Width()==0 && pane.Height()==0,"negative native inner extents become empty");
                record("scroller-negative-extent",true,"authored negative inner extents produce a zero-sized native pane");
                update(LR"([{"ScrollerUpdate":{"id":8,"size_inner_pane_x":0,"size_inner_pane_y":0}}])");
                zero_label=node(6).second.as<TextBlock>();
                require_check(zero_label.Visibility()==Visibility::Collapsed && zero_label.DesiredSize().Width==0 && zero_label.DesiredSize().Height==0,"zero-size glyphs have no layout extent");
                require_check(zero_label.Text()==L"Zero then visible","zero-size content retained");
                require_check(number(measurements.at(6),L"width")==0 && number(measurements.at(6),L"height")==0,"zero-size unconstrained measurement");
                require_check(number(measurements.at(7),L"width")==90 && number(measurements.at(7),L"height")==40,"zero-size explicit authored box retained");
                record("zero-font-initial",true,"no glyph extent; unconstrained axes zero; explicit90x40 box preserved");
                update(LR"([{"TextUpdate":{"id":6,"measure_generation":2,"style":{"font_size":18}}}])");
                require_check(label.ActualWidth()>30 && label.ActualWidth()<300,"intrinsic content width");
                require_check(label.ActualHeight()>20 && label.ActualHeight()<100,"intrinsic content height");
                require_check(std::abs(position.X-(300-label.ActualWidth()))<1,"right block alignment");
                require_check(std::abs(position.Y-(100-label.ActualHeight()))<1,"bottom block alignment");
                require_check(label.TextAlignment()==TextAlignment::Center,"multiline independent alignment");
                require_check(label.TextDecorations()==winrt::Windows::UI::Text::TextDecorations::Underline,"native underline");
                auto automatic=node(2).second.as<TextBlock>();require_check(automatic.ActualWidth()>30 && automatic.ActualHeight()>15,"implicit text visible");
                auto radio=node(3).second.as<RadioButtons>();require_check(radio.Items().Size()==2,"native radio items");
                for(auto item:radio.Items()){auto r=item.as<RadioButton>();auto brush=r.Foreground().try_as<Media::SolidColorBrush>();require_check(bool(brush),"radio has a solid foreground brush");auto c=brush.Color();require_check(c.R==255 && c.G==0 && c.B==0 && r.FontSize()==22,"radio item foreground/font");}
                button_label=content_label(node(4).second.as<Button>(),true,L"Styled button");
                first_radio_label=content_label(radio.Items().GetAt(0).as<RadioButton>(),true,L"First");
                auto dropdown=node(5).second.as<ComboBox>();require_check(bool(dropdown),"Dropdown is a native ComboBox");require_check(dropdown.Items().Size()==2 && dropdown.SelectedIndex()==0,"native dropdown selection");
                first_dropdown_label=dropdown_label(dropdown,0,true,L"First option");
                for(uint32_t i=0;i<dropdown.Items().Size();++i)dropdown_label(dropdown,i,true,Automation::AutomationProperties::GetName(dropdown.Items().GetAt(i).as<ComboBoxItem>()));
                record("initial-native-style",true,"fixed block, intrinsic sizing, independent line alignment, underline, RadioButton foreground");
                update(L"[{\"TextUpdate\":{\"id\":1,\"style\":{\"underline\":false,\"align_horizontal\":\"Center\",\"align_vertical\":\"Top\",\"align_multiline\":\"Right\"}}},{\"RadioListUpdate\":{\"id\":3,\"options\":[\"Replaced\",\"Second\",\"Third\"],\"style\":{\"fill\":{\"Rgb\":[0,1,0]}}}}]");
                update(LR"([{"ButtonUpdate":{"id":4,"content":"Changed button","style":{"underline":false}}},{"DropdownUpdate":{"id":5,"options":["Replaced option","Second option","New option"],"style":{"underline":false}}},{"RadioListUpdate":{"id":3,"style":{"underline":false}}}])");phase++;
            }else if(phase==1){
                auto scroll=node(8).second.as<ScrollViewer>();auto pane=scroll.Content().as<Canvas>();
                require_check(scroll==extent_scroll && pane.Width()==0 && pane.Height()==0,"zero extents retain native scroller identity");
                record("scroller-zero-extent",true,"explicit zero remains empty and preserves native identity");
                update(LR"([{"ScrollerUpdate":{"id":8,"size_x":100,"size_y":60,"size_inner_pane_x":160,"size_inner_pane_y":120}}])");
                require_check(node(6).second.as<TextBlock>()==zero_label && zero_label.Visibility()==Visibility::Visible,"positive size restores same native text");
                require_check(zero_label.FontSize()==18 && zero_label.ActualWidth()>30 && zero_label.ActualHeight()>10,"positive native glyphs restored");
                require_check(number(measurements.at(6),L"generation")==2 && number(measurements.at(6),L"width")>30 && number(measurements.at(6),L"height")>10,"restored unconstrained measurement");
                record("zero-font-restored",true,"same native text and positive intrinsic measurement");
                update(LR"([{"TextUpdate":{"id":6,"content":"Changed while zero","measure_generation":3,"style":{"font_size":0}}}])");
                require_check(std::abs(position.X-(300-label.ActualWidth())/2)<1 && std::abs(position.Y)<1,"reactive center/top block");
                require_check(label.TextAlignment()==TextAlignment::Right && label.TextDecorations()==winrt::Windows::UI::Text::TextDecorations::None,"reactive line alignment/underline clear");
                auto radio=node(3).second.as<RadioButtons>();require_check(radio.Items().Size()==3,"radio option append");for(auto item:radio.Items()){auto r=item.as<RadioButton>();auto brush=r.Foreground().try_as<Media::SolidColorBrush>();require_check(bool(brush),"radio has a solid foreground brush");auto c=brush.Color();require_check(c.G==255 && c.R==0 && r.FontSize()==22,"new/existing radio retained font and changed fill");}
                require_check(content_label(node(4).second.as<Button>(),false,L"Changed button")==button_label,"button label identity retained");
                require_check(content_label(radio.Items().GetAt(0).as<RadioButton>(),false,L"Replaced")==first_radio_label,"radio label identity retained");
                auto dropdown=node(5).second.as<ComboBox>();require_check(bool(dropdown),"Dropdown is a native ComboBox");require_check(dropdown.Items().Size()==3 && dropdown.SelectedIndex()==0,"dropdown option replacement retains selection");
                require_check(dropdown_label(dropdown,0,false,L"Replaced option")==first_dropdown_label,"dropdown label identity retained");
                for(uint32_t i=0;i<dropdown.Items().Size();++i)dropdown_label(dropdown,i,false,Automation::AutomationProperties::GetName(dropdown.Items().GetAt(i).as<ComboBoxItem>()));
                record("reactive-native-style",true,"new/existing native item styles, block/line alignment, underline clear");
                update(L"[{\"TextUpdate\":{\"id\":1,\"content\":\"A deliberately long unwrapped line extends beyond its authored box on both sides\",\"style\":{\"align_horizontal\":\"Center\"}}}]");phase++;
            }else if(phase==2){
                auto scroll=node(8).second.as<ScrollViewer>();auto pane=scroll.Content().as<Canvas>();
                require_check(scroll==extent_scroll && scroll.ViewportWidth()>0 && scroll.ViewportHeight()>0,
                    "positive viewport recovers on same native scroller");
                require_check(pane.Width()<=scroll.ViewportWidth() && pane.Height()<=scroll.ViewportHeight() &&
                    scroll.ScrollableWidth()==0 && scroll.ScrollableHeight()==0,
                    "disabled axes retain no native travel after positive extent updates");
                record("scroller-disabled-extent",true,"positive authored extents retain zero native travel while disabled");
                // Re-enable without resending the authored extents: the host
                // must retain them while its native disabled pane is clamped.
                update(LR"([{"ScrollerUpdate":{"id":8,"scroll_enabled_x":true,"scroll_enabled_y":true}}])");
                scene->root().UpdateLayout();scene->apply(json::JsonArray());scene->root().UpdateLayout();
                require_check(scroll==extent_scroll && pane.Width()==160 && pane.Height()==120,"positive inner extents recover on same native scroller");
                require_check(scroll.ScrollableWidth()>0 && scroll.ScrollableHeight()>0,"re-enabled authored extents provide native travel");
                record("scroller-positive-extent",true,"positive pane dimensions recover after negative and zero updates");
                update(LR"([{"ScrollerUpdate":{"id":8,"size_x":0,"size_y":0,"size_inner_pane_x":-28,"size_inner_pane_y":-36,"scroll_enabled_x":false,"scroll_enabled_y":false}}])");
                require_check(zero_label.Visibility()==Visibility::Collapsed && zero_label.Text()==L"Changed while zero","positive-to-zero retains updated content");
                require_check(number(measurements.at(6),L"generation")==3 && number(measurements.at(6),L"width")==0 && number(measurements.at(6),L"height")==0,"positive-to-zero clears cached measurement");
                record("zero-font-recollapsed",true,"content update retained and prior positive measurement cleared");
                update(LR"([{"TextUpdate":{"id":6,"measure_generation":4,"style":{"font_size":24}}}])");
                require_check(label.ActualWidth()>300 && position.X<0 && std::abs(position.X-(300-label.ActualWidth())/2)<1,"unclipped centered overflow");
                require_check(!host.Clip(),"unclipped Text must not add a box clip");record("unclipped-overflow",true,"centered intrinsic text extends on both sides");
                update(L"[{\"TextUpdate\":{\"id\":1,\"clip\":true}}]");phase++;
            }else if(phase==3){
                auto pane=extent_scroll.Content().as<Canvas>();
                require_check(node(8).second.as<ScrollViewer>()==extent_scroll && pane.Width()==0 && pane.Height()==0,"repeated negative extents clear prior positive pane dimensions");
                record("scroller-recollapsed-extent",true,"repeated responsive collapse preserves native identity and clears prior dimensions");
                require_check(node(6).second.as<TextBlock>()==zero_label && zero_label.Visibility()==Visibility::Visible && zero_label.FontSize()==24,"second positive-size transition retains native identity");
                require_check(zero_label.Text()==L"Changed while zero" && number(measurements.at(6),L"generation")==4 && number(measurements.at(6),L"width")>30 && number(measurements.at(6),L"height")>10,"updated zero-state content measured after restoration");
                record("zero-font-recovered",true,"updated content and generation survive repeated transitions");
                auto clip=host.Clip().try_as<Media::RectangleGeometry>();require_check(bool(clip) && clip.Rect().Width==300 && clip.Rect().Height==100,"explicit Text clip uses authored box");
                record("explicit-clip",true,"authored Text clip is separate from content alignment");
                update(LR"([{"DropdownUpdate":{"id":5,"selected_id":1}}])");phase++;
            }else if(phase==4){
                auto dropdown=node(5).second.as<ComboBox>();require_check(dropdown.SelectedIndex()==1,"programmatic selection changes");
                require_check(dropdown_label(dropdown,0,false,L"Replaced option")==first_dropdown_label,"old selected label returns to its popup item");
                dropdown_label(dropdown,1,false,L"Second option");
                require_check(dropdown_changes==0,"programmatic selection does not echo user event");
                record("selection-reparent",true,"same labels move between popup and selected faceplate");
                update(LR"([{"DropdownUpdate":{"id":5,"options":[]}}])");phase++;
            }else if(phase==5){
                auto dropdown=node(5).second.as<ComboBox>();require_check(dropdown.Items().Size()==0 && dropdown.SelectedIndex()==-1,"empty options clears native selection");
                require_check(dropdown_changes==0,"empty options does not emit invalid selection");
                update(LR"([{"DropdownUpdate":{"id":5,"options":["Refilled first","Refilled second","Refilled third"]}}])");phase++;
            }else if(phase==6){
                auto dropdown=node(5).second.as<ComboBox>();require_check(dropdown.Items().Size()==3 && dropdown.SelectedIndex()==1,"refilled options restores requested index");
                dropdown_label(dropdown,1,false,L"Refilled second");
                record("empty-refill",true,"new labels retain style and requested selection");
                update(LR"([{"DropdownUpdate":{"id":5,"options":["Only option"]}}])");phase++;
            }else if(phase==7){
                auto dropdown=node(5).second.as<ComboBox>();require_check(dropdown.Items().Size()==1 && dropdown.SelectedIndex()==-1,"selected shrink has no valid selection");
                dropdown_label(dropdown,0,false,L"Only option");
                require_check(dropdown_changes==0,"all programmatic dropdown updates suppress echo");
                record("selected-shrink",true,"removed selected label retires and no invalid user interrupt is emitted");
                if(layout_only){record("scope",true,"layout-only: native pointer gestures were not requested");finish(true);return;}
                update(LR"([{"TextUpdate":{"id":2,"selectable":true}}])");
                timer.Interval(std::chrono::milliseconds(GetDoubleClickTime()+100));phase++;
            }else if(phase==8 || phase==10 || phase==12 || phase==14){
                tap_automatic_text();phase++;
            }else{
                unsigned expected=(phase-7)/2;
                record("tap-count", clicks==expected, "phase="+std::to_string(phase)+" actual="+std::to_string(clicks)+" expected="+std::to_string(expected));
                require_check(clicks==expected,"one Click per tap across native selectable toggles");
                record("selectable-tap",true,"tap count="+std::to_string(clicks));
                if(phase==15){finish(true);return;}
                bool selectable=phase==11;
                update(selectable ? LR"([{"TextUpdate":{"id":2,"selectable":true}}])" : LR"([{"TextUpdate":{"id":2,"selectable":false}}])");phase++;
            }
        }catch(hresult_error const&e){record("failure",false,to_string(e.message()));finish(false);}catch(std::exception const&e){record("failure",false,e.what());finish(false);}
    }
    void OnLaunched(LaunchActivatedEventArgs const&) {
        try {
            Resources().MergedDictionaries().Append(XamlControlsResources());
            D3D_FEATURE_LEVEL level;
            check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0, D3D11_SDK_VERSION,
                device.put(), &level, nullptr));
            scene = std::make_unique<NativeScene>([this](auto const& value) { if(value.HasKey(L"FormDropdownChange")) ++dropdown_changes; if(value.HasKey(L"Click")) ++clicks; if(value.HasKey(L"TextMeasurementResponse")){auto response=value.GetNamedObject(L"TextMeasurementResponse");measurements.insert_or_assign(uint32_t(number(response,L"id")),response);} return false; },
                [](auto const&) -> hstring { throw std::runtime_error("No fonts in clip contract"); },
                [](auto const&) { throw std::runtime_error("No images in clip contract"); });
            window = Window(); window.Title(L"Pax native text style contract");
            window.Content(scene->root()); window.Activate();
            HWND foreground_window=nullptr;check_hresult(window.as<IWindowNative>()->get_WindowHandle(&foreground_window));SetForegroundWindow(foreground_window);
            scene->initialize(device.get()); scene->resize(400, 400, 1);
            update(L"[{\"TextCreate\":{\"id\":1,\"render_layer_id\":0}},{\"TextUpdate\":{\"id\":1,\"size_x\":300,\"size_y\":100,\"content\":\"Longer line\\nshort\",\"wrap\":false,\"style\":{\"font_size\":18,\"underline\":true,\"align_horizontal\":\"Right\",\"align_vertical\":\"Bottom\",\"align_multiline\":\"Center\"}}},{\"TextCreate\":{\"id\":2,\"render_layer_id\":0}},{\"TextUpdate\":{\"id\":2,\"size_x\":-1,\"size_y\":-1,\"content\":\"Implicit label\",\"measure_generation\":1,\"transform\":[1,0,0,1,0,110],\"style\":{\"font_size\":18}}},{\"RadioListCreate\":{\"id\":3,\"render_layer_id\":0}},{\"RadioListUpdate\":{\"id\":3,\"size_x\":300,\"size_y\":150,\"transform\":[1,0,0,1,0,160],\"options\":[\"First\",\"Second\"],\"selected_id\":0,\"style\":{\"font_size\":22,\"fill\":{\"Rgb\":[1,0,0]}}}}]");
            update(LR"([{"RadioListUpdate":{"id":3,"style":{"underline":true}}},{"ButtonCreate":{"id":4,"render_layer_id":0}},{"ButtonUpdate":{"id":4,"size_x":220,"size_y":40,"transform":[1,0,0,1,0,315],"content":"Styled button","style":{"underline":true,"font_size":18}}},{"DropdownCreate":{"id":5,"render_layer_id":0}},{"DropdownUpdate":{"id":5,"size_x":220,"size_y":40,"transform":[1,0,0,1,0,360],"options":["First option","Second option"],"selected_id":0,"style":{"underline":true,"font_size":18}}}])");
            update(LR"([{"TextCreate":{"id":6,"render_layer_id":0}},{"TextUpdate":{"id":6,"size_x":-1,"size_y":-1,"content":"Zero then visible","measure_generation":1,"transform":[1,0,0,1,240,110],"style":{"font_size":0}}},{"TextCreate":{"id":7,"render_layer_id":0}},{"TextUpdate":{"id":7,"size_x":90,"size_y":40,"content":"Fixed zero","measure_generation":1,"transform":[1,0,0,1,300,0],"style":{"font_size":0}}}])");
            update(LR"([{"ScrollerCreate":{"id":8,"render_layer_id":0}},{"ScrollerUpdate":{"id":8,"size_x":0,"size_y":0,"size_inner_pane_x":-28,"size_inner_pane_y":-36,"transform":[1,0,0,1,270,260],"scroll_enabled_x":false,"scroll_enabled_y":false}}])");
            timer = winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();
            // Give the external interactive harness time to activate the real
            // HWND before the short semantic phases reach their gesture checks.
            timer.Interval(std::chrono::milliseconds(1500));
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
    wchar_t scope[8]{};
    layout_only = GetEnvironmentVariableW(L"PAX_TEXT_STYLE_CONTRACT_LAYOUT_ONLY",scope,DWORD(std::size(scope))) == 1 && scope[0] == L'1';
    wchar_t log_path[32768];
    auto length = GetEnvironmentVariableW(L"PAX_TEXT_STYLE_CONTRACT_LOG", log_path, DWORD(std::size(log_path)));
    output.open(length && length < std::size(log_path) ? log_path : L"text-style-contract.jsonl");
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
