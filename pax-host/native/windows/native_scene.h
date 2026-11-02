#pragma once
#include "json_helpers.h"
#include "native_mask.h"
#include "keyboard.h"
#include "snap_canvas.h"
#include "wheel_input.h"
#include <atomic>
#include <cmath>
#include <functional>
#include <limits>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <winrt/Windows.System.h>
#include <winrt/Windows.UI.Text.h>
#include <winrt/Windows.UI.ViewManagement.h>
#include <winrt/Microsoft.UI.h>
#include <winrt/Microsoft.UI.Input.h>
#include <winrt/Microsoft.UI.Xaml.Automation.h>
#include <winrt/Microsoft.UI.Xaml.Automation.Peers.h>
#include <winrt/Microsoft.UI.Xaml.Controls.Primitives.h>
#include <winrt/Microsoft.UI.Xaml.Hosting.h>
#include <winrt/Microsoft.UI.Xaml.Input.h>
#include <winrt/Microsoft.UI.Xaml.Documents.h>

namespace pax::windows {
namespace xaml = winrt::Microsoft::UI::Xaml;
namespace controls = xaml::Controls;
namespace media = xaml::Media;
namespace composition = winrt::Microsoft::UI::Composition;

// ProtectedCursor is deliberately exposed through a real XAML subclass, so
// native descendants can retain their own text/resize cursors.
struct SceneCanvas : controls::CanvasT<SceneCanvas> {
    void cursor(winrt::Microsoft::UI::Input::InputCursor const& value) { ProtectedCursor(value); }
    xaml::Automation::Peers::AutomationPeer OnCreateAutomationPeer() {
        return xaml::Automation::Peers::FrameworkElementAutomationPeer(this->get_strong().as<xaml::FrameworkElement>());
    }
};

class NativeScene {
public:
    using Interrupt = std::function<bool(json::JsonObject const&)>;
private:
    struct EventHandlers {
        std::shared_ptr<bool> live = std::make_shared<bool>(true);
        std::vector<std::function<void()>> revoke;
        void retire() noexcept {
            // Detached controls may still be retained by UI Automation. The
            // guard also protects against an already-queued native callback.
            *live = false;
            for (auto& remove : revoke) { try { remove(); } catch (...) {} }
            revoke.clear();
        }
        void clear() { retire(); live = std::make_shared<bool>(true); }
        ~EventHandlers() { retire(); }
    };
    struct Node {
        std::wstring kind;
        uint32_t id = 0, layer = 0;
        std::optional<uint32_t> parent;
        controls::Grid host{nullptr};
        xaml::FrameworkElement control{nullptr};
        controls::TextBlock text_content{nullptr};
        controls::Canvas children{nullptr};
        winrt::com_ptr<SnapCanvas> snap_content;
        std::map<std::wstring, media::SolidColorBrush> button_brushes;
        controls::Canvas attached_to{nullptr};
        controls::Canvas mask_input{nullptr};
        media::MatrixTransform placement_transform{nullptr};
        media::RectangleGeometry placement_clip{nullptr};
        media::Matrix placed_matrix{1, 0, 0, 1, 0, 0};
        winrt::Windows::Foundation::Rect placed_clip{};
        media::Matrix world{1, 0, 0, 1, 0, 0};
        double width = 0, height = 0;
        double corner_radius = 0;
        uint32_t selected_id = 0;
        std::vector<winrt::hstring> options;
        std::vector<controls::TextBlock> option_labels;
        json::JsonObject text_style, link_style;
        json::JsonObject rich_text{nullptr};
        std::vector<xaml::Documents::Run> rich_runs;
        std::vector<xaml::Documents::Hyperlink> rich_links;
        bool text_layout_dirty = true;
        double slider_min = 0, slider_max = 100, slider_step = 1, slider_value = 0;
        composition::CompositionVisualSurface capture{nullptr};
        composition::SpriteVisual masked{nullptr};
        composition::CompositionDrawingSurface mask{nullptr};
        composition::CompositionRoundedRectangleGeometry rounded_geometry{nullptr};
        composition::CompositionGeometricClip rounded_clip{nullptr};
        std::vector<MaskHole> holes;
        std::optional<MaskPath> clip_path;
        bool clip_content = false;
        bool mask_dirty = false;
        bool focus_pending = false;
        bool scroll_intermediate = false;
        bool scroll_request_pending = false;
        bool scroll_drain_scheduled = false;
        bool scroll_enabled_x = false, scroll_enabled_y = false;
        bool scroll_geometry_initialized = false;
        double authored_pane_width = 0, authored_pane_height = 0;
        winrt::com_ptr<IUnknown> wheel_content_event;
        bool wheel_content_unhandled = false;
        bool snap_policy_pending = false;
        std::optional<double> requested_scroll_x, requested_scroll_y;
        bool scroll_reply_pending = false;
        // TextPatch uses -1 for an omitted dimension. Keep that constraint
        // separate from the measured size used by XAML placement and masks.
        std::optional<double> text_width_constraint, text_height_constraint;
        std::optional<uint64_t> measure_generation;
        std::optional<uint64_t> latest_measure_generation;
        EventHandlers events, mask_events, rich_text_events;
    };
    winrt::com_ptr<SceneCanvas> root_impl_ = winrt::make_self<SceneCanvas>();
    controls::Canvas root_ = root_impl_.as<controls::Canvas>();
    controls::Button focus_parking_;
    std::map<uint32_t, std::unique_ptr<Node>> nodes_;
    struct Layer {
        controls::Canvas canvas;
        controls::Canvas parent{nullptr};
    };
    std::map<uint32_t, Layer> layers_;
    NativeMaskPainter masks_;
    composition::Compositor compositor_{nullptr};
    Interrupt interrupt_;
    std::function<winrt::hstring(json::JsonObject const&)> local_font_;
    std::function<void(winrt::hstring const&)> load_image_;
    std::function<void(winrt::hstring const&)> navigate_;
    bool applying_ = false;
    bool initial_focus_pending_ = true;
    float dpr_ = 1;
    uint32_t layer_count_ = 1;
    uint64_t placement_transforms_created_ = 0, rectangular_clips_created_ = 0;
    uint64_t mask_brushes_created_ = 0, surface_brushes_created_ = 0, input_paths_created_ = 0;
    uint64_t capture_surfaces_created_ = 0, mask_updates_ = 0;
    EventHandlers root_events_;
    winrt::Windows::UI::ViewManagement::UISettings text_settings_;
    std::shared_ptr<std::atomic_bool> text_metrics_dirty_ = std::make_shared<std::atomic_bool>(false);
    struct HeldKey { KeyboardKey key; bool prevented = false; };
    std::map<uint32_t, HeldKey> pressed_keys_;
    std::map<uint32_t, winrt::Windows::Foundation::Point> canvas_presses_;
    std::wstring cursor_;
    using Modifiers = winrt::Windows::System::VirtualKeyModifiers;
    // Retain COM identity until the root completes the original routed event.
    // Nested content hooks and native-control overrides share this dispatch;
    // timestamps can repeat and are not event identities.
    winrt::com_ptr<IUnknown> wheel_event_;
    xaml::Input::PointerRoutedEventArgs wheel_args_{nullptr};
    bool wheel_prevented_ = false;
    void wheel(xaml::Input::PointerRoutedEventArgs const& args) {
        auto identity = args.as<IUnknown>();
        if (wheel_event_.get() != identity.get()) {
            wheel_event_ = std::move(identity); wheel_args_ = args;
            auto point = args.GetCurrentPoint(root_);
            auto properties = point.Properties();
            auto horizontal = properties.IsHorizontalMouseWheel();
            auto position = point.Position();
            json::JsonObject value;
            value.Insert(L"x", numeric(position.X)); value.Insert(L"y", numeric(position.Y));
            value.Insert(L"delta_x", numeric(horizontal ? properties.MouseWheelDelta() : 0));
            value.Insert(L"delta_y", numeric(horizontal ? 0 : -properties.MouseWheelDelta()));
            value.Insert(L"modifiers", modifiers(uint32_t(args.KeyModifiers())));
            wheel_prevented_ = interrupt_(event(L"Wheel", value));
        }
        args.Handled(args.Handled() || wheel_prevented_);
    }
    static json::JsonArray modifiers(uint32_t state) {
        json::JsonArray result;
        if (state & uint32_t(Modifiers::Shift)) result.Append(text(L"Shift"));
        if (state & uint32_t(Modifiers::Control)) result.Append(text(L"Control"));
        if (state & uint32_t(Modifiers::Menu)) result.Append(text(L"Alt"));
        if (state & uint32_t(Modifiers::Windows)) result.Append(text(L"Command"));
        return result;
    }
    static uint32_t modifier_state() {
        return ((GetKeyState(VK_SHIFT) & 0x8000) ? uint32_t(Modifiers::Shift) : 0u) |
            ((GetKeyState(VK_CONTROL) & 0x8000) ? uint32_t(Modifiers::Control) : 0u) |
            ((GetKeyState(VK_MENU) & 0x8000) ? uint32_t(Modifiers::Menu) : 0u) |
            (((GetKeyState(VK_LWIN) | GetKeyState(VK_RWIN)) & 0x8000) ? uint32_t(Modifiers::Windows) : 0u);
    }
    bool send_key(wchar_t const* kind, std::wstring const& key, uint32_t state, bool repeat) {
        json::JsonObject value;
        value.Insert(L"key", text(winrt::hstring(key))); value.Insert(L"modifiers", modifiers(state));
        value.Insert(L"is_repeat", json::JsonValue::CreateBooleanValue(repeat));
        return interrupt_(event(kind, value));
    }
    bool canvas_focused() const { return root_.FocusState() != xaml::FocusState::Unfocused; }
    void key(xaml::Input::KeyRoutedEventArgs const& args, bool down) {
        initial_focus_pending_ = false;
        // Preview events also tunnel to native descendants. Only the canvas's
        // own focus may dispatch globals; editing and IME remain with controls.
        if (!canvas_focused()) return;
        auto status = args.KeyStatus();
        uint32_t code = status.ScanCode | (status.IsExtendedKey ? 0x100u : 0u);
        if (!code) code = 0x10000u | uint32_t(args.Key());
        auto found = pressed_keys_.find(code);
        auto state = modifier_state();
        if (!down) {
            // A key pressed in a native control must not acquire a canvas up.
            if (found == pressed_keys_.end()) return;
            auto held = found->second; pressed_keys_.erase(found);
            args.Handled(send_key(L"KeyUp", held.key.name, state, false) || held.prevented || args.Handled());
            return;
        }
        bool repeat = found != pressed_keys_.end() || status.WasKeyDown;
        auto translated = keyboard_key(UINT(args.Key()), status.ScanCode);
        // Retain the first key name for its matching release even if modifiers
        // change while held. A repeated down still reflects current modifiers.
        pressed_keys_.try_emplace(code, HeldKey{translated});
        bool handled = send_key(L"KeyDown", translated.name, state, repeat);
        auto current = pressed_keys_.find(code);
        if (current != pressed_keys_.end()) current->second.prevented |= handled;
        if (!handled && *root_events_.live && canvas_focused() && current != pressed_keys_.end() &&
            !(state & (uint32_t(Modifiers::Control) | uint32_t(Modifiers::Windows))) &&
            (translated.printable || translated.name == L"Enter"))
            handled = send_key(L"KeyPress", translated.name, state, repeat);
        current = pressed_keys_.find(code);
        if (current != pressed_keys_.end()) current->second.prevented |= handled;
        args.Handled(handled || args.Handled());
    }
    bool canvas_pointer_target(xaml::Input::PointerRoutedEventArgs const& args) const {
        auto source = args.OriginalSource().try_as<xaml::DependencyObject>();
        while (source && source != root_) {
            // A Scroller owns a Pax canvas too; its scrollbar controls still
            // retain focus, but its otherwise empty content focuses the canvas.
            if (source.try_as<controls::Control>() && !source.try_as<controls::ScrollViewer>()) return false;
            if (auto label = source.try_as<controls::TextBlock>())
                if (label.IsTextSelectionEnabled()) return false;
            source = media::VisualTreeHelper::GetParent(source);
        }
        return source == root_;
    }
    void set_cursor(std::wstring const& name) {
        if (name == cursor_) return;
        using Shape = winrt::Microsoft::UI::Input::InputSystemCursorShape;
        static const std::map<std::wstring, Shape> shapes{
            {L"pointer", Shape::Hand}, {L"grab", Shape::Hand}, {L"grabbing", Shape::Hand},
            {L"help", Shape::Help}, {L"progress", Shape::AppStarting}, {L"wait", Shape::Wait},
            {L"text", Shape::IBeam}, {L"vertical-text", Shape::IBeam},
            {L"crosshair", Shape::Cross}, {L"cell", Shape::Cross},
            {L"move", Shape::SizeAll}, {L"all-scroll", Shape::SizeAll},
            {L"no-drop", Shape::UniversalNo}, {L"not-allowed", Shape::UniversalNo},
            {L"col-resize", Shape::SizeWestEast}, {L"ew-resize", Shape::SizeWestEast},
            {L"e-resize", Shape::SizeWestEast}, {L"w-resize", Shape::SizeWestEast},
            {L"row-resize", Shape::SizeNorthSouth}, {L"ns-resize", Shape::SizeNorthSouth},
            {L"n-resize", Shape::SizeNorthSouth}, {L"s-resize", Shape::SizeNorthSouth},
            {L"ne-resize", Shape::SizeNortheastSouthwest}, {L"sw-resize", Shape::SizeNortheastSouthwest},
            {L"nesw-resize", Shape::SizeNortheastSouthwest},
            {L"nw-resize", Shape::SizeNorthwestSoutheast}, {L"se-resize", Shape::SizeNorthwestSoutheast},
            {L"nwse-resize", Shape::SizeNorthwestSoutheast}
        };
        auto found = shapes.find(name);
        if (name == L"none") root_impl_->cursor(nullptr);
        else root_impl_->cursor(winrt::Microsoft::UI::Input::InputSystemCursor::Create(
            found == shapes.end() ? Shape::Arrow : found->second));
        cursor_ = name;
    }

