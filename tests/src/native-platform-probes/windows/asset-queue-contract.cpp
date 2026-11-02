#include <windows.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <algorithm>
#include <fstream>
#include <set>
#include <thread>
#include "assets.h"

using namespace pax::windows;

static std::wstring environment(wchar_t const* name) {
    wchar_t value[32768];
    auto size = GetEnvironmentVariableW(name, value, DWORD(std::size(value)));
    if (!size || size >= std::size(value)) throw std::runtime_error("Missing asset test path");
    return value;
}

static void require(bool value, char const* message) {
    if (!value) throw std::runtime_error(message);
}

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int) {
    auto output = std::filesystem::path(environment(L"PAX_ASSET_QUEUE_OUTPUT"));
    std::ofstream log(output / L"result.jsonl");
    try {
        winrt::init_apartment(winrt::apartment_type::multi_threaded);
        auto root = std::filesystem::path(environment(L"PAX_ASSET_QUEUE_ROOT"));
        std::vector<std::filesystem::path> files;
        for (auto const& entry : std::filesystem::directory_iterator(root / L"thumbs"))
            if (entry.is_regular_file()) files.push_back(entry.path());
        std::sort(files.begin(), files.end());
        require(files.size() >= 10, "Need ten distinct real thumbnail assets");
        std::vector<winrt::hstring> paths;
        for (size_t i = 0; i < 10; ++i)
            paths.emplace_back(files[i].lexically_relative(root).generic_wstring());
        json::JsonArray roots;
        roots.Append(text(winrt::hstring(root.wstring())));
        roots.Append(text(winrt::hstring(output.wstring())));
        json::JsonObject config;
        config.Insert(L"assets", roots);

        Assets assets;
        assets.configure(config);
        for (auto const& path : paths) assets.request(path);
        assets.request(paths.front()); // Already active.
        assets.request(paths.back()); // Still queued until the first poll.
        require(!assets.idle(), "Queued work must prevent idle");
        bool missing_rejected = false;
        try { assets.request(L"__missing_queue_contract__.png"); }
        catch (std::runtime_error const&) { missing_rejected = true; }
        require(missing_rejected, "Missing assets must fail synchronously");
        std::set<std::string> completed;
        auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(15);
        while (!assets.idle() && std::chrono::steady_clock::now() < deadline) {
            assets.poll([&](DecodedImage image) {
                require(image.width && image.height &&
                    image.rgba.size() == size_t(image.width) * image.height * 4,
                    "Invalid decoded pixels");
                require(completed.insert(image.path).second, "Duplicate asset completion");
            });
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        require(assets.idle() && completed.size() == 10, "Burst did not drain ten distinct assets");
        log << "{\"case\":\"burst\",\"decoded\":10,\"activeAndQueuedDuplicates\":true,"
               "\"missingRejectedSynchronously\":true,\"idle\":true}\n";

        // Completed requests may be issued again; deduplication covers outstanding work.
        assets.request(paths.front());
        size_t repeated = 0;
        while (!assets.idle() && std::chrono::steady_clock::now() < deadline) {
            assets.poll([&](DecodedImage) { ++repeated; });
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        require(assets.idle() && repeated == 1, "Completed asset could not be requested again");
        log << "{\"case\":\"repeatAfterCompletion\",\"decoded\":1}\n";

        // Decode failures remain visible at poll, including requests that began queued.
        std::ofstream(output / L"corrupt.png", std::ios::binary) << "not an image";
        bool decode_error = false;
        {
            Assets invalid;
            invalid.configure(config);
            for (size_t i = 0; i < 8; ++i) invalid.request(paths[i]);
            invalid.request(L"corrupt.png");
            deadline = std::chrono::steady_clock::now() + std::chrono::seconds(15);
            while (!invalid.idle() && std::chrono::steady_clock::now() < deadline) {
                try { invalid.poll([](DecodedImage) {}); }
                catch (winrt::hresult_error const&) { decode_error = true; break; }
                std::this_thread::sleep_for(std::chrono::milliseconds(10));
            }
            invalid.stop();
            while (!invalid.idle() && std::chrono::steady_clock::now() < deadline) {
                invalid.poll([](DecodedImage) {});
                std::this_thread::sleep_for(std::chrono::milliseconds(10));
            }
            require(invalid.idle(), "Decode error left an invalid future pending");
        }
        require(decode_error, "Queued decode failure was hidden");
        log << "{\"case\":\"queuedDecodeError\",\"propagated\":true}\n";

        // Mirror Session::close: stop first, then keep polling active work to idle.
        // The corrupt ninth request must never start after close.
        {
            Assets closing;
            closing.configure(config);
            for (size_t i = 0; i < 8; ++i) closing.request(paths[i]);
            closing.request(L"corrupt.png");
            closing.request(paths.back());
            closing.stop();
            closing.stop();
            closing.request(L"corrupt.png");
            size_t drained = 0;
            deadline = std::chrono::steady_clock::now() + std::chrono::seconds(15);
            while (!closing.idle() && std::chrono::steady_clock::now() < deadline) {
                closing.poll([&](DecodedImage) { ++drained; });
                std::this_thread::sleep_for(std::chrono::milliseconds(10));
            }
            require(closing.idle() && drained == 8, "Close started queued work or failed to drain");
            log << "{\"case\":\"normalCloseWithBacklog\",\"activeDrained\":8,"
                   "\"queuedCorruptDiscarded\":true,\"idempotent\":true,\"idle\":true}\n";
        }

        auto shutdown_started = std::chrono::steady_clock::now();
        {
            Assets abandoned;
            abandoned.configure(config);
            for (auto const& path : paths) abandoned.request(path);
        }
        auto shutdown_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - shutdown_started).count();
        require(shutdown_ms < 15000, "Outstanding asset shutdown exceeded deadline");
        log << "{\"case\":\"shutdownWithBacklog\",\"milliseconds\":" << shutdown_ms << "}\n";
        log << "{\"passed\":true}\n";
        return 0;
    } catch (winrt::hresult_error const& error) {
        log << "{\"error\":" << winrt::to_string(text(error.message()).Stringify()) << "}\n";
    } catch (std::exception const& error) {
        log << "{\"error\":" << winrt::to_string(text(winrt::to_hstring(error.what())).Stringify()) << "}\n";
    }
    return 1;
}
