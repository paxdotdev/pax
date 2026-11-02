#include <windows.h>
#include <unknwn.h>
#include <MddBootstrap.h>
#include <WindowsAppSDK-VersionInfo.h>
#undef GetCurrentTime
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.UI.Xaml.Interop.h>
#include <winrt/Microsoft.UI.Dispatching.h>
#include <winrt/Microsoft.UI.Xaml.Markup.h>
#include <winrt/Microsoft.UI.Xaml.XamlTypeInfo.h>
#include <chrono>
#include <fstream>
#include <limits>
#include "json_helpers.h"
#include "snap_canvas.h"

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace pax::windows;

namespace {
using SnapInfo = Primitives::IScrollSnapPointsInfo;
using Alignment = Primitives::SnapPointsAlignment;
using Points = winrt::Windows::Foundation::Collections::IVectorView<float>;
std::ofstream output;
int process_result = 1;
unsigned checks = 0;

void record(char const* stage, bool passed, std::string const& detail) {
    json::JsonObject row;
    row.Insert(L"stage", text(to_hstring(stage)));
    row.Insert(L"passed", json::JsonValue::CreateBooleanValue(passed));
    row.Insert(L"detail", text(to_hstring(detail)));
    output << to_string(row.Stringify()) << '\n'; output.flush();
}
void check(char const* stage, bool passed, std::string const& detail) {
    ++checks; record(stage, passed, detail);
    if (!passed) throw std::runtime_error(stage);
}
std::vector<float> values(Points const& points) {
    return std::vector<float>(points.begin(), points.end());
}
std::string describe(std::vector<float> const& points) {
    std::string result = "[";
    for (float point : points) { if (result.size() > 1) result += ','; result += std::to_string(point); }
    return result + ']';
}
void expect_points(char const* stage, Points const& points, std::vector<float> const& expected) {
    auto actual = values(points);
    check(stage, actual == expected, "actual=" + describe(actual) + "; expected=" + describe(expected));
}
template<class F> void expect_invalid(char const* stage, F&& action) {
    bool rejected = false;
    try { action(); }
    catch (std::invalid_argument const&) { rejected = true; }
    catch (hresult_error const& error) { if (error.code() != E_INVALIDARG) throw; rejected = true; }
    check(stage, rejected, "invalid input must fail before publishing state");
}

struct App : ApplicationT<App, Markup::IXamlMetadataProvider> {
    XamlTypeInfo::XamlControlsXamlMetaDataProvider metadata{nullptr};
    Window window{nullptr};
    ScrollViewer scroll{nullptr};
    winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer timer{nullptr};
    XamlTypeInfo::XamlControlsXamlMetaDataProvider provider() {
        if (!metadata) metadata = XamlTypeInfo::XamlControlsXamlMetaDataProvider();
        return metadata;
    }
    Markup::IXamlType GetXamlType(hstring const& name) { return provider().GetXamlType(name); }
    Markup::IXamlType GetXamlType(winrt::Windows::UI::Xaml::Interop::TypeName const& type) { return provider().GetXamlType(type); }
    com_array<Markup::XmlnsDefinition> GetXmlnsDefinitions() { return provider().GetXmlnsDefinitions(); }

    void finish(bool passed) {
        if (timer) timer.Stop();
        if (scroll) scroll.Content(nullptr);
        if (window) window.Close();
        process_result = passed ? 0 : 1;
        record("complete", passed, "checks=" + std::to_string(checks) + "; native content detached; Application exit requested");
        Exit();
    }

