#pragma once
#include "mask_path.h"
#include <algorithm>
#include <d2d1_1.h>
#include <windows.graphics.interop.h>
#include <winrt/Windows.Graphics.h>
#include <winrt/Windows.Data.Json.h>
#include <winrt/Microsoft.UI.Composition.h>
#include <winrt/Microsoft.UI.Composition.Interop.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Media.h>
#include <winrt/Microsoft.UI.Xaml.Shapes.h>

namespace pax::windows {
struct MaskPath {
    winrt::com_ptr<ID2D1PathGeometry> gpu;
    winrt::Microsoft::UI::Xaml::Media::PathGeometry input{nullptr};
};
struct MaskHole {
    MaskPath shape;
    std::vector<MaskPath> clips;
    float opacity = 1;
};

class NativeMaskPainter {
    // Composition retains the D2D geometry through this public Windows SDK
    // interop interface; it does not borrow a temporary path or XAML object.
    struct GeometrySource : winrt::implements<GeometrySource,
        winrt::Windows::Graphics::IGeometrySource2D,
        ABI::Windows::Graphics::IGeometrySource2DInterop> {
        winrt::com_ptr<ID2D1Geometry> geometry;
        explicit GeometrySource(winrt::com_ptr<ID2D1Geometry> value) : geometry(std::move(value)) {}
        HRESULT __stdcall GetGeometry(ID2D1Geometry** value) noexcept override {
            if (!value) return E_POINTER;
            geometry.copy_to(value); return S_OK;
        }
        HRESULT __stdcall TryGetGeometryUsingFactory(ID2D1Factory*, ID2D1Geometry** value) noexcept override {
            if (!value) return E_POINTER;
            *value = nullptr; return E_NOTIMPL;
        }
    };
    struct InputSink : winrt::implements<InputSink, ID2D1SimplifiedGeometrySink> {
        winrt::Microsoft::UI::Xaml::Media::PathGeometry path;
        winrt::Microsoft::UI::Xaml::Media::PathFigure figure{nullptr};
        HRESULT failure = S_OK;
        template<typename F> void attempt(F&& f) noexcept { try { f(); } catch (...) { failure = winrt::to_hresult(); } }
        void __stdcall SetFillMode(D2D1_FILL_MODE mode) noexcept override {
            attempt([&] { path.FillRule(mode == D2D1_FILL_MODE_WINDING ? winrt::Microsoft::UI::Xaml::Media::FillRule::Nonzero : winrt::Microsoft::UI::Xaml::Media::FillRule::EvenOdd); });
        }
        void __stdcall SetSegmentFlags(D2D1_PATH_SEGMENT) noexcept override {}
        void __stdcall BeginFigure(D2D1_POINT_2F point, D2D1_FIGURE_BEGIN begin) noexcept override {
            attempt([&] { figure = winrt::Microsoft::UI::Xaml::Media::PathFigure(); figure.StartPoint({point.x, point.y}); figure.IsFilled(begin == D2D1_FIGURE_BEGIN_FILLED); });
        }
        void __stdcall AddLines(D2D1_POINT_2F const* points, UINT32 count) noexcept override {
            attempt([&] { for (UINT32 i = 0; i < count; ++i) { winrt::Microsoft::UI::Xaml::Media::LineSegment segment; segment.Point({points[i].x, points[i].y}); figure.Segments().Append(segment); } });
        }
        void __stdcall AddBeziers(D2D1_BEZIER_SEGMENT const* segments, UINT32 count) noexcept override {
            attempt([&] { for (UINT32 i = 0; i < count; ++i) { auto const& s = segments[i]; winrt::Microsoft::UI::Xaml::Media::BezierSegment segment;
                segment.Point1({s.point1.x,s.point1.y}); segment.Point2({s.point2.x,s.point2.y}); segment.Point3({s.point3.x,s.point3.y}); figure.Segments().Append(segment); } });
        }
        void __stdcall EndFigure(D2D1_FIGURE_END end) noexcept override {
            attempt([&] { figure.IsClosed(end == D2D1_FIGURE_END_CLOSED); path.Figures().Append(figure); figure = nullptr; });
        }
        HRESULT __stdcall Close() noexcept override { return failure; }
    };
    winrt::com_ptr<ID2D1Factory1> factory_;
    winrt::com_ptr<ID2D1Device> device_;
    winrt::com_ptr<ID2D1DeviceContext> recorder_;
    winrt::Microsoft::UI::Composition::CompositionGraphicsDevice graphics_{nullptr};
public:
    winrt::Microsoft::UI::Composition::CompositionGeometricClip geometric_clip(
        winrt::com_ptr<ID2D1Geometry> const& path, winrt::Microsoft::UI::Composition::Compositor const& compositor) const {
        auto source = winrt::make<GeometrySource>(path);
        auto geometry = compositor.CreatePathGeometry(winrt::Microsoft::UI::Composition::CompositionPath(source));
        return compositor.CreateGeometricClip(geometry);
    }
    winrt::Microsoft::UI::Composition::CompositionGeometricClip geometric_clip(
        MaskPath const& path, winrt::Microsoft::UI::Composition::Compositor const& compositor) const {
        return geometric_clip(path.gpu.as<ID2D1Geometry>(), compositor);
    }
    // Accumulated compositor clips need no XAML copy of each intermediate
    // Boolean result. Input paths keep their separately constructed geometry.
    winrt::com_ptr<ID2D1Geometry> combine_gpu(ID2D1Geometry* a, ID2D1Geometry* b, D2D1_COMBINE_MODE mode) const {
        winrt::com_ptr<ID2D1PathGeometry> result;
        winrt::check_hresult(factory_->CreatePathGeometry(result.put()));
        winrt::com_ptr<ID2D1GeometrySink> output;
        winrt::check_hresult(result->Open(output.put()));
        winrt::check_hresult(a->CombineWithGeometry(b, mode, D2D1::Matrix3x2F::Identity(), output.get()));
        winrt::check_hresult(output->Close());
        return result.as<ID2D1Geometry>();
    }
    MaskPath transform(MaskPath const& source, D2D1_MATRIX_3X2_F matrix) const {
        MaskPath result;
        winrt::check_hresult(factory_->CreatePathGeometry(result.gpu.put()));
        winrt::com_ptr<ID2D1GeometrySink> output;
        winrt::check_hresult(result.gpu->Open(output.put()));
        winrt::check_hresult(source.gpu->Simplify(D2D1_GEOMETRY_SIMPLIFICATION_OPTION_CUBICS_AND_LINES, matrix, output.get()));
        winrt::check_hresult(output->Close());
        auto input = winrt::make_self<InputSink>();
        winrt::check_hresult(result.gpu->Simplify(D2D1_GEOMETRY_SIMPLIFICATION_OPTION_CUBICS_AND_LINES,
            D2D1::Matrix3x2F::Identity(), input.get()));
        winrt::check_hresult(input->Close()); result.input = input->path;
        return result;
    }
    MaskPath combine(MaskPath const& a, MaskPath const& b, D2D1_COMBINE_MODE mode) const {
        MaskPath result;
        winrt::check_hresult(factory_->CreatePathGeometry(result.gpu.put()));
        winrt::com_ptr<ID2D1GeometrySink> output;
        winrt::check_hresult(result.gpu->Open(output.put()));
        winrt::check_hresult(a.gpu->CombineWithGeometry(b.gpu.get(), mode, D2D1::Matrix3x2F::Identity(), output.get()));
        winrt::check_hresult(output->Close());
        auto input = winrt::make_self<InputSink>();
        winrt::check_hresult(result.gpu->Simplify(D2D1_GEOMETRY_SIMPLIFICATION_OPTION_CUBICS_AND_LINES,
            D2D1::Matrix3x2F::Identity(), input.get()));
        winrt::check_hresult(input->Close());
        result.input = input->path;
        return result;
    }
    MaskPath rectangle(float width, float height) const {
        return parse(winrt::to_hstring("M0 0 L" + std::to_string(width) + " 0 L" + std::to_string(width) + " " + std::to_string(height) + " L0 " + std::to_string(height) + " Z"));
    }
    MaskPath rounded_rectangle(float width, float height, float radius) const {
        radius = std::clamp(radius, 0.0f, std::min(width, height) / 2);
        winrt::com_ptr<ID2D1RoundedRectangleGeometry> rounded;
        winrt::check_hresult(factory_->CreateRoundedRectangleGeometry(
            D2D1::RoundedRect(D2D1::RectF(0, 0, width, height), radius, radius), rounded.put()));
        MaskPath result;
        winrt::check_hresult(factory_->CreatePathGeometry(result.gpu.put()));
        winrt::com_ptr<ID2D1GeometrySink> output;
        winrt::check_hresult(result.gpu->Open(output.put()));
        winrt::check_hresult(rounded->Simplify(D2D1_GEOMETRY_SIMPLIFICATION_OPTION_CUBICS_AND_LINES,
            D2D1::Matrix3x2F::Identity(), output.get()));
        winrt::check_hresult(output->Close());
        auto input = winrt::make_self<InputSink>();
        winrt::check_hresult(result.gpu->Simplify(D2D1_GEOMETRY_SIMPLIFICATION_OPTION_CUBICS_AND_LINES,
            D2D1::Matrix3x2F::Identity(), input.get()));
        winrt::check_hresult(input->Close()); result.input = input->path;
        return result;
    }
    void initialize(winrt::Microsoft::UI::Composition::Compositor const& compositor, ID3D11Device* device) {
        D2D1_FACTORY_OPTIONS options{};
        winrt::check_hresult(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED,
            __uuidof(ID2D1Factory1), &options, factory_.put_void()));
        winrt::com_ptr<IDXGIDevice> dxgi;
        winrt::check_hresult(device->QueryInterface(__uuidof(IDXGIDevice), dxgi.put_void()));
        winrt::check_hresult(factory_->CreateDevice(dxgi.get(), device_.put()));
        winrt::check_hresult(device_->CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE, recorder_.put()));
        winrt::check_hresult(compositor.as<winrt::Microsoft::UI::Composition::ICompositorInterop>()
            ->CreateGraphicsDevice(device_.get(), &graphics_));
    }

    MaskPath parse(winrt::hstring const& path) const {
        using namespace winrt::Microsoft::UI::Xaml::Media;
        MaskPath result;
        result.input = PathGeometry();
        result.input.FillRule(FillRule::Nonzero);
        winrt::check_hresult(factory_->CreatePathGeometry(result.gpu.put()));
        winrt::com_ptr<ID2D1GeometrySink> sink;
        winrt::check_hresult(result.gpu->Open(sink.put()));
        sink->SetFillMode(D2D1_FILL_MODE_WINDING);
        PathFigure figure{nullptr};
        for (auto const& item : parse_mask_path(winrt::to_string(path))) {
            auto p = [&](size_t i) { return winrt::Windows::Foundation::Point{item.points[i], item.points[i + 1]}; };
            auto d = [&](size_t i) { return D2D1::Point2F(item.points[i], item.points[i + 1]); };
            switch (item.command) {
            case 'M':
                if (figure) { sink->EndFigure(D2D1_FIGURE_END_OPEN); result.input.Figures().Append(figure); }
                figure = PathFigure(); figure.StartPoint(p(0)); figure.IsFilled(true);
                sink->BeginFigure(d(0), D2D1_FIGURE_BEGIN_FILLED); break;
            case 'L': { LineSegment s; s.Point(p(0)); figure.Segments().Append(s); sink->AddLine(d(0)); break; }
            case 'Q': { QuadraticBezierSegment s; s.Point1(p(0)); s.Point2(p(2)); figure.Segments().Append(s);
                sink->AddQuadraticBezier(D2D1::QuadraticBezierSegment(d(0), d(2))); break; }
            case 'C': { BezierSegment s; s.Point1(p(0)); s.Point2(p(2)); s.Point3(p(4)); figure.Segments().Append(s);
                sink->AddBezier(D2D1::BezierSegment(d(0), d(2), d(4))); break; }
            case 'Z':
                figure.IsClosed(true); result.input.Figures().Append(figure); figure = nullptr;
                sink->EndFigure(D2D1_FIGURE_END_CLOSED); break;
            }
        }
        if (figure) { sink->EndFigure(D2D1_FIGURE_END_OPEN); result.input.Figures().Append(figure); }
        winrt::check_hresult(sink->Close());
        return result;
    }

    std::vector<MaskHole> parse_holes(winrt::Windows::Data::Json::JsonArray const& entries) const {
        if (entries.Size() > 4096) throw std::invalid_argument("Native mask exceeds coverage-entry budget");
        std::vector<MaskHole> holes;
        for (auto const& value : entries) {
            auto entry = value.GetObject();
            MaskHole hole;
            hole.shape = parse(entry.GetNamedString(L"path"));
            auto opacity = entry.GetNamedValue(L"opacity", winrt::Windows::Data::Json::JsonValue::CreateNullValue());
            hole.opacity = opacity.ValueType() == winrt::Windows::Data::Json::JsonValueType::Null ? 1.0f : float(opacity.GetNumber());
            if (!std::isfinite(hole.opacity)) throw std::invalid_argument("Nonfinite native mask opacity");
            hole.opacity = std::clamp(hole.opacity, 0.0f, 1.0f);
            auto clips = entry.GetNamedArray(L"clips");
            if (clips.Size() > 64) throw std::invalid_argument("Native mask exceeds clip-depth budget");
            for (auto const& clip : clips) hole.shape = combine(hole.shape, parse(clip.GetString()), D2D1_COMBINE_MODE_INTERSECT);
            holes.push_back(std::move(hole));
        }
        return holes;
    }

    winrt::Microsoft::UI::Composition::CompositionDrawingSurface draw(
        float width, float height, float scale, std::vector<MaskHole> const& holes,
        winrt::Microsoft::UI::Composition::CompositionDrawingSurface surface = nullptr) {
        using namespace winrt::Microsoft::Graphics::DirectX;
        if (!(width > 0 && height > 0 && scale > 0) ||
            !std::isfinite(width * height * scale * scale) || width * height * scale * scale > 64 * 1024 * 1024)
            throw std::invalid_argument("Native mask exceeds surface budget");
        auto pixel_width = std::ceil(width * scale), pixel_height = std::ceil(height * scale);
        if (!surface || surface.Size().Width != pixel_width || surface.Size().Height != pixel_height)
            surface = graphics_.CreateDrawingSurface({pixel_width, pixel_height},
                DirectXPixelFormat::B8G8R8A8UIntNormalized, DirectXAlphaMode::Premultiplied);
        auto interop = surface.as<winrt::Microsoft::UI::Composition::ICompositionDrawingSurfaceInterop>();
        winrt::com_ptr<ID2D1DeviceContext> destination;
        POINT origin{};
        winrt::check_hresult(interop->BeginDraw(nullptr, __uuidof(ID2D1DeviceContext), destination.put_void(), &origin));
        try {
            destination->SetDpi(96, 96);
            destination->SetTransform(D2D1::Matrix3x2F::Scale(scale, scale) *
                D2D1::Matrix3x2F::Translation(float(origin.x), float(origin.y)));
            destination->Clear(D2D1::ColorF(1, 1, 1, 1));
            for (auto const& hole : holes) {
                if (!hole.opacity) continue;
                winrt::com_ptr<ID2D1CommandList> commands;
                winrt::check_hresult(recorder_->CreateCommandList(commands.put()));
                recorder_->SetTarget(commands.get());
                recorder_->BeginDraw();
                recorder_->SetTransform(D2D1::Matrix3x2F::Identity());
                for (auto const& clip : hole.clips) {
                    auto layer = D2D1::LayerParameters1();
                    layer.geometricMask = clip.gpu.get();
                    recorder_->PushLayer(layer, nullptr);
                }
                winrt::com_ptr<ID2D1SolidColorBrush> brush;
                winrt::check_hresult(recorder_->CreateSolidColorBrush(D2D1::ColorF(1, 1, 1, hole.opacity), brush.put()));
                recorder_->FillGeometry(hole.shape.gpu.get(), brush.get());
                for (size_t i = 0; i < hole.clips.size(); ++i) recorder_->PopLayer();
                winrt::check_hresult(recorder_->EndDraw());
                recorder_->SetTarget(nullptr);
                winrt::check_hresult(commands->Close());
                // Each coverage entry attenuates existing alpha by (1-opacity),
                // including intersections, exactly as the runtime mask contract.
                D2D1_POINT_2F offset{};
                destination->DrawImage(commands.get(), &offset, nullptr,
                    D2D1_INTERPOLATION_MODE_LINEAR, D2D1_COMPOSITE_MODE_DESTINATION_OUT);
            }
        } catch (...) {
            recorder_->SetTarget(nullptr);
            interop->EndDraw();
            throw;
        }
        winrt::check_hresult(interop->EndDraw());
        return surface;
    }
};
} // namespace pax::windows