    static media::SolidColorBrush color(json::IJsonValue const& value) {
        auto object = value.GetObject();
        auto rgba = object.HasKey(L"Rgba") ? object.GetNamedArray(L"Rgba") : object.GetNamedArray(L"Rgb");
        auto byte = [&](uint32_t index) { return uint8_t(std::round(std::clamp(rgba.GetNumberAt(index), 0.0, 1.0) * 255)); };
        return media::SolidColorBrush(winrt::Windows::UI::Color{rgba.Size() == 4 ? byte(3) : uint8_t(255), byte(0), byte(1), byte(2)});
    }
    static void state_brush(controls::Control const& control, wchar_t const* key, media::SolidColorBrush const& brush) {
        for (auto suffix : {L"", L"PointerOver", L"Pressed"})
            control.Resources().Insert(winrt::box_value(winrt::hstring(std::wstring(key) + suffix)), brush);
    }
    static media::SolidColorBrush button_brush(Node& node, controls::Button const& button,
        wchar_t const* key, json::IJsonValue const& value) {
        auto next = color(value);
        auto& retained = node.button_brushes.try_emplace(key, nullptr).first->second;
        if (retained) retained.Color(next.Color());
        else {
            retained = next;
            button.Resources().Insert(winrt::box_value(winrt::hstring(key)), retained);
        }
        return retained;
    }
    static media::Matrix relative(media::Matrix const& child, media::Matrix const& parent) {
        double det = parent.M11 * parent.M22 - parent.M12 * parent.M21;
        if (!std::isfinite(det) || std::abs(det) < 1e-12) throw std::invalid_argument("Singular native parent transform");
        double a = parent.M22 / det, b = -parent.M12 / det, c = -parent.M21 / det, d = parent.M11 / det;
        double x = -(parent.OffsetX * a + parent.OffsetY * c), y = -(parent.OffsetX * b + parent.OffsetY * d);
        return {child.M11 * a + child.M12 * c, child.M11 * b + child.M12 * d,
            child.M21 * a + child.M22 * c, child.M21 * b + child.M22 * d,
            child.OffsetX * a + child.OffsetY * c + x, child.OffsetX * b + child.OffsetY * d + y};
    }
    void send(wchar_t const* kind, uint32_t id, wchar_t const* field = nullptr, json::IJsonValue const& value = nullptr) {
        if (applying_) return;
        json::JsonObject args; args.Insert(L"id", numeric(id));
        if (field) args.Insert(field, value);
        interrupt_(event(kind, args));
    }
    void pointer(xaml::Input::PointerRoutedEventArgs const& args, wchar_t const* kind) {
        auto point = args.GetCurrentPoint(root_);
        auto p = point.Position();
        json::JsonObject value; value.Insert(L"x", numeric(p.X)); value.Insert(L"y", numeric(p.Y));
        auto props = point.Properties();
        auto update = props.PointerUpdateKind();
        using Update = winrt::Microsoft::UI::Input::PointerUpdateKind;
        wchar_t const* button;
        switch (update) {
        case Update::LeftButtonPressed: case Update::LeftButtonReleased: button = L"Left"; break;
        case Update::RightButtonPressed: case Update::RightButtonReleased: button = L"Right"; break;
        case Update::MiddleButtonPressed: case Update::MiddleButtonReleased: button = L"Middle"; break;
        default: button = props.IsRightButtonPressed() ? L"Right" : props.IsMiddleButtonPressed() ? L"Middle" : L"Left"; break;
        }
        value.Insert(L"button", text(button));
        value.Insert(L"modifiers", modifiers(uint32_t(args.KeyModifiers())));
        args.Handled(interrupt_(event(kind, value)) || args.Handled());
    }
    static bool selectable_text_source(xaml::DependencyObject source) {
        while (source) {
            if (auto label = source.try_as<controls::TextBlock>()) {
                auto id = xaml::Automation::AutomationProperties::GetAutomationId(label);
                return label.IsTextSelectionEnabled() && std::wstring_view(id).starts_with(L"pax-node-");
            }
            source = media::VisualTreeHelper::GetParent(source);
        }
        return false;
    }
    void wire_pointer(xaml::UIElement const& element, EventHandlers& handlers, bool removed_region = false) {
        auto live = handlers.live;
        auto pressed = element.PointerPressed([this, live, removed_region](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
            if (!*live) return;
            if (removed_region) focus_canvas();
            pointer(args, L"MouseDown"); if (removed_region) args.Handled(true);
        });
        handlers.revoke.push_back([element, pressed] { element.PointerPressed(pressed); });
        auto released = element.PointerReleased([this, live, removed_region](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
            if (!*live) return;
            pointer(args, L"MouseUp");
            // Selectable Text uses WinUI's tap recognition below, so selecting
            // text with a drag does not activate an enclosing Pax Link.
            if (!selectable_text_source(args.OriginalSource().try_as<xaml::DependencyObject>())) pointer(args, L"Click");
            if (removed_region) args.Handled(true);
        });
        handlers.revoke.push_back([element, released] { element.PointerReleased(released); });
        auto moved = element.PointerMoved([this, live, removed_region](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
            if (!*live) return;
            pointer(args, L"MouseMove"); if (removed_region) args.Handled(true);
        });
        handlers.revoke.push_back([element, moved] { element.PointerMoved(moved); });
    }
    Node& require(uint32_t id) {
        auto found = nodes_.find(id);
        if (found == nodes_.end()) throw std::invalid_argument("Native patch refers to an absent node");
        return *found->second;
    }
    static void detach(Node& node) {
        if (!node.attached_to) return;
        uint32_t index = 0;
        if (node.attached_to.Children().IndexOf(node.host, index)) node.attached_to.Children().RemoveAt(index);
        node.attached_to = nullptr;
    }
    void placement(Node& node) {
        auto target = root_;
        media::Matrix parent{1, 0, 0, 1, 0, 0};
        if (node.parent) {
            auto& owner = require(*node.parent);
            if (!owner.children) throw std::invalid_argument("Native parent is not a Frame or Scroller");
            target = owner.children; parent = owner.world;
        }
        if (node.attached_to != target) { detach(node); target.Children().Append(node.host); node.attached_to = target; }
        // Recompute relative placement even for unchanged children: a parent
        // may have moved or the node may have been reparented this frame.
        auto matrix = relative(node.world, parent);
        if (!node.placement_transform) {
            node.placement_transform = media::MatrixTransform(); ++placement_transforms_created_;
            node.placement_transform.Matrix(matrix); node.placed_matrix = matrix;
            node.host.RenderTransform(node.placement_transform);
        } else {
            auto const& previous = node.placed_matrix;
            if (matrix.M11 != previous.M11 || matrix.M12 != previous.M12 ||
                matrix.M21 != previous.M21 || matrix.M22 != previous.M22 ||
                matrix.OffsetX != previous.OffsetX || matrix.OffsetY != previous.OffsetY) {
                node.placement_transform.Matrix(matrix); node.placed_matrix = matrix;
            }
        }
        node.host.Width(node.width); node.host.Height(node.height);
        node.control.Width(node.width); node.control.Height(node.height);
        if (auto label = node.text_content) {
            if (node.text_layout_dirty) {
                // The authored box and the content block are separate. Measure
                // the block at its wrap constraint, then let Grid align it;
                // a no-wrap label may extend beyond an unclipped authored box.
                auto automatic = std::numeric_limits<double>::quiet_NaN();
                label.Width(automatic); label.Height(automatic); label.InvalidateMeasure();
                label.Measure({label.TextWrapping() == xaml::TextWrapping::NoWrap ?
                    std::numeric_limits<float>::infinity() : float(node.width),
                    std::numeric_limits<float>::infinity()});
                auto content = label.DesiredSize();
                label.Width(content.Width); label.Height(content.Height);
                node.text_layout_dirty = false;
            }
        }
        if (node.clip_content && !node.clip_path && !(node.kind == L"Scroller" && node.corner_radius > 0)) {
            winrt::Windows::Foundation::Rect rect{0, 0, float(node.width), float(node.height)};
            if (!node.placement_clip) {
                node.placement_clip = media::RectangleGeometry(); ++rectangular_clips_created_;
                node.placement_clip.Rect(rect); node.placed_clip = rect;
            } else if (rect.Width != node.placed_clip.Width || rect.Height != node.placed_clip.Height) {
                node.placement_clip.Rect(rect); node.placed_clip = rect;
            }
            if (node.host.Clip() != node.placement_clip) node.host.Clip(node.placement_clip);
        } else if (node.host.Clip()) node.host.Clip(nullptr);
    }
    static xaml::HorizontalAlignment horizontal_alignment(winrt::hstring const& value) {
        return value == L"Center" ? xaml::HorizontalAlignment::Center :
            value == L"Right" ? xaml::HorizontalAlignment::Right : xaml::HorizontalAlignment::Left;
    }
    static xaml::VerticalAlignment vertical_alignment(winrt::hstring const& value) {
        return value == L"Center" ? xaml::VerticalAlignment::Center :
            value == L"Bottom" ? xaml::VerticalAlignment::Bottom : xaml::VerticalAlignment::Top;
    }
    static xaml::TextAlignment line_alignment(winrt::hstring const& value) {
        return value == L"Center" ? xaml::TextAlignment::Center :
            value == L"Right" ? xaml::TextAlignment::Right : xaml::TextAlignment::Left;
    }
    static controls::TextBlock label_content(controls::ContentControl const& control, winrt::hstring const& value) {
        auto label = control.Content().try_as<controls::TextBlock>();
        if (!label) {
            label = controls::TextBlock();
            control.Content(label);
        }
        if (label.Text() != value) label.Text(value);
        // Keep the genuine control's accessible name stable when its content
        // is a native text element instead of a boxed string.
        xaml::Automation::AutomationProperties::SetName(control, value);
        return label;
    }
    static void text_label_style(controls::TextBlock const& label, json::JsonObject const& value) {
        if (present(value, L"underline")) label.TextDecorations(boolean(value, L"underline") ?
            winrt::Windows::UI::Text::TextDecorations::Underline : winrt::Windows::UI::Text::TextDecorations::None);
        if (present(value, L"align_multiline")) label.TextAlignment(line_alignment(value.GetNamedString(L"align_multiline")));
    }
    static void label_style(controls::ContentControl const& control, json::JsonObject const& value) {
        if (auto label = control.Content().try_as<controls::TextBlock>()) {
            // Font and foreground inherit from the native control, including
            // its visual states. Decorations belong to the text element.
            text_label_style(label, value);
        }
    }
    template<typename Control> void style(Control const& control, json::JsonObject const& style, bool zero_size_text = false) {
        char const* operation = "alignment/decorations";
        try {
            if (auto label = control.template try_as<controls::TextBlock>()) {
                if (present(style, L"underline")) label.TextDecorations(boolean(style, L"underline") ?
                    winrt::Windows::UI::Text::TextDecorations::Underline : winrt::Windows::UI::Text::TextDecorations::None);
                if (present(style, L"align_multiline")) label.TextAlignment(line_alignment(style.GetNamedString(L"align_multiline")));
                if (present(style, L"align_horizontal")) label.HorizontalAlignment(horizontal_alignment(style.GetNamedString(L"align_horizontal")));
                if (present(style, L"align_vertical")) label.VerticalAlignment(vertical_alignment(style.GetNamedString(L"align_vertical")));
            } else if (auto native = control.template try_as<controls::Control>()) {
                if (present(style, L"align_horizontal")) native.HorizontalContentAlignment(horizontal_alignment(style.GetNamedString(L"align_horizontal")));
                if (present(style, L"align_vertical")) native.VerticalContentAlignment(vertical_alignment(style.GetNamedString(L"align_vertical")));
                if (auto editor = native.try_as<controls::TextBox>(); editor && present(style, L"align_multiline"))
                    editor.TextAlignment(line_alignment(style.GetNamedString(L"align_multiline")));
            }
            operation = "font size";
            if (present(style, L"font_size")) {
                auto size = number(style, L"font_size");
                if (zero_size_text) {
                    // WinUI rejects FontSize(0). Collapse only the glyph block so
                    // authored boxes and retained text survive a zero-size state.
                    auto label = control.template as<controls::TextBlock>();
                    label.Visibility(size == 0 ? xaml::Visibility::Collapsed : xaml::Visibility::Visible);
                    if (size != 0) label.FontSize(size);
                } else control.FontSize(size);
            }
            operation = "foreground";
            if (present(style, L"fill")) control.Foreground(color(style.GetNamedValue(L"fill")));
            operation = "font family";
            if (present(style, L"font")) {
                auto font = style.GetNamedObject(L"font");
                auto system = font.GetNamedObject(font.HasKey(L"Local") ? L"Local" : font.HasKey(L"Web") ? L"Web" : L"System");
                if (font.HasKey(L"Local")) control.FontFamily(media::FontFamily(local_font_(system)));
                else if (font.HasKey(L"Web") && !string(system, L"url").empty()) {
                    auto url = system.GetNamedString(L"url");
                    if (std::wstring_view(url).find(L"://") != std::wstring_view::npos)
                        throw std::invalid_argument("Remote font URLs/CSS require an explicit native font download policy");
                    json::JsonObject local;
                    local.Insert(L"path", text(url)); local.Insert(L"family", text(string(system, L"family")));
                    control.FontFamily(media::FontFamily(local_font_(local)));
                }
                else if (present(system, L"family")) control.FontFamily(media::FontFamily(system.GetNamedString(L"family")));
                operation = "font weight";
                if (present(system, L"weight")) {
                    static const std::map<std::wstring, uint16_t> weights{{L"Thin",uint16_t{100}},{L"ExtraLight",uint16_t{200}},{L"Light",uint16_t{300}},{L"Normal",uint16_t{400}},{L"Medium",uint16_t{500}},{L"SemiBold",uint16_t{600}},{L"Bold",uint16_t{700}},{L"ExtraBold",uint16_t{800}},{L"Black",uint16_t{900}}};
                    control.FontWeight(winrt::Windows::UI::Text::FontWeight{weights.at(system.GetNamedString(L"weight").c_str())});
                }
                operation = "font style";
                if (present(system, L"style")) {
                    auto s = system.GetNamedString(L"style");
                    control.FontStyle(s == L"Italic" ? winrt::Windows::UI::Text::FontStyle::Italic : s == L"Oblique" ? winrt::Windows::UI::Text::FontStyle::Oblique : winrt::Windows::UI::Text::FontStyle::Normal);
                }
            }
        } catch (winrt::hresult_error const& error) {
            throw std::runtime_error(std::string("Native text style ") + operation +
                " font_size=" + std::to_string(number(style, L"font_size", -1)) + ": " +
                winrt::to_string(error.message()) + " HRESULT=" + std::to_string(int32_t(error.code())));
        }
    }
    void render_rich_text(Node& node) {
        auto label = node.text_content;
        auto document = node.rich_text;
        auto content = winrt::to_string(document.GetNamedString(L"text"));
        auto runs = document.GetNamedArray(L"runs");
        auto boundary = [&](size_t value) {
            return value <= content.size() && (value == content.size() ||
                (static_cast<unsigned char>(content[value]) & 0xc0) != 0x80);
        };
        // Build detached inlines before replacing the visible document. UTF-8
        // ranges come from the shared parser; substring conversion preserves
        // surrogate pairs without treating byte offsets as WinRT indices.
        std::vector<xaml::Documents::Inline> inlines;
        node.rich_runs.clear(); node.rich_links.clear();
        size_t previous = 0;
        xaml::Documents::Hyperlink current_link{nullptr};
        winrt::hstring current_destination;
        double current_link_id = 0;
        for (auto const& value : runs) {
            auto run = value.GetObject();
            auto index = [&](wchar_t const* key) {
                auto raw = number(run, key);
                if (!std::isfinite(raw) || raw < 0 || raw > content.size() || std::floor(raw) != raw)
                    throw std::invalid_argument("Rich text range must be a valid UTF-8 byte offset");
                return static_cast<size_t>(raw);
            };
            auto start = index(L"start"), end = index(L"end");
            if (start != previous || end <= start || !boundary(start) || !boundary(end))
                throw std::invalid_argument("Rich text ranges must cover text at UTF-8 boundaries");
            previous = end;
            xaml::Documents::Run native;
            native.Text(winrt::to_hstring(std::string_view(content).substr(start, end - start)));
            if (boolean(run, L"bold")) native.FontWeight(winrt::Windows::UI::Text::FontWeight{700});
            if (boolean(run, L"italic")) native.FontStyle(winrt::Windows::UI::Text::FontStyle::Italic);
            auto decorations = winrt::Windows::UI::Text::TextDecorations::None;
            if (boolean(run, L"underline")) decorations |= winrt::Windows::UI::Text::TextDecorations::Underline;
            if (boolean(run, L"strikethrough")) decorations |= winrt::Windows::UI::Text::TextDecorations::Strikethrough;
            if (present(run, L"underline") || present(run, L"strikethrough")) native.TextDecorations(decorations);
            if (present(run, L"font_family")) {
                auto family = run.GetNamedString(L"font_family");
                native.FontFamily(media::FontFamily(family == L"monospace" ? L"Consolas" : family));
            }
            if (present(run, L"size_scale")) {
                auto scale = number(run, L"size_scale");
                if (!std::isfinite(scale) || scale <= 0) throw std::invalid_argument("Rich text size scale must be positive and finite");
                native.FontSize(label.FontSize() * scale);
            }
            if (present(run, L"color")) {
                json::JsonObject rgba; rgba.Insert(L"Rgba", run.GetNamedValue(L"color"));
                native.Foreground(color(rgba));
            }
            node.rich_runs.push_back(native);
            if (present(run, L"link")) {
                auto destination = run.GetNamedString(L"link");
                // Formatting spans do not split one logical link into several
                // keyboard stops or independent activation targets.
                auto link_id = number(run, L"link_id", 0);
                if (!current_link || current_destination != destination || current_link_id != link_id) {
                    current_link = xaml::Documents::Hyperlink(); current_destination = destination; current_link_id = link_id;
                    style(current_link, node.link_style);
                    node.rich_links.push_back(current_link);
                    auto live = node.rich_text_events.live;
                    auto clicked = current_link.Click([this, live, destination](auto const&, auto const&) {
                        if (*live && navigate_) navigate_(destination);
                    });
                    node.rich_text_events.revoke.push_back([link = current_link, clicked] { link.Click(clicked); });
                    inlines.push_back(current_link);
                }
                current_link.Inlines().Append(native);
            } else { current_link = nullptr; current_destination = {}; inlines.push_back(native); }
        }
        if (previous != content.size()) throw std::invalid_argument("Rich text ranges do not cover the document");
        auto height = present(document, L"line_height") ? number(document, L"line_height") : 0;
        if (!std::isfinite(height) || height < 0) throw std::invalid_argument("Rich text line height must be finite and nonnegative");
        label.Text(L""); label.Inlines().Clear();
        for (auto const& value : inlines) label.Inlines().Append(value);
        label.LineHeight(height);
        label.LineStackingStrategy(height > 0 ? xaml::LineStackingStrategy::BlockLineHeight : xaml::LineStackingStrategy::MaxHeight);
    }
    void refresh_rich_text_style(Node& node) {
        // Keep native runs, selection, and hyperlink focus stable when only
        // base styling changes. Relative heading sizes need an explicit refresh;
        // all other run overrides stay constant or inherit from the TextBlock.
        auto runs = node.rich_text.GetNamedArray(L"runs");
        for (uint32_t i = 0; i < runs.Size(); ++i) {
            auto run = runs.GetObjectAt(i);
            if (present(run, L"size_scale"))
                node.rich_runs.at(i).FontSize(node.text_content.FontSize() * number(run, L"size_scale"));
        }
        for (auto const& link : node.rich_links) style(link, node.link_style);
    }
    void schedule_scroll_drain(Node& node) {
        if (node.scroll_drain_scheduled) return;
        node.scroll_drain_scheduled = true;
        auto live = node.events.live;
        auto id = node.id;
        if (!root_.DispatcherQueue().TryEnqueue([this, live, id] {
            if (!*live) return;
            auto& node = require(id);
            node.scroll_drain_scheduled = false;
            if (applying_) { schedule_scroll_drain(node); return; }
            apply_scroll_request(node);
            if (node.scroll_reply_pending) send_scroll_position(node);
        })) node.scroll_drain_scheduled = false;
    }
    void create(std::wstring const& kind, json::JsonObject const& patch) {
        uint32_t id = uint32_t(number(patch, L"id"));
        if (nodes_.contains(id)) throw std::invalid_argument("Duplicate native creation");
        auto node = std::make_unique<Node>(); node->id = id; node->kind = kind;
        node->layer = uint32_t(number(patch, L"render_layer_id"));
        layer_count_ = std::max(layer_count_, node->layer + 1);
        if (present(patch, L"parent_frame")) node->parent = uint32_t(number(patch, L"parent_frame"));
        node->host = controls::Grid();
        node->host.HorizontalAlignment(xaml::HorizontalAlignment::Left);
        node->host.VerticalAlignment(xaml::VerticalAlignment::Top);
        auto live = node->events.live;
        if (kind == L"Textbox") {
            auto editor_impl = winrt::make_self<WheelTextBox>();
            editor_impl->before = [this, live](auto const& args) { if (*live) wheel(args); };
            auto editor = editor_impl.as<controls::TextBox>();
            node->events.revoke.push_back([editor_impl] { editor_impl->retire_wheel(); });
            auto changed = editor.TextChanged([this, id, live](auto const& sender, auto const&) { if (*live) send(L"FormTextboxInput", id, L"text", text(sender.template as<controls::TextBox>().Text())); });
            node->events.revoke.push_back([editor, changed] { editor.TextChanged(changed); });
            auto focus = editor.LostFocus([this, id, live](auto const& sender, auto const&) { if (*live) send(L"FormTextboxChange", id, L"text", text(sender.template as<controls::TextBox>().Text())); });
            node->events.revoke.push_back([editor, focus] { editor.LostFocus(focus); });
            auto selected = editor.SelectionChanged([this, live](auto const&, auto const&) { if (*live && !applying_) interrupt_(event(L"SelectStart", json::JsonObject())); });
            node->events.revoke.push_back([editor, selected] { editor.SelectionChanged(selected); });
            node->control = editor;
        } else if (kind == L"Button") {
            controls::Button button;
            auto clicked = button.Click([this, id, live](auto const&, auto const&) { if (*live) send(L"FormButtonClick", id); });
            node->events.revoke.push_back([button, clicked] { button.Click(clicked); }); node->control = button;
        } else if (kind == L"Checkbox") {
            controls::CheckBox check;
            auto checked = check.Checked([this, id, live](auto const&, auto const&) { if (*live) send(L"FormCheckboxToggle", id, L"state", json::JsonValue::CreateBooleanValue(true)); });
            node->events.revoke.push_back([check, checked] { check.Checked(checked); });
            auto unchecked = check.Unchecked([this, id, live](auto const&, auto const&) { if (*live) send(L"FormCheckboxToggle", id, L"state", json::JsonValue::CreateBooleanValue(false)); });
            node->events.revoke.push_back([check, unchecked] { check.Unchecked(unchecked); });
            node->control = check;
        } else if (kind == L"Slider") {
            auto slider_impl = winrt::make_self<WheelSlider>();
            slider_impl->before = [this, live](auto const& args) { if (*live) wheel(args); };
            auto slider = slider_impl.as<controls::Slider>();
            node->events.revoke.push_back([slider_impl] { slider_impl->before = {}; });
            slider.SnapsTo(controls::Primitives::SliderSnapsTo::StepValues);
            auto changed = slider.ValueChanged([this, id, live](auto const& sender, controls::Primitives::RangeBaseValueChangedEventArgs const& args) {
                if (!*live || applying_) return;
                auto& node = require(id);
                auto index = std::clamp(std::round((args.NewValue() - node.slider_min) / node.slider_step), 0.0,
                    std::floor((node.slider_max - node.slider_min) / node.slider_step + 1e-12));
                auto value = std::clamp(node.slider_min + index * node.slider_step, node.slider_min, node.slider_max);
                if (value != args.NewValue()) {
                    applying_ = true;
                    try { sender.template as<controls::Slider>().Value(value); }
                    catch (...) { applying_ = false; throw; }
                    applying_ = false;
                }
                if (value == node.slider_value) return;
                node.slider_value = value;
                send(L"FormSliderChange", id, L"value", numeric(value));
            });
            node->events.revoke.push_back([slider, changed] { slider.ValueChanged(changed); });
            node->control = slider;
        } else if (kind == L"Dropdown") {
            auto dropdown_impl = winrt::make_self<WheelComboBox>();
            dropdown_impl->before = [this, live](auto const& args) { if (*live) wheel(args); };
            auto dropdown = dropdown_impl.as<controls::ComboBox>();
            node->events.revoke.push_back([dropdown_impl] { dropdown_impl->before = {}; });
            dropdown.HorizontalContentAlignment(xaml::HorizontalAlignment::Stretch);
            auto changed = dropdown.SelectionChanged([this, id, live](auto const& sender, auto const&) {
                if (!*live || applying_) return;
                auto index = sender.template as<controls::ComboBox>().SelectedIndex();
                if (index < 0) return;
                if (require(id).selected_id == uint32_t(index)) return;
                require(id).selected_id = uint32_t(index);
                send(L"FormDropdownChange", id, L"selected_id", numeric(index));
            });
            node->events.revoke.push_back([dropdown, changed] { dropdown.SelectionChanged(changed); });
            node->control = dropdown;
        } else if (kind == L"RadioList") {
            controls::RadioButtons radio;
            radio.MaxColumns(1);
            auto changed = radio.SelectionChanged([this, id, live](auto const& sender, auto const&) {
                if (!*live || applying_) return;
                auto index = sender.template as<controls::RadioButtons>().SelectedIndex();
                if (index < 0) return;
                if (require(id).selected_id == uint32_t(index)) return;
                require(id).selected_id = uint32_t(index);
                send(L"FormRadioListChange", id, L"selected_id", numeric(index));
            });
            node->events.revoke.push_back([radio, changed] { radio.SelectionChanged(changed); });
            node->control = radio;
        } else if (kind == L"Text") {
            controls::TextBlock label; label.HorizontalAlignment(xaml::HorizontalAlignment::Left);
            label.VerticalAlignment(xaml::VerticalAlignment::Top);
            // Keep a full-box capture source: masks use node-local authored
            // coordinates even when the intrinsic text is centered or overflows.
            controls::Grid content; content.Children().Append(label);
            node->text_content = label; node->control = content;
            // TextBlock handles pointer releases for selection. Observe its
            // native tap gesture without disabling selection or forwarding a
            // second Click through the root's pointer-release path.
            auto tapped = winrt::box_value(xaml::Input::TappedEventHandler(
                [this, live, label](auto const&, xaml::Input::TappedRoutedEventArgs const& args) {
                    if (!*live || !label.IsTextSelectionEnabled()) return;
                    auto point = args.GetPosition(root_);
                    json::JsonObject value;
                    value.Insert(L"x", numeric(point.X)); value.Insert(L"y", numeric(point.Y));
                    value.Insert(L"button", text(L"Left")); value.Insert(L"modifiers", modifiers(modifier_state()));
                    args.Handled(interrupt_(event(L"Click", value)) || args.Handled());
                }));
            label.AddHandler(xaml::UIElement::TappedEvent(), tapped, true);
            node->events.revoke.push_back([label, tapped] { label.RemoveHandler(xaml::UIElement::TappedEvent(), tapped); });
        }
        else if (kind == L"Frame" || kind == L"Scroller") {
            if (kind == L"Scroller") {
                node->snap_content = winrt::make_self<SnapCanvas>();
                node->children = node->snap_content.as<controls::Canvas>();
            } else node->children = controls::Canvas();
            if (kind == L"Frame") node->control = node->children;
            else {
                // Make the scene content itself the hit target for vectors.
                // Intercept its completed background click before ScrollViewer
                // can focus an unrelated native descendant on PointerReleased.
                node->children.Background(media::SolidColorBrush(winrt::Microsoft::UI::Colors::Transparent()));
                auto content = node->children;
                auto released = content.PointerReleased([this, live](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
                    if (!*live || !canvas_presses_.contains(args.Pointer().PointerId()) || !canvas_pointer_target(args)) return;
                    if (!canvas_focused()) focus_canvas();
                    pointer(args, L"MouseUp"); pointer(args, L"Click");
                    // Root's normal release listener then skips this event;
                    // nested scrollers also cannot dispatch it a second time.
                    args.Handled(true);
                });
                node->events.revoke.push_back([content, released] { content.PointerReleased(released); });
                controls::ScrollViewer scroll; scroll.Content(node->children);
                scroll.ZoomMode(controls::ZoomMode::Disabled);
                // Enabled with zero extent declines an axis without translating a
                // vertical wheel into horizontal motion. Authored-disabled axes
                // have a physically empty extent, independently of pane bounds.
                scroll.HorizontalScrollMode(controls::ScrollMode::Enabled);
                scroll.VerticalScrollMode(controls::ScrollMode::Enabled);
                scroll.IsHorizontalScrollChainingEnabled(true);
                scroll.IsVerticalScrollChainingEnabled(true);
                BeforeNativeWheel before_wheel = [this, id, live](xaml::Input::PointerRoutedEventArgs const& args) {
                        if (!*live) return;
                        auto& node = require(id);
                        node.wheel_content_event = args.as<IUnknown>();
                        node.wheel_content_unhandled = !args.Handled();
                        wheel(args);
                        node.wheel_content_unhandled &= !wheel_prevented_;
                    };
                auto wheel_bridge = ScrollViewerWheelBridge::create(scroll, std::move(before_wheel));
                auto after_wheel = winrt::box_value(xaml::Input::PointerEventHandler(
                    [this, id, live](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
                        if (!*live) return;
                        auto& node = require(id);
                        auto identity = args.as<IUnknown>();
                        if (node.wheel_content_event.get() != identity.get()) return;
                        auto eligible = node.wheel_content_unhandled;
                        node.wheel_content_event = nullptr;
                        if (!eligible || wheel_prevented_) return;
                        auto view = node.control.as<controls::ScrollViewer>();
                        auto horizontal = args.GetCurrentPoint(root_).Properties().IsHorizontalMouseWheel();
                        if ((horizontal ? view.ScrollableWidth() : view.ScrollableHeight()) == 0)
                            args.Handled(false); // Continue the same event to its native ancestor.
                    }));
                scroll.AddHandler(xaml::UIElement::PointerWheelChangedEvent(), after_wheel, true);
                auto changed = scroll.ViewChanged([this, id, live](auto const&, controls::ScrollViewerViewChangedEventArgs const& args) {
                    if (!*live) return;
                    auto& node = require(id);
                    node.scroll_intermediate = args.IsIntermediate();
                    if (!node.scroll_intermediate) {
                        node.scroll_request_pending = false;
                        if (node.requested_scroll_x || node.requested_scroll_y || node.snap_policy_pending)
                            schedule_scroll_drain(node);
                    }
                    node.scroll_reply_pending = true;
                    if (!applying_) send_scroll_position(node);
                });
                node->events.revoke.push_back([scroll, wheel_bridge, after_wheel, changed] {
                    wheel_bridge->retire();
                    scroll.RemoveHandler(xaml::UIElement::PointerWheelChangedEvent(), after_wheel);
                    scroll.ViewChanged(changed);
                });
                auto viewport_changed = [this, id, live](auto const&, auto const&) {
                    if (!*live) return;
                    auto& node = require(id);
                    node.snap_policy_pending = true;
                    if (!applying_) schedule_scroll_drain(node);
                };
                auto viewport_x = scroll.RegisterPropertyChangedCallback(controls::ScrollViewer::ViewportWidthProperty(), viewport_changed);
                auto viewport_y = scroll.RegisterPropertyChangedCallback(controls::ScrollViewer::ViewportHeightProperty(), viewport_changed);
                node->events.revoke.push_back([scroll, viewport_x, viewport_y] {
                    scroll.UnregisterPropertyChangedCallback(controls::ScrollViewer::ViewportWidthProperty(), viewport_x);
                    scroll.UnregisterPropertyChangedCallback(controls::ScrollViewer::ViewportHeightProperty(), viewport_y);
                });
                node->control = scroll;
            }
        } else if (kind == L"EventBlocker") {
            controls::Grid blocker; blocker.Background(media::SolidColorBrush(winrt::Microsoft::UI::Colors::Transparent())); node->control = blocker;
        } else throw std::invalid_argument("Unsupported native control family: " + winrt::to_string(kind));
        xaml::Automation::AutomationProperties::SetAutomationId(node->text_content ? node->text_content.as<xaml::DependencyObject>() : node->control.as<xaml::DependencyObject>(), L"pax-node-" + winrt::to_hstring(id));
        node->host.Children().Append(node->control);
        nodes_.emplace(id, std::move(node));
    }