    void run() {
        try {
            timer.Stop();
            auto implementation = make_self<SnapCanvas>();
            auto canvas = implementation.as<Canvas>();
            auto native = canvas.as<SnapInfo>();
            canvas.Width(500); canvas.Height(600);
            Button child; child.Content(box_value(L"Native child")); child.Width(80); child.Height(30);
            Canvas::SetLeft(child, 12); Canvas::SetTop(child, 18); canvas.Children().Append(child);
            scroll.Content(canvas); scroll.UpdateLayout();
            check("native-canvas-and-interface", bool(native.try_as<Canvas>()) &&
                scroll.Content().try_as<SnapInfo>() != nullptr, "direct native ScrollViewer content implements the interface");
            check("native-child-preserved", canvas.Children().Size() == 1 &&
                Canvas::GetLeft(child) == 12 && Canvas::GetTop(child) == 18 && child.IsHitTestVisible(),
                "real Button remains an absolute Canvas child; pointer delivery is qualified separately");
            check("irregular-axes", !native.AreHorizontalSnapPointsRegular() && !native.AreVerticalSnapPointsRegular(),
                "both axes expose irregular points");
            expect_points("initial-horizontal-empty", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {});
            expect_points("initial-vertical-empty", native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near), {});
            float offset = 999;
            check("regular-fallback", native.GetRegularSnapPoints(Orientation::Horizontal, Alignment::Near, offset) == 0 && offset == 0,
                "unused regular-point method initializes its out parameter");

            unsigned horizontal_events = 0, vertical_events = 0;
            bool senders_valid = true;
            struct Notification { std::vector<float> x, y; };
            std::vector<Notification> notifications;
            auto observe = [&](auto const& sender, bool horizontal) {
                if (horizontal) ++horizontal_events; else ++vertical_events;
                senders_valid = senders_valid && bool(sender.template try_as<SnapInfo>());
                notifications.push_back({values(native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near)),
                    values(native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near))});
            };
            auto horizontal_token = native.HorizontalSnapPointsChanged([&](auto const& sender, auto const&) { observe(sender, true); });
            auto vertical_token = native.VerticalSnapPointsChanged([&](auto const& sender, auto const&) { observe(sender, false); });
            // Revoke even after a failed assertion: delegates capture these stack locals.
            struct Subscriptions {
                SnapInfo source; event_token x, y;
                ~Subscriptions() { source.HorizontalSnapPointsChanged(x); source.VerticalSnapPointsChanged(y); }
            } subscriptions{native, horizontal_token, vertical_token};
            auto expect_events = [&](char const* stage, unsigned x, unsigned y) {
                check(stage, horizontal_events == x && vertical_events == y,
                    "actual=" + std::to_string(horizontal_events) + "," + std::to_string(vertical_events) +
                    "; expected=" + std::to_string(x) + "," + std::to_string(y));
            };
            auto expect_revision = [&](char const* stage, uint64_t expected) {
                check(stage, implementation->revision() == expected,
                    "actual=" + std::to_string(implementation->revision()) + "; expected=" + std::to_string(expected));
            };

