#include <windows.h>
#undef GetCurrentTime
#include <d3d11.h>
#include <dxgi1_2.h>
#include <d2d1_1.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.Foundation.Numerics.h>
#include <winrt/Windows.UI.Xaml.Interop.h>
#include <winrt/Microsoft.Graphics.DirectX.h>
#include <winrt/Microsoft.UI.h>
#include <winrt/Microsoft.UI.Composition.h>
#include <winrt/Microsoft.UI.Composition.Interop.h>
#include <winrt/Microsoft.UI.Dispatching.h>
#include <winrt/Microsoft.UI.Windowing.h>
#include <winrt/Microsoft.UI.Xaml.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Controls.Primitives.h>
#include <winrt/Microsoft.UI.Xaml.Hosting.h>
#include <winrt/Microsoft.UI.Xaml.Markup.h>
#include <winrt/Microsoft.UI.Xaml.Media.h>
#include <winrt/Microsoft.UI.Xaml.Shapes.h>
#include <winrt/Microsoft.UI.Xaml.Input.h>
#include <winrt/Microsoft.UI.Xaml.XamlTypeInfo.h>
#include <winrt/Microsoft.UI.Xaml.Automation.h>
#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <fstream>
#include <memory>
#include <string>
#include "dx12_producer.h"

using namespace winrt;
using namespace Windows::Foundation;
using namespace Windows::Foundation::Numerics;
using namespace Microsoft::Graphics::DirectX;
using namespace Microsoft::UI;
using namespace Microsoft::UI::Composition;
using namespace Microsoft::UI::Dispatching;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Controls;
using namespace Microsoft::UI::Xaml::Hosting;
using namespace Microsoft::UI::Xaml::Media;
namespace composition_interop = winrt::Microsoft::UI::Composition;