    void update(Node& node, json::JsonObject const& patch) {
        const double previous_width = node.width, previous_height = node.height;
        if (node.kind == L"Text") node.text_layout_dirty = true;
        if (present(patch, L"style"))
            for (auto const& field : patch.GetNamedObject(L"style"))
                if (field.Value().ValueType() != json::JsonValueType::Null) node.text_style.Insert(field.Key(), field.Value());
        if (patch.HasKey(L"parent_frame")) node.parent = present(patch, L"parent_frame") ? std::optional<uint32_t>(uint32_t(number(patch, L"parent_frame"))) : std::nullopt;
        if (present(patch, L"size_x")) {
            auto value = number(patch, L"size_x");
            if (node.kind == L"Text") {
                node.text_width_constraint = value >= 0 ? std::optional<double>(value) : std::nullopt;
                if (node.text_width_constraint) node.width = value;
            } else node.width = std::max(0.0, value);
            node.mask_dirty |= !node.holes.empty() || bool(node.clip_path) ||
                (node.kind == L"Scroller" && node.corner_radius > 0);
        }
        if (present(patch, L"size_y")) {
            auto value = number(patch, L"size_y");
            if (node.kind == L"Text") {
                node.text_height_constraint = value >= 0 ? std::optional<double>(value) : std::nullopt;
                if (node.text_height_constraint) node.height = value;
            } else node.height = std::max(0.0, value);
            node.mask_dirty |= !node.holes.empty() || bool(node.clip_path) ||
                (node.kind == L"Scroller" && node.corner_radius > 0);
        }
        if (present(patch, L"transform")) {
            auto t = patch.GetNamedArray(L"transform");
            if (t.Size() != 6) throw std::invalid_argument("Native affine transform requires six values");
            for (uint32_t index = 0; index < t.Size(); ++index) {
                auto value = t.GetAt(index);
                if (value.ValueType() != json::JsonValueType::Number || !std::isfinite(value.GetNumber()))
                    throw std::invalid_argument("Native " + winrt::to_string(node.kind) + " id=" +
                        std::to_string(node.id) + " transform[" + std::to_string(index) +
                        "] must be a finite number");
            }
            node.world = {t.GetNumberAt(0), t.GetNumberAt(1), t.GetNumberAt(2), t.GetNumberAt(3), t.GetNumberAt(4), t.GetNumberAt(5)};
            if (node.clip_path) node.mask_dirty = true;
        }
        if (present(patch, L"z_index")) controls::Canvas::SetZIndex(node.host, int(number(patch, L"z_index")));
        if (present(patch, L"opacity")) node.host.Opacity(std::clamp(number(patch, L"opacity"), 0.0, 1.0));
        if (auto editor = node.control.try_as<controls::TextBox>()) {
            if (present(patch, L"text") && editor.Text() != patch.GetNamedString(L"text")) editor.Text(patch.GetNamedString(L"text"));
            if (present(patch, L"placeholder")) editor.PlaceholderText(patch.GetNamedString(L"placeholder"));
            if (present(patch, L"is_text_area")) { editor.AcceptsReturn(boolean(patch, L"is_text_area")); editor.TextWrapping(boolean(patch, L"is_text_area") ? xaml::TextWrapping::Wrap : xaml::TextWrapping::NoWrap); }
            if (present(patch, L"style")) style(editor, patch.GetNamedObject(L"style"));
            if (present(patch, L"focus_on_mount")) {
                node.focus_pending = boolean(patch, L"focus_on_mount");
                if (node.focus_pending) initial_focus_pending_ = false;
            }
        } else if (auto button = node.control.try_as<controls::Button>()) {
            if (present(patch, L"content")) label_content(button, patch.GetNamedString(L"content"));
            label_style(button, node.text_style);
            if (present(patch, L"style")) {
                auto text_style = patch.GetNamedObject(L"style"); style(button, text_style);
                if (present(text_style, L"fill")) {
                    for (auto key : {L"ButtonForeground", L"ButtonForegroundPointerOver", L"ButtonForegroundPressed"})
                        button_brush(node, button, key, text_style.GetNamedValue(L"fill"));
                }
            }
            // Retain the brushes referenced by the native template so reactive
            // changes update normal/hover/pressed states without replacing it.
            if (present(patch, L"color")) button.Background(button_brush(node, button, L"ButtonBackground", patch.GetNamedValue(L"color")));
            if (present(patch, L"hover_color")) {
                button_brush(node, button, L"ButtonBackgroundPointerOver", patch.GetNamedValue(L"hover_color"));
                button_brush(node, button, L"ButtonBackgroundPressed", patch.GetNamedValue(L"hover_color"));
            }
            if (present(patch, L"outline_stroke_color")) {
                for (auto key : {L"ButtonBorderBrush", L"ButtonBorderBrushPointerOver", L"ButtonBorderBrushPressed"})
                    button_brush(node, button, key, patch.GetNamedValue(L"outline_stroke_color"));
                button.BorderBrush(node.button_brushes[L"ButtonBorderBrush"]);
            }
            if (present(patch, L"outline_stroke_width")) {
                auto width = number(patch, L"outline_stroke_width");
                button.BorderThickness(xaml::Thickness{width, width, width, width});
            }
        } else if (auto check = node.control.try_as<controls::CheckBox>()) {
            if (present(patch, L"checked")) check.IsChecked(boolean(patch, L"checked"));
        } else if (auto slider = node.control.try_as<controls::Slider>()) {
            node.slider_min = number(patch, L"min", node.slider_min);
            node.slider_max = number(patch, L"max", node.slider_max);
            node.slider_step = number(patch, L"step", node.slider_step);
            if (!std::isfinite(node.slider_min) || !std::isfinite(node.slider_max) ||
                !std::isfinite(node.slider_step) || node.slider_max < node.slider_min || node.slider_step <= 0 ||
                !std::isfinite((node.slider_max - node.slider_min) / node.slider_step))
                throw std::invalid_argument("Slider requires finite min <= max and positive step");
            // Set the expanding edge first so a disjoint range update does not
            // temporarily coerce the other authored endpoint.
            if (node.slider_min > slider.Maximum()) {
                slider.Maximum(node.slider_max); slider.Minimum(node.slider_min);
            } else {
                slider.Minimum(node.slider_min); slider.Maximum(node.slider_max);
            }
            slider.StepFrequency(node.slider_step); slider.SmallChange(node.slider_step);
            slider.LargeChange(node.slider_step);
            if (present(patch, L"value")) {
                auto value = number(patch, L"value");
                if (!std::isfinite(value)) throw std::invalid_argument("Slider value must be finite");
                slider.Value(std::clamp(value, node.slider_min, node.slider_max));
            }
            node.slider_value = slider.Value();
            if (present(patch, L"accent")) {
                auto brush = color(patch.GetNamedValue(L"accent"));
                state_brush(slider, L"SliderTrackValueFill", brush);
                state_brush(slider, L"SliderThumbBackground", brush);
                slider.Foreground(brush);
            }
            if (present(patch, L"background")) state_brush(slider, L"SliderTrackFill", color(patch.GetNamedValue(L"background")));
        } else if (auto dropdown = node.control.try_as<controls::ComboBox>()) {
            if (present(patch, L"selected_id")) node.selected_id = uint32_t(number(patch, L"selected_id"));
            if (present(patch, L"options")) {
                std::vector<winrt::hstring> options;
                for (auto const& option : patch.GetNamedArray(L"options"))
                    options.push_back(option.GetString());
                // Preserve item containers and focus when reactive updates
                // resend an unchanged list or change only a suffix.
                for (uint32_t i = 0; i < options.size(); ++i) {
                    if (i >= node.options.size()) {
                        controls::ComboBoxItem item;
                        node.option_labels.push_back(label_content(item, options[i]));
                        dropdown.Items().Append(item);
                    } else if (options[i] != node.options[i]) {
                        // WinUI moves the selected UIElement into the closed
                        // faceplate and temporarily clears its item's Content.
                        // Retain the label itself so reactive updates reach
                        // that same native text in either location.
                        node.option_labels[i].Text(options[i]);
                        xaml::Automation::AutomationProperties::SetName(
                            dropdown.Items().GetAt(i).as<controls::ComboBoxItem>(), options[i]);
                    }
                }
                // Return a selected UIElement from WinUI's faceplate before
                // removing its owning item. Removing that item first can make
                // ComboBox reparent the retained label through an invalid item.
                if (dropdown.SelectedIndex() >= 0 && uint32_t(dropdown.SelectedIndex()) >= options.size())
                    dropdown.SelectedIndex(-1);
                while (dropdown.Items().Size() > options.size()) dropdown.Items().RemoveAtEnd();
                node.options = std::move(options);
            }
            // An empty list or out-of-range index has no selected native item;
            // never turn the native -1 sentinel into a u32 change interrupt.
            dropdown.SelectedIndex(node.selected_id < dropdown.Items().Size() ? int(node.selected_id) : -1);
            node.option_labels.resize(node.options.size());
            if (present(patch, L"style")) style(dropdown, patch.GetNamedObject(L"style"));
            for (uint32_t i = 0; i < dropdown.Items().Size(); ++i) {
                auto item = dropdown.Items().GetAt(i).as<controls::ComboBoxItem>();
                style(item, node.text_style); style(node.option_labels[i], node.text_style);
            }
        } else if (auto radio = node.control.try_as<controls::RadioButtons>()) {
            if (present(patch, L"selected_id")) node.selected_id = uint32_t(number(patch, L"selected_id"));
            if (present(patch, L"options")) {
                std::vector<winrt::hstring> options;
                for (auto const& option : patch.GetNamedArray(L"options"))
                    options.push_back(option.GetString());
                for (uint32_t i = 0; i < options.size(); ++i) {
                    if (i >= node.options.size()) {
                        controls::RadioButton item; label_content(item, options[i]);
                        radio.Items().Append(item);
                    } else if (options[i] != node.options[i])
                        label_content(radio.Items().GetAt(i).as<controls::RadioButton>(), options[i]);
                }
                while (radio.Items().Size() > options.size()) radio.Items().RemoveAtEnd();
                node.options = std::move(options);
            }
            radio.SelectedIndex(node.selected_id < radio.Items().Size() ? int(node.selected_id) : -1);
            if (present(patch, L"style")) style(radio, patch.GetNamedObject(L"style"));
            for (auto const& value : radio.Items()) {
                auto item = value.as<controls::RadioButton>(); style(item, node.text_style);
                label_style(item, node.text_style);
                if (present(node.text_style, L"fill")) {
                    state_brush(item, L"RadioButtonForeground", color(node.text_style.GetNamedValue(L"fill")));
                }
            }
            if (present(patch, L"background")) state_brush(radio, L"RadioButtonOuterEllipseFill", color(patch.GetNamedValue(L"background")));
            if (present(patch, L"background_checked")) state_brush(radio, L"RadioButtonOuterEllipseCheckedFill", color(patch.GetNamedValue(L"background_checked")));
            if (present(patch, L"outline_color")) {
                auto brush = color(patch.GetNamedValue(L"outline_color"));
                state_brush(radio, L"RadioButtonOuterEllipseStroke", brush);
                state_brush(radio, L"RadioButtonOuterEllipseCheckedStroke", brush);
            }
            if (present(patch, L"outline_width"))
                radio.Resources().Insert(winrt::box_value(L"RadioButtonBorderThemeThickness"), winrt::box_value(number(patch, L"outline_width")));
        } else if (auto label = node.text_content) {
            if (boolean(patch, L"editable")) throw std::invalid_argument("Editable Text is not yet implemented by this Windows host");
            if (boolean(patch, L"markdown") && !present(patch, L"rich_text"))
                throw std::invalid_argument("Markdown Text requires the shared desktop rich-text normalization");
            if (present(patch, L"style_link")) for (auto const& field : patch.GetNamedObject(L"style_link"))
                if (field.Value().ValueType() != json::JsonValueType::Null) node.link_style.Insert(field.Key(), field.Value());
            if (patch.HasKey(L"rich_text")) {
                node.rich_text_events.clear(); node.rich_runs.clear(); node.rich_links.clear();
                node.rich_text = present(patch, L"rich_text") ? patch.GetNamedObject(L"rich_text") : json::JsonObject{nullptr};
            }
            if (!node.rich_text && present(patch, L"content")) {
                node.rich_text_events.clear(); label.Inlines().Clear();
                label.Text(patch.GetNamedString(L"content"));
                label.LineHeight(0); label.LineStackingStrategy(xaml::LineStackingStrategy::MaxHeight);
            }
            if (present(patch, L"selectable")) label.IsTextSelectionEnabled(boolean(patch, L"selectable"));
            if (present(patch, L"wrap")) label.TextWrapping(boolean(patch, L"wrap") ? xaml::TextWrapping::Wrap : xaml::TextWrapping::NoWrap);
            if (present(patch, L"style")) style(label, patch.GetNamedObject(L"style"), true);
            if (node.rich_text && (patch.HasKey(L"rich_text") || present(patch, L"style") || present(patch, L"style_link"))) {
                if (patch.HasKey(L"rich_text")) render_rich_text(node);
                else refresh_rich_text_style(node);
            }
            if (present(patch, L"measure_generation")) {
                node.measure_generation = uint64_t(number(patch, L"measure_generation"));
                node.latest_measure_generation = node.measure_generation;
            }
        } else if (auto scroll = node.control.try_as<controls::ScrollViewer>()) {
            auto native_extent = [&](wchar_t const* key) {
                auto value = number(patch, key);
                if (!std::isfinite(value)) throw std::invalid_argument(
                    "Scroller id=" + std::to_string(node.id) + " " + winrt::to_string(key) + " must be finite");
                // A responsive expression can become negative while its parent
                // is collapsed. Match ordinary native sizing with an empty box.
                return std::max(0.0, value);
            };
            char const* setter = "read scroll policy";
            try {
                auto previous_revision = node.snap_content->revision();
                auto previous_x_enabled = node.scroll_enabled_x, previous_y_enabled = node.scroll_enabled_y;
                auto previous_pane_width = node.authored_pane_width, previous_pane_height = node.authored_pane_height;
                if (present(patch, L"corner_radius")) {
                    auto radius = number(patch, L"corner_radius");
                    if (!std::isfinite(radius) || radius < 0) throw std::invalid_argument("Scroller radius must be finite and nonnegative");
                    if (node.corner_radius != radius) { node.corner_radius = radius; node.mask_dirty = true; }
                }
                setter = "size_inner_pane_x";
                if (present(patch, L"size_inner_pane_x")) node.authored_pane_width = native_extent(L"size_inner_pane_x");
                setter = "size_inner_pane_y";
                if (present(patch, L"size_inner_pane_y")) node.authored_pane_height = native_extent(L"size_inner_pane_y");
                setter = "scroll_enabled_x";
                if (present(patch, L"scroll_enabled_x")) node.scroll_enabled_x = boolean(patch, L"scroll_enabled_x");
                setter = "scroll_enabled_y";
                if (present(patch, L"scroll_enabled_y")) node.scroll_enabled_y = boolean(patch, L"scroll_enabled_y");
                const bool geometry_changed = !node.scroll_geometry_initialized ||
                    previous_width != node.width || previous_height != node.height ||
                    previous_pane_width != node.authored_pane_width || previous_pane_height != node.authored_pane_height ||
                    previous_x_enabled != node.scroll_enabled_x || previous_y_enabled != node.scroll_enabled_y ||
                    (present(patch, L"stroke_width") && number(patch, L"stroke_width") != scroll.BorderThickness().Left);
                if (geometry_changed) {
                    node.children.Width(node.scroll_enabled_x ? node.authored_pane_width : std::min(node.authored_pane_width, node.width));
                    node.children.Height(node.scroll_enabled_y ? node.authored_pane_height : std::min(node.authored_pane_height, node.height));
                    scroll.HorizontalScrollBarVisibility(node.scroll_enabled_x ? controls::ScrollBarVisibility::Auto : controls::ScrollBarVisibility::Hidden);
                    scroll.VerticalScrollBarVisibility(node.scroll_enabled_y ? controls::ScrollBarVisibility::Auto : controls::ScrollBarVisibility::Hidden);
                    node.scroll_geometry_initialized = true;
                }
                auto points = [](json::JsonArray const& values) {
                    std::vector<double> result; result.reserve(values.Size());
                    for (auto const& value : values) result.push_back(value.GetNumber());
                    return result;
                };
                setter = "snap extents";
                if (geometry_changed) node.snap_content->update_extents(node.width, node.height,
                    std::isnan(node.children.Width()) ? 0 : node.children.Width(),
                    std::isnan(node.children.Height()) ? 0 : node.children.Height());
                setter = "snap_points_x";
                if (present(patch, L"snap_points_x")) node.snap_content->set_horizontal_points(points(patch.GetNamedArray(L"snap_points_x")));
                setter = "snap_points_y";
                if (present(patch, L"snap_points_y")) node.snap_content->set_vertical_points(points(patch.GetNamedArray(L"snap_points_y")));
                bool snap_policy_changed = geometry_changed || previous_revision != node.snap_content->revision() ||
                    previous_x_enabled != node.scroll_enabled_x || previous_y_enabled != node.scroll_enabled_y;
                node.snap_policy_pending |= snap_policy_changed;
                if (present(patch, L"scroll_x")) node.requested_scroll_x = number(patch, L"scroll_x");
                if (present(patch, L"scroll_y")) node.requested_scroll_y = number(patch, L"scroll_y");
                if (present(patch, L"content_layer_id")) layer_count_ = std::max(layer_count_, uint32_t(number(patch, L"content_layer_id")) + 1);
            } catch (winrt::hresult_error const& error) {
                auto extent = [&](wchar_t const* key) {
                    return present(patch, key) ? winrt::to_string(patch.GetNamedValue(key).Stringify()) : "unchanged";
                };
                throw winrt::hresult_error(error.code(), winrt::to_hstring(
                    std::string("Scroller ") + setter + " inner_width=" + extent(L"size_inner_pane_x") +
                    " inner_height=" + extent(L"size_inner_pane_y") + ": " + winrt::to_string(error.message())));
            }
        }
        // Grid is a Panel, so the ordinary Control background path below does
        // not apply. A modal blocker must paint its authored native background.
        if (node.kind == L"EventBlocker" && present(patch, L"background"))
            node.control.as<controls::Grid>().Background(color(patch.GetNamedValue(L"background")));
        if (auto control = node.control.try_as<controls::Control>()) {
            if (present(patch, L"background")) control.Background(color(patch.GetNamedValue(L"background")));
            if (present(patch, L"stroke_color")) control.BorderBrush(color(patch.GetNamedValue(L"stroke_color")));
            if (present(patch, L"stroke_width")) { auto width = number(patch, L"stroke_width"); control.BorderThickness(xaml::Thickness{width, width, width, width}); }
            if (node.kind != L"Scroller" && present(patch, L"corner_radius")) {
                auto radius = std::max(0.0, number(patch, L"corner_radius"));
                control.CornerRadius(xaml::CornerRadius{radius, radius, radius, radius});
            }
        }
        if (present(patch, L"clip_path")) {
            auto value = patch.GetNamedString(L"clip_path");
            if (value.empty()) node.clip_path.reset(); else node.clip_path = masks_.parse(value);
            node.mask_dirty = true;
        }
        if (node.kind == L"Text" && present(patch, L"clip")) {
            auto clip = boolean(patch, L"clip");
            if (node.clip_content != clip) { node.clip_content = clip; node.mask_dirty = true; }
        }
        if (present(patch, L"clip_content")) {
            auto clip = boolean(patch, L"clip_content");
            if (node.clip_content != clip) { node.clip_content = clip; node.mask_dirty = true; }
        }
    }

