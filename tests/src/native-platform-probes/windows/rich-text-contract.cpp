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
#include <winrt/Microsoft.UI.Xaml.Automation.Provider.h>
#include <chrono>
#include <fstream>
#include <iterator>
#include "native_scene.h"
using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace pax::windows;
namespace {
std::ofstream output;int process_result=1;
void check(bool ok,std::string const& stage){json::JsonObject row;row.Insert(L"stage",text(to_hstring(stage)));row.Insert(L"passed",json::JsonValue::CreateBooleanValue(ok));output<<to_string(row.Stringify())<<'\n';output.flush();if(!ok)throw std::runtime_error(stage);}
struct App:ApplicationT<App,Markup::IXamlMetadataProvider>{
 XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};Window window{nullptr};std::unique_ptr<NativeScene> scene;com_ptr<ID3D11Device> device;
 winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer timer{nullptr};std::vector<hstring> navigation;
 XamlTypeInfo::XamlControlsXamlMetaDataProvider provider(){if(!metadata)metadata=XamlTypeInfo::XamlControlsXamlMetaDataProvider();return metadata;}
 Markup::IXamlType GetXamlType(hstring const& n){return provider().GetXamlType(n);}
 Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& t){return provider().GetXamlType(t);}
 com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions(){return provider().GetXmlnsDefinitions();}
 void update(json::JsonObject const& patch){json::JsonObject envelope;envelope.Insert(L"TextUpdate",patch);json::JsonArray messages;messages.Append(envelope);scene->apply(messages);scene->root().UpdateLayout();}
 TextBlock label(){for(auto child:scene->root().Children())if(auto host=child.try_as<Grid>())for(auto control:host.Children())if(auto content=control.try_as<Grid>())for(auto element:content.Children())if(auto result=element.try_as<TextBlock>())return result;throw std::runtime_error("TextBlock missing");}
 Automation::Provider::IInvokeProvider link_provider(Documents::Hyperlink const& link){
  hstring name;for(auto const& child:link.Inlines())name=name+child.as<Documents::Run>().Text();
  Automation::Peers::TextBlockAutomationPeer peer(label());
  for(auto const& child:peer.GetChildren())if(child.GetName()==name&&child.GetAutomationControlType()==Automation::Peers::AutomationControlType::Hyperlink)
   return child.GetPattern(Automation::Peers::PatternInterface::Invoke).as<Automation::Provider::IInvokeProvider>();
  throw std::runtime_error("Native hyperlink automation peer missing");
 }
 void finish(bool ok){if(timer)timer.Stop();if(scene){scene->close();scene.reset();}if(window)window.Close();process_result=ok?0:1;Exit();}
 void run(){try{
  timer.Stop();wchar_t path[32768];auto n=GetEnvironmentVariableW(L"PAX_RICH_TEXT_FIXTURES",path,DWORD(std::size(path)));check(n&&n<std::size(path),"shared-fixture-path-present");
  std::ifstream input(path);std::string bytes((std::istreambuf_iterator<char>(input)),{});auto fixtures=json::JsonArray::Parse(to_hstring(bytes));
  auto original=label();unsigned cases=0,styled_runs=0;Automation::Provider::IInvokeProvider retained{nullptr};
  for(auto const& fixture:fixtures){auto item=fixture.GetObject();if(!pax::windows::boolean(item,L"markdown"))continue;
   auto name=to_string(item.GetNamedString(L"name"));auto document=item.GetNamedObject(L"rich_text");
   json::JsonObject patch;patch.Insert(L"id",numeric(1));patch.Insert(L"content",item.GetNamedValue(L"content"));patch.Insert(L"markdown",json::JsonValue::CreateBooleanValue(true));patch.Insert(L"rich_text",document);update(patch);
   auto native=label();check(native==original,name+": native-control-identity-retained");auto expected=document.GetNamedArray(L"runs");
   std::vector<Documents::Run> flat;unsigned observed_links=0,expected_links=0;hstring prior_link;double prior_link_id=0;
   for(auto const& inline_element:native.Inlines()){
    if(auto plain=inline_element.try_as<Documents::Run>())flat.push_back(plain);
    else{auto link=inline_element.as<Documents::Hyperlink>();++observed_links;retained=link_provider(link);
     auto spec=expected.GetObjectAt(uint32_t(flat.size()));check(present(spec,L"link"),name+": genuine-native-hyperlink");auto before=navigation.size();retained.Invoke();check(navigation.size()==before+1&&navigation.back()==spec.GetNamedString(L"link"),name+": hyperlink-routes-through-host");
     for(auto const& child:link.Inlines())flat.push_back(child.as<Documents::Run>());
    }
   }
   check(flat.size()==expected.Size(),name+": run-count");hstring joined;
   for(uint32_t i=0;i<expected.Size();++i){auto spec=expected.GetObjectAt(i);auto run=flat[i];
    if(present(spec,L"link")){auto target=spec.GetNamedString(L"link");auto link_id=number(spec,L"link_id",0);if(prior_link!=target||prior_link_id!=link_id)++expected_links;prior_link=target;prior_link_id=link_id;}else prior_link={};
    joined=joined+run.Text();
    if(pax::windows::boolean(spec,L"bold"))check(run.FontWeight().Weight==700,name+": bold");
    if(pax::windows::boolean(spec,L"italic"))check(run.FontStyle()==winrt::Windows::UI::Text::FontStyle::Italic,name+": italic");
    if(pax::windows::boolean(spec,L"underline"))check((run.TextDecorations()&winrt::Windows::UI::Text::TextDecorations::Underline)!=winrt::Windows::UI::Text::TextDecorations::None,name+": underline");
    if(pax::windows::boolean(spec,L"strikethrough"))check((run.TextDecorations()&winrt::Windows::UI::Text::TextDecorations::Strikethrough)!=winrt::Windows::UI::Text::TextDecorations::None,name+": strikethrough");
    if(present(spec,L"font_family"))check(run.FontFamily().Source()==L"Consolas",name+": native-monospace-family");
    if(present(spec,L"size_scale"))check(std::abs(run.FontSize()-native.FontSize()*number(spec,L"size_scale"))<0.001,name+": relative-font-size");
    if(present(spec,L"color")){auto color=run.Foreground().as<Media::SolidColorBrush>().Color();auto rgba=spec.GetNamedArray(L"color");check(color.R==uint8_t(std::round(rgba.GetNumberAt(0)*255))&&color.G==uint8_t(std::round(rgba.GetNumberAt(1)*255))&&color.B==uint8_t(std::round(rgba.GetNumberAt(2)*255))&&color.A==uint8_t(std::round(rgba.GetNumberAt(3)*255)),name+": exact-native-range-color");}
    ++styled_runs;
   }
   check(observed_links==expected_links,name+": formatting-spans-share-one-native-link");
   check(joined==document.GetNamedString(L"text"),name+": exact-unicode-and-newlines");check(native.IsTextSelectionEnabled(),name+": selection-remains-enabled");
   check(native.LineHeight()==number(document,L"line_height",0),name+": line-height");
   auto count=native.Inlines().Size();auto first_inline=native.Inlines().GetAt(0);
   native.Select(native.ContentStart(),native.ContentEnd());auto selected=native.SelectedText();
   check(selected.empty()==document.GetNamedString(L"text").empty(),name+": real-selection-matches-empty-or-nonempty-document");
   update(json::JsonObject::Parse(LR"({"id":1,"style":{"font_size":32}})"));
   check(label()==original&&label().Inlines().GetAt(0)==first_inline&&label().FontSize()==32,name+": style-only-retains-native-runs");
   check(label().SelectedText()==selected,name+": actual-selection-retained-across-style");
   for(auto const& element:native.Inlines())if(auto link=element.try_as<Documents::Hyperlink>()){
    check(link.Focus(FocusState::Programmatic),name+": native-link-focused");
    update(json::JsonObject::Parse(LR"({"id":1,"style_link":{"fill":{"Rgba":[0.2,0.4,0.8,1]}}})"));
    check(Input::FocusManager::GetFocusedElement(native.XamlRoot())==link,name+": link-focus-retained-across-style");break;
   }
   update(json::JsonObject::Parse(LR"({"id":1,"style":{"font_size":0}})"));check(label().Visibility()==Visibility::Collapsed,name+": zero-font-collapses-only-glyphs");
   update(json::JsonObject::Parse(LR"({"id":1,"style":{"font_size":24}})"));check(label().Visibility()==Visibility::Visible&&label().Inlines().Size()==count,name+": positive-font-restores-runs");++cases;
  }
  update(json::JsonObject::Parse(LR"({"id":1,"content":"Plain after rich","markdown":false,"rich_text":null})"));check(label()==original&&label().Text()==L"Plain after rich"&&label().LineHeight()==0,"plain-mode-clears-rich-document");
  if(retained){auto before=navigation.size();try{retained.Invoke();}catch(hresult_error const&){}check(navigation.size()==before,"retired-hyperlink-cannot-navigate");}
  bool rejected=false;try{update(json::JsonObject::Parse(LR"({"id":1,"content":"**missing normalized runs**","markdown":true})"));}catch(std::invalid_argument const&){rejected=true;}catch(std::runtime_error const&){rejected=true;}
  check(rejected,"missing-normalization-is-explicit-error");check(cases>=5&&styled_runs>=10,"shared-fixtures-covered");check(true,"complete");finish(true);
 }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}catch(std::exception const& e){output<<e.what()<<'\n';finish(false);}}
 void OnLaunched(LaunchActivatedEventArgs const&){try{
  Resources().MergedDictionaries().Append(XamlControlsResources());D3D_FEATURE_LEVEL level;check_hresult(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_HARDWARE,nullptr,D3D11_CREATE_DEVICE_BGRA_SUPPORT,nullptr,0,D3D11_SDK_VERSION,device.put(),&level,nullptr));
  scene=std::make_unique<NativeScene>([](auto const&){return false;},[](auto const&){return hstring(L"Segoe UI");},[](auto const&){});scene->set_navigation([this](auto const& destination){navigation.push_back(destination);});
  window=Window();window.Title(L"Pax native rich text contract");window.Content(scene->root());window.Activate();scene->initialize(device.get());scene->resize(800,500,1);
  scene->apply(json::JsonArray::Parse(LR"([{"TextCreate":{"id":1,"render_layer_id":0}},{"TextUpdate":{"id":1,"size_x":700,"size_y":400,"selectable":true,"wrap":false,"content":"Initial","style":{"font_size":24,"fill":{"Rgba":[0,0,0,1]}}}}])"));
  timer=winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();timer.Interval(std::chrono::milliseconds(300));timer.Tick([this](auto const&,auto const&){run();});timer.Start();
 }catch(hresult_error const& e){output<<to_string(e.message())<<'\n';finish(false);}catch(std::exception const& e){output<<e.what()<<'\n';finish(false);}}
};
}
int WINAPI wWinMain(HINSTANCE,HINSTANCE,PWSTR,int){wchar_t path[32768];auto n=GetEnvironmentVariableW(L"PAX_RICH_TEXT_LOG",path,DWORD(std::size(path)));output.open(n&&n<std::size(path)?path:L"rich-text-contract.jsonl");if(!output)return 2;try{SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);PACKAGE_VERSION v{};v.Version=WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,WINDOWSAPPSDK_RELEASE_VERSION_TAG_W,v,MddBootstrapInitializeOptions_None));init_apartment(apartment_type::single_threaded);Application::Start([](auto const&){make<App>();});return process_result;}catch(hresult_error const& e){output<<to_string(e.message())<<'\n';}return 1;}