namespace {
constexpr float SceneWidth = 560;
constexpr float SceneHeight = 160;
constexpr float TileWidth = SceneWidth / 2;
constexpr float EditorWidth = 420;
constexpr float EditorHeight = 52;
constexpr float TransparentMaskWidth = 48;
bool useDx12 = false;
bool usePax = false;
bool allowWarp = false;

void log(std::string const& message) {
    std::ofstream stream("composition-probe.log", std::ios::app);
    stream << GetTickCount64() << " " << message << "\n";
    OutputDebugStringA((message + "\n").c_str());
}

TextBlock label(hstring const& text, double size = 16) {
    TextBlock result;
    result.Text(text);
    result.FontSize(size);
    result.TextWrapping(TextWrapping::Wrap);
    result.Foreground(SolidColorBrush(Colors::White()));
    return result;
}

struct DrawingTile {
    CompositionDrawingSurface surface{nullptr};
    SpriteVisual visual{nullptr};
    com_ptr<composition_interop::ICompositionDrawingSurfaceInterop> interop;
};

// Qualifies compositor-owned surfaces with either Direct2D or a DX12 producer.
// No swap chain or GPU readback participates in either path.
class DrawingSurfaces {
    com_ptr<ID3D11Device> device;
    com_ptr<ID3D11DeviceContext> immediate;
    com_ptr<ID2D1Factory1> factory;
    com_ptr<ID2D1Device> d2d;
    CompositionGraphicsDevice graphics{nullptr};
    std::unique_ptr<Dx12Producer> dx12;
public:
    void initialize(Compositor const& compositor) {
        auto interop = compositor.as<composition_interop::ICompositorInterop>();
        if (useDx12) {
            dx12 = std::make_unique<Dx12Producer>();
            dx12->initialize(device, immediate, allowWarp, usePax,
                static_cast<UINT>(SceneWidth), static_cast<UINT>(SceneHeight),
                static_cast<UINT>(EditorWidth), static_cast<UINT>(EditorHeight),
                static_cast<UINT>(TransparentMaskWidth), log);
            check_hresult(interop->CreateGraphicsDevice(device.get(), &graphics));
            log("CompositionGraphicsDevice accepted D3D11On12 device");
            return;
        }
        D3D_FEATURE_LEVEL level;
        HRESULT status = D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0, D3D11_SDK_VERSION,
            device.put(), &level, immediate.put());
        if (FAILED(status)) {
            log("D3D11 hardware creation failed HRESULT=" + std::to_string(static_cast<unsigned long>(status)));
            if (!allowWarp) check_hresult(status);
            log("D3D11 explicitly using WARP; this does not qualify hardware performance");
            check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_WARP, nullptr,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0, D3D11_SDK_VERSION,
                device.put(), &level, immediate.put()));
        } else {
            log("D3D11 hardware device created");
        }
        auto dxgi = device.as<IDXGIDevice>();
        com_ptr<IDXGIAdapter> adapter;
        check_hresult(dxgi->GetAdapter(adapter.put()));
        DXGI_ADAPTER_DESC description{};
        check_hresult(adapter->GetDesc(&description));
        log("adapter=" + to_string(hstring(description.Description)) + " feature_level=" + std::to_string(level));
        D2D1_FACTORY_OPTIONS options{};
        check_hresult(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED,
            __uuidof(ID2D1Factory1), &options, factory.put_void()));
        check_hresult(factory->CreateDevice(dxgi.get(), d2d.put()));
        check_hresult(interop->CreateGraphicsDevice(d2d.get(), &graphics));
    }

    DrawingTile create(Compositor const& compositor, float2 size) {
        DrawingTile tile;
        tile.surface = graphics.CreateDrawingSurface({size.x, size.y},
            DirectXPixelFormat::B8G8R8A8UIntNormalized, DirectXAlphaMode::Premultiplied);
        tile.interop = tile.surface.as<composition_interop::ICompositionDrawingSurfaceInterop>();
        tile.visual = compositor.CreateSpriteVisual();
        tile.visual.Size(size);
        tile.visual.Brush(compositor.CreateSurfaceBrush(tile.surface));
        return tile;
    }

    void begin_frame(float phase) { if (dx12) dx12->render(phase); }
    void end_frame() { if (dx12) dx12->finish_frame(); }

    void draw(DrawingTile const& tile, float phase, float xOffset, bool mask) {
        if (dx12) {
            com_ptr<ID3D11Texture2D> target;
            POINT origin{};
            check_hresult(tile.interop->BeginDraw(nullptr, __uuidof(ID3D11Texture2D), target.put_void(), &origin));
            try {
                dx12->copy_to(target.get(), origin, static_cast<UINT>(xOffset), mask);
            } catch (...) {
                tile.interop->EndDraw();
                throw;
            }
            check_hresult(tile.interop->EndDraw());
            return;
        }
        com_ptr<ID2D1DeviceContext> context;
        POINT origin{};
        check_hresult(tile.interop->BeginDraw(nullptr, __uuidof(ID2D1DeviceContext),
            context.put_void(), &origin));
        try {
            context->SetTransform(D2D1::Matrix3x2F::Translation(
                static_cast<float>(origin.x), static_cast<float>(origin.y)));
            context->Clear(D2D1::ColorF(0, 0, 0, 0));
            com_ptr<ID2D1SolidColorBrush> brush;
            check_hresult(context->CreateSolidColorBrush(D2D1::ColorF(1, 1, 1, 1), brush.put()));
            if (mask) {
                // The left 48 pixels remain transparent; the rest has partial and full coverage.
                brush->SetOpacity(0.35f);
                context->FillRectangle(D2D1::RectF(TransparentMaskWidth, 0, EditorWidth, EditorHeight), brush.get());
                brush->SetOpacity(1);
                context->FillEllipse(D2D1::Ellipse(D2D1::Point2F(230 + std::sin(phase) * 100, EditorHeight / 2),
                    76, 40), brush.get());
            } else {
                brush->SetColor(D2D1::ColorF(0.15f, 0.85f, 0.7f, 0.72f));
                context->FillEllipse(D2D1::Ellipse(D2D1::Point2F(280 + std::sin(phase) * 160 - xOffset, 80),
                    64, 64), brush.get());
                brush->SetColor(D2D1::ColorF(1, 0.55f, 0.2f, 1));
                context->DrawRectangle(D2D1::RectF(2, 2, TileWidth - 2, SceneHeight - 2), brush.get(), 2);
            }
        } catch (...) {
            tile.interop->EndDraw();
            throw;
        }
        check_hresult(tile.interop->EndDraw());
    }
};

struct Probe {
    Window window{nullptr};
    TextBox editor{nullptr};
    TextBlock status{nullptr};
    Shapes::Rectangle clearMaskInput{nullptr};
    ScrollViewer outer{nullptr};
    ScrollViewer inner{nullptr};
    DispatcherQueueTimer timer{nullptr};
    Compositor compositor{nullptr};
    DrawingSurfaces surfaces;
    std::array<DrawingTile, 2> tiles;
    DrawingTile mask;
    CompositionVisualSurface capturedControl{nullptr};
    Visual sourceControlVisual{nullptr};
    SpriteVisual maskedControl{nullptr};
    ContainerVisual tileRoot{nullptr};
    uint64_t frames = 0;
    uint64_t inputs = 0;
    uint64_t selections = 0;
    bool paused = false;
    bool nativeMaskEnabled = true;

