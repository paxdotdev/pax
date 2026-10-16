#pragma once
#include "json_helpers.h"
#include <filesystem>
#include <future>
#include <map>
#include <vector>
#include <wincodec.h>

namespace pax::windows {
struct DecodedImage {
    std::string path;
    uint32_t width = 0, height = 0;
    std::vector<uint8_t> rgba;
};
class Assets {
    std::vector<std::filesystem::path> roots_;
    std::vector<std::filesystem::path> font_roots_;
    std::map<std::wstring, std::future<DecodedImage>> pending_;
    std::filesystem::path executable_;
public:
    void configure(json::JsonObject const& configuration) {
        wchar_t executable[32768];
        auto size = GetModuleFileNameW(nullptr, executable, DWORD(std::size(executable)));
        if (!size || size == std::size(executable)) winrt::throw_last_error();
        executable_ = std::filesystem::path(executable).parent_path();
        if (present(configuration, L"assets")) for (auto const& root : configuration.GetNamedArray(L"assets"))
            roots_.emplace_back(root.GetString().c_str());
        if (present(configuration, L"fontAssets")) for (auto const& root : configuration.GetNamedArray(L"fontAssets"))
            font_roots_.emplace_back(root.GetString().c_str());
        else font_roots_ = roots_;
    }
    std::pair<size_t,std::filesystem::path> resolve(winrt::hstring const& path) const {
        std::filesystem::path relative(path.c_str());
        if (relative.empty()) throw std::invalid_argument("Empty Pax asset path");
        if (relative.is_absolute() || relative.has_root_name()) throw std::invalid_argument("Pax assets must be relative to a configured asset root");
        relative = relative.lexically_normal();
        if (*relative.begin() == L"assets") relative = relative.lexically_relative(L"assets");
        for (auto const& part : relative) if (part == L"..") throw std::invalid_argument("Pax asset path escapes its root");
        for (size_t i = 0; i < roots_.size(); ++i) {
            auto candidate = roots_[i] / relative;
            if (std::filesystem::is_regular_file(candidate)) return {i, relative};
        }
        throw std::runtime_error("Pax asset not found: " + winrt::to_string(path));
    }
    winrt::hstring font(json::JsonObject const& metadata) const {
        auto path = metadata.GetNamedString(L"path");
        auto [root, relative] = resolve(path);
        // Fonts must be staged by Cargo/CLI beside the executable: WinUI's
        // resource loader does not support arbitrary file:// font URIs.
        if (root >= font_roots_.size()) throw std::runtime_error("Missing staged native font asset root");
        auto staged = (font_roots_[root] / relative).lexically_normal();
        if (!std::filesystem::is_regular_file(staged))
            throw std::runtime_error("Local font is not staged in executable-relative assets: " + staged.string());
        auto executable_relative = staged.lexically_relative(executable_);
        if (executable_relative.empty() || executable_relative.is_absolute())
            throw std::runtime_error("Native font assets must be inside the executable directory");
        for (auto const& part : executable_relative) if (part == L"..")
            throw std::runtime_error("Native font assets must be inside the executable directory");
        auto relative_uri = executable_relative.generic_wstring();
        std::wstring escaped;
        for (auto c : relative_uri) {
            if (c == L' ') escaped += L"%20"; else if (c == L'#') escaped += L"%23";
            else if (c == L'%') escaped += L"%25"; else escaped += c;
        }
        auto family = string(metadata, L"family");
        if (family.empty()) throw std::invalid_argument("A local native font requires its font family name");
        return winrt::hstring(L"ms-appx:///" + escaped + L"#" + std::wstring(family));
    }
    void request(winrt::hstring const& path) {
        if (pending_.contains(path.c_str())) return;
        if (pending_.size() == 8) throw std::runtime_error("Windows image decoding exceeds eight pending assets");
        auto [index, relative] = resolve(path);
        auto file = roots_[index] / relative;
        pending_.emplace(path.c_str(), std::async(std::launch::async, [file, path] {
            winrt::init_apartment(winrt::apartment_type::multi_threaded);
            struct Apartment { ~Apartment() { winrt::uninit_apartment(); } } apartment;
            winrt::com_ptr<IWICImagingFactory> factory;
            winrt::check_hresult(CoCreateInstance(CLSID_WICImagingFactory, nullptr, CLSCTX_INPROC_SERVER,
                __uuidof(IWICImagingFactory), factory.put_void()));
            winrt::com_ptr<IWICBitmapDecoder> decoder;
            winrt::check_hresult(factory->CreateDecoderFromFilename(file.c_str(), nullptr, GENERIC_READ,
                WICDecodeMetadataCacheOnDemand, decoder.put()));
            winrt::com_ptr<IWICBitmapFrameDecode> frame;
            winrt::check_hresult(decoder->GetFrame(0, frame.put()));
            DecodedImage image; image.path = winrt::to_string(path);
            winrt::check_hresult(frame->GetSize(&image.width, &image.height));
            uint64_t size = uint64_t(image.width) * image.height * 4;
            if (!image.width || !image.height || size > 32 * 1024 * 1024)
                throw std::runtime_error("Decoded image exceeds the 32 MiB per-asset budget");
            winrt::com_ptr<IWICFormatConverter> converter;
            winrt::check_hresult(factory->CreateFormatConverter(converter.put()));
            winrt::check_hresult(converter->Initialize(frame.get(), GUID_WICPixelFormat32bppRGBA,
                WICBitmapDitherTypeNone, nullptr, 0, WICBitmapPaletteTypeCustom));
            image.rgba.resize(size);
            winrt::check_hresult(converter->CopyPixels(nullptr, image.width * 4, UINT(size), image.rgba.data()));
            return image;
        }));
    }
    template<typename F> void poll(F&& consume) {
        for (auto it = pending_.begin(); it != pending_.end();) {
            if (it->second.wait_for(std::chrono::seconds(0)) != std::future_status::ready) { ++it; continue; }
            auto value = it->second.get(); it = pending_.erase(it); consume(std::move(value));
        }
    }
    bool idle() const { return pending_.empty(); }
};
} // namespace pax::windows