    void measure_text(Node& node) {
        // A previous placement assigned concrete Width/Height. Clear omitted
        // axes before measuring, otherwise DesiredSize simply repeats that
        // old box (including the initial zero-sized box). An explicit zero
        // remains a real constraint, not an unconstrained measurement.
        auto automatic = std::numeric_limits<double>::quiet_NaN();
        auto unbounded = std::numeric_limits<float>::infinity();
        node.text_content.Width(node.text_width_constraint.value_or(automatic));
        node.text_content.Height(node.text_height_constraint.value_or(automatic));
        node.text_content.InvalidateMeasure();
        node.text_content.Measure({node.text_width_constraint ? float(*node.text_width_constraint) : unbounded,
            node.text_height_constraint ? float(*node.text_height_constraint) : unbounded});
        auto size = node.text_content.DesiredSize();
        auto width = node.text_width_constraint.value_or(size.Width);
        auto height = node.text_height_constraint.value_or(size.Height);
        if (!std::isfinite(width) || !std::isfinite(height) || width < 0 || height < 0)
            throw std::runtime_error("Native text measurement returned invalid bounds");
        if (node.width != width || node.height != height) node.mask_dirty |= !node.holes.empty() || bool(node.clip_path);
        node.width = width; node.height = height; node.text_layout_dirty = true;
    }