    void initialize() {
        window = Window();
        window.Title(L"PAX-1038 — native composition probe");
        StackPanel page;
        page.Spacing(6);
        page.Padding({10, 8, 10, 8});
        page.Background(SolidColorBrush(Windows::UI::Color{255, 20, 28, 39}));
        page.Children().Append(label(L"Native controls + GPU composition", 18));
        page.Children().Append(label(L"Edit and select in the field; scroll both axes. GPU tiles overlap a live native mask.", 12));

        StackPanel toolbar;
        toolbar.Orientation(Orientation::Horizontal);
        toolbar.Spacing(8);
        Button pause;
        pause.Content(box_value(L"Pause"));
        Automation::AutomationProperties::SetAutomationId(pause, L"pause-animation");
        pause.Click([this](auto const&, auto const&) { paused = !paused; log(paused ? "paused" : "resumed"); });
        toolbar.Children().Append(pause);
        Button check;
        check.Content(box_value(L"Select + scroll"));
        check.Click([this](auto const&, auto const&) {
            editor.Focus(FocusState::Programmatic);
            editor.Select(0, std::min(6, static_cast<int>(editor.Text().size())));
            inner.ChangeView(120.0, nullptr, nullptr, true);
            log("requested selection and horizontal scroll; editor identity retained");
        });
        toolbar.Children().Append(check);
        Button maskToggle;
        maskToggle.Content(box_value(L"Mask on / off"));
        maskToggle.Click([this](auto const&, auto const&) {
            nativeMaskEnabled = !nativeMaskEnabled;
            sourceControlVisual.Opacity(nativeMaskEnabled ? 0.0f : 1.0f);
            maskedControl.IsVisible(nativeMaskEnabled);
            clearMaskInput.IsHitTestVisible(nativeMaskEnabled);
            log(nativeMaskEnabled ? "native coverage enabled" : "native coverage disabled");
        });
        toolbar.Children().Append(maskToggle);
        page.Children().Append(toolbar);

        outer = ScrollViewer();
        Automation::AutomationProperties::SetAutomationId(outer, L"outer-scroll");
        outer.Height(190);
        outer.HorizontalScrollMode(ScrollMode::Disabled);
        outer.VerticalScrollBarVisibility(ScrollBarVisibility::Visible);
        StackPanel document;
        document.Spacing(8);
        inner = ScrollViewer();
        Automation::AutomationProperties::SetAutomationId(inner, L"inner-scroll");
        inner.Height(180);
        inner.HorizontalScrollMode(ScrollMode::Enabled);
        inner.HorizontalScrollBarVisibility(ScrollBarVisibility::Visible);
        inner.VerticalScrollMode(ScrollMode::Disabled);
        Canvas strip;
        strip.Width(1300);
        strip.Height(SceneHeight);
        Grid stage;
        stage.Width(SceneWidth);
        stage.Height(SceneHeight);
        stage.Background(SolidColorBrush(Windows::UI::Color{255, 39, 54, 68}));
        Canvas native;
        Grid controlHost;
        controlHost.Width(EditorWidth);
        controlHost.Height(EditorHeight);
        editor = TextBox();
        editor.Text(L"Native text — café 日本語 مرحبا");
        editor.PlaceholderText(L"Type and select here");
        editor.Width(EditorWidth);
        editor.Height(EditorHeight);
        editor.AcceptsReturn(true);
        editor.TextWrapping(TextWrapping::Wrap);
        editor.FontSize(15);
        Automation::AutomationProperties::SetAutomationId(editor, L"native-editor");
        editor.TextChanged([this](auto const&, auto const&) { ++inputs; log("text_changed count=" + std::to_string(inputs)); });
        editor.AddHandler(UIElement::PointerPressedEvent(), box_value(Microsoft::UI::Xaml::Input::PointerEventHandler(
            [](auto const&, auto const&) { log("native_pointer_pressed"); })), true);
        editor.SelectionChanged([this](auto const&, auto const&) { ++selections; });
        editor.TextCompositionStarted([](auto const&, auto const&) { log("ime_composition_started"); });
        editor.TextCompositionChanged([](auto const&, auto const&) { log("ime_composition_changed"); });
        editor.TextCompositionEnded([](auto const&, auto const&) { log("ime_composition_ended"); });
        Canvas::SetLeft(controlHost, 48);
        Canvas::SetTop(controlHost, 58);
        CompositeTransform transform;
        transform.Rotation(-3);
        transform.SkewX(4);
        controlHost.RenderTransform(transform);
        controlHost.Children().Append(editor);
        // Composition opacity does not alter XAML picking. The same known-zero
        // mask region explicitly belongs to the Pax input layer. Partial and
        // full coverage retain the actual editor's pointer/focus ownership.
        clearMaskInput = Shapes::Rectangle();
        clearMaskInput.Width(TransparentMaskWidth);
        clearMaskInput.Height(EditorHeight);
        clearMaskInput.HorizontalAlignment(HorizontalAlignment::Left);
        clearMaskInput.Fill(SolidColorBrush(Colors::Transparent()));
        Automation::AutomationProperties::SetAutomationId(clearMaskInput, L"clear-mask-input");
        clearMaskInput.PointerPressed([](auto const&, Microsoft::UI::Xaml::Input::PointerRoutedEventArgs const& args) {
            args.Handled(true);
            log("pax_clear_mask_pointer_pressed");
        });
        controlHost.Children().Append(clearMaskInput);
        native.Children().Append(controlHost);
        stage.Children().Append(native);
        Grid gpuHost;
        gpuHost.IsHitTestVisible(false);
        stage.Children().Append(gpuHost);
        strip.Children().Append(stage);
        auto extentLabel = label(L"Scroll extent — native content and GPU tiles", 15);
        Canvas::SetLeft(extentLabel, 600);
        Canvas::SetTop(extentLabel, 64);
        strip.Children().Append(extentLabel);
        inner.Content(strip);
        document.Children().Append(inner);
        document.Children().Append(label(L"Selection must survive native scrolling and GPU repaint.", 12));
        auto spacer = label(L"End of native scroll content");
        spacer.Height(450);
        document.Children().Append(spacer);
        outer.Content(document);
        page.Children().Append(outer);

        page.Children().Append(label(L"Live mirror: clear left edge, partial coverage, opaque moving region", 12));
        Grid mirrorHost;
        mirrorHost.Width(EditorWidth);
        mirrorHost.Height(EditorHeight);
        mirrorHost.HorizontalAlignment(HorizontalAlignment::Left);
        mirrorHost.Background(SolidColorBrush(Windows::UI::Color{255, 68, 43, 65}));
        mirrorHost.IsHitTestVisible(false);
        page.Children().Append(mirrorHost);
        status = label(L"Initializing compositor…", 11);
        page.Children().Append(status);
        window.Content(page);
        window.Activate();

        compositor = ElementCompositionPreview::GetElementVisual(stage).Compositor();
        surfaces.initialize(compositor);
        tileRoot = compositor.CreateContainerVisual();
        for (size_t i = 0; i < tiles.size(); ++i) {
            tiles[i] = surfaces.create(compositor, {TileWidth, SceneHeight});
            tiles[i].visual.Offset({TileWidth * static_cast<float>(i), 0, 0});
            tileRoot.Children().InsertAtTop(tiles[i].visual);
        }
        ElementCompositionPreview::SetElementChildVisual(gpuHost, tileRoot);
        auto rounded = compositor.CreateRoundedRectangleGeometry();
        rounded.Size({SceneWidth, SceneHeight});
        rounded.CornerRadius({16, 16});
        ElementCompositionPreview::GetElementVisual(stage).Clip(compositor.CreateGeometricClip(rounded));

        capturedControl = compositor.CreateVisualSurface();
        sourceControlVisual = ElementCompositionPreview::GetElementVisual(editor);
        capturedControl.SourceVisual(sourceControlVisual);
        capturedControl.SourceSize({EditorWidth, EditorHeight});
        mask = surfaces.create(compositor, {EditorWidth, EditorHeight});
        auto masked = compositor.CreateMaskBrush();
        masked.Source(compositor.CreateSurfaceBrush(capturedControl));
        masked.Mask(compositor.CreateSurfaceBrush(mask.surface));
        // The real XAML control stays in the input/focus tree. Suppress only its
        // original visual, as in WinUI Gallery's OpacityMaskView; the capture's
        // root opacity does not attenuate the redirected pixels.
        sourceControlVisual.Opacity(0);
        maskedControl = compositor.CreateSpriteVisual();
        maskedControl.Size({EditorWidth, EditorHeight});
        maskedControl.Brush(masked);
        ElementCompositionPreview::SetElementChildVisual(controlHost, maskedControl);
        auto mirror = compositor.CreateSpriteVisual();
        mirror.Size({EditorWidth, EditorHeight});
        mirror.Brush(masked);
        ElementCompositionPreview::SetElementChildVisual(mirrorHost, mirror);

        timer = DispatcherQueue::GetForCurrentThread().CreateTimer();
        timer.Interval(std::chrono::milliseconds(16));
        timer.Tick([this](auto const&, auto const&) {
            if (paused) return;
            try {
                auto phase = static_cast<float>(frames) / 90;
                surfaces.begin_frame(phase);
                for (size_t i = 0; i < tiles.size(); ++i)
                    surfaces.draw(tiles[i], phase, TileWidth * static_cast<float>(i), false);
                surfaces.draw(mask, phase, 0, true);
                surfaces.end_frame();
                ++frames;
                if (frames == 1) log("first_frame_presented producer=" + std::string(usePax ? "pax" : (useDx12 ? "dx12" : "d3d11")));
                if (frames % 60 == 0) {
                    status.Text(L"Frames: " + to_hstring(frames) + L" · native input: " + to_hstring(inputs)
                        + L" · selection changes: " + to_hstring(selections)
                        + L" · selection: " + to_hstring(editor.SelectionStart()) + L"/" + to_hstring(editor.SelectionLength()));
                    log("frame=" + std::to_string(frames) + " selection=" + std::to_string(editor.SelectionStart())
                        + "/" + std::to_string(editor.SelectionLength()));
                }
            } catch (hresult_error const& error) {
                timer.Stop();
                log("render failed: " + to_string(error.message()));
                status.Text(L"Rendering stopped: " + error.message());
            }
        });
        window.AppWindow().Closing([this, pause](auto const&, auto const&) {
            timer.Stop();
            // WinUI #11653: deactivate RichEdit while input services still exist.
            // Keep this in the app rather than moving focus in the checker.
            auto moved = pause.Focus(FocusState::Programmatic);
            log("closing focus_deactivated=" + std::string(moved ? "true" : "false"));
        });
        window.Closed([this](auto const&, auto const&) { timer.Stop(); log("closed"); });
        timer.Start();
        log("ready: native TextBox, nested ScrollViewer, two DrawingSurface tiles, live VisualSurface mask");
    }
};

