#pragma once
#include "pax_wgpu_producer.h"
#include <array>
#include <cmath>
#include <d3d11on12.h>
#include <d3d12.h>
#include <dxgi1_4.h>
#include <functional>
#include <memory>
#include <string>

// Bridges native patterns or Pax export textures into compositor-owned surfaces.
// One DX12 queue drives the producer and D3D11On12 copy without CPU pixel readback.
class Dx12Producer
{
    struct Image
    {
        winrt::com_ptr<ID3D12Resource> texture;
        winrt::com_ptr<ID3D11Resource> wrapped;
        D3D12_CPU_DESCRIPTOR_HANDLE rtv{};
        UINT width = 0;
        UINT height = 0;
    };
    winrt::com_ptr<ID3D12Device> device;
    winrt::com_ptr<ID3D12CommandQueue> queue;
    winrt::com_ptr<ID3D12CommandAllocator> allocator;
    winrt::com_ptr<ID3D12GraphicsCommandList> commands;
    winrt::com_ptr<ID3D12DescriptorHeap> descriptors;
    winrt::com_ptr<ID3D12Fence> fence;
    winrt::com_ptr<ID3D11On12Device> interop;
    winrt::com_ptr<ID3D11DeviceContext> immediate;
    winrt::handle completion;
    std::array<Image, 2> images;
    std::unique_ptr<PaxWgpuProducer> pax;
    UINT64 submitted = 0;
    UINT tileWidth = 0;
    UINT transparentWidth = 0;

    void wait_for_previous_frame()
    {
        if (fence->GetCompletedValue() >= submitted)
            return;
        winrt::check_hresult(fence->SetEventOnCompletion(submitted, completion.get()));
        if (WaitForSingleObject(completion.get(), 5000) != WAIT_OBJECT_0)
            winrt::throw_hresult(HRESULT_FROM_WIN32(ERROR_TIMEOUT));
    }

    static D3D12_RESOURCE_BARRIER transition(Image const &image, D3D12_RESOURCE_STATES before,
                                             D3D12_RESOURCE_STATES after)
    {
        D3D12_RESOURCE_BARRIER result{};
        result.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        result.Transition.pResource = image.texture.get();
        result.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        result.Transition.StateBefore = before;
        result.Transition.StateAfter = after;
        return result;
    }

  public:
    ~Dx12Producer()
    {
        if (fence)
        {
            try
            {
                if (immediate)
                    immediate->Flush();
                winrt::check_hresult(queue->Signal(fence.get(), ++submitted));
                wait_for_previous_frame();
            }
            catch (...)
            {
            }
        }
        for (auto &image : images)
        {
            image.wrapped = nullptr;
            image.texture = nullptr;
        }
        if (immediate)
            immediate->Flush();
        pax.reset();
    }

