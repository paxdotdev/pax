#pragma once
#include "json_helpers.h"
#include <deque>
#include <filesystem>
#include <future>
#include <map>
#include <set>
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
    struct Request { winrt::hstring path; std::filesystem::path file; };
    std::deque<Request> queued_;
    std::set<std::wstring> requested_;
    std::filesystem::path executable_;
    bool stopped_ = false;
    void start_queued() {
        while (!stopped_ && pending_.size() < 8 && !queued_.empty()) {
            auto const& request = queued_.front();
            pending_.emplace(request.path.c_str(), decode(request));
            queued_.pop_front();
        }
    }
    static std::future<DecodedImage> decode(Request const& request) {
        return std::async(std::launch::async, [file = request.file, path = request.path] {
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
        });
    }
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
        // The web-facing asset-root spelling is portable; other rooted paths
        // and network URLs remain outside the packaged Windows asset policy.
        std::wstring spelling(path);
        if (spelling.starts_with(L"/assets/")) spelling.erase(0, 1);
        std::filesystem::path relative(spelling);
        if (relative.empty()) throw std::invalid_argument("Empty Pax asset path");
        if (relative.is_absolute() || relative.has_root_name()) throw std::invalid_argument("Pax assets must be relative to a configured asset root");
        if (spelling.find(L':') != std::wstring::npos) throw std::invalid_argument("Pax asset paths cannot contain a URI scheme or alternate stream");
        relative = relative.lexically_normal();
        if (*relative.begin() == L"assets") relative = relative.lexically_relative(L"assets");
        for (auto const& part : relative) if (part == L"..") throw std::invalid_argument("Pax asset path escapes its root");
        for (size_t i = 0; i < roots_.size(); ++i) {
            auto candidate = roots_[i] / relative;
            if (std::filesystem::is_regular_file(candidate)) return {i, relative};
        }
        throw std::runtime_error("Pax asset not found: " + winrt::to_string(path));
    }
    std::filesystem::path native_image(winrt::hstring const& path) const {
        auto [index, relative] = resolve(path);
        auto root = std::filesystem::canonical(roots_[index]);
        auto file = std::filesystem::canonical(root / relative);
        auto component = file.begin();
        for (auto const& part : root) {
            if (component == file.end() || _wcsicmp(component->c_str(), part.c_str()) != 0)
                throw std::invalid_argument("Native image asset resolves outside its configured root");
            ++component;
        }
        return file;
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
        if (stopped_ || requested_.contains(path.c_str())) return;
        auto [index, relative] = resolve(path);
        // Bursts can exceed the decoder budget; queue paths, not decoded pixels.
        // Resolve synchronously so invalid paths still fail at the request site.
        queued_.push_back({path, roots_[index] / relative});
        requested_.emplace(path.c_str());
        start_queued();
    }
    template<typename F> void poll(F&& consume) {
        for (auto it = pending_.begin(); it != pending_.end();) {
            if (it->second.wait_for(std::chrono::seconds(0)) != std::future_status::ready) { ++it; continue; }
            auto future = std::move(it->second);
            requested_.erase(it->first);
            it = pending_.erase(it);
            // get() may throw; the consumed future must not remain pending.
            consume(future.get());
        }
        start_queued();
    }
    void stop() {
        // Closing drains only work already running, never the scene's backlog.
        stopped_ = true;
        queued_.clear();
        requested_.clear();
    }
    bool idle() const { return pending_.empty() && queued_.empty(); }
};
} // namespace pax::windows