struct App : ApplicationT<App, Markup::IXamlMetadataProvider> {
    std::unique_ptr<Probe> probe;
    XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};
    App() {
        log("App constructor");
        UnhandledException([](auto const&, UnhandledExceptionEventArgs const& args) {
            log("XAML unhandled HRESULT=" + std::to_string(static_cast<int32_t>(args.Exception()))
                + " message=" + to_string(args.Message()));
        });
    }
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider() {
        if (!metadata) {
            log("initializing XAML metadata provider");
            metadata = XamlTypeInfo::XamlControlsXamlMetaDataProvider();
            log("XAML metadata provider initialized");
        }
        return metadata;
    }
    Markup::IXamlType GetXamlType(hstring const& name) {
        return provider().GetXamlType(name);
    }
    Markup::IXamlType GetXamlType(Windows::UI::Xaml::Interop::TypeName const& type) {
        return provider().GetXamlType(type);
    }
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions() {
        return provider().GetXmlnsDefinitions();
    }
    void OnLaunched(LaunchActivatedEventArgs const&) {
        try {
            log("OnLaunched");
            Resources().MergedDictionaries().Append(XamlControlsResources());
            log("XAML control resources initialized");
            probe = std::make_unique<Probe>();
            probe->initialize();
        } catch (hresult_error const& error) {
            log("initialization failed HRESULT=" + std::to_string(static_cast<unsigned long>(error.code()))
                + " message=" + to_string(error.message()));
            Exit();
        }
    }
};
}

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR commandLine, int) {
    try {
        std::wstring options(commandLine ? commandLine : L"");
        usePax = options.find(L"--pax") != std::wstring::npos;
        useDx12 = usePax || options.find(L"--dx12") != std::wstring::npos;
        allowWarp = options.find(L"--allow-warp") != std::wstring::npos;
        log("start producer=" + std::string(usePax ? "pax" : (useDx12 ? "dx12" : "d3d11"))
            + " allow_warp=" + (allowWarp ? "true" : "false"));
        init_apartment(apartment_type::single_threaded);
        log("STA initialized; entering Application.Start");
        Application::Start([](auto const&) {
            log("Application.Start callback");
            make<App>();
            log("App created");
        });
        log("Application.Start returned cleanly");
        return 0;
    } catch (hresult_error const& error) {
        log("startup failed: " + to_string(error.message()));
        return static_cast<int>(error.code());
    }
}
