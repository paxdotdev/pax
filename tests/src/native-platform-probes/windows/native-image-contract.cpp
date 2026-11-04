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
#include <array>
#include <algorithm>
#include <chrono>
#include <fstream>
#include <memory>
#include "assets.h"
#include "native_scene.h"

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace pax::windows;
namespace {
std::ofstream output;
int contract_exit_code = 1;
std::filesystem::path root, out;
std::wstring environment(wchar_t const* name) {
    wchar_t value[32768];
    auto size = GetEnvironmentVariableW(name, value, DWORD(std::size(value)));
    if (!size || size >= std::size(value)) throw std::runtime_error("Missing native image contract path");
    return value;
}
void record(char const* stage, bool passed, std::string const& detail) {
    json::JsonObject row;
    row.Insert(L"stage", text(to_hstring(stage)));
    row.Insert(L"passed", json::JsonValue::CreateBooleanValue(passed));
    row.Insert(L"detail", text(to_hstring(detail)));
    output << to_string(row.Stringify()) << '\n'; output.flush();
}
void check(bool value, char const* stage) {
    if (!value) throw std::runtime_error(stage);
    record(stage, true, "actual native object / decoder / pixel contract");
}
struct App : ApplicationT<App, Markup::IXamlMetadataProvider> {
    XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};
    Window window{nullptr};
    winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer timer{nullptr};
    std::unique_ptr<NativeScene> scene;
    Assets assets;
    com_ptr<ID3D11Device> device;
    unsigned ticks = 0, phase = 0, snapshot_failed_tick = 0;
    double large_svg_initial_width = 0, large_svg_initial_height = 0;
    bool svg_open = false, snapshot_done = false, snapshot_started = false;
    std::string snapshot_error;
    NativeImages resources;
    std::vector<std::shared_ptr<NativeImageResource>> burst;
    std::shared_ptr<NativeImageResource> bad;
    Image retained{nullptr};
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider() {
        if (!metadata) metadata = XamlTypeInfo::XamlControlsXamlMetaDataProvider();
        return metadata;
    }
    Markup::IXamlType GetXamlType(hstring const& name) { return provider().GetXamlType(name); }
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& type) { return provider().GetXamlType(type); }
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions() { return provider().GetXmlnsDefinitions(); }
    void finish(bool passed) {
        if (timer) timer.Stop();
        resources.close(); burst.clear(); bad.reset();
        if (scene) { scene->close(); scene.reset(); }
        if (window) window.Close();
        contract_exit_code = passed ? 0 : 1;
        record("complete", passed, "native resources retired; Application exit requested");
        Exit();
    }
    void update(wchar_t const* patch) { scene->apply(json::JsonArray::Parse(patch)); scene->root().UpdateLayout(); }
    Grid host(uint32_t id) {
        for (auto child : scene->root().Children()) if (auto grid = child.try_as<Grid>())
            for (auto content : grid.Children())
                if (Automation::AutomationProperties::GetAutomationId(content) == L"pax-node-" + to_hstring(id)) return grid;
        throw std::runtime_error("NativeImage node absent");
    }
    Image image(uint32_t id) { return host(id).Children().GetAt(0).as<Canvas>().Children().GetAt(0).as<Image>(); }
    Media::Matrix fit(uint32_t id) { return image(id).RenderTransform().as<Media::MatrixTransform>().Matrix(); }
    fire_and_forget snapshot() {
        try {
            Media::Imaging::RenderTargetBitmap bitmap;
            co_await bitmap.RenderAsync(scene->root());
            auto buffer = co_await bitmap.GetPixelsAsync();
            std::vector<uint8_t> pixels(buffer.Length());
            auto reader = Windows::Storage::Streams::DataReader::FromBuffer(buffer);
            reader.ReadBytes(pixels);
            auto folder = co_await Windows::Storage::StorageFolder::GetFolderFromPathAsync(hstring(out.native()));
            auto file = co_await folder.CreateFileAsync(L"native-image.png", Windows::Storage::CreationCollisionOption::ReplaceExisting);
            auto stream = co_await file.OpenAsync(Windows::Storage::FileAccessMode::ReadWrite);
            auto encoder = co_await Windows::Graphics::Imaging::BitmapEncoder::CreateAsync(
                Windows::Graphics::Imaging::BitmapEncoder::PngEncoderId(), stream);
            encoder.SetPixelData(Windows::Graphics::Imaging::BitmapPixelFormat::Bgra8,
                Windows::Graphics::Imaging::BitmapAlphaMode::Premultiplied,
                bitmap.PixelWidth(), bitmap.PixelHeight(), 96, 96, pixels);
            co_await encoder.FlushAsync(); stream.Close();
            auto pixel = [&](uint32_t x, uint32_t y) {
                auto at = (size_t(y) * bitmap.PixelWidth() + x) * 4;
                if (at + 4 > pixels.size()) throw std::runtime_error("Captured image too small");
                return std::array<uint8_t,4>{pixels[at],pixels[at+1],pixels[at+2],pixels[at+3]};
            };
            auto channel_near = [](int a, int b) { return std::abs(a-b) <= 5; };
            auto p = pixel(40,30);
            check(channel_near(p[0],255) && channel_near(p[1],255) && channel_near(p[2],255), "contain-retains-letterbox-pixels");
            p = pixel(40,70); check(p[2]>245 && p[0]<8 && p[1]<8, "native-png-paints-red-half");
            p = pixel(160,30); check(p[2]>245 && p[0]<8, "cover-fills-authored-box");
            p = pixel(220,30); check(p[0]>245 && p[2]<8, "cover-centers-crop-with-both-source-halves");
            auto left = pixel(265,70), right = pixel(355,70);
            check(left[1]>245 && left[0]<45 && right[0]>220 && right[1]>245,
                "native-svg-gradient-alpha-composites-over-background");
            p=pixel(265,30);check(p[1]>245 && p[0]<45,"svg-fill-reaches-top-edge");
            p=pixel(265,110);check(p[1]>245 && p[0]<45,"svg-fill-reaches-bottom-edge");
            p=pixel(40,170);check(p[2]>245 && p[0]<8,"large-cover-downscale-retains-left-source");
            p=pixel(100,170);check(p[0]>245 && p[2]<8,"large-cover-downscale-retains-right-source");
            p=pixel(160,170);check(channel_near(p[0],255) && channel_near(p[1],255) && channel_near(p[2],255),"large-contain-downscale-retains-letterbox");
            p=pixel(160,210);check(p[2]>245 && p[0]<8,"large-contain-downscale-retains-left-source");
            p=pixel(220,210);check(p[0]>245 && p[2]<8,"large-contain-downscale-retains-right-source");
            p=pixel(270,170);check(p[1]>245 && p[0]<70 && p[2]<70,"shared-svg-tall-fill-reaches-top");
            p=pixel(270,270);check(p[1]>245 && p[0]<70 && p[2]<70,"shared-svg-tall-fill-reaches-bottom");
            p=pixel(370,170);check(p[2]>245 && p[1]<8 && p[0]<8,"large-authored-svg-renders-first-half");
            p=pixel(400,170);check(p[0]>245 && p[1]<8 && p[2]<8,"large-authored-svg-renders-second-half");
        } catch (hresult_error const& error) { snapshot_error = to_string(error.message()); }
          catch (std::exception const& error) { snapshot_error = error.what(); }
        snapshot_done = true;
    }
    void tick() {
        try {
            if (++ticks > 300) throw std::runtime_error("Native image contract timed out");
            scene->poll_native_images();
            if (phase == 0) {
                resources.poll();
                check(resources.pending_count()<=8,"decode-concurrency-bounded");
                if (!svg_open || image(1).Source().as<Media::Imaging::BitmapImage>().PixelWidth()==0 || image(4).Source().as<Media::Imaging::BitmapImage>().PixelWidth()==0 || image(1).Opacity()==0 || image(2).Opacity()==0 || image(3).Opacity()==0 || image(4).Opacity()==0 || image(5).Opacity()==0 || image(6).Opacity()==0 || image(7).Opacity()==0) return;
                if (!std::all_of(burst.begin(),burst.end(),[](auto const& source){return source->ready;})) return;
                record("large-authored-svg-initial-layout",true,"initial="+std::to_string(large_svg_initial_width)+"x"+std::to_string(large_svg_initial_height));
                check(large_svg_initial_width>0 && large_svg_initial_height>0,
                    "large-authored-svg-first-layout-observed");
                auto large_svg=image(7).Source().as<Media::Imaging::SvgImageSource>();
                check(large_svg.RasterizePixelWidth()*large_svg.RasterizePixelHeight()*4<=32*1024*1024 &&
                    large_svg.RasterizePixelWidth()<=8192 && large_svg.RasterizePixelHeight()<=8192,
                    "final-svg-raster-settings-respect-logical-bounds");
                check(image(3).Source()==image(6).Source(),"differently-sized-svg-consumers-share-source");
                check(std::isfinite(fit(6).M11) && fit(6).M11==0 && fit(6).M22==0,"zero-size-before-decode-keeps-finite-fit");
                update(LR"([{"NativeImageUpdate":{"id":6,"size_x":80,"size_y":120}}])");
                check(image(1).Source() == image(2).Source(), "mounted-identical-raster-consumers-share-source");
                check(image(1).Source().as<Media::Imaging::BitmapImage>().PixelWidth()==64,
                    "raster-native-decoder-retains-intrinsic-size");
                check(!image(1).Source().as<Media::Imaging::BitmapImage>().AutoPlay(),"native-images-do-not-autoplay-multiframe-assets");
                check(fit(1).M11==fit(1).M22 && fit(1).OffsetY==25 &&
                    fit(2).M11==fit(2).M22 && fit(2).OffsetX==-50 && fit(3).M11!=fit(3).M22,
                    "contain-cover-fill-use-centered-native-transforms");
                check(bool(host(2).Clip()), "cover-is-clipped-to-authored-bounds");
                for(uint32_t id=1;id<=3;++id) {
                    auto content=host(id).Children().GetAt(0).as<Canvas>();
                    check(content.ActualWidth()==100 && content.ActualHeight()==100,"mask-capture-retains-full-authored-box");
                }
                check(!host(1).IsHitTestVisible() && !host(2).IsHitTestVisible() && !host(3).IsHitTestVisible(),
                    "native-images-defer-input-to-engine-and-scrollers");
                check(burst.size()==10 && resources.source_count()==10, "ten-distinct-native-assets-drain-queued-burst");
                auto weak = std::weak_ptr<NativeImageResource>(burst.front());
                burst.clear();resources.poll();
                check(weak.expired() && resources.source_count()==0,"last-consumer-releases-decoded-source");
                phase=1;return;
            }
            if (phase == 1) {
                if (!snapshot_started) {
                    // ImageOpened invalidates native measure/arrange. Sample a
                    // subsequent layout pass rather than its notification tick.
                    scene->root().UpdateLayout();
                    for (uint32_t id=1;id<=3;++id) {
                        auto current=image(id);auto desired=current.DesiredSize();
                        record("settled-image-layout",true,"id="+std::to_string(id)+
                            " authored="+std::to_string(current.Width())+"x"+std::to_string(current.Height())+
                            " actual="+std::to_string(current.ActualWidth())+"x"+std::to_string(current.ActualHeight())+
                            " desired="+std::to_string(desired.Width)+"x"+std::to_string(desired.Height));
                    }
                    auto source=image(3).Source().as<Media::Imaging::SvgImageSource>();
                    auto matrix=fit(3);
                    record("native-svg-raster-and-fit",true,"raster="+std::to_string(source.RasterizePixelWidth())+"x"+std::to_string(source.RasterizePixelHeight())+" scale="+std::to_string(matrix.M11)+","+std::to_string(matrix.M22)+" offset="+std::to_string(matrix.OffsetX)+","+std::to_string(matrix.OffsetY));
                    snapshot_started=true;snapshot();return;
                }
                if (!snapshot_done) return;
                if (!snapshot_error.empty()) {
                    if (!snapshot_failed_tick) {
                        snapshot_failed_tick=ticks;record("snapshot-failed",false,snapshot_error);
                    }
                    // Leave a short bounded window for the external runner to
                    // capture actual presentation when pixel readback disagrees.
                    if (ticks-snapshot_failed_tick<50) return;
                    throw std::runtime_error(snapshot_error);
                }
                update(LR"([{"NativeImageUpdate":{"id":1,"url":"assets/fade.svg","size_x":120,"size_y":80,"opacity":0.5,"fit":"fill","transform":[1,0,0,1,30,160]}},{"NativeMaskUpdate":{"id":1,"entries":[{"path":"M0 0 L20 0 L20 100 L0 100 Z","opacity":1,"clips":[]}]}}])");
                phase=2;return;
            }
            if (phase == 2) {
                if(image(1).Opacity()==0)return;
                check(image(1).Source()==image(3).Source(),"source-change-reuses-existing-svg");
                check(std::abs(fit(1).M11*image(1).Width()-120)<0.01 &&
                    std::abs(fit(1).M22*image(1).Height()-80)<0.01,"source-change-remeasures-before-refitting");
                auto resized=host(1).Children().GetAt(0).as<Canvas>();
                check(resized.ActualWidth()==120 && resized.ActualHeight()==80,
                    "source-change-and-resize-preserve-authored-mask-space");
                check(host(1).Opacity()==0.5 && host(1).RenderTransform().as<Media::MatrixTransform>().Matrix().OffsetY==160,
                    "image-uses-shared-opacity-and-placement");
                check(bool(xaml::Hosting::ElementCompositionPreview::GetElementVisual(host(1).Children().GetAt(0)).Clip()),
                    "native-image-participates-in-occlusion-mask");
                // Delete an image immediately after submitting another source.
                // A retained UIA object must remain blank after late completion.
                update(LR"([{"NativeImageUpdate":{"id":1,"url":"assets/burst-0.png"}}])");
                retained=image(1);scene->poll_native_images();
                update(LR"([{"NativeImageDelete":1},{"NativeImageCreate":{"id":1,"render_layer_id":0}},{"NativeImageUpdate":{"id":1,"size_x":100,"size_y":100,"url":""}}])");
                check(!retained.Source() && !image(1).Source(),"delete-recreate-retires-old-image-source");
                auto abandoned=resources.acquire(assets.native_image(L"assets/burst-0.png"));
                resources.poll();std::weak_ptr<NativeImageResource> weak=abandoned;abandoned.reset();resources.poll();
                check(weak.expired(),"active-decode-does-not-retain-retired-resource");
                bad=resources.acquire(assets.native_image(L"assets/corrupt.png"));
                phase=3;return;
            }
            if (phase == 3) {
                bool failed=false;
                try {resources.poll();} catch(std::runtime_error const&){failed=true;}
                if(!failed)return;
                check(!bad->error.empty(),"invalid-live-image-reports-asynchronous-decode-error");
                check(!retained.Source() && !image(1).Source(),"late-completion-cannot-resurrect-deleted-image");
                resources.close();resources.close();
                NativeImages closing;
                std::vector<std::shared_ptr<NativeImageResource>> closing_sources;
                for(unsigned i=0;i<10;++i)closing_sources.push_back(closing.acquire(assets.native_image(L"assets/burst-"+to_hstring(i)+L".png")));
                closing.poll();check(closing.pending_count()==8,"close-fixture-has-active-and-queued-native-loads");
                closing.close();closing.close();closing_sources.clear();
                check(closing.source_count()==0 && closing.pending_count()==0,"close-cancels-and-retires-native-image-work");
                finish(true);
            }
        } catch(hresult_error const& error){record("failure",false,to_string(error.message()));finish(false);}
          catch(std::exception const& error){record("failure",false,error.what());finish(false);}
    }
    void OnLaunched(LaunchActivatedEventArgs const&) {
        try {
            Resources().MergedDictionaries().Append(XamlControlsResources());
            json::JsonArray roots;roots.Append(text(hstring(root.native())));
            json::JsonObject config;config.Insert(L"assets",roots);assets.configure(config);
            check(assets.native_image(L"/assets/halves.png")==assets.native_image(L"assets/halves.png"),"asset-root-url-alias-resolves-locally");
            for(auto path:{L"../outside.png",L"https://example.com/a.png",L"C:\\Windows\\win.ini",L"assets/halves.png:secret",L"assets/missing.png"}) {
                bool rejected=false;try{assets.native_image(path);}catch(std::exception const&){rejected=true;}
                check(rejected,"invalid-or-missing-asset-path-rejected");
            }
            D3D_FEATURE_LEVEL level;
            check_hresult(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_HARDWARE,nullptr,D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                nullptr,0,D3D11_SDK_VERSION,device.put(),&level,nullptr));
            scene=std::make_unique<NativeScene>([](auto const&){return false;},[](auto const&){return hstring(L"Segoe UI");},
                [](auto const&){throw std::runtime_error("No canvas images in native image contract");});
            scene->set_native_image_path([this](auto const& path){return assets.native_image(path);});
            scene->root().Background(Media::SolidColorBrush(winrt::Microsoft::UI::Colors::White()));
            window=Window();window.Title(L"Pax NativeImage contract");window.Content(scene->root());window.Activate();
            scene->initialize(device.get());scene->resize(420,320,1);
            update(LR"([{"NativeImageCreate":{"id":1,"render_layer_id":0}},{"NativeImageCreate":{"id":2,"render_layer_id":0}},{"NativeImageCreate":{"id":3,"render_layer_id":0}},{"NativeImageUpdate":{"id":1,"size_x":100,"size_y":100,"url":"assets/halves.png","fit":"contain","transform":[1,0,0,1,20,20]}},{"NativeImageUpdate":{"id":2,"size_x":100,"size_y":100,"url":"/assets/halves.png","fit":"cover","transform":[1,0,0,1,140,20]}},{"NativeImageUpdate":{"id":3,"size_x":100,"size_y":100,"url":"/assets/fade.svg","fit":"fill","transform":[1,0,0,1,260,20]}}])");
            update(LR"([{"NativeImageCreate":{"id":4,"render_layer_id":0}},{"NativeImageCreate":{"id":5,"render_layer_id":0}},{"NativeImageUpdate":{"id":4,"size_x":100,"size_y":100,"url":"assets/large-halves.png","fit":"cover","transform":[1,0,0,1,20,160]}},{"NativeImageUpdate":{"id":5,"size_x":100,"size_y":100,"url":"assets/large-halves.png","fit":"contain","transform":[1,0,0,1,140,160]}}])");
            update(LR"([{"NativeImageCreate":{"id":6,"render_layer_id":0}},{"NativeImageUpdate":{"id":6,"size_x":80,"size_y":120,"url":"assets/fade.svg","fit":"fill","transform":[1,0,0,1,260,160]}},{"NativeImageUpdate":{"id":6,"size_x":0,"size_y":0}}])");
            update(LR"([{"NativeImageCreate":{"id":7,"render_layer_id":0}},{"NativeImageUpdate":{"id":7,"size_x":60,"size_y":60,"url":"assets/large-authored.svg","fit":"contain","transform":[1,0,0,1,350,160]}}])");
            image(7).Source().as<Media::Imaging::SvgImageSource>().Opened([this](auto const& source,auto const&){
                if(!std::isfinite(source.RasterizePixelWidth())) {
                    image(7).UpdateLayout();
                    large_svg_initial_width=image(7).ActualWidth();
                    large_svg_initial_height=image(7).ActualHeight();
                }
            });
            image(3).ImageOpened([this](auto const&,auto const&){svg_open=true;});
            for(unsigned i=0;i<10;++i)burst.push_back(resources.acquire(assets.native_image(L"assets/burst-"+to_hstring(i)+L".png")));
            timer=winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();
            timer.Interval(std::chrono::milliseconds(100));timer.Tick([this](auto const&,auto const&){tick();});timer.Start();
        } catch(hresult_error const& error){record("startup",false,to_string(error.message()));finish(false);}
          catch(std::exception const& error){record("startup",false,error.what());finish(false);}
    }
};
}
int WINAPI wWinMain(HINSTANCE,HINSTANCE,PWSTR,int) {
    try {
        root=environment(L"PAX_NATIVE_IMAGE_CONTRACT_ROOT");out=environment(L"PAX_NATIVE_IMAGE_CONTRACT_OUTPUT");
        output.open(out/L"result.jsonl");if(!output)return 2;
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        PACKAGE_VERSION minimum{};minimum.Version=WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;
        check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,WINDOWSAPPSDK_RELEASE_VERSION_TAG_W,minimum,MddBootstrapInitializeOptions_None));
        init_apartment(apartment_type::single_threaded);
        Application::Start([](auto const&){make<App>();});
        record("application-returned",contract_exit_code==0,"require process exit code zero as well");return contract_exit_code;
    } catch(hresult_error const& error){record("fatal",false,to_string(error.message()));}
      catch(std::exception const& error){record("fatal",false,error.what());}
    return 1;
}