    void initialize(winrt::com_ptr<ID3D11Device> &device11, winrt::com_ptr<ID3D11DeviceContext> &context11,
                    bool allowWarp, bool usePax, UINT sceneWidth, UINT sceneHeight, UINT editorWidth,
                    UINT editorHeight, UINT clearWidth,
                    std::function<void(std::string const &)> const &report)
    {
        tileWidth = sceneWidth / 2;
        transparentWidth = clearWidth;
        if (usePax)
        {
            pax = std::make_unique<PaxWgpuProducer>();
            auto output =
                pax->initialize(allowWarp, sceneWidth, sceneHeight, editorWidth, editorHeight, clearWidth);
            device.copy_from(static_cast<ID3D12Device *>(output.device));
            queue.copy_from(static_cast<ID3D12CommandQueue *>(output.queue));
            images[0].texture.copy_from(static_cast<ID3D12Resource *>(output.scene));
            images[1].texture.copy_from(static_cast<ID3D12Resource *>(output.mask));
            report("Using Pax/wgpu DX12 device, queue, and dedicated export textures");
        }
        if (!usePax)
        {
            winrt::com_ptr<IDXGIFactory4> factory;
            winrt::check_hresult(CreateDXGIFactory1(__uuidof(IDXGIFactory4), factory.put_void()));
            for (UINT index = 0;; ++index)
            {
                winrt::com_ptr<IDXGIAdapter1> adapter;
                auto result = factory->EnumAdapters1(index, adapter.put());
                if (result == DXGI_ERROR_NOT_FOUND)
                    break;
                winrt::check_hresult(result);
                DXGI_ADAPTER_DESC1 description{};
                winrt::check_hresult(adapter->GetDesc1(&description));
                if ((description.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) != 0)
                    continue;
                result = D3D12CreateDevice(adapter.get(), D3D_FEATURE_LEVEL_11_0, __uuidof(ID3D12Device),
                                           device.put_void());
                report("DX12 adapter=" + winrt::to_string(winrt::hstring(description.Description)) +
                       " creation_hresult=" + std::to_string(static_cast<unsigned long>(result)));
                if (SUCCEEDED(result))
                    break;
            }
            if (!device)
            {
                if (!allowWarp)
                    winrt::throw_hresult(DXGI_ERROR_UNSUPPORTED);
                winrt::com_ptr<IDXGIAdapter> warp;
                winrt::check_hresult(factory->EnumWarpAdapter(__uuidof(IDXGIAdapter), warp.put_void()));
                winrt::check_hresult(D3D12CreateDevice(warp.get(), D3D_FEATURE_LEVEL_11_0,
                                                       __uuidof(ID3D12Device), device.put_void()));
                report("DX12 explicitly using WARP; this does not qualify hardware performance");
            }
            D3D12_COMMAND_QUEUE_DESC queueDescription{};
            queueDescription.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
            winrt::check_hresult(device->CreateCommandQueue(&queueDescription, __uuidof(ID3D12CommandQueue),
                                                            queue.put_void()));
        }
        IUnknown *queues[] = {queue.get()};
        winrt::check_hresult(D3D11On12CreateDevice(device.get(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0,
                                                   queues, 1, 0, device11.put(), context11.put(), nullptr));
        interop = device11.as<ID3D11On12Device>();
        immediate = context11;
        if (!pax)
        {
            winrt::check_hresult(device->CreateCommandAllocator(
                D3D12_COMMAND_LIST_TYPE_DIRECT, __uuidof(ID3D12CommandAllocator), allocator.put_void()));
            winrt::check_hresult(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, allocator.get(),
                                                           nullptr, __uuidof(ID3D12GraphicsCommandList),
                                                           commands.put_void()));
            winrt::check_hresult(commands->Close());
        }
        winrt::check_hresult(
            device->CreateFence(0, D3D12_FENCE_FLAG_NONE, __uuidof(ID3D12Fence), fence.put_void()));
        completion.attach(CreateEventW(nullptr, FALSE, FALSE, nullptr));
        if (!completion)
            winrt::throw_last_error();
        D3D12_CPU_DESCRIPTOR_HANDLE rtv{};
        if (!pax)
        {
            D3D12_DESCRIPTOR_HEAP_DESC heapDescription{};
            heapDescription.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
            heapDescription.NumDescriptors = static_cast<UINT>(images.size());
            winrt::check_hresult(device->CreateDescriptorHeap(
                &heapDescription, __uuidof(ID3D12DescriptorHeap), descriptors.put_void()));
            rtv = descriptors->GetCPUDescriptorHandleForHeapStart();
        }
        for (size_t index = 0; index < images.size(); ++index)
        {
            auto &image = images[index];
            image.width = index == 0 ? sceneWidth : editorWidth;
            image.height = index == 0 ? sceneHeight : editorHeight;
            if (!pax)
            {
                image.rtv = rtv;
                rtv.ptr += device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
                D3D12_HEAP_PROPERTIES heap{};
                heap.Type = D3D12_HEAP_TYPE_DEFAULT;
                D3D12_RESOURCE_DESC description{};
                description.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
                description.Width = image.width;
                description.Height = image.height;
                description.DepthOrArraySize = 1;
                description.MipLevels = 1;
                description.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
                description.SampleDesc.Count = 1;
                description.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
                winrt::check_hresult(device->CreateCommittedResource(
                    &heap, D3D12_HEAP_FLAG_NONE, &description, D3D12_RESOURCE_STATE_COPY_SOURCE, nullptr,
                    __uuidof(ID3D12Resource), image.texture.put_void()));
                device->CreateRenderTargetView(image.texture.get(), nullptr, image.rtv);
            }
            D3D11_RESOURCE_FLAGS flags{};
            winrt::check_hresult(interop->CreateWrappedResource(
                image.texture.get(), &flags, D3D12_RESOURCE_STATE_COPY_SOURCE,
                D3D12_RESOURCE_STATE_COPY_SOURCE, __uuidof(ID3D11Resource), image.wrapped.put_void()));
            ID3D11Resource *resources[] = {image.wrapped.get()};
            interop->ReleaseWrappedResources(resources, 1);
        }
        immediate->Flush();
        report("DX12 producer and D3D11On12 copy initialized on one queue");
    }

