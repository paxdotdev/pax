#pragma once
#include <functional>
#include <memory>
#include <vector>
#include <winrt/Microsoft.UI.Dispatching.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Input.h>
#include <winrt/Microsoft.UI.Xaml.Media.h>

namespace pax::windows {
using BeforeNativeWheel = std::function<void(winrt::Microsoft::UI::Xaml::Input::PointerRoutedEventArgs const&)>;

// Authored content may have zero extent and disappear from the hit route.
// Observe each immediate template-root visual before its owning ScrollViewer
// handles the original event. This covers content, blank viewport, and chrome
// without depending on private part names or walking descendant controls.
class ScrollViewerWheelBridge : public std::enable_shared_from_this<ScrollViewerWheelBridge> {
    using Viewer = winrt::Microsoft::UI::Xaml::Controls::ScrollViewer;
    using UIElement = winrt::Microsoft::UI::Xaml::UIElement;
    Viewer viewer_{nullptr};
    BeforeNativeWheel before_;
    std::vector<UIElement> parts_;
    winrt::Windows::Foundation::IInspectable handler_{nullptr};
    winrt::event_token loaded_{};
    int64_t template_changed_ = 0;
    bool registered_ = false, queued_ = false, retired_ = false;
    std::vector<UIElement> template_roots() {
        namespace media = winrt::Microsoft::UI::Xaml::Media;
        std::vector<UIElement> parts;
        for (int i = 0; i < media::VisualTreeHelper::GetChildrenCount(viewer_); ++i)
            if (auto part = media::VisualTreeHelper::GetChild(viewer_, i).try_as<UIElement>()) parts.push_back(part);
        return parts;
    }
    void detach_parts() noexcept {
        for (auto const& part : parts_) {
            try { part.RemoveHandler(UIElement::PointerWheelChangedEvent(), handler_); } catch (...) {}
        }
        parts_.clear();
    }
    void refresh() {
        if (retired_) return;
        viewer_.ApplyTemplate();
        auto parts = template_roots();
        if (parts == parts_) return;
        detach_parts(); parts_ = std::move(parts);
        for (auto const& part : parts_) part.AddHandler(UIElement::PointerWheelChangedEvent(), handler_, true);
    }
    void queue_refresh() {
        if (retired_ || queued_) return;
        queued_ = true;
        auto weak = weak_from_this();
        if (!viewer_.DispatcherQueue().TryEnqueue([weak] {
            if (auto self = weak.lock(); self && !self->retired_) {
                self->queued_ = false; self->refresh();
            }
        })) queued_ = false;
    }
    void bind() {
        auto weak = weak_from_this();
        handler_ = winrt::box_value(winrt::Microsoft::UI::Xaml::Input::PointerEventHandler(
            [weak](auto const&, auto const& args) {
                if (auto self = weak.lock(); self && !self->retired_ && self->before_) self->before_(args);
            }));
        loaded_ = viewer_.Loaded([weak](auto const&, auto const&) { if (auto self = weak.lock()) self->refresh(); });
        template_changed_ = viewer_.RegisterPropertyChangedCallback(
            winrt::Microsoft::UI::Xaml::Controls::Control::TemplateProperty(),
            [weak](auto const&, auto const&) {
                if (auto self = weak.lock()) { self->detach_parts(); self->queue_refresh(); }
            });
        registered_ = true;
        if (viewer_.IsLoaded()) queue_refresh();
    }
public:
    static std::shared_ptr<ScrollViewerWheelBridge> create(Viewer const& viewer, BeforeNativeWheel before) {
        auto value = std::shared_ptr<ScrollViewerWheelBridge>(new ScrollViewerWheelBridge);
        value->viewer_ = viewer; value->before_ = std::move(before); value->bind(); return value;
    }
    void retire() noexcept {
        if (retired_) return;
        retired_ = true; before_ = {}; detach_parts();
        if (registered_) {
            try { viewer_.Loaded(loaded_); } catch (...) {}
            try { viewer_.UnregisterPropertyChangedCallback(
                winrt::Microsoft::UI::Xaml::Controls::Control::TemplateProperty(), template_changed_); } catch (...) {}
        }
        registered_ = false; handler_ = nullptr; viewer_ = nullptr;
    }
    ~ScrollViewerWheelBridge() { retire(); }
};

// These public native overrides run before each control changes its value.
// Delegating unchanged events to the base preserves native wheel behavior.
struct WheelSlider : winrt::Microsoft::UI::Xaml::Controls::SliderT<WheelSlider> {
    BeforeNativeWheel before;
    void OnPointerWheelChanged(winrt::Microsoft::UI::Xaml::Input::PointerRoutedEventArgs const& args) {
        if (before) before(args);
        if (!args.Handled()) SliderT<WheelSlider>::OnPointerWheelChanged(args);
    }
};
struct WheelComboBox : winrt::Microsoft::UI::Xaml::Controls::ComboBoxT<WheelComboBox> {
    BeforeNativeWheel before;
    void OnPointerWheelChanged(winrt::Microsoft::UI::Xaml::Input::PointerRoutedEventArgs const& args) {
        if (before) before(args);
        if (!args.Handled()) ComboBoxT<WheelComboBox>::OnPointerWheelChanged(args);
    }
};

// TextBox's internal ScrollViewer handles wheel input before TextBox's own
// override. Its documented ContentElement part owns the same template-root
// bridge as a scene scroller, including native scrollbar and empty-area routes.
struct WheelTextBox : winrt::Microsoft::UI::Xaml::Controls::TextBoxT<WheelTextBox> {
    BeforeNativeWheel before;
private:
    std::shared_ptr<ScrollViewerWheelBridge> bridge_;
    bool retired_ = false;
    void detach_template() noexcept {
        if (bridge_) bridge_->retire();
        bridge_.reset();
    }
public:
    void OnApplyTemplate() {
        detach_template();
        TextBoxT<WheelTextBox>::OnApplyTemplate();
        if (retired_) return;
        auto viewer = this->GetTemplateChild(L"ContentElement").try_as<winrt::Microsoft::UI::Xaml::Controls::ScrollViewer>();
        if (!viewer) return;
        auto weak = this->get_weak();
        bridge_ = ScrollViewerWheelBridge::create(viewer, [weak](auto const& args) {
            if (auto self = weak.get(); self && !self->retired_ && self->before) self->before(args);
        });
    }
    void retire_wheel() noexcept { retired_ = true; before = {}; detach_template(); }
    ~WheelTextBox() { retire_wheel(); }
};
} // namespace pax::windows
