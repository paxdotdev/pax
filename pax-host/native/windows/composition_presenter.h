#pragma once
#include <algorithm>
#include <array>
#include <cstdint>
#include <functional>
#include <limits>
#include <stdexcept>
#include <vector>
#include <d3d11on12.h>
#include <d3d12.h>
#include <winrt/Microsoft.Graphics.DirectX.h>
#include <winrt/Microsoft.UI.Composition.h>
#include <winrt/Microsoft.UI.Composition.Interop.h>

namespace pax::windows {

// The runtime owns the exports and their WGPU state. Before submission it must
// put every image in COPY_SOURCE on this same DIRECT queue. The host returns it
// in COPY_SOURCE, and retires its token only after the copy fence completes.
// DrawingSurface owns the compositor's pixels: the compositor never reads the
// reusable runtime export after its copy has retired.
class CompositionPresenter {
public:
    using Surface = winrt::Microsoft::UI::Composition::CompositionDrawingSurface;
    static constexpr size_t FrameCapacity = 3;
    static constexpr size_t ImageCapacity = 64;
    static constexpr uint64_t MaxFrameBytes = 256ull * 1024 * 1024;

    struct Copy {
        ID3D12Resource* source = nullptr;
        Surface destination{nullptr};
        uint32_t width = 0;
        uint32_t height = 0;
    };
    struct Statistics {
        uint64_t submitted = 0;
        uint64_t retired = 0;
        uint64_t backpressure = 0;
        size_t in_flight = 0;
        size_t peak_in_flight = 0;
    };

private:
    struct Wrapped {
        winrt::com_ptr<ID3D12Resource> source;
        winrt::com_ptr<ID3D11Resource> resource;
        Surface destination{nullptr};
    };
    struct Frame {
        uint64_t token = 0;
        uint64_t fence = 0;
        std::vector<Wrapped> images;
    };
    winrt::com_ptr<ID3D12Device> device_;
    winrt::com_ptr<ID3D12CommandQueue> queue_;
    winrt::com_ptr<ID3D12Fence> fence_;
    winrt::com_ptr<ID3D11Device> device11_;
    winrt::com_ptr<ID3D11DeviceContext> immediate_;
    winrt::com_ptr<ID3D11On12Device> interop_;
    winrt::Microsoft::UI::Composition::CompositionGraphicsDevice graphics_{nullptr};
    std::array<Frame, FrameCapacity> frames_;
    std::function<void(uint64_t, ID3D12Fence*, uint64_t)> retire_;
    Statistics statistics_;
    uint64_t next_fence_ = 0;
    bool accepting_ = true;

    void clear_wrappers(Frame& frame) {
        frame.images.clear();
        // D3D11On12 defers resource destruction until Flush. Drop all wrapper
        // references before releasing the runtime's ownership token.
        immediate_->Flush();
    }

public:
    CompositionPresenter() = default;
    CompositionPresenter(CompositionPresenter const&) = delete;
    CompositionPresenter& operator=(CompositionPresenter const&) = delete;

    // The owner must stop submitting and poll until idle before destruction.
    // A device-loss owner may abandon the whole device only after marking its
    // runtime failed; it must never recycle unfinished frame tokens.
    bool idle() const { return statistics_.in_flight == 0; }
    Statistics const& statistics() const { return statistics_; }
    ID3D11Device* drawing_device() const { return device11_.get(); }
    void stop() { accepting_ = false; }

    void initialize(winrt::Microsoft::UI::Composition::Compositor const& compositor,
                    ID3D12Device* device, ID3D12CommandQueue* queue,
                    std::function<void(uint64_t, ID3D12Fence*, uint64_t)> retire) {
        if (!device || !queue || queue->GetDesc().Type != D3D12_COMMAND_LIST_TYPE_DIRECT)
            throw std::invalid_argument("Pax composition requires a DX12 DIRECT queue");
        device_.copy_from(device);
        queue_.copy_from(queue);
        retire_ = std::move(retire);
        IUnknown* queues[] = {queue};
        winrt::check_hresult(D3D11On12CreateDevice(device, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            nullptr, 0, queues, 1, 0, device11_.put(), immediate_.put(), nullptr));
        interop_ = device11_.as<ID3D11On12Device>();
        winrt::check_hresult(device->CreateFence(0, D3D12_FENCE_FLAG_NONE,
            __uuidof(ID3D12Fence), fence_.put_void()));
        winrt::check_hresult(compositor.as<winrt::Microsoft::UI::Composition::ICompositorInterop>()
            ->CreateGraphicsDevice(device11_.get(), &graphics_));
        for (auto& frame : frames_) frame.images.reserve(ImageCapacity);
    }

