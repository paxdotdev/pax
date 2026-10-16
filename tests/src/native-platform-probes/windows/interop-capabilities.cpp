// Console-only API boundary check. No window, WinUI, or presentation is exercised.
#include <windows.h>
#include <d3d12.h>
#include <d3d11on12.h>
#include <dcomp.h>
#include <dxgi1_4.h>
#include <wrl/client.h>
#include <cstdio>

using Microsoft::WRL::ComPtr;

static HRESULT step(char const *name, HRESULT hr)
{
    std::printf("{\"event\":\"step\",\"name\":\"%s\",\"hresult\":\"0x%08lX\",\"succeeded\":%s}\n", name,
                static_cast<unsigned long>(hr), SUCCEEDED(hr) ? "true" : "false");
    std::fflush(stdout);
    return hr;
}

static void require(char const *name, HRESULT hr)
{
    if (FAILED(step(name, hr)))
        throw hr;
}

static int result(bool interop, bool composition, bool compositionWithoutRenderingDevice)
{
    std::printf("{\"event\":\"result\",\"schemaVersion\":1,\"passed\":%s,"
                "\"d3d11On12ResourcePassed\":%s,\"directCompositionDeviceCreated\":%s,"
                "\"directCompositionWithoutRenderingDeviceCreated\":%s,"
                "\"actualPaxTested\":false,\"nativeCompositionQualified\":false,"
                "\"physicalPerformanceQualified\":false}\n",
                interop && composition ? "true" : "false", interop ? "true" : "false",
                composition ? "true" : "false", compositionWithoutRenderingDevice ? "true" : "false");
    std::fflush(stdout);
    return interop && composition ? 0 : 1;
}