    void update_mask(Node& node) {
        char const* operation = "source visual";
        try {
            ++mask_updates_;
            using xaml::Hosting::ElementCompositionPreview;
            auto source = ElementCompositionPreview::GetElementVisual(node.control);
            auto holes = node.holes;
            // Invisible coverage must not force a capture of the native subtree.
            std::erase_if(holes, [](MaskHole const& hole) { return hole.opacity == 0; });
            std::optional<MaskHole> rounded_outside;
            if (node.kind == L"Scroller" && node.clip_content && node.corner_radius > 0 && node.width > 0 && node.height > 0) {
                // A geometric compositor clip needs no viewport-sized bitmap or
                // capture surface. XAML hit testing uses the matching outside path.
                if (!node.rounded_geometry) {
                    node.rounded_geometry = compositor_.CreateRoundedRectangleGeometry();
                    node.rounded_clip = compositor_.CreateGeometricClip(node.rounded_geometry);
                }
                auto radius = float(std::min(node.corner_radius, std::min(node.width, node.height) / 2));
                node.rounded_geometry.Size({float(node.width), float(node.height)});
                node.rounded_geometry.CornerRadius({radius, radius});
                operation = "rounded outside geometry";
                MaskHole outside;
                outside.shape = masks_.combine(masks_.rectangle(float(node.width), float(node.height)),
                    masks_.rounded_rectangle(float(node.width), float(node.height), radius),
                    D2D1_COMBINE_MODE_EXCLUDE);
                rounded_outside = std::move(outside);
            } else if (node.rounded_clip) {
                node.rounded_clip = nullptr; node.rounded_geometry = nullptr;
            }
            operation = "ancestor clip geometry";
            if (node.clip_path && node.width > 0 && node.height > 0) {
                // FramePatch.clip_path is world-space; NativeMaskUpdate coverage
                // is already local. Keep these two established contracts distinct.
                auto inverse = relative(media::Matrix{1, 0, 0, 1, 0, 0}, node.world);
                auto local_clip = masks_.transform(*node.clip_path, D2D1::Matrix3x2F(float(inverse.M11), float(inverse.M12),
                    float(inverse.M21), float(inverse.M22), float(inverse.OffsetX), float(inverse.OffsetY)));
                MaskHole outside;
                outside.shape = masks_.combine(masks_.rectangle(float(node.width), float(node.height)), local_clip, D2D1_COMBINE_MODE_EXCLUDE);
                holes.push_back(std::move(outside));
            }
            const bool opaque_only = std::all_of(holes.begin(), holes.end(),
                [](MaskHole const& hole) { return hole.opacity == 1; });
            if (opaque_only && !holes.empty() && node.width > 0 && node.height > 0) {
                // Keep native controls in their original visible subtree whenever
                // binary coverage suffices, avoiding a capture surface and mask
                // texture while preserving the original native input subtree.
                operation = "opaque visible geometry";
                auto visible = masks_.rectangle(float(node.width), float(node.height)).gpu.as<ID2D1Geometry>();
                for (auto const& hole : holes)
                    visible = masks_.combine_gpu(visible.get(), hole.shape.gpu.get(), D2D1_COMBINE_MODE_EXCLUDE);
                if (rounded_outside)
                    visible = masks_.combine_gpu(visible.get(), rounded_outside->shape.gpu.get(), D2D1_COMBINE_MODE_EXCLUDE);
                operation = "opaque clip materialization";
                auto clip = masks_.geometric_clip(visible, compositor_);
                operation = "opaque clip assignment";
                source.Clip(clip);
                source.Opacity(1); ElementCompositionPreview::SetElementChildVisual(node.host, nullptr);
                node.mask = nullptr; node.capture = nullptr; node.masked = nullptr;
            } else if (holes.empty() || node.width <= 0 || node.height <= 0) {
                // Placement owns the Grid's XAML Clip property. Put this handout
                // clip on the displayed control instead, so later layout does not
                // overwrite it through that property.
                operation = "rounded source clip assignment";
                source.Clip(node.rounded_clip);
                source.Opacity(1); ElementCompositionPreview::SetElementChildVisual(node.host, nullptr);
                node.mask = nullptr; node.capture = nullptr; node.masked = nullptr;
            } else {
                // Clip the displayed capture once; clipping its source as well
                // would attenuate the antialiased rounded edge twice.
                operation = "fractional capture";
                source.Clip(nullptr);
                node.mask = masks_.draw(float(node.width), float(node.height), dpr_, holes, node.mask);
                if (!node.capture) { ++capture_surfaces_created_; node.capture = compositor_.CreateVisualSurface(); node.capture.SourceVisual(source); }
                node.capture.SourceSize({float(node.width), float(node.height)});
                auto brush = compositor_.CreateMaskBrush(); ++mask_brushes_created_; surface_brushes_created_ += 2;
                brush.Source(compositor_.CreateSurfaceBrush(node.capture)); brush.Mask(compositor_.CreateSurfaceBrush(node.mask));
                if (!node.masked) node.masked = compositor_.CreateSpriteVisual();
                node.masked.IsHitTestVisible(false);
                node.masked.Size({float(node.width), float(node.height)}); node.masked.Brush(brush);
                node.masked.Clip(node.rounded_clip);
                source.Opacity(0); ElementCompositionPreview::SetElementChildVisual(node.host, node.masked);
            }
            if (rounded_outside) holes.push_back(std::move(*rounded_outside));
            operation = "input path retirement";
            node.mask_events.clear();
            // Geometry is a single-parent XAML object. Detached Paths can remain
            // alive through compositor/UIA references, including across a zero-
            // size collapse. Release Data before removing or replacing paths.
            if (node.mask_input)
                for (auto const& child : node.mask_input.Children())
                    if (auto path = child.try_as<xaml::Shapes::Path>()) path.Data(nullptr);
            if (holes.empty() || node.width <= 0 || node.height <= 0) {
                if (node.mask_input) { uint32_t index = 0; if (node.host.Children().IndexOf(node.mask_input, index)) node.host.Children().RemoveAt(index); }
                node.mask_input = nullptr;
                node.mask_dirty = false; return;
            }
            if (!node.mask_input) { node.mask_input = controls::Canvas(); node.host.Children().Append(node.mask_input); }
            node.mask_input.Children().Clear();
            node.mask_input.Width(node.width); node.mask_input.Height(node.height);
            for (auto const& hole : holes) {
                if (hole.opacity < 1) continue;
                operation = "input XAML path";
                xaml::Shapes::Path path; ++input_paths_created_;
                operation = "input path geometry assignment";
                path.Data(hole.shape.input);
                path.Fill(media::SolidColorBrush(winrt::Microsoft::UI::Colors::Transparent()));
                path.Stretch(media::Stretch::None); wire_pointer(path, node.mask_events, true);
                xaml::UIElement element = path;
                node.mask_input.Children().Append(element);
                // Both compositor and XAML hit testing must see the exact region;
                // a disconnected corner path's bounding box covers its interior.
                // Keep the visual clip when native input bypasses Path.Data.
                operation = "input clip materialization";
                auto input_clip = masks_.geometric_clip(hole.shape, compositor_);
                operation = "input clip assignment";
                ElementCompositionPreview::GetElementVisual(path).Clip(input_clip);
            }
            node.mask_dirty = false;
        } catch (winrt::hresult_error const& error) {
            throw winrt::hresult_error(error.code(), winrt::to_hstring(
                std::string("Native mask ") + operation + ": " + winrt::to_string(error.message())));
        }
    }

public:
    NativeScene(Interrupt interrupt, std::function<winrt::hstring(json::JsonObject const&)> local_font,
                std::function<void(winrt::hstring const&)> load_image)
        : interrupt_(std::move(interrupt)), local_font_(std::move(local_font)), load_image_(std::move(load_image)) {
        root_.Background(media::SolidColorBrush(winrt::Microsoft::UI::Colors::Transparent()));
        // WinUI exposes focus directly on UIElement. Focus the actual drawing
        // canvas; a ContentControl wrapper may delegate focus to a descendant.
        root_.IsTabStop(true);
        xaml::Automation::AutomationProperties::SetName(root_, L"Pax canvas");
        xaml::Automation::AutomationProperties::SetAutomationId(root_, L"pax-canvas");
        focus_parking_.Opacity(0); focus_parking_.Width(1); focus_parking_.Height(1);
        focus_parking_.IsTabStop(false);
        root_.Children().Append(focus_parking_); wire_pointer(root_, root_events_);
        auto live = root_events_.live;
        auto root_wheel = winrt::box_value(xaml::Input::PointerEventHandler(
            [this, live](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
                if (!*live) return;
                wheel(args);
                wheel_event_ = nullptr; wheel_args_ = nullptr; wheel_prevented_ = false;
            }));
        root_.AddHandler(xaml::UIElement::PointerWheelChangedEvent(), root_wheel, true);
        root_events_.revoke.push_back([this, root_wheel] {
            root_.RemoveHandler(xaml::UIElement::PointerWheelChangedEvent(), root_wheel);
            wheel_event_ = nullptr; wheel_args_ = nullptr;
        });
        // UISettings may notify off the UI thread. Coalesce notifications into
        // the next native batch without capturing this scene or touching XAML.
        auto metrics_dirty = text_metrics_dirty_;
        auto scale_changed = text_settings_.TextScaleFactorChanged([metrics_dirty](auto const&, auto const&) {
            metrics_dirty->store(true, std::memory_order_relaxed);
        });
        root_events_.revoke.push_back([settings = text_settings_, scale_changed] {
            settings.TextScaleFactorChanged(scale_changed);
        });
        // ScrollViewer can consume a press on its otherwise empty content.
        // Observe it only for focus ownership, without redispatching the event
        // or taking focus from editors, buttons, selectable text or scrollbars.
        auto focus_press = winrt::box_value(xaml::Input::PointerEventHandler(
            [this, live](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
                if (!*live) return;
                initial_focus_pending_ = false;
                auto id = args.Pointer().PointerId();
                canvas_presses_.erase(id);
                if (canvas_pointer_target(args)) {
                    auto point = args.GetCurrentPoint(root_);
                    if (point.Properties().IsLeftButtonPressed()) canvas_presses_[id] = point.Position();
                    focus_canvas();
                }
            }));
        root_.AddHandler(xaml::UIElement::PointerPressedEvent(), focus_press, true);
        root_events_.revoke.push_back([this, focus_press] { root_.RemoveHandler(xaml::UIElement::PointerPressedEvent(), focus_press); });
        // Clear tracking even when native controls or scene content handled the
        // release. Focus is settled before ScrollViewer's class handler runs.
        auto focus_release = winrt::box_value(xaml::Input::PointerEventHandler(
            [this, live](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
                if (*live) canvas_presses_.erase(args.Pointer().PointerId());
            }));
        root_.AddHandler(xaml::UIElement::PointerReleasedEvent(), focus_release, true);
        root_events_.revoke.push_back([this, focus_release] { root_.RemoveHandler(xaml::UIElement::PointerReleasedEvent(), focus_release); });
        auto focus_move = winrt::box_value(xaml::Input::PointerEventHandler(
            [this, live](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
                if (!*live) return;
                auto press = canvas_presses_.find(args.Pointer().PointerId());
                if (press == canvas_presses_.end()) return;
                auto position = args.GetCurrentPoint(root_).Position();
                auto dpi = UINT(std::round(96 * dpr_));
                if (std::abs(position.X - press->second.X) * dpr_ > GetSystemMetricsForDpi(SM_CXDRAG, dpi) ||
                    std::abs(position.Y - press->second.Y) * dpr_ > GetSystemMetricsForDpi(SM_CYDRAG, dpi))
                    canvas_presses_.erase(press);
            }));
        root_.AddHandler(xaml::UIElement::PointerMovedEvent(), focus_move, true);
        root_events_.revoke.push_back([this, focus_move] { root_.RemoveHandler(xaml::UIElement::PointerMovedEvent(), focus_move); });
        auto focus_cancel = winrt::box_value(xaml::Input::PointerEventHandler(
            [this, live](auto const&, xaml::Input::PointerRoutedEventArgs const& args) {
                if (*live) canvas_presses_.erase(args.Pointer().PointerId());
            }));
        root_.AddHandler(xaml::UIElement::PointerCanceledEvent(), focus_cancel, true);
        root_.AddHandler(xaml::UIElement::PointerCaptureLostEvent(), focus_cancel, true);
        root_events_.revoke.push_back([this, focus_cancel] {
            root_.RemoveHandler(xaml::UIElement::PointerCanceledEvent(), focus_cancel);
            root_.RemoveHandler(xaml::UIElement::PointerCaptureLostEvent(), focus_cancel);
        });
        auto down = root_.PreviewKeyDown([this, live](auto const&, xaml::Input::KeyRoutedEventArgs const& args) { if (*live) key(args, true); });
        root_events_.revoke.push_back([this, down] { root_.PreviewKeyDown(down); });
        auto up = root_.PreviewKeyUp([this, live](auto const&, xaml::Input::KeyRoutedEventArgs const& args) { if (*live) key(args, false); });
        root_events_.revoke.push_back([this, up] { root_.PreviewKeyUp(up); });
        auto lost = root_.LostFocus([this, live](auto const&, auto const&) { if (*live && !canvas_focused()) release_keys(); });
        root_events_.revoke.push_back([this, lost] { root_.LostFocus(lost); });
    }
    ~NativeScene() { root_events_.retire(); for (auto& [id, node] : nodes_) { node->events.retire(); node->mask_events.retire(); node->rich_text_events.retire(); } }
    controls::Canvas root() const { return root_; }
    void set_navigation(std::function<void(winrt::hstring const&)> navigate) { navigate_ = std::move(navigate); }
    std::string composition_diagnostics() const {
        uint64_t masks = 0, mask_bytes = 0, captures = 0, rounded = 0;
        for (auto const& [id, node] : nodes_) {
            if (node->mask) {
                ++masks;
                mask_bytes += uint64_t(std::ceil(node->width * dpr_)) *
                    uint64_t(std::ceil(node->height * dpr_)) * 4;
            }
            if (node->capture) ++captures;
            if (node->rounded_clip) ++rounded;
        }
        // Pixel storage excludes compositor/driver allocation overhead and the
        // separate runtime DX12 export pool. Captures are counted, not sized.
        return "native_composition nodes=" + std::to_string(nodes_.size()) +
            " rounded_clips=" + std::to_string(rounded) + " alpha_masks=" + std::to_string(masks) +
            " mask_pixel_bytes=" + std::to_string(mask_bytes) + " capture_surfaces=" + std::to_string(captures) +
            " created_transforms=" + std::to_string(placement_transforms_created_) +
            " created_rect_clips=" + std::to_string(rectangular_clips_created_) +
            " created_mask_brushes=" + std::to_string(mask_brushes_created_) +
            " created_surface_brushes=" + std::to_string(surface_brushes_created_) +
            " created_input_paths=" + std::to_string(input_paths_created_) +
            " created_capture_surfaces=" + std::to_string(capture_surfaces_created_) +
            " mask_updates=" + std::to_string(mask_updates_);
    }
    bool focus_canvas() {
        initial_focus_pending_ = false;
        bool accepted = root_.Focus(xaml::FocusState::Programmatic);
        return accepted && canvas_focused();
    }
    void initialize_canvas_focus() {
        if (!initial_focus_pending_) return;
        // The first accepted scene establishes actual bounds and attachment.
        // A prior user interaction or authored mount-focus request wins.
        initial_focus_pending_ = false;
        root_.UpdateLayout();
        focus_canvas();
    }
    void release_keys() {
        auto held = std::move(pressed_keys_); pressed_keys_.clear();
        for (auto const& [code, held_key] : held) send_key(L"KeyUp", held_key.key.name, 0, false);
    }
    uint32_t layer_count() const { return layer_count_; }
    void initialize(ID3D11Device* device) {
        compositor_ = xaml::Hosting::ElementCompositionPreview::GetElementVisual(root_).Compositor();
        masks_.initialize(compositor_, device);
    }
    controls::Canvas layer(uint32_t id, std::optional<uint32_t> owner) {
        auto parent = owner ? require(*owner).children : root_;
        if (!parent) throw std::invalid_argument("Canvas owner is not a native container");
        auto [found, inserted] = layers_.try_emplace(id);
        auto& layer = found->second;
        if (inserted) {
            layer.canvas.IsHitTestVisible(false);
            // Reparenting must not turn insertion order into vector z-order.
            controls::Canvas::SetZIndex(layer.canvas, -1000000 + int(id));
        }
        if (layer.parent != parent) {
            if (layer.parent) { uint32_t index = 0; if (layer.parent.Children().IndexOf(layer.canvas, index)) layer.parent.Children().RemoveAt(index); }
            parent.Children().Append(layer.canvas); layer.parent = parent;
        }
        return layer.canvas;
    }
    void apply_scroll_request(Node& node) {
        auto scroll = node.control.as<controls::ScrollViewer>();
        if (!node.snap_policy_pending && !node.requested_scroll_x && !node.requested_scroll_y) return;
        double x = node.requested_scroll_x.value_or(scroll.HorizontalOffset());
        double y = node.requested_scroll_y.value_or(scroll.VerticalOffset());
        bool policy_changed = node.snap_policy_pending;
        // ScrollViewer has no request correlation IDs. Keep a later authored
        // target queued until the previous native final event arrives, so an
        // older callback cannot acknowledge a newer ChangeView request.
        if (node.scroll_request_pending) return;
        if (!policy_changed && std::abs(x - scroll.HorizontalOffset()) <= 1e-5 &&
            std::abs(y - scroll.VerticalOffset()) <= 1e-5) {
            node.requested_scroll_x.reset(); node.requested_scroll_y.reset(); return;
        }
        // Native extents and snap lookup depend on arranged content. A stable
        // position echo must not force layout; only changed policy/geometry does.
        if (policy_changed) {
            scroll.UpdateLayout();
            if (!scroll.IsLoaded() || scroll.ViewportWidth() <= 0 || scroll.ViewportHeight() <= 0) return;
            // Border/padding/chrome can make the arranged viewport smaller than
            // the authored box. A disabled axis must have genuinely zero native
            // extent, including fractional authored overflow near the threshold.
            double width = node.scroll_enabled_x ? node.authored_pane_width : std::min(node.authored_pane_width, scroll.ViewportWidth());
            double height = node.scroll_enabled_y ? node.authored_pane_height : std::min(node.authored_pane_height, scroll.ViewportHeight());
            if (node.children.Width() != width || node.children.Height() != height) {
                node.children.Width(width); node.children.Height(height); scroll.UpdateLayout();
            }
            node.snap_content->update_extents(scroll.ViewportWidth(), scroll.ViewportHeight(), width, height);
            node.snap_content->apply_to(scroll, node.scroll_enabled_x, node.scroll_enabled_y);
        }
        if (!scroll.IsLoaded() || scroll.ViewportWidth() <= 0 || scroll.ViewportHeight() <= 0) return;
        node.requested_scroll_x.reset(); node.requested_scroll_y.reset(); node.snap_policy_pending = false;
        bool prior_pending = node.scroll_request_pending;
        // Retire the previous request's deferred reply before submitting a new
        // asynchronous native change. A synchronous new ViewChanged can set it
        // again; otherwise only the new native callback may acknowledge progress.
        node.scroll_reply_pending = false;
        node.scroll_request_pending = true;
        bool accepted = scroll.ChangeView(x, y, nullptr, true);
        if (!accepted) node.scroll_request_pending = prior_pending;
        // A mandatory-adjusted target may already equal the current offset.
        // A genuine accepted native request must finish before that fallback.
        if (!accepted && !node.scroll_intermediate && !node.scroll_request_pending)
            node.scroll_reply_pending = true;
    }
    void send_scroll_position(Node& node) {
        // A final callback from an earlier native request must not overwrite a
        // newer authored target that is still queued for the next batch.
        if (node.requested_scroll_x || node.requested_scroll_y) return;
        node.scroll_reply_pending = false;
        auto scroll = node.control.as<controls::ScrollViewer>();
        json::JsonObject args; args.Insert(L"id", numeric(node.id));
        args.Insert(L"scroll_x", numeric(scroll.HorizontalOffset())); args.Insert(L"scroll_y", numeric(scroll.VerticalOffset()));
        args.Insert(L"presentation_scroll_x", numeric(scroll.HorizontalOffset())); args.Insert(L"presentation_scroll_y", numeric(scroll.VerticalOffset()));
        interrupt_(event(L"ScrollerPosition", args));
    }
    void resize(double width, double height, float dpr) {
        root_.Width(width); root_.Height(height);
        if (dpr != dpr_) { dpr_ = dpr; for (auto& [id, node] : nodes_) node->mask_dirty = bool(node->mask); }
    }
    void apply(json::JsonArray const& messages) {
        applying_ = true;
        char const* operation = "read message";
        winrt::hstring message_name;
        uint32_t node_id = 0;
        try {
            for (auto const& message : messages) {
                operation = "read message"; message_name = {}; node_id = 0;
                auto object = message.GetObject();
                if (object.Size() != 1) throw std::invalid_argument("Invalid native message envelope");
                auto pair = object.First().Current(); std::wstring name(pair.Key());
                message_name = pair.Key(); operation = "apply message";
                if (pair.Value().ValueType() == json::JsonValueType::Object)
                    node_id = uint32_t(number(pair.Value().GetObject(), L"id"));
                if (name == L"SetCursor") { set_cursor(std::wstring(pair.Value().GetObject().GetNamedString(L"cursor"))); continue; }
                if (name == L"ImageLoad") { auto patch = pair.Value().GetObject(); if (present(patch, L"path")) load_image_(patch.GetNamedString(L"path")); continue; }
                if (name == L"ShrinkLayersTo") {
                    layer_count_ = uint32_t(pair.Value().GetNumber());
                    for (auto it = layers_.lower_bound(layer_count_); it != layers_.end();) {
                        uint32_t index = 0;
                        if (it->second.parent && it->second.parent.Children().IndexOf(it->second.canvas, index))
                            it->second.parent.Children().RemoveAt(index);
                        it = layers_.erase(it);
                    }
                    continue;
                }
                if (name == L"NativeCullUpdate") continue; // Advisory only: preserve native focus, editing and a11y identity.
                if (name == L"NativeMaskUpdate") {
                    auto patch = pair.Value().GetObject(); auto& node = require(uint32_t(number(patch, L"id")));
                    node.holes = masks_.parse_holes(patch.GetNamedArray(L"entries")); node.mask_dirty = true; continue;
                }
                if (name.ends_with(L"Create")) create(name.substr(0, name.size() - 6), pair.Value().GetObject());
                else if (name.ends_with(L"Update")) { auto patch = pair.Value().GetObject(); update(require(uint32_t(number(patch, L"id"))), patch); }
                else if (name.ends_with(L"Delete")) {
                    uint32_t id = uint32_t(pair.Value().GetNumber()); auto& node = require(id); detach(node); nodes_.erase(id);
                } else throw std::invalid_argument("Unsupported Windows native message: " + winrt::to_string(name));
            }
            operation = "place node"; message_name = {};
            for (auto& [id, node] : nodes_) { node_id = id; placement(*node); }
            for (auto& [id, node] : nodes_) if (node->measure_generation) {
                // Attach first so inherited XAML settings are available to
                // native measurement, then place the resolved dimensions.
                operation = "measure text"; node_id = id;
                measure_text(*node); placement(*node);
            }
            for (auto& [id, node] : nodes_) {
                operation = "update mask/focus"; node_id = id;
                if (node->mask_dirty) update_mask(*node);
                // A create/update patch can arrive before XAML has attached the
                // editor. Retry requested mount focus after placement/layout.
                if (node->focus_pending) if (auto editor = node->control.try_as<controls::TextBox>())
                    if (editor.Focus(xaml::FocusState::Programmatic)) node->focus_pending = false;
            }
            operation = "apply scroll request";
            for (auto& [id, node] : nodes_) if (node->kind == L"Scroller") { node_id = id; apply_scroll_request(*node); }
            applying_ = false;
            for (auto& [id, node] : nodes_) if (node->scroll_reply_pending) send_scroll_position(*node);
            for (auto& [id, node] : nodes_) if (node->measure_generation) {
                json::JsonObject args;
                args.Insert(L"id", numeric(id)); args.Insert(L"generation", numeric(double(*node->measure_generation)));
                args.Insert(L"width", numeric(node->width)); args.Insert(L"height", numeric(node->height));
                node->measure_generation.reset(); interrupt_(event(L"TextMeasurementResponse", args));
            }
            if (text_metrics_dirty_->exchange(false, std::memory_order_relaxed)) {
                for (auto& [id, node] : nodes_) if (node->kind == L"Text") node->text_layout_dirty = true;
                for (auto& [id, node] : nodes_) if (node->latest_measure_generation &&
                    (!node->text_width_constraint || !node->text_height_constraint)) {
                    json::JsonObject args;
                    args.Insert(L"id", numeric(id));
                    args.Insert(L"generation", numeric(double(*node->latest_measure_generation)));
                    interrupt_(event(L"TextMeasurementInvalidated", args));
                }
            }
        } catch (winrt::hresult_error const& error) {
            applying_ = false;
            throw std::runtime_error(std::string(operation) + " " + winrt::to_string(message_name) +
                " id=" + std::to_string(node_id) + ": " + winrt::to_string(error.message()) +
                " HRESULT=" + std::to_string(int32_t(error.code())));
        } catch (...) { applying_ = false; throw; }
    }
    void close() {
        release_keys();
        applying_ = true;
        root_events_.retire();
        focus_parking_.Focus(xaml::FocusState::Programmatic);
        for (auto& [id, node] : nodes_) {
            xaml::Hosting::ElementCompositionPreview::SetElementChildVisual(node->host, nullptr);
            detach(*node);
        }
        nodes_.clear(); layers_.clear(); root_.Children().Clear();
    }
};
} // namespace pax::windows