    void render(float phase)
    {
        // One frame in flight bounds this feasibility probe. Production pacing
        // must retain resources across asynchronous frames instead of waiting here.
        wait_for_previous_frame();
        if (pax)
        {
            pax->render(phase);
            return;
        }
        winrt::check_hresult(allocator->Reset());
        winrt::check_hresult(commands->Reset(allocator.get(), nullptr));
        for (size_t index = 0; index < images.size(); ++index)
        {
            auto &image = images[index];
            auto begin =
                transition(image, D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_RENDER_TARGET);
            commands->ResourceBarrier(1, &begin);
            float empty[] = {0, 0, 0, 0};
            float partial[] = {0.35f, 0.35f, 0.35f, 0.35f};
            commands->ClearRenderTargetView(image.rtv, empty, 0, nullptr);
            if (index == 1)
            {
                D3D12_RECT coverage{static_cast<LONG>(transparentWidth), 0, static_cast<LONG>(image.width),
                                    static_cast<LONG>(image.height)};
                commands->ClearRenderTargetView(image.rtv, partial, 1, &coverage);
            }
            LONG center =
                static_cast<LONG>((index == 0 ? 280 : 230) + std::sin(phase) * (index == 0 ? 160 : 100));
            D3D12_RECT region{center - 64, index == 0 ? 16L : 0L, center + 64,
                              index == 0 ? 144L : static_cast<LONG>(image.height)};
            float turquoise[] = {0.108f, 0.612f, 0.504f, 0.72f};
            float opaque[] = {1, 1, 1, 1};
            commands->ClearRenderTargetView(image.rtv, index == 0 ? turquoise : opaque, 1, &region);
            auto end =
                transition(image, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATE_COPY_SOURCE);
            commands->ResourceBarrier(1, &end);
        }
        winrt::check_hresult(commands->Close());
        ID3D12CommandList *lists[] = {commands.get()};
        queue->ExecuteCommandLists(1, lists);
    }

    void copy_to(ID3D11Texture2D *destination, POINT origin, UINT xOffset, bool mask)
    {
        auto &image = images[mask ? 1 : 0];
        ID3D11Resource *resources[] = {image.wrapped.get()};
        interop->AcquireWrappedResources(resources, 1);
        D3D11_BOX box{xOffset, 0, 0, xOffset + (mask ? image.width : tileWidth), image.height, 1};
        immediate->CopySubresourceRegion(destination, 0, origin.x, origin.y, 0, image.wrapped.get(), 0, &box);
        interop->ReleaseWrappedResources(resources, 1);
        // EndDraw publishes the destination; submit its copy first.
        immediate->Flush();
    }

    void finish_frame()
    {
        immediate->Flush();
        winrt::check_hresult(queue->Signal(fence.get(), ++submitted));
    }
};