    Surface create_surface(uint32_t width, uint32_t height) const {
        if (!width || !height || uint64_t(width) * height * 4 > MaxFrameBytes)
            throw std::invalid_argument("Pax composition surface exceeds its pixel budget");
        using namespace winrt::Microsoft::Graphics::DirectX;
        return graphics_.CreateDrawingSurface({float(width), float(height)},
            DirectXPixelFormat::B8G8R8A8UIntNormalized, DirectXAlphaMode::Premultiplied);
    }

    // Polling never waits for GPU completion. UI input/tick continues when the
    // queue is saturated, and close uses the same retirement path as rendering.
    void poll() {
        auto completed = fence_->GetCompletedValue();
        if (completed == std::numeric_limits<uint64_t>::max()) {
            winrt::check_hresult(device_->GetDeviceRemovedReason());
            winrt::throw_hresult(DXGI_ERROR_DEVICE_REMOVED);
        }
        for (auto& frame : frames_) {
            if (!frame.token || frame.fence > completed) continue;
            auto token = frame.token;
            auto value = frame.fence;
            clear_wrappers(frame);
            frame.token = frame.fence = 0;
            --statistics_.in_flight;
            ++statistics_.retired;
            retire_(token, fence_.get(), value);
        }
    }

    bool can_submit() {
        poll();
        if (!accepting_ || statistics_.in_flight == FrameCapacity) {
            ++statistics_.backpressure;
            return false;
        }
        return true;
    }

    // Once this returns, or throws after ownership is accepted, token remains
    // owned here until retirement. Validation occurs before ownership transfer;
    // callers must terminate rendering on any copy/EndDraw/device failure.
    void submit(uint64_t token, std::vector<Copy> const& copies) {
        if (!token || copies.empty() || copies.size() > ImageCapacity || !can_submit())
            throw std::invalid_argument("Invalid or saturated Pax composition submission");
        uint64_t bytes = 0;
        for (auto const& copy : copies) {
            if (!copy.source || !copy.destination || !copy.width || !copy.height)
                throw std::invalid_argument("Missing Pax composition copy input");
            auto desc = copy.source->GetDesc();
            bytes += uint64_t(copy.width) * copy.height * 4;
            auto size = copy.destination.Size();
            if (desc.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D ||
                desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM || desc.SampleDesc.Count != 1 ||
                desc.Width != copy.width || desc.Height != copy.height ||
                size.Width != float(copy.width) || size.Height != float(copy.height) ||
                bytes > MaxFrameBytes)
                throw std::invalid_argument("Pax composition requires bounded exact-size BGRA8 exports");
        }
        auto& frame = *std::find_if(frames_.begin(), frames_.end(), [](auto const& f) { return !f.token; });
        frame.token = token;
        frame.fence = ++next_fence_;
        ++statistics_.in_flight;
        statistics_.peak_in_flight = std::max(statistics_.peak_in_flight, statistics_.in_flight);
        try {
            for (auto const& copy : copies) {
                Wrapped wrapped;
                wrapped.source.copy_from(copy.source);
                // Resize may detach the old tile immediately after submission.
                // Its compositor surface stays owned until this copy retires.
                wrapped.destination = copy.destination;
                D3D11_RESOURCE_FLAGS flags{};
                winrt::check_hresult(interop_->CreateWrappedResource(copy.source, &flags,
                    D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_COPY_SOURCE,
                    __uuidof(ID3D11Resource), wrapped.resource.put_void()));
                frame.images.push_back(std::move(wrapped));
                auto& image = frame.images.back();
                // CreateWrappedResource starts acquired. Release/flush before
                // acquiring it for this copy, as required by the state tracker.
                ID3D11Resource* resources[] = {image.resource.get()};
                interop_->ReleaseWrappedResources(resources, 1);
                immediate_->Flush();
                auto surface = copy.destination.as<winrt::Microsoft::UI::Composition::ICompositionDrawingSurfaceInterop>();
                winrt::com_ptr<ID3D11Texture2D> destination;
                POINT origin{};
                winrt::check_hresult(surface->BeginDraw(nullptr, __uuidof(ID3D11Texture2D),
                    destination.put_void(), &origin));
                interop_->AcquireWrappedResources(resources, 1);
                D3D11_BOX box{0, 0, 0, copy.width, copy.height, 1};
                immediate_->CopySubresourceRegion(destination.get(), 0, origin.x, origin.y, 0,
                    image.resource.get(), 0, &box);
                interop_->ReleaseWrappedResources(resources, 1);
                immediate_->Flush();
                winrt::check_hresult(surface->EndDraw());
            }
            immediate_->Flush();
            winrt::check_hresult(queue_->Signal(fence_.get(), frame.fence));
            ++statistics_.submitted;
        } catch (...) {
            // If the device is healthy, this fence still retires any accepted
            // partial copies. If Signal fails, the owner enters device-loss
            // teardown and never returns this token to the reuse pool.
            immediate_->Flush();
            if (FAILED(queue_->Signal(fence_.get(), frame.fence)))
                frame.fence = std::numeric_limits<uint64_t>::max();
            throw;
        }
    }
};
} // namespace pax::windows