            expect_revision("initial-revision", 0);
            implementation->update_extents(100, 100, 500, 600);
            expect_events("empty-extent-change-silent", 0, 0);
            expect_revision("empty-extent-change-advances-revision", 1);
            implementation->set_horizontal_points({300, 100, 100, -10, 900});
            expect_points("horizontal-sort-clamp-deduplicate", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {0, 100, 300, 400});
            expect_events("horizontal-only-notification", 1, 0);
            expect_revision("horizontal-points-advance-revision", 2);
            implementation->set_vertical_points({450, 250, 250, -20, 800});
            expect_points("vertical-sort-clamp-deduplicate", native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near), {0, 250, 450, 500});
            expect_events("vertical-only-notification", 1, 1);
            expect_revision("vertical-points-advance-revision", 3);
            check("populated-axes", implementation->has_horizontal_points() && implementation->has_vertical_points(), "both axes enable native snapping");
            implementation->set_horizontal_points({300, 100, 100, -10, 900});
            implementation->set_vertical_points({450, 250, 250, -20, 800});
            implementation->update_extents(100, 100, 500, 600);
            expect_events("identical-updates-silent", 1, 1);
            expect_revision("identical-updates-preserve-revision", 3);
            implementation->set_horizontal_points({900, 100, 300, -10, 100});
            expect_events("equivalent-effective-points-silent", 1, 1);
            expect_revision("equivalent-effective-points-preserve-revision", 3);

            auto prior_x = native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near);
            auto prior_y = native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near);
            implementation->update_extents(100, 100, 250, 300);
            expect_points("shrink-horizontal", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {0, 100, 150});
            expect_points("shrink-vertical", native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near), {0, 200});
            expect_events("shrink-both-notifications", 2, 2);
            expect_revision("both-axis-shrink-advances-revision-once", 4);
            check("both-axes-published-before-notifications", notifications.size() == 4 &&
                notifications[2].x == std::vector<float>({0, 100, 150}) && notifications[2].y == std::vector<float>({0, 200}) &&
                notifications[3].x == notifications[2].x && notifications[3].y == notifications[2].y,
                "each extent-change callback observes both new arrays");
            expect_points("old-horizontal-view-immutable", prior_x, {0, 100, 300, 400});
            expect_points("old-vertical-view-immutable", prior_y, {0, 250, 450, 500});
            implementation->update_extents(100, 100, 500, 600);
            expect_points("regrow-restores-authored-horizontal", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {0, 100, 300, 400});
            expect_points("regrow-restores-authored-vertical", native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near), {0, 250, 450, 500});
            expect_events("regrow-both-notifications", 3, 3);
            expect_revision("both-axis-regrow-advances-revision-once", 5);
            implementation->update_extents(150, 200, 500, 600);
            expect_points("viewport-resize-horizontal", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {0, 100, 300, 350});
            expect_points("viewport-resize-vertical", native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near), {0, 250, 400});
            expect_events("viewport-resize-notifications", 4, 4);
            expect_revision("viewport-resize-advances-revision", 6);

            implementation->set_horizontal_points({});
            expect_points("clear-horizontal", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {});
            expect_points("clear-retains-other-axis", native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near), {0, 250, 400});
            check("horizontal-cleared-flag", !implementation->has_horizontal_points() && implementation->has_vertical_points(), "one axis cleared independently");
            expect_events("clear-horizontal-notification", 5, 4);
            expect_revision("clear-horizontal-advances-revision", 7);
            implementation->set_horizontal_points({});
            implementation->set_vertical_points({});
            implementation->set_vertical_points({});
            expect_events("clear-repeated-silent", 5, 5);
            expect_revision("repeated-clear-preserves-revision", 8);
            check("both-cleared-flags", !implementation->has_horizontal_points() && !implementation->has_vertical_points(), "empty lists disable both axes");
            implementation->set_horizontal_points({100, std::nextafter(100.0, 101.0)});
            expect_points("float-representation-deduplicated", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {100});
            expect_events("refill-notification", 6, 5);
            expect_revision("refill-advances-revision", 9);
            expect_invalid("reject-nan-point", [&] { implementation->set_horizontal_points({std::numeric_limits<double>::quiet_NaN()}); });
            expect_invalid("reject-infinite-point", [&] { implementation->set_vertical_points({std::numeric_limits<double>::infinity()}); });
            expect_invalid("reject-negative-extent", [&] { implementation->update_extents(150, 200, 500, -1); });
            expect_invalid("reject-infinite-extent", [&] { implementation->update_extents(150, 200, std::numeric_limits<double>::infinity(), 600); });
            expect_points("invalid-update-preserves-horizontal", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {100});
            expect_points("invalid-update-preserves-vertical", native.GetIrregularSnapPoints(Orientation::Vertical, Alignment::Near), {});
            expect_events("invalid-updates-silent", 6, 5);
            expect_revision("invalid-updates-preserve-revision", 9);
            check("event-senders-implement-interface", senders_valid, "Changed event sender is queryable as the native provider");
            native.HorizontalSnapPointsChanged(horizontal_token);
            implementation->set_horizontal_points({200});
            expect_events("revoked-subscription-silent", 6, 5);
            expect_points("revocation-preserves-provider", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {200});
            expect_revision("unobserved-point-change-advances-revision", 10);
            // Both dimensions change while each scrollable range stays the same.
            // The owner must still refresh native geometry without inventing a
            // point-list change notification or retargeting identical patches.
            implementation->update_extents(151, 201, 501, 601);
            expect_revision("unchanged-points-new-extents-advance-revision", 11);
            expect_points("new-extents-retain-effective-points", native.GetIrregularSnapPoints(Orientation::Horizontal, Alignment::Near), {200});
            expect_events("unchanged-points-new-extents-no-notifications", 6, 5);
            implementation->update_extents(151, 201, 501, 601);
            expect_revision("repeated-new-extents-preserve-revision", 11);
            finish(true);
        } catch (hresult_error const& error) { record("failure", false, to_string(error.message())); finish(false); }
          catch (std::exception const& error) { record("failure", false, error.what()); finish(false); }
    }

    void OnLaunched(LaunchActivatedEventArgs const&) {
        try {
            Resources().MergedDictionaries().Append(XamlControlsResources());
            scroll = ScrollViewer(); scroll.Width(100); scroll.Height(100);
            scroll.HorizontalScrollMode(ScrollMode::Enabled); scroll.VerticalScrollMode(ScrollMode::Enabled);
            scroll.HorizontalScrollBarVisibility(ScrollBarVisibility::Hidden); scroll.VerticalScrollBarVisibility(ScrollBarVisibility::Hidden);
            scroll.HorizontalSnapPointsAlignment(Alignment::Near); scroll.VerticalSnapPointsAlignment(Alignment::Near);
            window = Window(); window.Title(L"Pax snap point contract"); window.Content(scroll); window.Activate();
            timer = winrt::Microsoft::UI::Dispatching::DispatcherQueue::GetForCurrentThread().CreateTimer();
            timer.Interval(std::chrono::milliseconds(100)); timer.Tick([this](auto const&, auto const&) { run(); }); timer.Start();
        } catch (hresult_error const& error) { record("startup", false, to_string(error.message())); finish(false); }
          catch (std::exception const& error) { record("startup", false, error.what()); finish(false); }
    }
};
}

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int) {
    wchar_t log_path[32768];
    auto length = GetEnvironmentVariableW(L"PAX_SNAP_CONTRACT_LOG", log_path, DWORD(std::size(log_path)));
    output.open(length && length < std::size(log_path) ? log_path : L"snap-point-contract.jsonl");
    if (!output) return 2;
    try {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        PACKAGE_VERSION minimum{}; minimum.Version = WINDOWSAPPSDK_RUNTIME_VERSION_UINT64;
        check_hresult(MddBootstrapInitialize2(WINDOWSAPPSDK_RELEASE_MAJORMINOR,
            WINDOWSAPPSDK_RELEASE_VERSION_TAG_W, minimum, MddBootstrapInitializeOptions_None));
        init_apartment(apartment_type::single_threaded);
        Application::Start([](auto const&) { make<App>(); });
        record("application-returned", process_result == 0, "require process exit code zero as well");
        return process_result;
    } catch (hresult_error const& error) { record("fatal", false, to_string(error.message())); }
      catch (std::exception const& error) { record("fatal", false, error.what()); }
    return 1;
}
