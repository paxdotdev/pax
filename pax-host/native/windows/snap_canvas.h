#pragma once
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <utility>
#include <vector>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Controls.Primitives.h>

namespace pax::windows {

// Retain authored anchors and resolve legal landing points in local pixels.
// ScrollViewer consumes the public IScrollSnapPointsInfo content interface.
// Canvas layout, original wheel routing, and momentum stay native.
struct SnapCanvas : winrt::Microsoft::UI::Xaml::Controls::CanvasT<
    SnapCanvas, winrt::Microsoft::UI::Xaml::Controls::Primitives::IScrollSnapPointsInfo> {
private:
    using Orientation = winrt::Microsoft::UI::Xaml::Controls::Orientation;
    using Alignment = winrt::Microsoft::UI::Xaml::Controls::Primitives::SnapPointsAlignment;
    using SnapInfo = winrt::Microsoft::UI::Xaml::Controls::Primitives::IScrollSnapPointsInfo;
    using Points = winrt::Windows::Foundation::Collections::IVectorView<float>;
    using Changed = winrt::Windows::Foundation::EventHandler<winrt::Windows::Foundation::IInspectable>;

    struct Axis {
        // Retain the unclamped values so growing the content restores anchors
        // that were outside the previous scrollable extent.
        std::vector<double> authored;
        std::vector<float> effective;
        Points view = winrt::single_threaded_vector<float>().GetView();
    };
    Axis horizontal_, vertical_;
    uint64_t revision_ = 0;
    double viewport_width_ = 0, viewport_height_ = 0;
    double content_width_ = 0, content_height_ = 0;
    winrt::event<Changed> horizontal_changed_, vertical_changed_;

    static void validate_extent(double value) {
        if (!std::isfinite(value) || value < 0 || value > std::numeric_limits<float>::max())
            throw std::invalid_argument("Scroller snap extents must be finite, nonnegative XAML dimensions");
    }

    static std::vector<float> resolve(std::vector<double> const& points, double viewport, double content) {
        const double maximum = std::max(0.0, content - viewport);
        std::vector<float> result;
        result.reserve(points.size());
        for (double point : points) {
            if (!std::isfinite(point)) throw std::invalid_argument("Scroller snap points must be finite");
            result.push_back(static_cast<float>(std::clamp(point, 0.0, maximum)));
        }
        std::sort(result.begin(), result.end());
        result.erase(std::unique(result.begin(), result.end()), result.end());
        return result;
    }

    static Points make_view(std::vector<float> const& values) {
        // Replace the collection instead of mutating it: a native consumer may
        // retain an earlier IVectorView while processing a manipulation.
        return winrt::single_threaded_vector<float>(std::vector<float>(values)).GetView();
    }

    void set_points(Axis& axis, std::vector<double> points, double viewport, double content,
        winrt::event<Changed>& changed) {
        if (points == axis.authored) return;
        auto effective = resolve(points, viewport, content);
        const bool updated = effective != axis.effective;
        auto view = updated ? make_view(effective) : axis.view;
        axis.authored = std::move(points);
        if (!updated) return;
        axis.effective = std::move(effective);
        axis.view = std::move(view);
        ++revision_;
        changed(this->get_strong().as<SnapInfo>(), nullptr);
    }

public:
    // Omitted patch fields must not call these setters; an empty list removes
    // that axis's anchors. All operations run on the owning XAML UI thread.
    void set_horizontal_points(std::vector<double> points) {
        set_points(horizontal_, std::move(points), viewport_width_, content_width_, horizontal_changed_);
    }
    void set_vertical_points(std::vector<double> points) {
        set_points(vertical_, std::move(points), viewport_height_, content_height_, vertical_changed_);
    }

    void update_extents(double viewport_width, double viewport_height, double content_width, double content_height) {
        validate_extent(viewport_width); validate_extent(viewport_height);
        validate_extent(content_width); validate_extent(content_height);
        if (viewport_width == viewport_width_ && viewport_height == viewport_height_ &&
            content_width == content_width_ && content_height == content_height_) return;
        auto horizontal = resolve(horizontal_.authored, viewport_width, content_width);
        auto vertical = resolve(vertical_.authored, viewport_height, content_height);
        const bool changed_x = horizontal != horizontal_.effective;
        const bool changed_y = vertical != vertical_.effective;
        auto horizontal_view = changed_x ? make_view(horizontal) : horizontal_.view;
        auto vertical_view = changed_y ? make_view(vertical) : vertical_.view;
        viewport_width_ = viewport_width; viewport_height_ = viewport_height;
        content_width_ = content_width; content_height_ = content_height;
        horizontal_.effective = std::move(horizontal); horizontal_.view = std::move(horizontal_view);
        vertical_.effective = std::move(vertical); vertical_.view = std::move(vertical_view);
        ++revision_;
        // Publish both axes before callbacks can query the new geometry.
        auto sender = this->get_strong().as<SnapInfo>();
        if (changed_x) horizontal_changed_(sender, nullptr);
        if (changed_y) vertical_changed_(sender, nullptr);
    }

    void apply_to(winrt::Microsoft::UI::Xaml::Controls::ScrollViewer const& scroll,
        bool horizontal_enabled, bool vertical_enabled) const {
        using namespace winrt::Microsoft::UI::Xaml::Controls;
        using namespace winrt::Microsoft::UI::Xaml::Controls::Primitives;
        scroll.HorizontalSnapPointsAlignment(SnapPointsAlignment::Near);
        scroll.VerticalSnapPointsAlignment(SnapPointsAlignment::Near);
        scroll.HorizontalSnapPointsType(horizontal_enabled && has_horizontal_points() ? SnapPointsType::Mandatory : SnapPointsType::None);
        scroll.VerticalSnapPointsType(vertical_enabled && has_vertical_points() ? SnapPointsType::Mandatory : SnapPointsType::None);
    }

    uint64_t revision() const noexcept { return revision_; }
    bool has_horizontal_points() const noexcept { return !horizontal_.effective.empty(); }
    bool has_vertical_points() const noexcept { return !vertical_.effective.empty(); }
    bool AreHorizontalSnapPointsRegular() const noexcept { return false; }
    bool AreVerticalSnapPointsRegular() const noexcept { return false; }

    Points GetIrregularSnapPoints(Orientation orientation, Alignment) const {
        if (orientation == Orientation::Horizontal) return horizontal_.view;
        if (orientation == Orientation::Vertical) return vertical_.view;
        throw std::invalid_argument("Unknown snap point orientation");
    }
    float GetRegularSnapPoints(Orientation, Alignment, float& offset) const noexcept {
        offset = 0;
        return 0;
    }

    winrt::event_token HorizontalSnapPointsChanged(Changed const& handler) { return horizontal_changed_.add(handler); }
    void HorizontalSnapPointsChanged(winrt::event_token const& token) noexcept { horizontal_changed_.remove(token); }
    winrt::event_token VerticalSnapPointsChanged(Changed const& handler) { return vertical_changed_.add(handler); }
    void VerticalSnapPointsChanged(winrt::event_token const& token) noexcept { vertical_changed_.remove(token); }
};

} // namespace pax::windows
