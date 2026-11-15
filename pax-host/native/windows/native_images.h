#pragma once
#include <deque>
#include <filesystem>
#include <map>
#include <memory>
#include <sstream>
#include <string>
#include <stdexcept>
#include <vector>
#include <winrt/Windows.Graphics.Imaging.h>
#include <winrt/Windows.Storage.h>
#include <winrt/Windows.Storage.Streams.h>
#include <winrt/Microsoft.UI.Xaml.Media.Imaging.h>

namespace pax::windows {
// One source is shared by its mounted consumers. Pending work holds only a weak
// reference: source replacement, node deletion and scene close cannot publish
// an old decode into a new node, even if the native operation finishes late.
struct NativeImageResource {
    winrt::Microsoft::UI::Xaml::Media::ImageSource source{nullptr};
    std::filesystem::path file;
    std::string error;
    uint64_t decoded_bytes = 0;
    bool ready = false;
};
class NativeImages {
    using Resource = NativeImageResource;
    using Action = winrt::Windows::Foundation::IAsyncAction;
    using Status = winrt::Windows::Foundation::AsyncStatus;
    std::map<std::wstring, std::weak_ptr<Resource>> sources_;
    std::deque<std::weak_ptr<Resource>> queued_;
    struct Pending { std::weak_ptr<Resource> resource; Action action; };
    std::vector<Pending> pending_;
    bool closed_ = false;
    static Action load(std::weak_ptr<Resource> weak) {
        auto cancellation = co_await winrt::get_cancellation_token();
        cancellation.enable_propagation();
        auto owner_thread = GetCurrentThreadId();
        char const* stage = "retain source";
        try {
            auto resource = weak.lock();
            if (!resource) co_return;
            auto source = resource->source;
            auto path = winrt::hstring(resource->file.native());
            resource.reset();
            stage = "open file";
            auto file = co_await winrt::Windows::Storage::StorageFile::GetFileFromPathAsync(path);
            stage = "open stream";
            auto stream = co_await file.OpenReadAsync();
            stage = "encoded size";
            if (stream.Size() > 32 * 1024 * 1024)
                throw std::runtime_error("Native image exceeds the 32 MiB encoded-asset budget");
            namespace imaging = winrt::Microsoft::UI::Xaml::Media::Imaging;
            stage = "query image source";
            if (auto svg = source.try_as<imaging::SvgImageSource>()) {
                stage = "set SVG source";
                auto status = co_await svg.SetSourceAsync(stream);
                if (status != imaging::SvgImageSourceLoadStatus::Success)
                    throw std::runtime_error("WinUI could not decode the native SVG image");
            } else {
                stage = "create bitmap decoder";
                auto decoder = co_await winrt::Windows::Graphics::Imaging::BitmapDecoder::CreateAsync(stream);
                stage = "decoded dimensions";
                uint64_t bytes = uint64_t(decoder.PixelWidth()) * decoder.PixelHeight() * 4;
                if (!bytes || bytes > 32 * 1024 * 1024)
                    throw std::runtime_error("Native image exceeds the 32 MiB decoded-asset budget");
                if (auto current = weak.lock()) current->decoded_bytes = bytes;
                stage = "rewind stream";
                stream.Seek(0);
                stage = "set bitmap source";
                co_await source.as<imaging::BitmapImage>().SetSourceAsync(stream);
            }
            stage = "close stream";
            stream.Close();
            if (auto current = weak.lock()) current->ready = true;
        } catch (winrt::hresult_canceled const&) {
            // Cancellation accompanies retirement; it is not a live asset error.
        } catch (winrt::hresult_error const& error) {
            if (auto current = weak.lock()) {
                std::ostringstream detail;
                detail << stage << ": HRESULT=0x" << std::hex << uint32_t(error.code().value)
                    << std::dec << " owner_thread=" << owner_thread
                    << " current_thread=" << GetCurrentThreadId() << ": "
                    << winrt::to_string(error.message());
                current->error = detail.str();
            }
        } catch (std::exception const& error) {
            if (auto current = weak.lock()) current->error = error.what();
        }
    }
public:
    ~NativeImages() { close(); }
    std::shared_ptr<Resource> acquire(std::filesystem::path const& file) {
        if (closed_) throw std::runtime_error("Native image requested after scene close");
        auto key = file.native();
        if (auto found = sources_.find(key); found != sources_.end())
            if (auto resource = found->second.lock()) return resource;
        auto resource = std::make_shared<Resource>(); resource->file = file;
        namespace imaging = winrt::Microsoft::UI::Xaml::Media::Imaging;
        // The native Image first supplies SVG intrinsic dimensions through
        // layout. The scene then pins the source raster at that ratio before
        // applying fit, avoiding WinUI's second aspect-preserving stretch.
        // SVG backing storage is separate from the bitmap byte count.
        if (_wcsicmp(file.extension().c_str(), L".svg") == 0) resource->source = imaging::SvgImageSource();
        else {
            imaging::BitmapImage bitmap;
            bitmap.AutoPlay(false); // This host's image contract is a still frame.
            resource->source = bitmap;
        }
        sources_[key] = resource; queued_.push_back(resource);
        return resource;
    }
    void rasterize_svg(std::shared_ptr<Resource> const& resource, double width, double height) {
        if (closed_) throw std::runtime_error("Native SVG rasterization requested after scene close");
        auto svg = resource->source.as<winrt::Microsoft::UI::Xaml::Media::Imaging::SvgImageSource>();
        resource->ready = false;
        svg.RasterizePixelWidth(width); svg.RasterizePixelHeight(height);
        // Changing raster properties after the first decode retains its old
        // viewport in WinUI. Reload once with the bounded intrinsic-ratio
        // viewport installed before SetSourceAsync. Mounted consumers share it.
        queued_.push_back(resource);
    }
    void poll() {
        if (closed_) return;
        for (auto it = pending_.begin(); it != pending_.end();) {
            if (it->resource.expired()) it->action.Cancel();
            if (it->action.Status() == Status::Started) { ++it; continue; }
            // load captures and reports failures through the live resource.
            if (it->action.Status() != Status::Canceled) it->action.GetResults();
            it = pending_.erase(it);
        }
        for (auto it = sources_.begin(); it != sources_.end();) {
            auto resource = it->second.lock();
            if (!resource) { it = sources_.erase(it); continue; }
            if (!resource->error.empty())
                throw std::runtime_error("Native image " + winrt::to_string(winrt::hstring(resource->file.native())) + ": " + resource->error);
            ++it;
        }
        while (pending_.size() < 8 && !queued_.empty()) {
            auto weak = queued_.front(); queued_.pop_front();
            if (!weak.expired()) pending_.push_back({weak, load(weak)});
        }
    }
    void close() noexcept {
        if (closed_) return;
        closed_ = true;
        for (auto& request : pending_) { try { request.action.Cancel(); } catch (...) {} }
        pending_.clear(); queued_.clear(); sources_.clear();
    }
    size_t source_count() const {
        size_t count = 0;
        for (auto const& [path, weak] : sources_) if (!weak.expired()) ++count;
        return count;
    }
    size_t pending_count() const { return pending_.size(); }
    uint64_t bitmap_bytes() const {
        uint64_t bytes = 0;
        for (auto const& [path, weak] : sources_) if (auto source = weak.lock()) bytes += source->decoded_bytes;
        return bytes;
    }
    std::string diagnostics() const {
        size_t ready = 0, failed = 0, queued = 0;
        std::string paths;
        for (auto const& [path, weak] : sources_) if (auto source = weak.lock()) {
            ready += source->ready; failed += !source->error.empty();
            if (!paths.empty()) paths += ";";
            paths += winrt::to_string(winrt::hstring(source->file.native())) + (source->ready ? "[ready]" : "[pending]");
        }
        for (auto const& weak : queued_) if (!weak.expired()) ++queued;
        return "native_images sources=" + std::to_string(source_count()) +
            " ready=" + std::to_string(ready) + " failed=" + std::to_string(failed) +
            " pending=" + std::to_string(pending_count()) + " queued=" + std::to_string(queued) +
            " bitmap_bytes=" + std::to_string(bitmap_bytes()) + " paths=" + paths;
    }
};
} // namespace pax::windows