int main()
{
    bool interopPassed = false;
    bool compositionCreated = false;
    bool compositionWithoutRenderingDevice = false;
    bool comInitialized = false;
    try
    {
        require("CoInitializeEx", CoInitializeEx(nullptr, COINIT_MULTITHREADED));
        comInitialized = true;
        // Null renderingDevice is a documented factory activation path. Check
        // it independently, even when DX12 or D3D11On12 is unavailable.
        auto dcomp = LoadLibraryExW(L"dcomp.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
        step("loadDComp", dcomp ? S_OK : HRESULT_FROM_WIN32(GetLastError()));
        auto createComposition = dcomp ? reinterpret_cast<decltype(&DCompositionCreateDevice2)>(
                                             GetProcAddress(dcomp, "DCompositionCreateDevice2"))
                                       : nullptr;
        if (dcomp)
            step("resolveDCompositionCreateDevice2",
                 createComposition ? S_OK : HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND));
        if (createComposition)
        {
            ComPtr<IDCompositionDevice> composition;
            compositionWithoutRenderingDevice =
                SUCCEEDED(step("DCompositionCreateDevice2WithoutRenderingDevice",
                               createComposition(nullptr, IID_PPV_ARGS(&composition))));
            if (compositionWithoutRenderingDevice)
                compositionWithoutRenderingDevice = SUCCEEDED(
                    step("compositionWithoutRenderingDeviceNonNull", composition ? S_OK : E_POINTER));
        }
        ComPtr<IDXGIFactory4> factory;
        require("CreateDXGIFactory1", CreateDXGIFactory1(IID_PPV_ARGS(&factory)));
        ComPtr<ID3D12Device> device;
        for (UINT index = 0;; ++index)
        {
            ComPtr<IDXGIAdapter1> adapter;
            auto hr = factory->EnumAdapters1(index, &adapter);
            if (hr == DXGI_ERROR_NOT_FOUND)
                break;
            require("EnumAdapters1", hr);
            DXGI_ADAPTER_DESC1 desc{};
            require("GetDesc1", adapter->GetDesc1(&desc));
            bool software = (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) != 0;
            std::printf("{\"event\":\"adapter\",\"index\":%u,\"vendorId\":%u,\"deviceId\":%u,"
                        "\"luidHigh\":%ld,\"luidLow\":%lu,\"software\":%s}\n",
                        index, desc.VendorId, desc.DeviceId, desc.AdapterLuid.HighPart,
                        desc.AdapterLuid.LowPart, software ? "true" : "false");
            std::fflush(stdout);
            if (software)
                continue;
            if (SUCCEEDED(step("D3D12CreateDevice", D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0,
                                                                      IID_PPV_ARGS(&device)))))
                break;
        }
        require("nonSoftwareDevice", device ? S_OK : DXGI_ERROR_UNSUPPORTED);
        D3D12_FEATURE_DATA_D3D12_OPTIONS options{};
        if (SUCCEEDED(
                step("CheckFeatureSupportD3D12Options",
                     device->CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS, &options, sizeof(options)))))
        {
            std::printf("{\"event\":\"dx12Features\",\"resourceBindingTier\":%u}\n",
                        static_cast<unsigned>(options.ResourceBindingTier));
            std::fflush(stdout);
        }
        D3D12_COMMAND_QUEUE_DESC queueDesc{};
        queueDesc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
        ComPtr<ID3D12CommandQueue> queue;
        require("CreateCommandQueue", device->CreateCommandQueue(&queueDesc, IID_PPV_ARGS(&queue)));
        ComPtr<ID3D11Device> device11;
        ComPtr<ID3D11DeviceContext> immediate;
        IUnknown *queues[] = {queue.Get()};
        // Resolve optional APIs explicitly so missing exports produce evidence
        // instead of failing process startup before the first JSON event.
        auto d3d11 = LoadLibraryExW(L"d3d11.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
        require("loadD3D11", d3d11 ? S_OK : HRESULT_FROM_WIN32(GetLastError()));
        auto create11 = reinterpret_cast<decltype(&D3D11On12CreateDevice)>(
            GetProcAddress(d3d11, "D3D11On12CreateDevice"));
        require("resolveD3D11On12CreateDevice", create11 ? S_OK : HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND));
        require("D3D11On12CreateDevice", create11(device.Get(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0,
                                                  queues, 1, 0, &device11, &immediate, nullptr));
        ComPtr<ID3D11On12Device> interop;
        require("queryD3D11On12", device11.As(&interop));
        ComPtr<IDXGIDevice> dxgi;
        require("queryDXGIDevice", device11.As(&dxgi));
        if (createComposition)
        {
            ComPtr<IDCompositionDevice> composition;
            compositionCreated = SUCCEEDED(step("DCompositionCreateDevice2WithDxgiDevice",
                                                createComposition(dxgi.Get(), IID_PPV_ARGS(&composition))));
            if (compositionCreated)
                compositionCreated =
                    SUCCEEDED(step("compositionDeviceNonNull", composition ? S_OK : E_POINTER));
        }
        // Keep dynamically loaded code resident until process teardown, after
        // every COM object and any driver-owned callback has been released.
        D3D12_HEAP_PROPERTIES heap{};
        heap.Type = D3D12_HEAP_TYPE_DEFAULT;
        D3D12_RESOURCE_DESC textureDesc{};
        textureDesc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
        textureDesc.Width = textureDesc.Height = 16;
        textureDesc.DepthOrArraySize = textureDesc.MipLevels = 1;
        textureDesc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        textureDesc.SampleDesc.Count = 1;
        textureDesc.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
        ComPtr<ID3D12Resource> texture;
        require("CreateCommittedResource",
                device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &textureDesc,
                                                D3D12_RESOURCE_STATE_RENDER_TARGET, nullptr,
                                                IID_PPV_ARGS(&texture)));
        D3D11_RESOURCE_FLAGS flags{};
        flags.BindFlags = D3D11_BIND_RENDER_TARGET;
        ComPtr<ID3D11Resource> wrapped;
        require("CreateWrappedResource",
                interop->CreateWrappedResource(texture.Get(), &flags, D3D12_RESOURCE_STATE_RENDER_TARGET,
                                               D3D12_RESOURCE_STATE_COPY_SOURCE, IID_PPV_ARGS(&wrapped)));
        ComPtr<ID3D11RenderTargetView> view;
        require("CreateRenderTargetView", device11->CreateRenderTargetView(wrapped.Get(), nullptr, &view));
        ComPtr<ID3D12Fence> fence;
        require("CreateFence", device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)));
        HANDLE completion = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        require("CreateEvent", completion ? S_OK : HRESULT_FROM_WIN32(GetLastError()));
        ID3D11Resource *resources[] = {wrapped.Get()};
        // New wrappers start acquired. Release once, then exercise reacquisition.
        interop->ReleaseWrappedResources(resources, 1);
        immediate->Flush();
        interop->AcquireWrappedResources(resources, 1);
        float color[] = {0.25f, 0.0f, 0.0f, 0.5f};
        immediate->ClearRenderTargetView(view.Get(), color);
        interop->ReleaseWrappedResources(resources, 1);
        immediate->Flush();
        auto hr = step("queueSignal", queue->Signal(fence.Get(), 1));
        if (SUCCEEDED(hr))
            hr = step("SetEventOnCompletion", fence->SetEventOnCompletion(1, completion));
        if (SUCCEEDED(hr))
        {
            auto wait = WaitForSingleObject(completion, 5000);
            hr = step("fenceWait",
                      wait == WAIT_OBJECT_0
                          ? S_OK
                          : HRESULT_FROM_WIN32(wait == WAIT_TIMEOUT ? ERROR_TIMEOUT : GetLastError()));
        }
        // If idleness cannot be established, avoid releasing possibly in-flight
        // resources explicitly. The OS tears down this diagnostic process.
        if (FAILED(hr))
            ExitProcess(result(false, compositionCreated, compositionWithoutRenderingDevice));
        CloseHandle(completion);
        require("GetDeviceRemovedReason", device->GetDeviceRemovedReason());
        interopPassed = true;
    }
    catch (HRESULT)
    {
    }
    if (comInitialized)
        CoUninitialize();
    return result(interopPassed, compositionCreated, compositionWithoutRenderingDevice);
}
